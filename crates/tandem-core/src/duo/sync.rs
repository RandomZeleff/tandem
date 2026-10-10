//! Gives the guest the host's mods (6b). What Modrinth knows is downloaded from it, the
//! rest is sent by the host over the session; the guest's mods the host does not have are
//! switched off (renamed `.disabled`, like the Content tab does), never deleted.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;

use serde::Serialize;
use tokio::io::AsyncWriteExt;

use super::guest::Guest;
use super::protocol::{InstanceSummary, ModRef};
use super::{fail, hash_file};
use crate::content::modrinth;
use crate::context::Context;
use crate::download::{self, DownloadTask, Progress};
use crate::error::Result;
use crate::install::{InstallProgress, Stage};

const DISABLED_SUFFIX: &str = ".disabled";
/// Mods asked of the host at once.
const HOST_CONCURRENCY: usize = 4;

/// What a sync did, for the player.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    /// Downloaded from Modrinth.
    pub downloaded: usize,
    /// Sent by the host.
    pub from_host: usize,
    /// Switched back on (already there, but disabled).
    pub enabled: usize,
    /// The guest's mods the host does not have, switched off.
    pub disabled: usize,
    /// Host mods that could not be fetched (file names).
    pub unavailable: Vec<String>,
}

/// A mod file of the folder, enabled or not.
struct Local {
    path: PathBuf,
    sha1: String,
    enabled: bool,
}

fn scan(dir: &Path) -> Vec<Local> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?;
            let enabled = name.ends_with(".jar");
            if !enabled && !name.ends_with(&format!(".jar{DISABLED_SUFFIX}")) {
                return None;
            }
            Some(Local {
                sha1: hash_file(&path)?,
                path,
                enabled,
            })
        })
        .collect()
}

fn enabled_path(disabled: &Path) -> PathBuf {
    let name = disabled.to_string_lossy();
    PathBuf::from(
        name.strip_suffix(DISABLED_SUFFIX)
            .unwrap_or(&name)
            .to_owned(),
    )
}

fn disabled_path(enabled: &Path) -> PathBuf {
    let mut name = enabled.as_os_str().to_owned();
    name.push(DISABLED_SUFFIX);
    PathBuf::from(name)
}

/// Makes the enabled mods of `game_dir` the same files as the host's.
pub async fn sync_mods<F>(
    ctx: &Context,
    guest: &Guest,
    game_dir: &Path,
    host: &InstanceSummary,
    on_progress: F,
) -> Result<SyncReport>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    let mods_dir = game_dir.join("mods");
    tokio::fs::create_dir_all(&mods_dir).await?;
    let local = {
        let dir = mods_dir.clone();
        tokio::task::spawn_blocking(move || scan(&dir))
            .await
            .map_err(fail)?
    };
    let wanted: HashSet<&str> = host.mods.iter().map(|m| m.sha1.as_str()).collect();
    let mut report = SyncReport::default();

    // Mods the host does not have: off. Then host mods already there but off: on.
    for file in local.iter().filter(|f| f.enabled) {
        if !wanted.contains(file.sha1.as_str()) {
            tokio::fs::rename(&file.path, disabled_path(&file.path)).await?;
            report.disabled += 1;
        }
    }
    let mut have: HashSet<&str> = local
        .iter()
        .filter(|f| f.enabled && wanted.contains(f.sha1.as_str()))
        .map(|f| f.sha1.as_str())
        .collect();
    for file in local.iter().filter(|f| !f.enabled) {
        if wanted.contains(file.sha1.as_str()) && have.insert(file.sha1.as_str()) {
            let on = enabled_path(&file.path);
            if !tokio::fs::try_exists(&on).await.unwrap_or(false) {
                tokio::fs::rename(&file.path, on).await?;
                report.enabled += 1;
            } else {
                have.remove(file.sha1.as_str());
            }
        }
    }
    let missing: Vec<&ModRef> = host
        .mods
        .iter()
        .filter(|m| !have.contains(m.sha1.as_str()))
        .collect();
    tracing::info!(
        missing = missing.len(),
        disabled = report.disabled,
        enabled = report.enabled,
        "duo mod sync planned"
    );
    if missing.is_empty() {
        return Ok(report);
    }

    // Modrinth first: faster and spares the host's upload.
    on_progress(InstallProgress {
        stage: Stage::Metadata,
        download: Progress::default(),
    });
    let sha1s: Vec<String> = missing.iter().map(|m| m.sha1.clone()).collect();
    let known = match modrinth::versions_by_hash(ctx, &sha1s).await {
        Ok(known) => known,
        Err(err) => {
            tracing::warn!(error = %err, "Modrinth hash lookup failed, asking the host");
            HashMap::new()
        }
    };
    let mut from_modrinth: Vec<(&ModRef, DownloadTask)> = Vec::new();
    let mut from_host = Vec::new();
    for m in &missing {
        let file = known
            .get(&m.sha1)
            .and_then(|v| v.files.iter().find(|f| f.hashes.sha1 == m.sha1));
        match file {
            Some(file) => from_modrinth.push((
                m,
                DownloadTask {
                    url: file.url.clone(),
                    dest: mods_dir.join(&m.file_name),
                    sha1: Some(m.sha1.clone()),
                    size: Some(file.size),
                },
            )),
            None => from_host.push(*m),
        }
    }
    if !from_modrinth.is_empty() {
        let tasks = from_modrinth.iter().map(|(_, t)| t.clone()).collect();
        if let Err(err) = download::download_all(&ctx.http, tasks, 8, |download| {
            on_progress(InstallProgress {
                stage: Stage::Downloading,
                download,
            })
        })
        .await
        {
            tracing::warn!(error = %err, "some Modrinth downloads failed, asking the host");
        }
        // Files land only once checked: whatever is not there, the host may still send.
        for (m, task) in from_modrinth {
            if tokio::fs::try_exists(&task.dest).await.unwrap_or(false) {
                report.downloaded += 1;
            } else {
                from_host.push(m);
            }
        }
    }

    // The rest comes from the host, a few files at a time (one QUIC stream each).
    let total = from_host.len();
    let progress = |done: usize| {
        on_progress(InstallProgress {
            stage: Stage::Copying,
            download: Progress {
                done_files: done,
                total_files: total,
                ..Progress::default()
            },
        })
    };
    progress(0);
    let mut waiting = from_host.into_iter();
    let mut running = FuturesUnordered::new();
    for m in waiting.by_ref().take(HOST_CONCURRENCY) {
        running.push(fetch_one(guest, m, &mods_dir));
    }
    let mut done = 0;
    while let Some((m, result)) = running.next().await {
        done += 1;
        progress(done);
        if let Some(next) = waiting.next() {
            running.push(fetch_one(guest, next, &mods_dir));
        }
        match result {
            Ok(()) => report.from_host += 1,
            Err(err) => {
                tracing::warn!(file = %m.file_name, error = %err, "mod not received from the host");
                report.unavailable.push(m.file_name.clone());
            }
        }
    }
    tracing::info!(?report, "duo mod sync done");
    Ok(report)
}

async fn fetch_one<'a>(guest: &Guest, m: &'a ModRef, mods_dir: &Path) -> (&'a ModRef, Result<()>) {
    (m, fetch_from_host(guest, m, mods_dir).await)
}

/// Receives one mod from the host into `mods_dir`, checked against its SHA-1.
async fn fetch_from_host(guest: &Guest, m: &ModRef, mods_dir: &Path) -> Result<()> {
    if m.file_name.contains(['/', '\\']) || m.file_name.starts_with('.') {
        return Err(fail("nom de fichier refusé"));
    }
    let dest = mods_dir.join(&m.file_name);
    let partial = mods_dir.join(format!(".{}.part", m.sha1));
    let mut file = tokio::fs::File::create(&partial).await?;
    let received = guest.fetch_mod(&m.sha1, &mut file).await;
    file.flush().await?;
    drop(file);
    if let Err(err) = received {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err(err);
    }
    if download::sha1_file(&partial).await? != m.sha1 {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err(fail("fichier reçu abîmé"));
    }
    tokio::fs::rename(&partial, &dest).await?;
    Ok(())
}
