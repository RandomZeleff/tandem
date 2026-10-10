//! Gives the guest's copy of the host's instance the host's mods and pack files (6b).
//! Mods Modrinth knows are downloaded from it, the rest is sent by the host over the
//! session; the copy's mods the host does not have are switched off (renamed
//! `.disabled`, like the Content tab does), never deleted. Pack files (configs,
//! scripts…) come from the host. Only ever run on a copy Tandem made, never on the
//! player's own instances.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;

use serde::Serialize;
use tokio::io::AsyncWriteExt;

use super::guest::Guest;
use super::protocol::{self, InstanceSummary, ModRef, PackFile};
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
    /// Pack files (configs, scripts…) copied from the host.
    pub files: usize,
    /// Content scripts the host does not have, removed.
    pub files_removed: usize,
    /// Host files that could not be fetched (mod names, pack file paths).
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
    receive(
        guest,
        Ask::Mod(&m.sha1),
        &mods_dir.join(&m.file_name),
        &m.sha1,
    )
    .await
}

/// What is asked of the host.
enum Ask<'a> {
    Mod(&'a str),
    PackFile(&'a str),
}

/// Receives a file from the host into `dest` (through a partial file next to it), checked
/// against its SHA-1.
async fn receive(guest: &Guest, ask: Ask<'_>, dest: &Path, sha1: &str) -> Result<()> {
    let dir = dest.parent().ok_or_else(|| fail("dossier introuvable"))?;
    tokio::fs::create_dir_all(dir).await?;
    // Named after the destination: identical files (same SHA-1) may arrive side by side.
    let name = dest
        .file_name()
        .ok_or_else(|| fail("nom de fichier introuvable"))?
        .to_string_lossy();
    let partial = dir.join(format!(".{name}.part"));
    let mut file = tokio::fs::File::create(&partial).await?;
    let received = match ask {
        Ask::Mod(sha1) => guest.fetch_mod(sha1, &mut file).await,
        Ask::PackFile(path) => guest.fetch_pack_file(path, &mut file).await,
    };
    file.flush().await?;
    drop(file);
    if let Err(err) = received {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err(err);
    }
    if download::sha1_file(&partial).await? != sha1 {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err(fail("fichier reçu abîmé"));
    }
    tokio::fs::rename(&partial, dest).await?;
    Ok(())
}

/// Brings a copy of the host's instance in line with it: mods, then pack files.
pub async fn sync_instance<F>(
    ctx: &Context,
    guest: &Guest,
    game_dir: &Path,
    host: &InstanceSummary,
    on_progress: F,
) -> Result<SyncReport>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    let mut report = sync_mods(ctx, guest, game_dir, host, &on_progress).await?;
    match guest.pack_files().await {
        Ok(files) => sync_pack_files(guest, game_dir, &files, &mut report, &on_progress).await?,
        // An older host: mods only.
        Err(err) => tracing::warn!(error = %err, "pack files not synced"),
    }
    tracing::info!(?report, "duo instance sync done");
    Ok(report)
}

/// Copies the host's pack files that differ or are missing; in the
/// [`protocol::MIRRORED_ROOTS`], removes the files the host does not have.
async fn sync_pack_files<F>(
    guest: &Guest,
    game_dir: &Path,
    files: &[PackFile],
    report: &mut SyncReport,
    on_progress: &F,
) -> Result<()>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    let files: Vec<PackFile> = files
        .iter()
        .filter(|f| protocol::pack_path_ok(&f.path))
        .cloned()
        .collect();
    let (local, wanted) = {
        let (dir, files) = (game_dir.to_owned(), files.clone());
        tokio::task::spawn_blocking(move || {
            let local: HashMap<String, String> = super::pack_files(&dir)
                .into_iter()
                .map(|f| (f.path, f.sha1))
                .collect();
            let wanted: Vec<PackFile> = files
                .into_iter()
                .filter(|f| local.get(&f.path) != Some(&f.sha1))
                .collect();
            (local, wanted)
        })
        .await
        .map_err(fail)?
    };

    // Content scripts the host does not have.
    let hosts: HashSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
    for path in local.keys() {
        let root = path.split('/').next().unwrap_or_default();
        if protocol::MIRRORED_ROOTS.contains(&root) && !hosts.contains(path.as_str()) {
            tokio::fs::remove_file(game_dir.join(path)).await?;
            report.files_removed += 1;
        }
    }

    let total = wanted.len();
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
    let mut waiting = wanted.iter();
    let mut running = FuturesUnordered::new();
    for f in waiting.by_ref().take(HOST_CONCURRENCY) {
        running.push(receive_pack_file(guest, f, game_dir));
    }
    let mut done = 0;
    while let Some((f, result)) = running.next().await {
        done += 1;
        progress(done);
        if let Some(next) = waiting.next() {
            running.push(receive_pack_file(guest, next, game_dir));
        }
        match result {
            Ok(()) => report.files += 1,
            Err(err) => {
                tracing::warn!(file = %f.path, error = %err, "pack file not received from the host");
                report.unavailable.push(f.path.clone());
            }
        }
    }
    Ok(())
}

async fn receive_pack_file<'a>(
    guest: &Guest,
    f: &'a PackFile,
    game_dir: &Path,
) -> (&'a PackFile, Result<()>) {
    let dest = f
        .path
        .split('/')
        .fold(game_dir.to_owned(), |dir, part| dir.join(part));
    (
        f,
        receive(guest, Ask::PackFile(&f.path), &dest, &f.sha1).await,
    )
}
