//! JVM defaults: memory picked from the machine and the instance, and G1 tuning.

use std::path::Path;

/// Memory left to the OS and other apps before Minecraft gets the rest.
const SYSTEM_RESERVE_MB: u64 = 6144;
const MIN_MEMORY_MB: u64 = 1024;

/// Total physical memory, in MiB.
pub fn total_memory_mb() -> u64 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    system.total_memory() / (1024 * 1024)
}

/// Memory to give an instance that has no explicit setting.
///
/// What the game wants grows with its mods; the cap leaves the OS room to breathe
/// (half the RAM on small machines, everything but 6 GiB on bigger ones).
pub fn auto_memory_mb(total_mb: u64, mod_count: usize) -> u32 {
    let wanted: u64 = match mod_count {
        0 => 2048,
        1..=50 => 4096,
        51..=150 => 6144,
        _ => 8192,
    };
    let cap = (total_mb / 2).max(total_mb.saturating_sub(SYSTEM_RESERVE_MB));
    let mb = wanted.min(cap) / 512 * 512;
    mb.max(MIN_MEMORY_MB) as u32
}

/// Mod jars in an instance (enabled ones only: disabled mods end in `.disabled`).
pub fn count_mods(game_dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(game_dir.join("mods")) else {
        return 0;
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "jar"))
        .count()
}

/// G1 tuned for a game client: short pauses and a large young generation, since
/// Minecraft allocates many short-lived objects. Valid from Java 8 onwards.
/// Skipped when the player picked a garbage collector in their own JVM arguments.
pub fn gc_flags(extra_args: &[String]) -> Vec<String> {
    if extra_args
        .iter()
        .any(|a| a.starts_with("-XX:+Use") && a.ends_with("GC"))
    {
        return Vec::new();
    }
    [
        "-XX:+UseG1GC",
        "-XX:+ParallelRefProcEnabled",
        "-XX:MaxGCPauseMillis=200",
        "-XX:+UnlockExperimentalVMOptions",
        "-XX:+DisableExplicitGC",
        "-XX:G1NewSizePercent=30",
        "-XX:G1MaxNewSizePercent=40",
        "-XX:G1HeapRegionSize=8M",
        "-XX:G1ReservePercent=20",
        "-XX:G1MixedGCCountTarget=4",
        "-XX:InitiatingHeapOccupancyPercent=15",
        "-XX:G1MixedGCLiveThresholdPercent=90",
        "-XX:SurvivorRatio=32",
        "-XX:MaxTenuringThreshold=1",
    ]
    .map(str::to_owned)
    .to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_grows_with_mods_and_fits_the_machine() {
        let gib = 1024;
        assert_eq!(auto_memory_mb(16 * gib, 0), 2048);
        assert_eq!(auto_memory_mb(16 * gib, 30), 4096);
        assert_eq!(auto_memory_mb(16 * gib, 120), 6144);
        assert_eq!(auto_memory_mb(32 * gib, 300), 8192);
        // 8 GiB: half for the game at most.
        assert_eq!(auto_memory_mb(8 * gib, 300), 4096);
        // 16 GiB: 10 GiB cap, a big pack still gets its 8.
        assert_eq!(auto_memory_mb(16 * gib, 300), 8192);
        // Tiny machines never go below 1 GiB.
        assert_eq!(auto_memory_mb(1536, 10), 1024);
    }

    #[test]
    fn player_gc_choice_wins() {
        assert!(gc_flags(&[]).contains(&"-XX:+UseG1GC".to_owned()));
        assert!(gc_flags(&["-XX:+UseZGC".to_owned()]).is_empty());
        assert!(!gc_flags(&["-XX:+UseStringDeduplication".to_owned()]).is_empty());
    }
}
