//! The player who opens their world: publishes the invite code, welcomes guests and
//! relays each of their game connections to the world opened to LAN.

use std::collections::{HashMap, HashSet};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iroh::endpoint::{Connection, Incoming, RecvStream, SendStream, VarInt};
use iroh::Endpoint;
use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
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
    /// Folder of `instance`: guests may ask for its mods (those listed in `instance`)
    /// and its pack files (see [`protocol::PACK_ROOTS`]).
    pub game_dir: Option<PathBuf>,
    /// A world Tandem runs itself (6e): served at once, no LAN discovery.
    pub world: Option<LanWorld>,
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
    /// A guest now plays with another instance.
    GuestInstance {
        id: String,
        instance: Option<InstanceSummary>,
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
    /// Pack files last listed to a guest: the only ones it may then ask for.
    pack_files: Mutex<HashSet<String>>,
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

        let fixed_world = config.world.clone();
        let (world_tx, world_rx) = watch::channel(fixed_world.clone());
        let shared = Arc::new(Shared {
            code: code.clone(),
            config,
            world: world_rx,
            guests: Mutex::new(HashMap::new()),
            pack_files: Mutex::new(HashSet::new()),
            emit: Arc::new(emit),
        });
        let mut tasks = Vec::new();

        if fixed_world.is_none() {
            tasks.push(tokio::spawn(async move {
                if let Err(err) = lan::watch(world_tx).await {
                    tracing::warn!(error = %err, "LAN discovery stopped");
                }
            }));
        }
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

    // The guest's instance, picked after the welcome or changed later.
    let messages = tokio::spawn({
        let (shared, id) = (shared.clone(), id.clone());
        async move {
            while let Ok(Some(message)) = reader.next().await {
                if let Message::Instance { instance } = message {
                    (shared.emit)(HostEvent::GuestInstance {
                        id: id.clone(),
                        instance,
                    });
                }
            }
        }
    });

    // Each game connection of the guest, and each mod it asks for, is a new stream.
    while let Ok((send, mut recv)) = conn.accept_bi().await {
        let shared = shared.clone();
        tokio::spawn(async move {
            let mut tag = [0u8; 1];
            if recv.read_exact(&mut tag).await.is_err() {
                return;
            }
            match tag[0] {
                protocol::STREAM_GAME => relay_game(send, recv, &shared).await,
                protocol::STREAM_MOD => send_mod(send, recv, &shared).await,
                protocol::STREAM_PACK_LIST => send_pack_list(send, &shared).await,
                protocol::STREAM_PACK_FILE => send_pack_file(send, recv, &shared).await,
                _ => {}
            }
        });
    }

    updates.abort();
    links.abort();
    messages.abort();
    lock(&shared.guests).remove(&id);
    tracing::info!(guest = %player, "guest left");
    (shared.emit)(HostEvent::GuestLeft { id });
}

/// Carries one game connection of a guest to the world opened to LAN.
async fn relay_game(send: SendStream, mut recv: RecvStream, shared: &Shared) {
    let Some(world) = shared.world.borrow().clone() else {
        // No world open: the guest's game shows a connection error.
        let _ = recv.stop(VarInt::from_u32(0));
        return;
    };
    match TcpStream::connect((Ipv4Addr::LOCALHOST, world.port)).await {
        Ok(tcp) => pipe(send, recv, tcp).await,
        Err(err) => tracing::warn!(error = %err, "LAN world unreachable"),
    }
}

/// Sends a guest one of the mods of this session's instance (only those: the request is
/// a SHA-1 looked up in the summary given to guests).
async fn send_mod(send: SendStream, mut recv: RecvStream, shared: &Shared) {
    let mut sha1 = [0u8; 40];
    if recv.read_exact(&mut sha1).await.is_err() {
        return;
    }
    let sha1 = String::from_utf8_lossy(&sha1);
    let path = shared
        .config
        .instance
        .as_ref()
        .and_then(|i| i.mods.iter().find(|m| m.sha1 == sha1))
        .zip(shared.config.game_dir.as_ref())
        .map(|(m, dir)| dir.join("mods").join(&m.file_name));
    send_file(send, path).await;
}

/// Lists the instance's pack files (configs, scripts…) to a guest.
async fn send_pack_list(mut send: SendStream, shared: &Shared) {
    let files = match shared.config.game_dir.clone() {
        Some(dir) => tokio::task::spawn_blocking(move || super::pack_files(&dir))
            .await
            .unwrap_or_default(),
        None => Vec::new(),
    };
    *lock(&shared.pack_files) = files.iter().map(|f| f.path.clone()).collect();
    tracing::info!(files = files.len(), "pack files listed to a guest");
    let mut line = serde_json::to_vec(&files).unwrap_or_else(|_| b"[]".to_vec());
    line.push(b'\n');
    let _ = send.write_all(&line).await;
    let _ = send.finish();
    let _ = send.stopped().await;
}

/// Sends a guest one of the pack files listed to it.
async fn send_pack_file(send: SendStream, recv: RecvStream, shared: &Shared) {
    let mut lines = BufReader::new(recv.take(4096)).lines();
    let Ok(Some(path)) = lines.next_line().await else {
        return;
    };
    let listed = protocol::pack_path_ok(&path) && lock(&shared.pack_files).contains(&path);
    let file = shared
        .config
        .game_dir
        .as_ref()
        .filter(|_| listed)
        .map(|dir| dir.join(&path));
    send_file(send, file).await;
}

/// Answers a file request: found and its bytes, or missing.
async fn send_file(mut send: SendStream, path: Option<PathBuf>) {
    let file = match &path {
        Some(path) => tokio::fs::File::open(path).await.ok(),
        None => None,
    };
    let Some(mut file) = file else {
        let _ = send.write_all(&[protocol::MOD_MISSING]).await;
        let _ = send.finish();
        return;
    };
    if send.write_all(&[protocol::MOD_FOUND]).await.is_ok() {
        if let Err(err) = tokio::io::copy(&mut file, &mut send).await {
            tracing::warn!(path = ?path, error = %err, "file not sent to a guest");
        }
    }
    let _ = send.finish();
    // Let the last bytes leave before the stream is dropped.
    let _ = send.stopped().await;
}
