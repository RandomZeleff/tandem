//! Dev tool: play together without a server.
//!
//! cargo run -p tandem-core --example duo -- selftest      # host + guest in one process
//! cargo run -p tandem-core --example duo -- host [name]   # host the world opened to LAN
//! cargo run -p tandem-core --example duo -- join <code> [name]
//!
//! `selftest` opens a stand-in "world" answering Minecraft's server list ping, announces
//! it like the game does, hosts it, joins it by its code and pings it through the tunnel.

use std::time::{Duration, Instant};

use tandem_core::duo::guest::{Guest, GuestEvent};
use tandem_core::duo::host::{Host, HostConfig, HostEvent};
use tandem_core::duo::lan::{self, LanWorld};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

fn varint(mut value: i32, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value = ((value as u32) >> 7) as i32;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}

async fn read_varint(stream: &mut TcpStream) -> std::io::Result<i32> {
    let mut value = 0i32;
    for shift in (0..35).step_by(7) {
        let byte = stream.read_u8().await?;
        value |= ((byte & 0x7f) as i32) << shift;
        if byte & 0x80 == 0 {
            break;
        }
    }
    Ok(value)
}

fn packet(id: i32, body: &[u8]) -> Vec<u8> {
    let mut inner = Vec::new();
    varint(id, &mut inner);
    inner.extend_from_slice(body);
    let mut out = Vec::new();
    varint(inner.len() as i32, &mut out);
    out.extend(inner);
    out
}

fn string(s: &str, out: &mut Vec<u8>) {
    varint(s.len() as i32, out);
    out.extend_from_slice(s.as_bytes());
}

async fn read_packet(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let len = read_varint(stream).await? as usize;
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await?;
    Ok(data)
}

/// Answers the server list ping like a 1.21 world would.
async fn stand_in_world() -> std::io::Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    tokio::spawn(async move {
        while let Ok((mut tcp, _)) = listener.accept().await {
            tokio::spawn(async move {
                let _handshake = read_packet(&mut tcp).await?;
                let _request = read_packet(&mut tcp).await?;
                let mut body = Vec::new();
                string(
                    r#"{"version":{"name":"1.21.4","protocol":769},"players":{"max":8,"online":1},"description":{"text":"Monde de test"}}"#,
                    &mut body,
                );
                tcp.write_all(&packet(0, &body)).await?;
                // Ping → pong.
                let ping = read_packet(&mut tcp).await?;
                tcp.write_all(&packet(1, &ping[1..])).await?;
                Ok::<_, std::io::Error>(())
            });
        }
    });
    Ok(port)
}

/// Server list ping, as the game's multiplayer screen does it. Returns the status JSON
/// and the ping time.
async fn ping(port: u16) -> std::io::Result<(String, Duration)> {
    let mut tcp = TcpStream::connect(("127.0.0.1", port)).await?;
    let mut handshake = Vec::new();
    varint(769, &mut handshake);
    string("127.0.0.1", &mut handshake);
    handshake.extend_from_slice(&port.to_be_bytes());
    varint(1, &mut handshake);
    tcp.write_all(&packet(0, &handshake)).await?;
    tcp.write_all(&packet(0, &[])).await?;
    let status = read_packet(&mut tcp).await?;
    let json = String::from_utf8_lossy(&status[2..]).into_owned();
    let started = Instant::now();
    tcp.write_all(&packet(1, &42i64.to_be_bytes())).await?;
    read_packet(&mut tcp).await?;
    Ok((json, started.elapsed()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info,iroh=warn,noq=warn")
        .init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let http = reqwest::Client::new();
    match args.first().map(String::as_str) {
        Some("selftest") => {
            let port = stand_in_world().await?;
            // Announced exactly like an "Open to LAN" world.
            let (_announcing, announces) = watch::channel(Some(LanWorld {
                motd: "Zeleff - Monde de test".into(),
                port,
            }));
            tokio::spawn(lan::announce(announces));

            let started = Instant::now();
            let (world_tx, mut world_rx) = watch::channel(None);
            let host = Host::start(
                http.clone(),
                HostConfig {
                    player: "Zeleff".into(),
                    instance: None,
                },
                move |event| {
                    if let HostEvent::World { world } = &event {
                        let _ = world_tx.send(world.clone());
                    }
                    println!(
                        "[host] {}",
                        serde_json::to_string(&event).unwrap_or_default()
                    );
                },
            )
            .await?;
            println!(
                "hosting with code {} ({:?})",
                host.code(),
                started.elapsed()
            );
            tokio::time::timeout(Duration::from_secs(10), world_rx.wait_for(|w| w.is_some()))
                .await
                .map_err(|_| "the LAN world was not detected")??;
            println!("world detected: {:?}", host.world());

            let started = Instant::now();
            let (guest, joined) = Guest::join(
                http.clone(),
                &host.code().to_lowercase(),
                "Léo",
                None,
                |event| {
                    println!(
                        "[guest] {}",
                        serde_json::to_string(&event).unwrap_or_default()
                    );
                },
            )
            .await?;
            println!(
                "joined in {:?}: {}",
                started.elapsed(),
                serde_json::to_string(&joined)?
            );
            let (status, rtt) = ping(guest.port()).await?;
            println!("ping through the tunnel: {rtt:?}\nstatus: {status}");
            println!("link: {:?}", guest.link());
            let (_, rtt) = ping(guest.port()).await?;
            println!("second ping: {rtt:?}");

            // The guest's announce must reach this computer's LAN list.
            let wrong = Guest::join(http.clone(), "0000-0000", "X", None, |_| {}).await;
            println!("unknown code: {:?}", wrong.err().map(|e| e.to_string()));
            host.stop().await;
            tokio::time::sleep(Duration::from_millis(500)).await;
            guest.leave().await;
        }
        Some("host") => {
            let name = args.get(1).cloned().unwrap_or_else(|| "Hôte".into());
            let host = Host::start(
                http,
                HostConfig {
                    player: name,
                    instance: None,
                },
                |event| {
                    println!(
                        "[host] {}",
                        serde_json::to_string(&event).unwrap_or_default()
                    );
                },
            )
            .await?;
            println!("code: {}  (Ctrl+C to stop)", host.code());
            tokio::signal::ctrl_c().await?;
            host.stop().await;
        }
        Some("join") => {
            let code = args.get(1).ok_or("usage: duo join <code> [name]")?;
            let name = args.get(2).cloned().unwrap_or_else(|| "Invité".into());
            let (guest, joined) = Guest::join(http, code, &name, None, |event| {
                if !matches!(event, GuestEvent::Link { .. }) {
                    println!(
                        "[guest] {}",
                        serde_json::to_string(&event).unwrap_or_default()
                    );
                }
            })
            .await?;
            println!(
                "joined {}: connect the game to 127.0.0.1:{}",
                joined.host_player, joined.port
            );
            tokio::signal::ctrl_c().await?;
            guest.leave().await;
        }
        _ => println!("usage: duo selftest | host [name] | join <code> [name]"),
    }
    Ok(())
}
