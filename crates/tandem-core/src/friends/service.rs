//! The friends service, running while the launcher is open: an iroh endpoint with the
//! launcher's lasting key, one connection per friend online, kept open (QUIC keep-alive),
//! over which presence, friend requests and invites travel.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex as SyncMutex};
use std::time::Duration;

use iroh::endpoint::{presets, Connection, SendStream, VarInt};
use iroh::{Endpoint, EndpointId, SecretKey};
use serde::Serialize;
use tokio::sync::{broadcast, Mutex};
use tokio::task::JoinHandle;

use super::protocol::{self, Message, Reader};
use super::{fail, own_code, publish_code, resolve_code, Friend, FriendState, Presence};
use crate::db::Database;
use crate::error::Result;

/// How often friends who are not connected are tried again.
const RETRY_EVERY: Duration = Duration::from_secs(30);
/// How often the friend code is published again (the relay keeps it two hours).
const REPUBLISH_EVERY: Duration = Duration::from_secs(30 * 60);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
/// A launcher that is not a friend may only send a request: closed after this.
const STRANGER_TIMEOUT: Duration = Duration::from_secs(60);
const CLOSE_DUPLICATE: VarInt = VarInt::from_u32(1);
const CLOSE_REMOVED: VarInt = VarInt::from_u32(2);
const CLOSE_STRANGER: VarInt = VarInt::from_u32(3);

/// Who plays on this launcher, as friends see it.
#[derive(Debug, Clone, Default)]
pub struct Me {
    pub name: String,
    pub mc_uuid: String,
    pub avatar: Option<String>,
}

/// A friend with what is known of them right now.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendView {
    #[serde(flatten)]
    pub friend: Friend,
    /// What they do, `None` when offline (or appearing so).
    pub presence: Option<Presence>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    /// The list, someone's state or presence changed: read it again.
    Changed,
    /// Someone wants to be friends.
    Request { id: String, name: String },
    /// A friend invites the player to their world.
    #[serde(rename_all = "camelCase")]
    Invite {
        id: String,
        name: String,
        code: String,
        instance: Option<String>,
        world: Option<String>,
    },
}

struct Peer {
    conn: Connection,
    send: Arc<Mutex<SendStream>>,
    /// Who opened the connection: with two at once, the one opened by the smaller id stays.
    opened_by: EndpointId,
    presence: Option<Presence>,
}

struct Inner {
    db: Database,
    endpoint: Endpoint,
    code: String,
    me: SyncMutex<Me>,
    /// What this player shares; `None` to appear offline.
    presence: SyncMutex<Option<Presence>>,
    peers: Mutex<HashMap<EndpointId, Peer>>,
    connecting: SyncMutex<HashSet<EndpointId>>,
    events: broadcast::Sender<Event>,
    tasks: SyncMutex<Vec<JoinHandle<()>>>,
}

#[derive(Clone)]
pub struct Friends {
    inner: Arc<Inner>,
}

impl Friends {
    /// Opens the endpoint, publishes the friend code and starts reaching friends.
    pub async fn start(
        db: Database,
        http: reqwest::Client,
        key: SecretKey,
        me: Me,
        presence: Option<Presence>,
    ) -> Result<Self> {
        let endpoint = Endpoint::builder(presets::N0)
            .secret_key(key)
            .alpns(vec![super::ALPN.to_vec()])
            .bind()
            .await
            .map_err(|e| {
                fail(format!(
                    "impossible d'ouvrir la connexion pair à pair ({e})"
                ))
            })?;
        let code = own_code(&db).await?;
        let (events, _) = broadcast::channel(64);
        let friends = Self {
            inner: Arc::new(Inner {
                db,
                endpoint,
                code,
                me: SyncMutex::new(me),
                presence: SyncMutex::new(presence),
                peers: Mutex::new(HashMap::new()),
                connecting: SyncMutex::new(HashSet::new()),
                events,
                tasks: SyncMutex::new(Vec::new()),
            }),
        };

        let publisher = {
            let f = friends.clone();
            tokio::spawn(async move {
                loop {
                    let wait = match publish_code(&http, &f.inner.code, f.id()).await {
                        Ok(()) => REPUBLISH_EVERY,
                        Err(err) => {
                            tracing::warn!(error = %err, "could not publish the friend code");
                            Duration::from_secs(60)
                        }
                    };
                    tokio::time::sleep(wait).await;
                }
            })
        };
        let acceptor = {
            let f = friends.clone();
            tokio::spawn(async move {
                while let Some(incoming) = f.inner.endpoint.accept().await {
                    let f = f.clone();
                    tokio::spawn(async move {
                        let Ok(conn) = incoming.await else { return };
                        let Ok(Ok((send, recv))) =
                            tokio::time::timeout(HANDSHAKE_TIMEOUT, conn.accept_bi()).await
                        else {
                            return;
                        };
                        let opened_by = conn.remote_id();
                        f.run_peer(conn, send, recv, opened_by).await;
                    });
                }
            })
        };
        let connector = {
            let f = friends.clone();
            tokio::spawn(async move {
                loop {
                    f.reach_all().await;
                    tokio::time::sleep(RETRY_EVERY).await;
                }
            })
        };
        friends
            .lock_tasks()
            .extend([publisher, acceptor, connector]);
        tracing::info!(id = %friends.id().fmt_short(), "friends service started");
        Ok(friends)
    }

    fn lock_tasks(&self) -> std::sync::MutexGuard<'_, Vec<JoinHandle<()>>> {
        self.inner.tasks.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn id(&self) -> EndpointId {
        self.inner.endpoint.id()
    }

    /// This launcher's friend code, `XXXX-XXXX`.
    pub fn code(&self) -> String {
        crate::duo::invite::display(&self.inner.code)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.inner.events.subscribe()
    }

    fn emit(&self, event: Event) {
        let _ = self.inner.events.send(event);
    }

    /// Friends, requests sent and received, with who is online.
    pub async fn list(&self) -> Result<Vec<FriendView>> {
        let friends = self.inner.db.list_friends().await?;
        let peers = self.inner.peers.lock().await;
        Ok(friends
            .into_iter()
            .map(|friend| {
                let presence = friend
                    .endpoint()
                    .filter(|_| friend.state == FriendState::Friend)
                    .and_then(|id| peers.get(&id))
                    .and_then(|p| p.presence.clone());
                FriendView { friend, presence }
            })
            .collect())
    }

    pub fn set_me(&self, me: Me) {
        *self.inner.me.lock().unwrap_or_else(|e| e.into_inner()) = me;
    }

    fn me(&self) -> Me {
        self.inner
            .me
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn presence(&self) -> Option<Presence> {
        self.inner
            .presence
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Shares what the player does with every friend online (`None`: appear offline).
    pub async fn set_presence(&self, presence: Option<Presence>) {
        {
            let mut current = self
                .inner
                .presence
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if *current == presence {
                return;
            }
            *current = presence.clone();
        }
        let message = Message::Presence { presence };
        for (id, send) in self.connected_friends().await {
            if let Err(err) = protocol::write(&mut *send.lock().await, &message).await {
                tracing::debug!(friend = %id.fmt_short(), error = %err, "presence not sent");
            }
        }
    }

    async fn connected_friends(&self) -> Vec<(EndpointId, Arc<Mutex<SendStream>>)> {
        let friends: HashSet<String> = match self.inner.db.list_friends().await {
            Ok(list) => list
                .into_iter()
                .filter(|f| f.state == FriendState::Friend)
                .map(|f| f.id)
                .collect(),
            Err(_) => return Vec::new(),
        };
        self.inner
            .peers
            .lock()
            .await
            .iter()
            .filter(|(id, _)| friends.contains(&id.to_string()))
            .map(|(id, p)| (*id, p.send.clone()))
            .collect()
    }

    async fn sender(&self, id: EndpointId) -> Option<Arc<Mutex<SendStream>>> {
        self.inner
            .peers
            .lock()
            .await
            .get(&id)
            .map(|p| p.send.clone())
    }

    async fn send(&self, id: EndpointId, message: &Message) -> bool {
        match self.sender(id).await {
            Some(send) => protocol::write(&mut *send.lock().await, message)
                .await
                .is_ok(),
            None => false,
        }
    }

    /// Sends a friend request to the launcher behind a friend code.
    pub async fn add(&self, http: &reqwest::Client, code: &str) -> Result<Friend> {
        let id = resolve_code(http, code).await?.ok_or_else(|| {
            fail("aucun joueur n'a ce code d'ami (vérifie-le, ou demande-lui de lancer Tandem)")
        })?;
        if id == self.id() {
            return Err(fail("c'est ton propre code"));
        }
        let key = id.to_string();
        match self.inner.db.friend(&key).await?.map(|f| f.state) {
            Some(FriendState::Friend) => return Err(fail("vous êtes déjà amis")),
            Some(FriendState::Incoming) => self.accept(&key).await?,
            _ => {
                self.inner
                    .db
                    .set_friend(&key, "…", FriendState::Outgoing)
                    .await?;
                if !self.send(id, &Message::Request).await {
                    self.reach(id);
                }
            }
        }
        self.emit(Event::Changed);
        self.inner
            .db
            .friend(&key)
            .await?
            .ok_or_else(|| fail("ami introuvable"))
    }

    /// Accepts a friend request.
    pub async fn accept(&self, id: &str) -> Result<()> {
        let endpoint: EndpointId = id.parse().map_err(fail)?;
        self.inner
            .db
            .set_friend(id, "…", FriendState::Friend)
            .await?;
        if self.send(endpoint, &Message::Accept).await {
            let presence = self.presence();
            self.send(endpoint, &Message::Presence { presence }).await;
        } else {
            self.reach(endpoint);
        }
        self.emit(Event::Changed);
        Ok(())
    }

    /// Removes a friend, cancels a request sent or declines one received.
    pub async fn remove(&self, id: &str) -> Result<()> {
        self.inner.db.remove_friend(id).await?;
        if let Ok(endpoint) = id.parse::<EndpointId>() {
            self.send(endpoint, &Message::Remove).await;
            if let Some(peer) = self.inner.peers.lock().await.remove(&endpoint) {
                peer.conn.close(CLOSE_REMOVED, b"removed");
            }
        }
        self.emit(Event::Changed);
        Ok(())
    }

    /// Invites a friend online to the world being hosted.
    pub async fn invite(
        &self,
        id: &str,
        code: &str,
        instance: Option<String>,
        world: Option<String>,
    ) -> Result<()> {
        let endpoint: EndpointId = id.parse().map_err(fail)?;
        let message = Message::Invite {
            code: code.to_owned(),
            instance,
            world,
        };
        if self.send(endpoint, &message).await {
            Ok(())
        } else {
            Err(fail("ton ami n'est pas en ligne"))
        }
    }

    /// Closes every connection: friends see the player go offline at once.
    pub async fn shutdown(&self) {
        for task in self.lock_tasks().drain(..) {
            task.abort();
        }
        self.inner.endpoint.close().await;
    }

    /// Tries every friend (and request sent) that is not connected.
    async fn reach_all(&self) {
        let Ok(list) = self.inner.db.list_friends().await else {
            return;
        };
        let connected: HashSet<EndpointId> =
            self.inner.peers.lock().await.keys().copied().collect();
        for friend in list {
            if friend.state == FriendState::Incoming {
                continue;
            }
            if let Some(id) = friend.endpoint().filter(|id| !connected.contains(id)) {
                self.reach(id);
            }
        }
    }

    /// Connects to a launcher in the background, unless already trying.
    fn reach(&self, id: EndpointId) {
        if !self
            .inner
            .connecting
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id)
        {
            return;
        }
        let f = self.clone();
        tokio::spawn(async move {
            let connected = async {
                let conn = tokio::time::timeout(
                    HANDSHAKE_TIMEOUT,
                    f.inner.endpoint.connect(id, super::ALPN),
                )
                .await
                .ok()?
                .ok()?;
                let (send, recv) = conn.open_bi().await.ok()?;
                Some((conn, send, recv))
            }
            .await;
            f.inner
                .connecting
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&id);
            if let Some((conn, send, recv)) = connected {
                let me = f.id();
                f.run_peer(conn, send, recv, me).await;
            }
        });
    }

    /// One connection with another launcher, from the greeting until it closes.
    async fn run_peer(
        &self,
        conn: Connection,
        mut send: SendStream,
        recv: iroh::endpoint::RecvStream,
        opened_by: EndpointId,
    ) {
        let id = conn.remote_id();
        let me = self.me();
        let hello = Message::Hello {
            version: protocol::VERSION,
            name: me.name,
            mc_uuid: me.mc_uuid,
            avatar: me.avatar,
        };
        if protocol::write(&mut send, &hello).await.is_err() {
            return;
        }
        let mut reader = Reader::new(recv);
        let Ok(Ok(Some(Message::Hello {
            version,
            name,
            mc_uuid,
            avatar,
        }))) = tokio::time::timeout(HANDSHAKE_TIMEOUT, reader.next()).await
        else {
            return;
        };
        if version != protocol::VERSION {
            tracing::info!(peer = %id.fmt_short(), version, "friend speaks another protocol version");
            return;
        }
        let key = id.to_string();
        let name: String = name.chars().take(32).collect();
        if self.inner.db.friend(&key).await.ok().flatten().is_some() {
            let _ = self
                .inner
                .db
                .set_friend_profile(&key, &name, &mc_uuid, avatar.as_deref())
                .await;
        }

        let send = Arc::new(Mutex::new(send));
        if !self.register(id, &conn, send.clone(), opened_by).await {
            return;
        }
        self.emit(Event::Changed);
        tracing::info!(peer = %name, "friend launcher connected");

        let state = self
            .inner
            .db
            .friend(&key)
            .await
            .ok()
            .flatten()
            .map(|f| f.state);
        let greeting = match state {
            Some(FriendState::Outgoing) => vec![Message::Request],
            // Accept too: a friend who missed the answer becomes one now.
            Some(FriendState::Friend) => vec![
                Message::Accept,
                Message::Presence {
                    presence: self.presence(),
                },
            ],
            _ => Vec::new(),
        };
        for message in &greeting {
            let _ = protocol::write(&mut *send.lock().await, message).await;
        }
        if state.is_none() {
            // A stranger: only a friend request is expected.
            let conn = conn.clone();
            let f = self.clone();
            let key = key.clone();
            tokio::spawn(async move {
                tokio::time::sleep(STRANGER_TIMEOUT).await;
                if f.inner.db.friend(&key).await.ok().flatten().is_none() {
                    conn.close(CLOSE_STRANGER, b"not a friend");
                }
            });
        }

        while let Ok(Some(message)) = reader.next().await {
            if let Err(err) = self
                .handle(id, &name, &mc_uuid, avatar.as_deref(), message)
                .await
            {
                tracing::debug!(peer = %name, error = %err, "friends message failed");
            }
        }

        let mut peers = self.inner.peers.lock().await;
        if peers
            .get(&id)
            .is_some_and(|p| p.conn.stable_id() == conn.stable_id())
        {
            peers.remove(&id);
            drop(peers);
            self.emit(Event::Changed);
            tracing::info!(peer = %name, "friend launcher disconnected");
        }
    }

    /// Keeps one connection per launcher; returns false when this one is dropped.
    async fn register(
        &self,
        id: EndpointId,
        conn: &Connection,
        send: Arc<Mutex<SendStream>>,
        opened_by: EndpointId,
    ) -> bool {
        let mut peers = self.inner.peers.lock().await;
        if let Some(existing) = peers.get(&id) {
            let keep_new = opened_by < existing.opened_by && existing.conn.close_reason().is_none()
                || existing.conn.close_reason().is_some();
            if !keep_new {
                conn.close(CLOSE_DUPLICATE, b"duplicate");
                return false;
            }
            existing.conn.close(CLOSE_DUPLICATE, b"duplicate");
        }
        peers.insert(
            id,
            Peer {
                conn: conn.clone(),
                send,
                opened_by,
                presence: None,
            },
        );
        true
    }

    async fn handle(
        &self,
        id: EndpointId,
        name: &str,
        mc_uuid: &str,
        avatar: Option<&str>,
        message: Message,
    ) -> Result<()> {
        let key = id.to_string();
        let db = &self.inner.db;
        let state = db.friend(&key).await?.map(|f| f.state);
        match (message, state) {
            (Message::Request, None) => {
                db.set_friend(&key, name, FriendState::Incoming).await?;
                db.set_friend_profile(&key, name, mc_uuid, avatar).await?;
                self.emit(Event::Request {
                    id: key,
                    name: name.to_owned(),
                });
                self.emit(Event::Changed);
            }
            // Both asked: friends.
            (Message::Request, Some(FriendState::Outgoing))
            | (Message::Accept, Some(FriendState::Outgoing)) => {
                db.set_friend(&key, name, FriendState::Friend).await?;
                self.send(id, &Message::Accept).await;
                let presence = self.presence();
                self.send(id, &Message::Presence { presence }).await;
                self.emit(Event::Changed);
            }
            (Message::Request, Some(FriendState::Friend)) => {
                self.send(id, &Message::Accept).await;
                let presence = self.presence();
                self.send(id, &Message::Presence { presence }).await;
            }
            // They still think we are friends, but the player removed them.
            (Message::Accept | Message::Presence { .. }, None) => {
                self.send(id, &Message::Remove).await;
            }
            (Message::Remove, Some(_)) => {
                db.remove_friend(&key).await?;
                if let Some(peer) = self.inner.peers.lock().await.remove(&id) {
                    peer.conn.close(CLOSE_REMOVED, b"removed");
                }
                self.emit(Event::Changed);
            }
            (Message::Presence { presence }, Some(FriendState::Friend)) => {
                if let Some(peer) = self.inner.peers.lock().await.get_mut(&id) {
                    peer.presence = presence;
                }
                self.emit(Event::Changed);
            }
            (
                Message::Invite {
                    code,
                    instance,
                    world,
                },
                Some(FriendState::Friend),
            ) => {
                self.emit(Event::Invite {
                    id: key,
                    name: name.to_owned(),
                    code,
                    instance,
                    world,
                });
            }
            _ => {}
        }
        Ok(())
    }
}

impl std::fmt::Debug for Friends {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Friends").field("id", &self.id()).finish()
    }
}
