//! Dev tool: list the files of an instance that Tandem does not track.
//!
//! cargo run -p tandem-core --example local -- <instance-id>
//!
//! Uses `TANDEM_DATA_DIR` if set.

use tandem_core::content::local;
use tandem_core::paths::DataDir;
use tandem_core::Context;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let id = std::env::args()
        .nth(1)
        .ok_or("usage: local <instance-id>")?;
    let ctx = Context::init(DataDir::from_env()?).await?;
    let started = std::time::Instant::now();
    let files = local::list(&ctx, &id).await?;
    let tracked = ctx.db.list_content(&id).await?.len();
    println!(
        "{tracked} tracked, {} untracked ({:?})",
        files.len(),
        started.elapsed()
    );
    for f in files.iter().take(12) {
        println!(
            "  {:?} {} — {} {}",
            f.kind,
            f.name,
            f.file_name,
            if f.enabled { "" } else { "(off)" }
        );
    }
    Ok(())
}
