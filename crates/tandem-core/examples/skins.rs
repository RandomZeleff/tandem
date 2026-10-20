//! Dev tool: the skin library without the app.
//!
//! cargo run -p tandem-core --example skins -- defaults
//! cargo run -p tandem-core --example skins -- player <name>
//! cargo run -p tandem-core --example skins -- file <skin.png>
//! cargo run -p tandem-core --example skins -- list | remove <id>
//!
//! Uses `TANDEM_DATA_DIR` if set.

use tandem_core::paths::DataDir;
use tandem_core::{skins, Context};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = Context::init(DataDir::from_env()?).await?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).map(String::as_str).ok_or("missing argument");
    let show = |s: &skins::LibrarySkin| {
        println!(
            "{} · {} · {:?} · {} bytes",
            s.id,
            s.name,
            s.variant,
            s.texture.len()
        )
    };
    match arg(0)? {
        "defaults" => skins::defaults(&ctx).await?.iter().for_each(show),
        "player" => show(&skins::add_player(&ctx, arg(1)?).await?),
        "file" => show(&skins::add_file(&ctx, arg(1)?.as_ref()).await?),
        "list" => skins::library(&ctx).await?.iter().for_each(show),
        "remove" => skins::remove(&ctx, arg(1)?).await?,
        other => return Err(format!("unknown command {other}").into()),
    }
    Ok(())
}
