//! Dev tool: install a version and launch it offline, without the UI.
//!
//! cargo run -p tandem-core --example play -- <version> [username] [seconds]
//!
//! Uses `TANDEM_DATA_DIR` if set. When `seconds` is given, the game is killed after that delay.

use std::time::Duration;

use tandem_core::launch::{self, LaunchSpec, DEFAULT_MEMORY_MB};
use tandem_core::paths::DataDir;
use tandem_core::{install, Context};
use tokio::io::{AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let mut args = std::env::args().skip(1);
    let version = args
        .next()
        .ok_or("usage: play <version> [username] [seconds]")?;
    let username = args.next().unwrap_or_else(|| "TandemDev".into());
    let seconds: Option<u64> = args.next().map(|s| s.parse()).transpose()?;

    let ctx = Context::init(DataDir::from_env()?).await?;
    let account = ctx.db.add_offline_account(&username).await?;
    let game_dir = ctx.data.instance_dir(&format!("dev-{version}"));

    let started = std::time::Instant::now();
    let last = std::sync::Mutex::new(0usize);
    let prepared = install::prepare(&ctx, &version, &game_dir, |p| {
        let mut last = last.lock().unwrap();
        if p.download.done_files != *last && p.download.done_files % 250 == 0 {
            *last = p.download.done_files;
            println!(
                "{:?} {}/{} files, {}/{} MB",
                p.stage,
                p.download.done_files,
                p.download.total_files,
                p.download.done_bytes / 1_000_000,
                p.download.total_bytes / 1_000_000
            );
        }
    })
    .await?;
    println!("prepared in {:.1?}", started.elapsed());

    let command = launch::build_command(&LaunchSpec {
        prepared: &prepared,
        game_dir: &game_dir,
        assets_root: &ctx.data.assets(),
        libraries_dir: &ctx.data.libraries(),
        account: &account,
        access_token: "0",
        memory_mb: DEFAULT_MEMORY_MB,
        extra_jvm_args: Vec::new(),
    });
    let mut child = launch::spawn(&command)?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("[game] {line}");
        }
    });

    match seconds {
        Some(s) => {
            tokio::select! {
                status = child.wait() => println!("game exited early: {:?}", status?),
                _ = tokio::time::sleep(Duration::from_secs(s)) => {
                    println!("still running after {s}s, stopping");
                    child.kill().await?;
                }
            }
        }
        None => println!("game exited: {:?}", child.wait().await?),
    }
    Ok(())
}
