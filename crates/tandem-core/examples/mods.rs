//! Dev tool: manage Modrinth content of an instance without the UI.
//!
//! cargo run -p tandem-core --example mods -- <loader@version | instance:<id>> [project[@version_id]]... [--update]
//!
//! `loader@version` creates a new instance; `instance:<id>` reuses one. Projects are
//! installed (optionally pinned to a version id), then available updates are listed and,
//! with `--update`, applied. Launch the result with the `play` example (`instance:<id>`).
//! Uses `TANDEM_DATA_DIR` if set.

use tandem_core::content;
use tandem_core::instance::{self, NewInstance};
use tandem_core::paths::DataDir;
use tandem_core::Context;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("warn").init();
    let usage = "usage: mods <loader@version | instance:<id>> [project[@version_id]]... [--update]";
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let apply_updates = args.iter().any(|a| a == "--update");
    args.retain(|a| a != "--update");
    let target = args.first().ok_or(usage)?.clone();

    let ctx = Context::init(DataDir::from_env()?).await?;
    let instance = match target.strip_prefix("instance:") {
        Some(id) => ctx.db.get_instance(id).await?,
        None => {
            let (loader, game_version) = target.split_once('@').ok_or(usage)?;
            instance::create(
                &ctx,
                NewInstance {
                    name: format!("Dev {loader} {game_version}"),
                    game_version: game_version.to_owned(),
                    loader: serde_json::from_value(loader.into())?,
                    loader_version: None,
                },
            )
            .await?
        }
    };
    println!(
        "instance {} ({} {} {:?})",
        instance.id, instance.game_version, instance.loader, instance.loader_version
    );

    for spec in &args[1..] {
        let (project, version) = match spec.split_once('@') {
            Some((project, version)) => (project, Some(version)),
            None => (spec.as_str(), None),
        };
        for item in content::install(&ctx, &instance, project, version).await? {
            println!(
                "installed {} {}{}",
                item.title,
                item.version_number,
                if item.is_dependency {
                    " (dependency)"
                } else {
                    ""
                }
            );
        }
    }

    let updates = content::check_updates(&ctx, &instance).await?;
    println!("{} update(s) available", updates.len());
    for update in &updates {
        println!(
            "  {}: {} -> {}",
            update.title, update.current_version, update.new_version
        );
    }
    if apply_updates && !updates.is_empty() {
        for item in content::update(&ctx, &instance, None).await? {
            println!(
                "updated {} -> {} ({})",
                item.title, item.version_number, item.file_name
            );
        }
    }

    println!("content:");
    for item in ctx.db.list_content(&instance.id).await? {
        println!(
            "  [{}] {} {} {}",
            if item.enabled { "on " } else { "off" },
            item.title,
            item.version_number,
            item.file_name
        );
    }
    Ok(())
}
