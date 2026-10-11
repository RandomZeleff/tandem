//! Dev tool: run an instance's world in its own process (6e), without the UI.
//!
//! cargo run -p tandem-core --example server -- <instance> <world folder> [seconds] [port]
//!
//! Uses `TANDEM_DATA_DIR` if set. Prints the server's console, stops it after `seconds`
//! (60 by default).

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
        host: None,
    };
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
    Ok(())
}
