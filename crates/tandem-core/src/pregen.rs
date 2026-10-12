//! Prepares a world for playing with friends (chantier 3, D46): the land around the
//! spawn is generated before anyone explores it, the biggest cost for the host's PC when
//! several players head into new land.
//!
//! The world runs in its own server (as when hosted apart, Fabric and Quilt only) with
//! Chunky added at launch, like Tandem Optimizations: downloaded from Modrinth, never
//! written into the instance. Tandem drives it through the console and reads its
//! progress lines.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::Serialize;

use crate::content::modrinth;
use crate::download::{self, DownloadTask};
use crate::error::{Error, Result};
use crate::instance::Instance;
use crate::server::{self, RunningServer, ServerOptions};
use crate::{jvm, Context};

/// Chunky on Modrinth.
const CHUNKY: &str = "fALzjamp";
/// Mod ids that provide Fabric API, which Chunky needs on Fabric and Quilt.
const FABRIC_API_IDS: &[&str] = &["fabric-api", "fabric", "quilted_fabric_api"];

/// Radii offered, in blocks around the spawn (a square of twice that side).
pub const RADII: [u32; 3] = [500, 1000, 2000];

/// Chunks in the square Chunky generates for `radius` blocks around the centre.
pub fn chunk_count(radius: u32) -> u64 {
    let side = u64::from(radius).div_ceil(16) * 2 + 1;
    side * side
}

/// Where a preparation is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub chunks: u64,
    pub total: u64,
    pub percent: f32,
    /// Chunks per second.
    pub rate: f32,
    pub eta_secs: Option<u64>,
    pub done: bool,
}

/// A Chunky console line about the task, e.g. `[Chunky] Task running for
/// minecraft:overworld. Processed: 1520 chunks (9.35%), ETA: 0:03:21, Rate: 74.3 cps,
/// Current: 12, -5` or `[Chunky] Task finished for minecraft:overworld. Processed: 16254
/// chunks (100.00%), Total time: 0:03:40`.
pub fn parse_line(line: &str, total: u64) -> Option<Progress> {
    let rest = &line[line.find("[Chunky] Task ")?..];
    let done = rest.contains("Task finished");
    if !done && !rest.contains("Task running") {
        return None;
    }
    let chunks = after(rest, "Processed: ")?
        .split_whitespace()
        .next()?
        .replace([',', '.', '\u{a0}'], "")
        .parse()
        .ok()?;
    let percent = after(rest, "(")
        .and_then(|p| p.split('%').next())
        .and_then(|p| p.replace(',', ".").parse().ok())
        .unwrap_or(0.0);
    let rate = after(rest, "Rate: ")
        .and_then(|r| r.split_whitespace().next())
        .and_then(|r| r.replace(',', ".").parse().ok())
        .unwrap_or(0.0);
    let eta_secs = after(rest, "ETA: ").and_then(|e| duration_secs(e.split(',').next()?));
    Some(Progress {
        chunks,
        total,
        percent: if done { 100.0 } else { percent },
        rate,
        eta_secs: if done { Some(0) } else { eta_secs },
        done,
    })
}

fn after<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    text.find(marker).map(|i| &text[i + marker.len()..])
}

/// `h:mm:ss` or `mm:ss`.
fn duration_secs(text: &str) -> Option<u64> {
    text.trim()
        .split(':')
        .try_fold(0u64, |acc, part| Some(acc * 60 + part.parse::<u64>().ok()?))
}

/// A preparation running: the world's server generating the land.
pub struct Pregen {
    server: RunningServer,
    total: u64,
}

impl Pregen {
    /// Next progress report; `None` when the server stopped without finishing.
    pub async fn next(&mut self) -> Option<Progress> {
        while let Some(line) = self.server.lines.recv().await {
            if let Some(progress) = parse_line(&line, self.total) {
                return Some(progress);
            }
        }
        None
    }

    /// Pauses the task (Chunky keeps it) and stops the server, saving the world.
    pub async fn stop(mut self) -> Result<()> {
        let _ = self.server.command("chunky pause").await;
        self.server.stop().await
    }
}

/// Starts the world's server with Chunky and the generation of `radius` blocks around
/// the spawn. The world must not be open in the game.
pub async fn start(ctx: &Context, instance: &Instance, world: &str, radius: u32) -> Result<Pregen> {
    if !server::supports(instance.loader) {
        return Err(Error::LoaderNotSupported(
            instance.loader.as_str().to_owned(),
        ));
    }
    let mods = server::mod_ids(&ctx.data.instance_dir(&instance.id).join("mods")).await;
    if !FABRIC_API_IDS.iter().any(|id| mods.contains(*id)) {
        return Err(Error::Pregen(
            "il faut Fabric API dans l'instance pour préparer le monde".into(),
        ));
    }
    let extra_mods = match chunky_jar(ctx, instance, &mods).await? {
        Some(jar) => vec![jar],
        None => Vec::new(),
    };
    let prepared = server::prepare(ctx, instance).await?;
    let options = ServerOptions {
        world: world.to_owned(),
        port: server::free_port()?,
        // The game is closed: the server can have what it would get next to it.
        memory_mb: server::memory_mb(jvm::total_memory_mb(), jvm::count_mods(&prepared.dir), 0),
        max_players: 1,
        online_mode: true,
        host: None,
        extra_mods,
    };
    let mut running = server::start(ctx, &prepared, &options).await?;
    for command in [
        "chunky world minecraft:overworld".to_owned(),
        "chunky shape square".to_owned(),
        "chunky spawn".to_owned(),
        format!("chunky radius {radius}"),
        "chunky start".to_owned(),
        // Chunky asks to confirm when a paused task of this world exists.
        "chunky confirm".to_owned(),
    ] {
        running.command(&command).await?;
    }
    tracing::info!(instance = %instance.id, world, radius, "world preparation started");
    Ok(Pregen {
        server: running,
        total: chunk_count(radius),
    })
}

/// Chunky for this instance, downloaded once into `<data>/mods/chunky/`; `None` when the
/// instance already has it (twice the same mod would stop the server).
async fn chunky_jar(
    ctx: &Context,
    instance: &Instance,
    mods: &BTreeSet<String>,
) -> Result<Option<PathBuf>> {
    if mods.contains("chunky") {
        return Ok(None);
    }
    let versions = modrinth::project_versions(
        ctx,
        CHUNKY,
        &instance.game_version,
        modrinth::mod_loaders(instance.loader),
    )
    .await?;
    let release = versions.iter().position(|v| v.version_type == "release");
    let version = versions.get(release.unwrap_or(0)).ok_or_else(|| {
        Error::Pregen(format!(
            "Chunky n'existe pas pour Minecraft {}",
            instance.game_version
        ))
    })?;
    let file = version
        .primary_file()
        .ok_or_else(|| Error::Pregen("Chunky n'a pas de fichier".into()))?;
    let dest = ctx
        .data
        .root()
        .join("mods")
        .join("chunky")
        .join(&file.filename);
    download::download_all(
        &ctx.http,
        vec![DownloadTask {
            url: file.url.clone(),
            dest: dest.clone(),
            sha1: Some(file.hashes.sha1.clone()),
            size: Some(file.size),
        }],
        1,
        |_| {},
    )
    .await?;
    Ok(Some(dest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_chunky_progress() {
        let line = "[12:00:01] [Server thread/INFO]: [Chunky] Task running for minecraft:overworld. Processed: 1520 chunks (9.35%), ETA: 0:03:21, Rate: 74.3 cps, Current: 12, -5";
        let p = parse_line(line, 15876).unwrap();
        assert_eq!(p.chunks, 1520);
        assert!((p.percent - 9.35).abs() < 0.01);
        assert!((p.rate - 74.3).abs() < 0.01);
        assert_eq!(p.eta_secs, Some(201));
        assert!(!p.done);

        let line = "[Chunky] Task finished for minecraft:overworld. Processed: 16254 chunks (100.00%), Total time: 0:03:40";
        let p = parse_line(line, 15876).unwrap();
        assert!(p.done);
        assert_eq!(p.chunks, 16254);

        assert!(parse_line("[Chunky] Task started for minecraft:overworld", 1).is_none());
        assert!(parse_line("Done (12.3s)! For help, type \"help\"", 1).is_none());
    }

    #[test]
    fn counts_chunks_of_the_square() {
        // As Chunky counted: 1521 chunks for 300 blocks.
        assert_eq!(chunk_count(300), 39 * 39);
        assert_eq!(chunk_count(500), 65 * 65);
    }
}
