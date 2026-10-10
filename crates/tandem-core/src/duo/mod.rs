//! Playing together without a server (D5, D37). The host opens its world to LAN in the
//! game as usual; Tandem tunnels it over an iroh QUIC connection (direct when hole
//! punching works, through iroh's relays otherwise) to the guest, whose game sees it as
//! a LAN world of its own network. No mod, every version and loader.

pub mod guest;
pub mod host;
pub mod invite;
pub mod lan;
pub mod protocol;

use std::collections::HashSet;
use std::path::Path;

use iroh::endpoint::{presets, Connection, RecvStream, SendStream, VarInt};
use iroh::Endpoint;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::context::Context;
use crate::error::{Error, Result};
use crate::instance::Instance;
use protocol::{InstanceSummary, ModRef, PackRef};

/// iroh application protocol of Tandem's sessions.
pub const ALPN: &[u8] = b"tandem/duo/1";

/// Marks the worlds Tandem announces on a guest's network, so a host on the same network
/// (tests on one computer) never takes them for its own.
pub(crate) const RELAYED_MARK: &str = "(via Tandem)";

/// Why a host closed a guest's connection.
pub(crate) const CLOSE_HOST_LEFT: VarInt = VarInt::from_u32(1);
pub(crate) const CLOSE_KICKED: VarInt = VarInt::from_u32(2);
pub(crate) const CLOSE_REFUSED: VarInt = VarInt::from_u32(3);

pub(crate) fn fail(err: impl std::fmt::Display) -> Error {
    Error::Duo(err.to_string())
}

async fn bind_endpoint(accept: bool) -> Result<Endpoint> {
    let builder = Endpoint::builder(presets::N0);
    let builder = if accept {
        builder.alpns(vec![ALPN.to_vec()])
    } else {
        builder
    };
    builder.bind().await.map_err(|e| {
        fail(format!(
            "impossible d'ouvrir la connexion pair à pair ({e})"
        ))
    })
}

/// Copies a game TCP connection both ways over a QUIC stream until either side closes.
async fn pipe(mut send: SendStream, mut recv: RecvStream, tcp: TcpStream) {
    let _ = tcp.set_nodelay(true);
    let (mut tcp_read, mut tcp_write) = tcp.into_split();
    let upstream = async {
        let _ = tokio::io::copy(&mut tcp_read, &mut send).await;
        let _ = send.finish();
    };
    let downstream = async {
        let _ = tokio::io::copy(&mut recv, &mut tcp_write).await;
        let _ = tcp_write.shutdown().await;
    };
    tokio::join!(upstream, downstream);
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
        pack: instance
            .pack_project_id
            .clone()
            .zip(instance.pack_version_id.clone())
            .map(|(project_id, version_id)| PackRef {
                project_id,
                version_id,
            }),
        mods,
    })
}

/// The player's instance that best matches the host's: same Minecraft version and loader,
/// then the same modpack version, then the fewest differing mods. `None` when no instance
/// has the host's version and loader.
pub async fn best_match(
    ctx: &Context,
    host: &InstanceSummary,
) -> Result<Option<(Instance, InstanceSummary, InstanceDiff)>> {
    let mut candidates: Vec<Instance> = ctx
        .db
        .list_instances()
        .await?
        .into_iter()
        .filter(|i| i.game_version == host.game_version && i.loader.to_string() == host.loader)
        .collect();
    // Same modpack first (same version before), then the most recently played.
    let pack_rank = |i: &Instance| match &host.pack {
        Some(pack) if i.pack_project_id.as_deref() == Some(pack.project_id.as_str()) => {
            if i.pack_version_id.as_deref() == Some(pack.version_id.as_str()) {
                0
            } else {
                1
            }
        }
        _ => 2,
    };
    candidates.sort_by(|a, b| {
        pack_rank(a)
            .cmp(&pack_rank(b))
            .then_with(|| b.last_played_at.cmp(&a.last_played_at))
    });
    let mut best: Option<(Instance, InstanceSummary, InstanceDiff)> = None;
    for instance in candidates {
        let mine = summary(ctx, &instance).await?;
        let diff = compare(&mine, host);
        if diff.matches() {
            return Ok(Some((instance, mine, diff)));
        }
        let gap = |d: &InstanceDiff| d.missing.len() + d.extra.len();
        if best.as_ref().is_none_or(|(_, _, b)| gap(&diff) < gap(b)) {
            best = Some((instance, mine, diff));
        }
    }
    Ok(best)
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
            pack: None,
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
