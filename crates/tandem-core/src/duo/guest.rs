//! The player who joins: finds the host from the code, then runs a local stand-in server
//! that the game connects to (and sees on its LAN list), each connection being carried
//! to the host's world.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use iroh::endpoint::{ApplicationClose, Connection, ConnectionError, SendStream};
use iroh::Endpoint;
use serde::Serialize;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

use super::lan::{self, LanWorld};
use super::protocol::{self, InstanceSummary, Message, Reader, WorldInfo};
use super::{
    bind_endpoint, fail, invite, link_status, pipe, LinkStatus, CLOSE_HOST_LEFT, CLOSE_KICKED,
    RELAYED_MARK,
};
use crate::error::Result;

const LINK_EVERY: Duration = Duration::from_secs(2);

/// What the host told the guest on arrival.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Joined {
    pub host_player: String,
    pub host_instance: Option<InstanceSummary>,
    pub world: Option<WorldInfo>,
    /// Local port the guest's game connects to (`127.0.0.1:<port>`).
    pub port: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GuestEvent {
    World {
        world: Option<WorldInfo>,
    },
    Link {
        link: LinkStatus,
    },
    /// The session ended; `reason` is meant for the player.
    Closed {
        reason: String,
    },
}

pub struct Guest {
    endpoint: Endpoint,
    conn: Connection,
    /// Control stream to the host.
    control: Mutex<SendStream>,
    port: u16,
    tasks: Vec<JoinHandle<()>>,
}

/// Why the connection ended, in the player's words.
fn closed_reason(err: &ConnectionError) -> String {
    match err {
        ConnectionError::ApplicationClosed(ApplicationClose { error_code, .. })
            if *error_code == CLOSE_KICKED =>
        {
            "L'hôte t'a retiré de la partie.".into()
        }
        ConnectionError::ApplicationClosed(ApplicationClose { error_code, .. })
            if *error_code == CLOSE_HOST_LEFT =>
        {
            "L'hôte a arrêté la partie.".into()
        }
        ConnectionError::LocallyClosed => "Tu as quitté la partie.".into(),
        ConnectionError::TimedOut => "La connexion avec l'hôte a été perdue.".into(),
        _ => "La connexion avec l'hôte s'est fermée.".into(),
    }
}

impl Guest {
    /// Finds the host behind `code`, introduces the player and starts the local server.
    pub async fn join(
        http: reqwest::Client,
        code: &str,
        player: &str,
        instance: Option<InstanceSummary>,
        emit: impl Fn(GuestEvent) + Send + Sync + 'static,
    ) -> Result<(Guest, Joined)> {
        let code = invite::normalize(code)
            .ok_or_else(|| fail("ce code n'est pas valide (8 caractères, comme 7K2P-QX9M)"))?;
        let host = invite::resolve(&http, &code).await?.ok_or_else(|| {
            fail("aucune partie avec ce code. Vérifie-le, ou demande à l'hôte s'il a bien lancé l'invitation.")
        })?;
        let endpoint = bind_endpoint(false).await?;
        let conn = match tokio::time::timeout(
            Duration::from_secs(30),
            endpoint.connect(host, super::ALPN),
        )
        .await
        {
            Ok(Ok(conn)) => conn,
            Ok(Err(err)) => {
                tracing::warn!(error = %err, "could not reach the host");
                endpoint.close().await;
                return Err(fail(
                    "impossible de joindre l'hôte. Il a peut-être arrêté la partie.",
                ));
            }
            Err(_) => {
                endpoint.close().await;
                return Err(fail("l'hôte ne répond pas. Réessaie dans un instant."));
            }
        };

        let (mut send, recv) = conn.open_bi().await.map_err(fail)?;
        send.write_all(&[protocol::STREAM_CONTROL])
            .await
            .map_err(fail)?;
        protocol::write(
            &mut send,
            &Message::Hello {
                version: protocol::VERSION,
                proof: invite::proof(&code),
                player: player.to_owned(),
                instance,
            },
        )
        .await?;
        let mut reader = Reader::new(recv);
        let (host_player, host_instance, world) =
            match tokio::time::timeout(Duration::from_secs(20), reader.next()).await {
                Ok(Ok(Some(Message::Welcome {
                    player,
                    instance,
                    world,
                }))) => (player, instance, world),
                Ok(Ok(Some(Message::Refused { reason }))) => {
                    endpoint.close().await;
                    return Err(fail(reason));
                }
                _ => {
                    endpoint.close().await;
                    return Err(fail("l'hôte n'a pas répondu à la demande."));
                }
            };

        // The local stand-in server. It listens on every interface because the game
        // connects to the address the LAN announce came from (this computer's network
        // address), but only connections from this computer are accepted.
        let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
        let port = listener.local_addr()?.port();
        let announced = |world: &Option<WorldInfo>| {
            world.as_ref().map(|w| LanWorld {
                motd: format!("{host_player} · {} {RELAYED_MARK}", w.motd),
                port,
            })
        };
        let (announce_tx, announce_rx) = watch::channel(announced(&world));
        let emit = Arc::new(emit);
        let mut tasks = Vec::new();

        tasks.push(tokio::spawn(async move {
            let _ = lan::announce(announce_rx).await;
        }));
        tasks.push(tokio::spawn({
            let conn = conn.clone();
            async move {
                while let Ok((tcp, peer)) = listener.accept().await {
                    if !from_this_computer(&tcp, peer) {
                        tracing::warn!(%peer, "refused a connection from another computer");
                        continue;
                    }
                    let conn = conn.clone();
                    tokio::spawn(async move {
                        let Ok((mut send, recv)) = conn.open_bi().await else {
                            return;
                        };
                        if send.write_all(&[protocol::STREAM_GAME]).await.is_ok() {
                            pipe(send, recv, tcp).await;
                        }
                    });
                }
            }
        }));
        tasks.push(tokio::spawn({
            let (emit, host_player) = (emit.clone(), host_player.clone());
            async move {
                while let Ok(Some(message)) = reader.next().await {
                    if let Message::World { world } = message {
                        let lan = world.as_ref().map(|w| LanWorld {
                            motd: format!("{host_player} · {} {RELAYED_MARK}", w.motd),
                            port,
                        });
                        let _ = announce_tx.send(lan);
                        emit(GuestEvent::World { world });
                    }
                }
            }
        }));
        tasks.push(tokio::spawn({
            let (conn, emit) = (conn.clone(), emit.clone());
            async move {
                loop {
                    tokio::time::sleep(LINK_EVERY).await;
                    if let Some(link) = link_status(&conn) {
                        emit(GuestEvent::Link { link });
                    }
                }
            }
        }));
        tasks.push(tokio::spawn({
            let (conn, emit) = (conn.clone(), emit.clone());
            async move {
                let err = conn.closed().await;
                tracing::info!(error = %err, "duo session closed");
                emit(GuestEvent::Closed {
                    reason: closed_reason(&err),
                });
            }
        }));

        tracing::info!(host = %host_player, port, link = ?link_status(&conn), "joined");
        Ok((
            Guest {
                endpoint,
                conn,
                control: Mutex::new(send),
                port,
                tasks,
            },
            Joined {
                host_player,
                host_instance,
                world,
                port,
            },
        ))
    }

    /// The local port the game connects to.
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn link(&self) -> Option<LinkStatus> {
        link_status(&self.conn)
    }

    /// Tells the host which instance the player now plays with.
    pub async fn set_instance(&self, instance: Option<InstanceSummary>) -> Result<()> {
        protocol::write(
            &mut *self.control.lock().await,
            &Message::Instance { instance },
        )
        .await
    }

    pub async fn leave(self) {
        self.conn.close(0u32.into(), b"left");
        // Let the "closed" event out before the tasks go.
        tokio::time::sleep(Duration::from_millis(100)).await;
        for task in &self.tasks {
            task.abort();
        }
        self.endpoint.close().await;
    }
}

/// A connection from this computer: loopback, or its own network address.
fn from_this_computer(tcp: &TcpStream, peer: SocketAddr) -> bool {
    peer.ip().is_loopback() || tcp.local_addr().is_ok_and(|local| local.ip() == peer.ip())
}
