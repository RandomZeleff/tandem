//! Dev tool: run an instance's world in its own process (6e), without the UI.
//!
//! cargo run -p tandem-core --example server -- <instance> <world folder> [seconds] [port]
//!
//! Uses `TANDEM_DATA_DIR` if set. Prints the server's console, stops it after `seconds`
//! (60 by default). `TANDEM_HOST=<uuid>:<name>`: that player is the host (solo player
//! handed to the server and back); its position in `level.dat` is printed before and after.

use std::time::{Duration, Instant};

use tandem_core::paths::DataDir;
use tandem_core::server::{self, ServerOptions};
use tandem_core::{jvm, Context};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let mut args = std::env::args().skip(1);
    let usage = "usage: server <instance> <world> [seconds] [port]";
    let instance_id = args.next().ok_or(usage)?;
    let world = args.next().ok_or(usage)?;
    let seconds: u64 = args.next().map(|s| s.parse()).transpose()?.unwrap_or(60);
    let port: u16 = args.next().map(|s| s.parse()).transpose()?.unwrap_or(25620);

    let ctx = Context::init(DataDir::from_env()?).await?;
    let instance = ctx.db.get_instance(&instance_id).await?;
    let started = Instant::now();
    let prepared = server::prepare(&ctx, &instance).await?;
    println!(
        "prepared in {:.1?}: {}",
        started.elapsed(),
        prepared.dir.display()
    );
    let options = ServerOptions {
        world,
        port,
        // `TANDEM_MEMORY_MB=4096`: heap size instead of the automatic one.
        memory_mb: std::env::var("TANDEM_MEMORY_MB")
            .ok()
            .and_then(|mb| mb.parse().ok())
            .unwrap_or_else(|| {
                jvm::auto_memory_mb(jvm::total_memory_mb(), jvm::count_mods(&prepared.dir))
            }),
        max_players: 20,
        online_mode: false,
        host: std::env::var("TANDEM_HOST")
            .ok()
            .and_then(|h| h.split_once(':').map(|(u, n)| (u.to_owned(), n.to_owned()))),
    };
    let level = ctx
        .data
        .instance_dir(&instance_id)
        .join("saves")
        .join(&options.world)
        .join("level.dat");
    // `TANDEM_SET_SOLO_POS=x,y,z`: moves the solo player first (test of the handover).
    if let Ok(pos) = std::env::var("TANDEM_SET_SOLO_POS") {
        set_solo_pos(&level, &pos);
    }
    println!("solo player before: {}", solo_pos(&level));
    let mut running = server::start(&ctx, &prepared, &options).await?;
    println!(
        "ready in {:.1?} on port {}",
        started.elapsed(),
        running.port
    );
    let deadline = tokio::time::Instant::now() + Duration::from_secs(seconds);
    loop {
        tokio::select! {
            line = running.lines.recv() => match line {
                Some(line) => println!("{line}"),
                None => {
                    println!("server exited: {:?}", running.wait().await);
                    return Ok(());
                }
            },
            _ = tokio::time::sleep_until(deadline) => break,
        }
    }
    running.stop().await?;
    println!("stopped after {:.1?}", started.elapsed());
    println!("solo player after: {}", solo_pos(&level));
    Ok(())
}

/// `Data.Player.Pos` of a `level.dat`, for the test.
fn solo_pos(level: &std::path::Path) -> String {
    use std::io::Read;
    let mut bytes = Vec::new();
    let Ok(file) = std::fs::File::open(level) else {
        return "no level.dat".into();
    };
    if flate2::read::GzDecoder::new(file)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return "unreadable".into();
    }
    let Ok(fastnbt::Value::Compound(root)) = fastnbt::from_bytes::<fastnbt::Value>(&bytes) else {
        return "unreadable".into();
    };
    match root.get("Data") {
        Some(fastnbt::Value::Compound(data)) => match data.get("Player") {
            Some(fastnbt::Value::Compound(player)) => format!("{:?}", player.get("Pos")),
            _ => "no Player".into(),
        },
        _ => "no Data".into(),
    }
}

fn set_solo_pos(level: &std::path::Path, pos: &str) {
    use std::io::{Read, Write};
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(std::fs::File::open(level).unwrap())
        .read_to_end(&mut bytes)
        .unwrap();
    let mut root: fastnbt::Value = fastnbt::from_bytes(&bytes).unwrap();
    let coords: Vec<fastnbt::Value> = pos
        .split(',')
        .map(|c| fastnbt::Value::Double(c.trim().parse().unwrap()))
        .collect();
    if let fastnbt::Value::Compound(root) = &mut root {
        if let Some(fastnbt::Value::Compound(data)) = root.get_mut("Data") {
            if let Some(fastnbt::Value::Compound(player)) = data.get_mut("Player") {
                player.insert("Pos".into(), fastnbt::Value::List(coords));
            }
        }
    }
    let mut gz = flate2::write::GzEncoder::new(
        std::fs::File::create(level).unwrap(),
        flate2::Compression::default(),
    );
    gz.write_all(&fastnbt::to_bytes(&root).unwrap()).unwrap();
    gz.finish().unwrap();
}
