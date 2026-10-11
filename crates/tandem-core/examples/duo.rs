//! Dev tool: play together without a server.
//!
//! cargo run -p tandem-core --example duo -- selftest      # host + guest in one process
//! cargo run -p tandem-core --example duo -- host [name]   # host the world opened to LAN
//! cargo run -p tandem-core --example duo -- join <code> [name]
//! cargo run -p tandem-core --example duo -- stand-in       # fake world, logs who connects
//! cargo run -p tandem-core --example duo -- copytest <instance-id>
//! cargo run -p tandem-core --example duo -- copyinto <instance-id>   # real copy instance
//! cargo run -p tandem-core --example duo -- copydelete <copy-id>
//!
//! `copytest` hosts a copy of the instance's mods (plus one mod Modrinth does not know)
//! and syncs them into an empty folder through a real session, as a guest's copy would.
//!
//! `selftest` opens a stand-in "world" answering Minecraft's server list ping, announces
//! it like the game does, hosts it, joins it by its code and pings it through the tunnel.

use std::time::{Duration, Instant};

use tandem_core::duo::guest::{Guest, GuestEvent};
use tandem_core::duo::host::{Host, HostConfig, HostEvent};
use tandem_core::duo::lan::{self, LanWorld};
use tandem_core::duo::protocol::{InstanceSummary, ModRef, PACK_ROOTS};
use tandem_core::duo::sync;
use tandem_core::paths::DataDir;
use tandem_core::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

fn copy_dir(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    let Ok(entries) = std::fs::read_dir(from) else {
        return Ok(());
    };
    std::fs::create_dir_all(to)?;
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to.join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name()))?;
        }
    }
    Ok(())
}

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
                let handshake = read_packet(&mut tcp).await?;
                // Last byte: 1 = server list ping, 2 = joining the game.
                if handshake.last() == Some(&2) {
                    println!("[world] a game is joining (login started)");
                    return Ok(());
                }
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

            // A mod the guest will ask the host for (3 MB, not on Modrinth).
            let game_dir = tempfile::tempdir()?;
            std::fs::create_dir_all(game_dir.path().join("mods"))?;
            let jar: Vec<u8> = (0..3_000_000u32).map(|i| (i * 7 % 251) as u8).collect();
            std::fs::write(game_dir.path().join("mods").join("my-tweaks.jar"), &jar)?;
            let jar_sha1 = sha1_smol::Sha1::from(&jar).digest().to_string();
            let summary = InstanceSummary {
                name: "Test".into(),
                game_version: "1.21.1".into(),
                loader: "fabric".into(),
                loader_version: None,
                pack: None,
                mods: vec![ModRef {
                    file_name: "my-tweaks.jar".into(),
                    sha1: jar_sha1.clone(),
                }],
            };

            let started = Instant::now();
            let (world_tx, mut world_rx) = watch::channel(None);
            let host = Host::start(
                http.clone(),
                HostConfig {
                    world: None,
                    player: "Zeleff".into(),
                    instance: Some(summary),
                    game_dir: Some(game_dir.path().to_owned()),
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

            // A mod sent by the host, then one it does not have.
            let started = Instant::now();
            let mut received = Vec::new();
            guest.fetch_mod(&jar_sha1, &mut received).await?;
            let same = sha1_smol::Sha1::from(&received).digest().to_string() == jar_sha1;
            println!(
                "mod received: {} bytes in {:?}, same file: {same}",
                received.len(),
                started.elapsed()
            );
            let unknown = guest.fetch_mod(&"0".repeat(40), &mut Vec::new()).await;
            println!("unknown mod: {:?}", unknown.err().map(|e| e.to_string()));

            // The guest's announce must reach this computer's LAN list.
            let wrong = Guest::join(http.clone(), "0000-0000", "X", None, |_| {}).await;
            println!("unknown code: {:?}", wrong.err().map(|e| e.to_string()));
            host.stop().await;
            tokio::time::sleep(Duration::from_millis(500)).await;
            guest.leave().await;
        }
        Some("copytest") => {
            let id = args.get(1).ok_or("usage: duo copytest <instance-id>")?;
            let ctx = Context::init(DataDir::from_env()?).await?;
            let instance = ctx.db.get_instance(id).await?;

            // The host's mods: the instance's, and one only the host has.
            let host_dir = tempfile::tempdir()?;
            let host_mods = host_dir.path().join("mods");
            std::fs::create_dir_all(&host_mods)?;
            for entry in std::fs::read_dir(ctx.data.instance_dir(id).join("mods"))? {
                let path = entry?.path();
                if path.extension().is_some_and(|e| e == "jar") {
                    std::fs::copy(&path, host_mods.join(path.file_name().unwrap()))?;
                }
            }
            let private: Vec<u8> = (0..2_000_000u32).map(|i| (i * 13 % 241) as u8).collect();
            std::fs::write(host_mods.join("host-only-tweaks.jar"), &private)?;
            // Its pack files (configs, scripts…).
            for root in PACK_ROOTS {
                copy_dir(
                    &ctx.data.instance_dir(id).join(root),
                    &host_dir.path().join(root),
                )?;
            }
            let host_files = tandem_core::duo::pack_files(host_dir.path());
            println!("host: {} pack files", host_files.len());
            let mut host_copy = instance.clone();
            host_copy.id = "host-copy".into();
            let summary = tandem_core::duo::summary_of_dir(&host_copy, &host_mods).await?;
            println!("host: {} mods", summary.mods.len());

            // The guest: an extra mod to switch off, one of the host's mods switched off.
            let guest_dir = tempfile::tempdir()?;
            let guest_mods = guest_dir.path().join("mods");
            std::fs::create_dir_all(&guest_mods)?;
            std::fs::write(guest_mods.join("guest-only.jar"), b"not the host's")?;
            let first = &summary.mods[0];
            std::fs::copy(
                host_mods.join(&first.file_name),
                guest_mods.join(format!("{}.disabled", first.file_name)),
            )?;
            // A content script the host does not have, and a config the guest changed.
            std::fs::create_dir_all(guest_dir.path().join("kubejs/startup_scripts"))?;
            std::fs::write(
                guest_dir.path().join("kubejs/startup_scripts/extra.js"),
                b"// adds an item",
            )?;
            if let Some(config) = host_files.iter().find(|f| f.path.starts_with("config/")) {
                let path = guest_dir.path().join(&config.path);
                std::fs::create_dir_all(path.parent().unwrap())?;
                std::fs::write(path, b"changed by the guest")?;
            }

            let host = Host::start(
                http.clone(),
                HostConfig {
                    world: None,
                    player: "Zeleff".into(),
                    instance: Some(summary.clone()),
                    game_dir: Some(host_dir.path().to_owned()),
                },
                |_| {},
            )
            .await?;
            let (guest, _) = Guest::join(http.clone(), &host.code(), "Léo", None, |_| {}).await?;
            let started = Instant::now();
            let report = sync::sync_instance(&ctx, &guest, guest_dir.path(), &summary, |p| {
                if p.download.total_files > 0 && p.download.done_files == p.download.total_files {
                    println!(
                        "  {:?} {}/{}",
                        p.stage, p.download.done_files, p.download.total_files
                    );
                }
            })
            .await?;
            println!(
                "synced in {:?}: {}",
                started.elapsed(),
                serde_json::to_string(&report)?
            );
            let after = tandem_core::duo::summary_of_dir(&host_copy, &guest_mods).await?;
            let diff = tandem_core::duo::compare(&after, &summary);
            println!(
                "same mods as the host: {} (missing {}, extra {})",
                diff.matches(),
                diff.missing.len(),
                diff.extra.len()
            );
            println!(
                "guest-only.jar switched off: {}",
                guest_mods.join("guest-only.jar.disabled").exists()
            );
            let guest_files = tandem_core::duo::pack_files(guest_dir.path());
            println!(
                "same pack files as the host: {} ({} files), extra script removed: {}",
                host_files.iter().all(|f| guest_files.contains(f)),
                guest_files.len(),
                !guest_dir
                    .path()
                    .join("kubejs/startup_scripts/extra.js")
                    .exists()
            );
            host.stop().await;
            guest.leave().await;
        }
        Some("copyinto") => {
            // Like a guest's "prepare the instance": a new instance of the same version and
            // loader, filled through a real session from the hosted one. Launch it with
            // the `play` example.
            let id = args.get(1).ok_or("usage: duo copyinto <instance-id>")?;
            let ctx = Context::init(DataDir::from_env()?).await?;
            let instance = ctx.db.get_instance(id).await?;
            let summary = tandem_core::duo::summary(&ctx, &instance).await?;
            let copy = tandem_core::instance::create(
                &ctx,
                tandem_core::instance::NewInstance {
                    name: format!("{} (copie)", instance.name),
                    game_version: summary.game_version.clone(),
                    loader: instance.loader,
                    loader_version: summary.loader_version.clone(),
                },
            )
            .await?;
            let host = Host::start(
                http.clone(),
                HostConfig {
                    world: None,
                    player: "Zeleff".into(),
                    instance: Some(summary.clone()),
                    game_dir: Some(ctx.data.instance_dir(id)),
                },
                |_| {},
            )
            .await?;
            let (guest, _) = Guest::join(http.clone(), &host.code(), "Léo", None, |_| {}).await?;
            let started = Instant::now();
            let report = sync::sync_instance(
                &ctx,
                &guest,
                &ctx.data.instance_dir(&copy.id),
                &summary,
                |_| {},
            )
            .await?;
            println!(
                "copy {} filled in {:?}: {}",
                copy.id,
                started.elapsed(),
                serde_json::to_string(&report)?
            );
            host.stop().await;
            guest.leave().await;
        }
        Some("copydelete") => {
            // Removes an instance made by `copyinto` (only those).
            let id = args.get(1).ok_or("usage: duo copydelete <copy-id>")?;
            if !id.ends_with("-copie") {
                return Err("only instances made by copyinto".into());
            }
            let ctx = Context::init(DataDir::from_env()?).await?;
            tandem_core::instance::delete(&ctx, id).await?;
            println!("{id} deleted");
        }
        Some("tunnel") => {
            // A server on this computer seen through a real session, as a guest's game
            // would: prints the local port to join, then the traffic every 5 s.
            let port: u16 = args
                .get(1)
                .and_then(|p| p.parse().ok())
                .ok_or("usage: duo tunnel <server-port>")?;
            let (_announcing, announces) = watch::channel(Some(LanWorld {
                motd: "Zeleff - Load test".into(),
                port,
            }));
            tokio::spawn(lan::announce(announces));
            let (world_tx, mut world_rx) = watch::channel(None);
            let host = Host::start(
                http.clone(),
                HostConfig {
                    world: None,
                    player: "Zeleff".into(),
                    instance: None,
                    game_dir: None,
                },
                move |event| {
                    if let HostEvent::World { world } = &event {
                        let _ = world_tx.send(world.clone());
                    }
                },
            )
            .await?;
            tokio::time::timeout(Duration::from_secs(10), world_rx.wait_for(|w| w.is_some()))
                .await
                .map_err(|_| "the server was not seen as a LAN world")??;
            let (guest, _) = Guest::join(http.clone(), &host.code(), "Léo", None, |_| {}).await?;
            println!("guest port {}", guest.port());
            let started = Instant::now();
            loop {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => break,
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {}
                }
                let (sent, received) = guest.traffic();
                let link = guest.link();
                println!(
                    "traffic t={} sent={sent} received={received} ping={} direct={}",
                    started.elapsed().as_secs(),
                    link.map_or(0, |l| l.ping_ms),
                    link.is_some_and(|l| l.direct)
                );
            }
            host.stop().await;
            guest.leave().await;
        }
        Some("stand-in") => {
            let port = stand_in_world().await?;
            println!("stand-in world on 127.0.0.1:{port} (Ctrl+C to stop)");
            tokio::signal::ctrl_c().await?;
        }
        Some("host") => {
            let name = args.get(1).cloned().unwrap_or_else(|| "Hôte".into());
            let host = Host::start(
                http,
                HostConfig {
                    world: None,
                    player: name,
                    instance: None,
                    game_dir: None,
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
        _ => println!("usage: duo selftest | stand-in | host [name] | join <code> [name]"),
    }
    Ok(())
}
