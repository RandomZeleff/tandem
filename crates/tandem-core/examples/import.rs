//! Dev tool: list instances of other launchers, or import one.
//!
//! cargo run -p tandem-core --example import                  # what `scan` finds
//! cargo run -p tandem-core --example import -- <folder>      # what a picked folder holds
//! cargo run -p tandem-core --example import -- <n|folder> import [n]
//!
//! Uses `TANDEM_DATA_DIR` if set.

use tandem_core::launchers::{self, Found};
use tandem_core::paths::DataDir;
use tandem_core::{instance, Context};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ctx = Context::init(DataDir::from_env()?).await?;
    let started = std::time::Instant::now();
    let (found, pick): (Vec<Found>, Option<usize>) = match args.first() {
        Some(arg) if arg.parse::<usize>().is_err() => (
            launchers::inspect_folder(&ctx, std::path::Path::new(arg)).await?,
            args.get(2).and_then(|n| n.parse().ok()),
        ),
        first => (launchers::scan().await, first.and_then(|n| n.parse().ok())),
    };
    println!("{} instance(s) in {:?}", found.len(), started.elapsed());
    for (i, f) in found.iter().enumerate() {
        println!(
            "{i:>2}. [{:?}] {} — {} {} {} · {} mods · {} worlds · pack {:?} · icon {} · {}",
            f.source,
            f.name,
            f.game_version,
            f.loader,
            f.loader_version.as_deref().unwrap_or("-"),
            f.mods,
            f.worlds,
            f.pack_project_id,
            f.icon.is_some(),
            f.game_dir.display()
        );
    }
    if !args.iter().any(|a| a == "import") {
        return Ok(());
    }
    let f = found.get(pick.unwrap_or(0)).ok_or("no such instance")?;
    let started = std::time::Instant::now();
    let created = instance::create(&ctx, launchers::new_instance(&ctx, f).await?).await?;
    launchers::import(&ctx, f, &created, |p| {
        if p.download.done_files == p.download.total_files {
            println!(
                "{:?}: {} files, {} MB",
                p.stage,
                p.download.total_files,
                p.download.total_bytes / 1_000_000
            );
        }
    })
    .await?;
    let instance = ctx.db.get_instance(&created.id).await?;
    let content = ctx.db.list_content(&created.id).await?;
    println!(
        "imported as {} ({} {} {:?}, pack {:?} {:?}) in {:?}, {} files identified on Modrinth",
        instance.id,
        instance.game_version,
        instance.loader,
        instance.loader_version,
        instance.pack_project_id,
        instance.pack_version,
        started.elapsed(),
        content.len()
    );
    Ok(())
}
