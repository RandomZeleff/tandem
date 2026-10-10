//! Dev tool: list the Java installations Tandem finds.
//!
//! cargo run -p tandem-core --example java
//!
//! Uses `TANDEM_DATA_DIR` if set.

use tandem_core::java_detect;
use tandem_core::paths::DataDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = DataDir::from_env()?;
    let started = std::time::Instant::now();
    let found = java_detect::installed(&data);
    for java in &found {
        let origin = if java.managed { "Tandem" } else { java.vendor.as_deref().unwrap_or("?") };
        println!("Java {:>2}  {:<12} {:<22} {}", java.major, java.version, origin, java.path);
    }
    println!("{} found in {:?}", found.len(), started.elapsed());
    Ok(())
}
