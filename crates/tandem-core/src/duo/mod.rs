//! Playing together without a server (D5, D37). The host opens its world to LAN in the
//! game as usual; Tandem tunnels it over an iroh QUIC connection (direct when hole
//! punching works, through iroh's relays otherwise) to the guest, whose game sees it as
//! a LAN world of its own network. No mod, every version and loader.

pub mod invite;
pub mod lan;
pub mod protocol;

use std::collections::HashSet;
use std::path::Path;

use iroh::endpoint::Connection;
use serde::Serialize;

use crate::context::Context;
use crate::error::{Error, Result};
use crate::instance::Instance;
use protocol::{InstanceSummary, ModRef};

/// iroh application protocol of Tandem's sessions.
pub const ALPN: &[u8] = b"tandem/duo/1";

pub(crate) fn fail(err: impl std::fmt::Display) -> Error {
    Error::Duo(err.to_string())
}

/// How the two players are linked right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkStatus {
    /// Round trip, in milliseconds.
    pub ping_ms: u32,
    /// Straight between the two computers, or through a relay.
    pub direct: bool,
}

pub fn link_status(conn: &Connection) -> Option<LinkStatus> {
    let paths = conn.paths();
    let selected = paths.iter().find(|p| p.is_selected())?;
    Some(LinkStatus {
        ping_ms: selected.rtt().as_millis().min(u32::MAX as u128) as u32,
        direct: selected.is_ip(),
    })
}

/// The instance's version, loader and enabled mods (hashed), sent to the other player.
pub async fn summary(ctx: &Context, instance: &Instance) -> Result<InstanceSummary> {
    let mods_dir = ctx.data.instance_dir(&instance.id).join("mods");
    let mods = tokio::task::spawn_blocking(move || hash_mods(&mods_dir))
        .await
        .map_err(fail)?;
    Ok(InstanceSummary {
        name: instance.name.clone(),
        game_version: instance.game_version.clone(),
        loader: instance.loader.to_string(),
        loader_version: instance.loader_version.clone(),
        mods,
    })
}

fn hash_mods(dir: &Path) -> Vec<ModRef> {
    let mut mods: Vec<ModRef> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "jar"))
                .filter_map(|p| {
                    let bytes = std::fs::read(&p).ok()?;
                    Some(ModRef {
                        file_name: p.file_name()?.to_string_lossy().into_owned(),
                        sha1: sha1_smol::Sha1::from(&bytes).digest().to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    mods.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    mods
}

/// How a guest's instance differs from the host's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceDiff {
    pub same_game: bool,
    pub same_loader: bool,
    /// Host mods the guest does not have (same file).
    pub missing: Vec<ModRef>,
    /// Guest mods the host does not have.
    pub extra: Vec<ModRef>,
}

impl InstanceDiff {
    pub fn matches(&self) -> bool {
        self.same_game && self.same_loader && self.missing.is_empty() && self.extra.is_empty()
    }
}

pub fn compare(mine: &InstanceSummary, host: &InstanceSummary) -> InstanceDiff {
    let my_mods: HashSet<&str> = mine.mods.iter().map(|m| m.sha1.as_str()).collect();
    let host_mods: HashSet<&str> = host.mods.iter().map(|m| m.sha1.as_str()).collect();
    InstanceDiff {
        same_game: mine.game_version == host.game_version,
        same_loader: mine.loader == host.loader,
        missing: host
            .mods
            .iter()
            .filter(|m| !my_mods.contains(m.sha1.as_str()))
            .cloned()
            .collect(),
        extra: mine
            .mods
            .iter()
            .filter(|m| !host_mods.contains(m.sha1.as_str()))
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(game: &str, mods: &[&str]) -> InstanceSummary {
        InstanceSummary {
            name: "I".into(),
            game_version: game.into(),
            loader: "fabric".into(),
            loader_version: None,
            mods: mods
                .iter()
                .map(|s| ModRef {
                    file_name: format!("{s}.jar"),
                    sha1: (*s).into(),
                })
                .collect(),
        }
    }

    #[test]
    fn compares_instances_by_mod_files() {
        let host = summary("1.21.1", &["a", "b", "c"]);
        let guest = summary("1.21.1", &["a", "d"]);
        let diff = compare(&guest, &host);
        assert!(diff.same_game && diff.same_loader && !diff.matches());
        assert_eq!(
            diff.missing
                .iter()
                .map(|m| m.sha1.as_str())
                .collect::<Vec<_>>(),
            ["b", "c"]
        );
        assert_eq!(diff.extra[0].sha1, "d");
        assert!(compare(&host, &host).matches());
        assert!(!compare(&summary("1.20.1", &[]), &summary("1.21.1", &[])).same_game);
    }
}
