//! Dev tool: list mods of an instance that declare they cannot run together.
//!
//! cargo run -p tandem-core --example conflicts -- <instance-id>
//!
//! Uses `TANDEM_DATA_DIR` if set.

use tandem_core::content::deps;
use tandem_core::paths::DataDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = DataDir::from_env()?;
    let id = std::env::args()
        .nth(1)
        .ok_or("usage: conflicts <instance-id>")?;
    let started = std::time::Instant::now();
    let mods = deps::scan_cached(&data, &id, &data.instance_dir(&id));
    let found = deps::conflicts(&mods);
    let declared: usize = mods.iter().map(|m| m.breaks.len()).sum();
    println!(
        "{} mods, {} declared incompatibilities, {} conflicts ({:?})",
        mods.len(),
        declared,
        found.len(),
        started.elapsed()
    );
    for c in &found {
        println!("  {} <> {}", c.name, c.other_name);
    }
    Ok(())
}
