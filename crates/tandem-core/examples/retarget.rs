//! Dev tool: show (or apply) what moving an instance to another version does.
//!
//! cargo run -p tandem-core --example retarget -- <instance-id> <game-version> [loader] [apply]
//!
//! Uses `TANDEM_DATA_DIR` if set.

use tandem_core::content::retarget::{self, Target};
use tandem_core::meta::loader::Loader;
use tandem_core::paths::DataDir;
use tandem_core::Context;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("warn").init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let ctx = Context::init(DataDir::from_env()?).await?;
    let instance = ctx
        .db
        .get_instance(args.first().ok_or("instance id")?)
        .await?;
    let loader: Loader = match args.get(2).map(String::as_str) {
        Some(name) if name != "apply" => serde_json::from_value(serde_json::json!(name))?,
        _ => instance.loader,
    };
    let target = Target {
        game_version: args.get(1).ok_or("game version")?.clone(),
        loader,
        loader_version: None,
    };
    let started = std::time::Instant::now();
    let plan = if args.iter().any(|a| a == "apply") {
        retarget::apply(&ctx, &instance, &target).await?
    } else {
        retarget::plan(&ctx, &instance, &target).await?
    };
    println!(
        "loader {:?}, kept {}, updated {}, unavailable {}, unknown {} ({:?})",
        plan.loader_version,
        plan.kept,
        plan.updated.len(),
        plan.unavailable.len(),
        plan.unknown.len(),
        started.elapsed()
    );
    for change in plan.updated.iter().take(5) {
        println!("  update {} {} -> {}", change.title, change.from, change.to);
    }
    for title in plan.unavailable.iter().take(8) {
        println!("  unavailable {title}");
    }
    Ok(())
}
