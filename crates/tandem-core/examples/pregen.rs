//! Dev tool: prepare a world for playing with friends (chantier 3), without the UI.
//!
//! cargo run -p tandem-core --example pregen -- <instance> <world folder> [radius] [stop after s]
//!
//! Uses `TANDEM_DATA_DIR` if set. Prints each progress report; `stop after` pauses the
//! task early (as the Cancel button does).

use std::time::{Duration, Instant};

use tandem_core::paths::DataDir;
use tandem_core::{pregen, Context};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let mut args = std::env::args().skip(1);
    let usage = "usage: pregen <instance> <world> [radius] [stop after s]";
    let instance_id = args.next().ok_or(usage)?;
    let world = args.next().ok_or(usage)?;
    let radius: u32 = args.next().map(|s| s.parse()).transpose()?.unwrap_or(500);
    let stop_after = args
        .next()
        .map(|s| s.parse().map(Duration::from_secs))
        .transpose()?;

    let ctx = Context::init(DataDir::from_env()?).await?;
    let instance = ctx.db.get_instance(&instance_id).await?;
    let started = Instant::now();
    let mut task = pregen::start(&ctx, &instance, &world, radius).await?;
    println!(
        "generating {} chunks, server ready in {:.1?}",
        pregen::chunk_count(radius),
        started.elapsed()
    );
    let generating = Instant::now();
    loop {
        let next = match stop_after {
            Some(limit) => {
                match tokio::time::timeout_at((generating + limit).into(), task.next()).await {
                    Ok(next) => next,
                    Err(_) => {
                        println!("stopping early");
                        break;
                    }
                }
            }
            None => task.next().await,
        };
        match next {
            Some(p) => {
                println!(
                    "{:6.1}s {}/{} ({:.1}%) {:.1} cps eta {:?}",
                    generating.elapsed().as_secs_f32(),
                    p.chunks,
                    p.total,
                    p.percent,
                    p.rate,
                    p.eta_secs
                );
                if p.done {
                    break;
                }
            }
            None => {
                println!("server stopped before the end");
                return Ok(());
            }
        }
    }
    task.stop().await?;
    println!("done in {:.1?}", started.elapsed());
    Ok(())
}
