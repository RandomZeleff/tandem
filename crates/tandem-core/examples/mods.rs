//! Dev tool: create an instance and install Modrinth projects into it, without the UI.
//!
//! cargo run -p tandem-core --example mods -- <loader@version> <project>...
//!
//! Then launch it with `cargo run -p tandem-core --example play -- instance:<id>`.
//! Uses `TANDEM_DATA_DIR` if set.

use tandem_core::content::{self, modrinth};
use tandem_core::instance::{self, NewInstance};
use tandem_core::paths::DataDir;
use tandem_core::Context;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let mut args = std::env::args().skip(1);
    let usage = "usage: mods <loader@version> <project>...";
    let target = args.next().ok_or(usage)?;
    let (loader, game_version) = target.split_once('@').ok_or(usage)?;
    let projects: Vec<String> = args.collect();

    let ctx = Context::init(DataDir::from_env()?).await?;
    let created = instance::create(
        &ctx,
        NewInstance {
            name: format!("Dev {loader} {game_version}"),
            game_version: game_version.to_owned(),
            loader: serde_json::from_value(loader.into())?,
            loader_version: None,
        },
    )
    .await?;
    println!(
        "instance {} ({} {:?})",
        created.id, created.loader, created.loader_version
    );

    let results = modrinth::search(
        &ctx,
        &modrinth::SearchFilter {
            query: "",
            kind: content::ContentKind::Mod,
            game_version: Some(game_version),
            loader: Some(created.loader),
            offset: 0,
            limit: 5,
        },
    )
    .await?;
    println!("top mods ({} total):", results.total_hits);
    for hit in &results.hits {
        println!("  {} ({}) by {}", hit.title, hit.slug, hit.author);
    }

    for project in &projects {
        for item in content::install(&ctx, &created, project).await? {
            println!(
                "installed {} {} -> {}/{}{}",
                item.title,
                item.version_number,
                item.kind.folder(),
                item.file_name,
                if item.is_dependency {
                    " (dependency)"
                } else {
                    ""
                }
            );
        }
    }
    Ok(())
}
