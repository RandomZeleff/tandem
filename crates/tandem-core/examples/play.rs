//! Dev tool: install a version and launch it offline, without the UI.
//!
//! cargo run -p tandem-core --example play -- <[loader@]version | instance:<id>> [username] [seconds]
//!
//! `loader` is `fabric`, `quilt`, `forge` or `neoforge`.
//!
//! Uses `TANDEM_DATA_DIR` if set. When `seconds` is given, the game is killed after that delay.

use std::time::Duration;

use tandem_core::launch::{self, LaunchSpec};
use tandem_core::meta::{self, loader::Loader};
use tandem_core::paths::DataDir;
use tandem_core::stats::ProcessSampler;
use tandem_core::{crash, install, jvm, Context};
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
    // `instance:<id>` launches an existing instance (see the `mods` example).
    let existing = match version.strip_prefix("instance:") {
        Some(id) => Some(ctx.db.get_instance(id).await?),
        None => None,
    };
    let game_dir = match &existing {
        Some(instance) => ctx.data.instance_dir(&instance.id),
        None => ctx
            .data
            .instance_dir(&format!("dev-{}", version.replace('@', "-"))),
    };

    // `fabric@1.21.4` / `quilt@1.21.4`: latest loader version for that game version.
    let (loader, game_version) = match version.split_once('@') {
        Some((loader, game)) => (serde_json::from_value(loader.into())?, game),
        None => (Loader::Vanilla, version.as_str()),
    };
    let loader_version = match (&existing, loader) {
        (Some(_), _) | (None, Loader::Vanilla) => None,
        (None, loader) => {
            let versions = meta::loader::list_versions(&ctx, loader, game_version).await?;
            let latest = versions
                .iter()
                .find(|v| v.recommended)
                .or_else(|| versions.iter().find(|v| v.stable))
                .ok_or("no loader version")?;
            println!("{loader} {}", latest.version);
            Some(latest.version.clone())
        }
    };
    let target = match &existing {
        Some(instance) => install::Target::of(instance),
        None => install::Target {
            game_version,
            loader,
            loader_version: loader_version.as_deref(),
        },
    };

    let started = std::time::Instant::now();
    let last = std::sync::Mutex::new(0usize);
    let prepared = install::prepare(&ctx, target, &game_dir, |p| {
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
        memory_mb: jvm::auto_memory_mb(jvm::total_memory_mb(), jvm::count_mods(&game_dir)),
        extra_jvm_args: Vec::new(),
    });
    let mut child = launch::spawn(&command)?;
    if let Some(pid) = child.id() {
        tokio::spawn(async move {
            let mut sampler = ProcessSampler::new(pid);
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                let Some(stats) = sampler.sample() else { break };
                println!(
                    "[stats] {} MB, {:.0} % CPU",
                    stats.memory_bytes / 1_000_000,
                    stats.cpu_percent
                );
            }
        });
    }
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;
    tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("[game] {line}");
        }
    });
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("[game:err] {line}");
        }
    });

    match seconds {
        Some(s) => {
            tokio::select! {
                status = child.wait() => {
                    let status = status?;
                    println!("game exited early: {status:?}");
                    if !status.success() {
                        let report = newest_crash_report(&game_dir);
                        let analysis = crash::analyze_exit(&game_dir, report.as_deref());
                        println!("{analysis:#?}");
                    }
                }
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

fn newest_crash_report(game_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    std::fs::read_dir(game_dir.join("crash-reports"))
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}
