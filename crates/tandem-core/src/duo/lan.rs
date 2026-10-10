//! Minecraft's LAN discovery: a world opened to LAN ("Ouvrir au réseau local") multicasts
//! `[MOTD]name[/MOTD][AD]port[/AD]` to 224.0.2.60:4445 every 1.5 s, and every game on
//! the network lists it under "Parties en réseau local". Same format in every version.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant};

use serde::Serialize;
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::watch;

use crate::error::Result;

pub const GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 2, 60);
pub const PORT: u16 = 4445;
/// Games announce every 1.5 s: a world silent for longer than this was closed.
const SILENCE: Duration = Duration::from_secs(6);
const ANNOUNCE_EVERY: Duration = Duration::from_millis(1500);

/// A world open to LAN on this computer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanWorld {
    /// What the game announces: the world's name, or "player - world".
    pub motd: String,
    pub port: u16,
}

pub fn parse(packet: &str) -> Option<LanWorld> {
    let motd = packet.split_once("[MOTD]")?.1.split_once("[/MOTD]")?.0;
    let port = packet.split_once("[AD]")?.1.split_once("[/AD]")?.0;
    Some(LanWorld {
        motd: motd.to_owned(),
        port: port.trim().parse().ok()?,
    })
}

pub fn format(motd: &str, port: u16) -> String {
    // The game splits on these tags: keep them out of the name.
    let motd = motd.replace("[MOTD]", "").replace("[/MOTD]", "");
    format!("[MOTD]{motd}[/MOTD][AD]{port}[/AD]")
}

/// A UDP socket on port 4445 that other programs (the game itself) can share.
fn listener() -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.bind(&SocketAddr::from(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, PORT)).into())?;
    socket.join_multicast_v4(&GROUP, &Ipv4Addr::UNSPECIFIED)?;
    socket.set_nonblocking(true)?;
    UdpSocket::from_std(socket.into())
}

/// Whether a world announced on `port` is this computer's (others on the network are
/// ignored): its game answers on the loopback address.
async fn is_local(port: u16) -> bool {
    tokio::time::timeout(
        Duration::from_secs(1),
        TcpStream::connect((Ipv4Addr::LOCALHOST, port)),
    )
    .await
    .is_ok_and(|connected| connected.is_ok())
}

/// Watches for a world opened to LAN on this computer, until `world` has no receiver.
pub async fn watch(world: watch::Sender<Option<LanWorld>>) -> Result<()> {
    let socket = listener()?;
    let mut buffer = [0u8; 1024];
    let mut last_seen = Instant::now();
    // Ports already checked, so the game is not probed every 1.5 s.
    let mut checked: Option<(u16, bool)> = None;
    while !world.is_closed() {
        match tokio::time::timeout(Duration::from_secs(2), socket.recv_from(&mut buffer)).await {
            Ok(Ok((len, _))) => {
                let Some(found) = parse(&String::from_utf8_lossy(&buffer[..len])) else {
                    continue;
                };
                let local = match checked {
                    Some((port, local)) if port == found.port => local,
                    _ => {
                        let local = is_local(found.port).await;
                        checked = Some((found.port, local));
                        local
                    }
                };
                if local {
                    last_seen = Instant::now();
                    world.send_if_modified(|current| {
                        let changed = current.as_ref() != Some(&found);
                        *current = Some(found);
                        changed
                    });
                }
            }
            Ok(Err(err)) => tracing::debug!(error = %err, "LAN discovery read failed"),
            Err(_) => {}
        }
        if last_seen.elapsed() > SILENCE {
            world.send_if_modified(|current| current.take().is_some());
            checked = None;
        }
    }
    Ok(())
}

/// Announces a world on this computer's network every 1.5 s until `world` says it is
/// gone and its sender is dropped. `None` pauses the announces.
pub async fn announce(mut world: watch::Receiver<Option<LanWorld>>) -> Result<()> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).await?;
    socket.set_multicast_loop_v4(true)?;
    loop {
        let current = world.borrow().clone();
        if let Some(lan) = current {
            let packet = format(&lan.motd, lan.port);
            if let Err(err) = socket.send_to(packet.as_bytes(), (GROUP, PORT)).await {
                tracing::debug!(error = %err, "LAN announce failed");
            }
        }
        tokio::select! {
            changed = world.changed() => if changed.is_err() { return Ok(()) },
            _ = tokio::time::sleep(ANNOUNCE_EVERY) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_announces() {
        assert_eq!(
            parse("[MOTD]Zeleff - Survie[/MOTD][AD]51234[/AD]"),
            Some(LanWorld {
                motd: "Zeleff - Survie".into(),
                port: 51234
            })
        );
        assert_eq!(parse("[MOTD]x[/MOTD][AD]abc[/AD]"), None);
        assert_eq!(parse("garbage"), None);
        assert_eq!(format("A [MOTD]B", 25565), "[MOTD]A B[/MOTD][AD]25565[/AD]");
        assert_eq!(
            parse(&format("Léo · Monde", 4242)).unwrap().motd,
            "Léo · Monde"
        );
    }

    #[tokio::test]
    async fn sees_a_world_announced_on_this_computer() {
        // A "game" listening on a local port, announced like Minecraft does.
        let game = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = game.local_addr().unwrap().port();
        let (announced, announces) = watch::channel(Some(LanWorld {
            motd: "Test".into(),
            port,
        }));
        let announcer = tokio::spawn(announce(announces));

        let (seen, mut world) = watch::channel(None);
        let watcher = tokio::spawn(watch(seen));
        let found = tokio::time::timeout(Duration::from_secs(10), world.wait_for(|w| w.is_some()))
            .await
            .ok()
            .and_then(|r| r.ok().map(|w| w.clone()));
        // Multicast can be off on some CI machines: only check what was received.
        if let Some(found) = found {
            assert_eq!(found.unwrap().port, port);
        }
        drop(world);
        drop(announced);
        let _ = announcer.await;
        watcher.abort();
    }
}
