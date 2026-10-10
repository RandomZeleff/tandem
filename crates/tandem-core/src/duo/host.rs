//! The player who opens their world: publishes the invite code, welcomes guests and
//! relays each of their game connections to the world opened to LAN.

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iroh::endpoint::{Connection, Incoming, VarInt};
use iroh::Endpoint;
use serde::Serialize;
use tokio::net::TcpStream;
use tokio::sync::watch;
use tokio::task::JoinHandle;

use super::lan::{self, LanWorld};
use super::protocol::{self, InstanceSummary, Message, Reader, WorldInfo};
use super::{
    bind_endpoint, fail, invite, link_status, pipe, LinkStatus, CLOSE_HOST_LEFT, CLOSE_KICKED,
    CLOSE_REFUSED,
};
use crate::error::Result;

/// The invite record is published again this often (the relay keeps it for a while).
const REPUBLISH_EVERY: Duration = Duration::from_secs(4 * 60);
const LINK_EVERY: Duration = Duration::from_secs(2);

pub struct HostConfig {
    pub player: String,
    pub instance: Option<InstanceSummary>,
}

/// A guest, as the host's interface shows it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuestInfo {
    /// Short form of the guest's endpoint, stable for the session.
    pub id: String,
    pub player: String,
    pub instance: Option<InstanceSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum HostEvent {
    /// The world opened to LAN on this computer (none until the player opens one).
    World {
        world: Option<LanWorld>,
    },
    GuestJoined {
        guest: GuestInfo,
    },
    GuestLeft {
        id: String,
    },
    Link {
        id: String,
        link: LinkStatus,
    },
}

type Emit = Arc<dyn Fn(HostEvent) + Send + Sync>;

struct Shared {
    code: String,
    config: HostConfig,
    world: watch::Receiver<Option<LanWorld>>,
    guests: Mutex<HashMap<String, Connection>>,
    emit: Emit,
}

/// A hosting session. Dropping it without [`Host::stop`] leaves guests to time out.
pub struct Host {
    code: String,
    endpoint: Endpoint,
    shared: Arc<Shared>,
    tasks: Vec<JoinHandle<()>>,
}

fn world_info(world: &Option<LanWorld>) -> Option<WorldInfo> {
    world.as_ref().map(|w| WorldInfo {
        motd: w.motd.clone(),
    })
}

impl Host {
    /// Opens the endpoint and publishes a new invite code. Fails when the code cannot be
    /// published (no network): nobody could join.
    pub async fn start(
        http: reqwest::Client,
        config: HostConfig,
        emit: impl Fn(HostEvent) + Send + Sync + 'static,
    ) -> Result<Host> {
        let endpoint = bind_endpoint(true).await?;
        if tokio::time::timeout(Duration::from_secs(15), endpoint.online())
            .await
            .is_err()
        {
            endpoint.close().await;
            return Err(fail(
                "pas de connexion à Internet : impossible de créer une invitation",
            ));
        }
        let code = invite::normalize(&invite::new_code()).unwrap_or_default();
        if let Err(err) = invite::publish(&http, &code, endpoint.id()).await {
            endpoint.close().await;
            return Err(fail(format!("impossible de publier l'invitation ({err})")));
        }

        let (world_tx, world_rx) = watch::channel(None);
        let shared = Arc::new(Shared {
            code: code.clone(),
            config,
            world: world_rx,
            guests: Mutex::new(HashMap::new()),
            emit: Arc::new(emit),
        });
        let mut tasks = Vec::new();

        tasks.push(tokio::spawn(async move {
            if let Err(err) = lan::watch(world_tx).await {
                tracing::warn!(error = %err, "LAN discovery stopped");
            }
        }));
        tasks.push(tokio::spawn({
            let shared = shared.clone();
            let mut world = shared.world.clone();
            async move {
                while world.changed().await.is_ok() {
                    let current = world.borrow_and_update().clone();
                    tracing::info!(world = ?current, "LAN world changed");
                    (shared.emit)(HostEvent::World { world: current });
                }
            }
        }));
        tasks.push(tokio::spawn({
            let (code, id) = (code.clone(), endpoint.id());
            async move {
                loop {
                    tokio::time::sleep(REPUBLISH_EVERY).await;
                    if let Err(err) = invite::publish(&http, &code, id).await {
                        tracing::warn!(error = %err, "invite not republished");
                    }
                }
            }
        }));
        tasks.push(tokio::spawn({
            let (endpoint, shared) = (endpoint.clone(), shared.clone());
            async move {
                while let Some(incoming) = endpoint.accept().await {
                    tokio::spawn(welcome(incoming, shared.clone()));
                }
            }
        }));
        tracing::info!(code = %invite::display(&code), "hosting");
        Ok(Host {
            code,
            endpoint,
            shared,
            tasks,
        })
    }

    /// The code to give, as `XXXX-XXXX`.
    pub fn code(&self) -> String {
        invite::display(&self.code)
    }

    pub fn world(&self) -> Option<LanWorld> {
        self.shared.world.borrow().clone()
    }

    /// Disconnects a guest (they can come back with the code).
    pub fn kick(&self, id: &str) {
        if let Some(conn) = lock(&self.shared.guests).remove(id) {
            conn.close(CLOSE_KICKED, b"kicked");
        }
    }

    /// Ends the session for everyone.
    pub async fn stop(self) {
        for task in &self.tasks {
            task.abort();
        }
        for (_, conn) in lock(&self.shared.guests).drain() {
            conn.close(CLOSE_HOST_LEFT, b"host left");
        }
        self.endpoint.close().await;
        tracing::info!("hosting stopped");
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn short_id(conn: &Connection) -> String {
    conn.remote_id().fmt_short().to_string()
}

/// Checks a newcomer, then serves them until they leave.
async fn welcome(incoming: Incoming, shared: Arc<Shared>) {
    let Ok(Ok(conn)) = tokio::time::timeout(Duration::from_secs(30), incoming).await else {
        return;
    };
    let Ok(Ok((mut send, mut recv))) =
        tokio::time::timeout(Duration::from_secs(15), conn.accept_bi()).await
    else {
        return;
    };
    let mut tag = [0u8; 1];
    if recv.read_exact(&mut tag).await.is_err() || tag[0] != protocol::STREAM_CONTROL {
        conn.close(CLOSE_REFUSED, b"protocol");
        return;
    }
    let mut reader = Reader::new(recv);
    let hello = tokio::time::timeout(Duration::from_secs(15), reader.next()).await;
    let Ok(Ok(Some(Message::Hello {
        version,
        proof,
        player,
        instance,
    }))) = hello
    else {
        conn.close(CLOSE_REFUSED, b"protocol");
        return;
    };
    let refusal = if version != protocol::VERSION {
        Some("Ta version de Tandem et celle de l'hôte ne se comprennent pas : mettez-les à jour.")
    } else if proof != invite::proof(&shared.code) {
        Some("Code d'invitation incorrect.")
    } else {
        None
    };
    if let Some(reason) = refusal {
        let _ = protocol::write(
            &mut send,
            &Message::Refused {
                reason: reason.into(),
            },
        )
        .await;
        let _ = send.finish();
        tokio::time::sleep(Duration::from_millis(500)).await;
        conn.close(CLOSE_REFUSED, b"refused");
        return;
    }
    let welcome = Message::Welcome {
        player: shared.config.player.clone(),
        instance: shared.config.instance.clone(),
        world: world_info(&shared.world.borrow()),
    };
    if protocol::write(&mut send, &welcome).await.is_err() {
        return;
    }

    let id = short_id(&conn);
    tracing::info!(guest = %player, id = %id, link = ?link_status(&conn), "guest joined");
    lock(&shared.guests).insert(id.clone(), conn.clone());
    (shared.emit)(HostEvent::GuestJoined {
        guest: GuestInfo {
            id: id.clone(),
            player: player.clone(),
            instance,
        },
    });

    // World opened or closed: tell the guest.
    let updates = tokio::spawn({
        let mut world = shared.world.clone();
        async move {
            while world.changed().await.is_ok() {
                let info = world_info(&world.borrow_and_update());
                if protocol::write(&mut send, &Message::World { world: info })
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });
    let links = tokio::spawn({
        let (conn, shared, id) = (conn.clone(), shared.clone(), id.clone());
        async move {
            loop {
                tokio::time::sleep(LINK_EVERY).await;
                if let Some(link) = link_status(&conn) {
                    (shared.emit)(HostEvent::Link {
                        id: id.clone(),
                        link,
                    });
                }
            }
        }
    });

    // Each game connection of the guest arrives as a new stream.
    while let Ok((send, mut recv)) = conn.accept_bi().await {
        let world = shared.world.borrow().clone();
        tokio::spawn(async move {
            let mut tag = [0u8; 1];
            if recv.read_exact(&mut tag).await.is_err() || tag[0] != protocol::STREAM_GAME {
                return;
            }
            let Some(world) = world else {
                // No world open: the guest's game shows a connection error.
                let _ = recv.stop(VarInt::from_u32(0));
                return;
            };
            match TcpStream::connect((Ipv4Addr::LOCALHOST, world.port)).await {
                Ok(tcp) => pipe(send, recv, tcp).await,
                Err(err) => tracing::warn!(error = %err, "LAN world unreachable"),
            }
        });
    }

    updates.abort();
    links.abort();
    lock(&shared.guests).remove(&id);
    tracing::info!(guest = %player, "guest left");
    (shared.emit)(HostEvent::GuestLeft { id });
}
