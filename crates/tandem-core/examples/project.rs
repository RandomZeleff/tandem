//! Dev tool: read a Modrinth project page the way the launcher does, and time it.
//!
//! cargo run -p tandem-core --example project -- <project-slug | id>
//!
//! Uses `TANDEM_DATA_DIR` if set (nothing is written there).

use std::time::Instant;

use tandem_core::content::project::{self, ProjectCache};
use tandem_core::paths::DataDir;
use tandem_core::Context;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let id = std::env::args()
        .nth(1)
        .ok_or("usage: project <project-slug | id>")?;
    let ctx = Context::init(DataDir::from_env()?).await?;
    let cache = ProjectCache::default();

    let started = Instant::now();
    let page = project::details(&ctx, &cache, &id).await?;
    println!(
        "{} ({}) by {} — {} links, {} images, description {} → {} bytes of HTML [{:?}]",
        page.title,
        page.project_type,
        page.authors
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        page.links.len(),
        page.gallery.len(),
        page.summary.len(),
        page.body_html.len(),
        started.elapsed()
    );

    let started = Instant::now();
    let versions = project::versions(&ctx, &cache, &page.id, None).await?;
    println!(
        "{} versions, recommended {:?} [{:?}]",
        versions.versions.len(),
        versions.recommended,
        started.elapsed()
    );

    if let Some(version) = versions
        .recommended
        .as_deref()
        .or(versions.versions.first().map(|v| v.id.as_str()))
    {
        let started = Instant::now();
        let deps = project::dependencies(&ctx, &cache, &page.id, version).await?;
        println!(
            "{} dependencies of {version} [{:?}]",
            deps.len(),
            started.elapsed()
        );
        for dep in deps.iter().take(5) {
            println!(
                "  {} {} ({})",
                dep.kind, dep.project.title, dep.project.project_type
            );
        }
        let started = Instant::now();
        let again = project::dependencies(&ctx, &cache, &page.id, version).await?;
        println!(
            "again from cache: {} [{:?}]",
            again.len(),
            started.elapsed()
        );
        let log = project::changelog(&ctx, &cache, &page.id, version).await?;
        println!("changelog: {} bytes of HTML", log.len());
    }
    Ok(())
}
