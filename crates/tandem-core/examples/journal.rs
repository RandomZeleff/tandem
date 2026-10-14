//! Dev tool: print the logbook of every world of an instance.
//!
//! cargo run -p tandem-core --example journal -- <instance-id>
//!
//! Uses `TANDEM_DATA_DIR` if set.

use tandem_core::paths::DataDir;
use tandem_core::{journal, worlds};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = DataDir::from_env()?;
    let id = std::env::args()
        .nth(1)
        .ok_or("usage: journal <instance-id>")?;
    let game_dir = data.instance_dir(&id);
    for world in worlds::list(&game_dir) {
        let started = std::time::Instant::now();
        let logbook = journal::read(&game_dir, &world.folder)?;
        println!("== {} ({:?})", world.name, started.elapsed());
        println!("{logbook:#?}");
    }
    Ok(())
}
