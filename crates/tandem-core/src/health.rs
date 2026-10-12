//! How the shared world keeps up, for the host's load indicator (chantier 4): read from
//! the log lines of the server (the world apart's console, or the game's own output for
//! a world opened to LAN).
//!
//! Two sources: Tandem Optimizations' report every 100 ticks (Fabric and Quilt), and
//! Minecraft's own "Can't keep up!" warning (every loader, but only once it is late).

use serde::Serialize;

/// What one log line says about the world's health.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Signal {
    /// Average and longest tick over the last 100 ticks, in ms (50 = the limit).
    Report { mspt: f32, max: f32, players: u32 },
    /// The server fell behind by this many ms.
    #[serde(rename_all = "camelCase")]
    Behind { behind_ms: u64 },
}

const REPORT: &str = "[Tandem] world health: ";
const BEHIND: &str = "Can't keep up! Is the server overloaded? Running ";

/// The signal in a log line, if it carries one.
pub fn parse(line: &str) -> Option<Signal> {
    if let Some(i) = line.find(REPORT) {
        let mut mspt = None;
        let mut max = None;
        let mut players = 0;
        for field in line[i + REPORT.len()..].split_whitespace() {
            match field.split_once('=') {
                Some(("mspt", v)) => mspt = v.parse().ok(),
                Some(("max", v)) => max = v.parse().ok(),
                Some(("players", v)) => players = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        return Some(Signal::Report {
            mspt: mspt?,
            max: max.or(mspt)?,
            players,
        });
    }
    let i = line.find(BEHIND)?;
    let behind_ms = line[i + BEHIND.len()..]
        .split("ms")
        .next()?
        .trim()
        .parse()
        .ok()?;
    Some(Signal::Behind { behind_ms })
}

/// Memory the system can still give, in bytes.
pub fn free_memory_bytes() -> u64 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    system.available_memory()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_signals() {
        assert_eq!(
            parse("[15:00:01] [Server thread/INFO]: [Tandem] world health: mspt=12.3 max=40.1 players=3"),
            Some(Signal::Report {
                mspt: 12.3,
                max: 40.1,
                players: 3
            })
        );
        assert_eq!(
            parse("[23:32:47] [Server thread/WARN]: Can't keep up! Is the server overloaded? Running 4557ms or 91 ticks behind"),
            Some(Signal::Behind { behind_ms: 4557 })
        );
        assert_eq!(parse("[Server thread/INFO]: Done (12.3s)!"), None);
        assert_eq!(parse("[Tandem] world health: players=2"), None);
    }
}
