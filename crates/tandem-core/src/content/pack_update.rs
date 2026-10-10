//! Moving an instance made from a Modrinth modpack to another version of the pack.
//!
//! Only what the previous version installed, and the player left untouched, is
//! replaced or removed: a config the player tuned stays, a mod they disabled stays
//! disabled, `options.txt` is never overwritten. Every file the update replaces or
//! removes is moved to `.tandem/rollback/`, so the previous version can come back.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::mrpack::{self, PackIndex};
use crate::context::Context;
use crate::download::{self, DownloadTask};
use crate::error::{Error, Result};
use crate::install::{InstallProgress, Stage};
use crate::instance::{Instance, PackOrigin};
use crate::meta::loader::Loader;
use crate::paths::DataDir;
use crate::store;
use crate::worlds::{self, BackupKind};

const STATE_FILE: &str = ".tandem/pack.json";
const ROLLBACK_DIR: &str = ".tandem/rollback";
const ROLLBACK_MANIFEST: &str = "rollback.json";

/// Files the player owns even when a pack ships them.
const PLAYER_FILES: &[&str] = &[
    "options.txt",
    "servers.dat",
    "optionsof.txt",
    "optionsshaders.txt",
];

const DISABLED: &str = ".disabled";

/// What a version of a pack put into the instance: paths with their SHA-1.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackState {
    pub version_id: String,
    /// Downloaded files (mods, packs…).
    pub files: BTreeMap<String, String>,
    /// Files copied from `overrides/` and `client-overrides/`.
    pub overrides: BTreeMap<String, String>,
}

impl PackState {
    pub fn load(game_dir: &Path) -> Option<Self> {
        let bytes = std::fs::read(game_dir.join(STATE_FILE)).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub fn save(&self, game_dir: &Path) -> Result<()> {
        let path = game_dir.join(STATE_FILE);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

fn sha1_bytes(bytes: &[u8]) -> String {
    sha1_smol::Sha1::from(bytes).digest().to_string()
}

fn sha1_file(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut hasher = sha1_smol::Sha1::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(hasher.digest().to_string())
}

/// Override files of a `.mrpack` (client ones win), by path, with their zip entry.
fn override_entries(pack: &Path) -> Result<BTreeMap<String, usize>> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(pack)?)?;
    let mut entries = BTreeMap::new();
    for prefix in ["overrides/", "client-overrides/"] {
        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            if entry.is_dir() {
                continue;
            }
            let Some(name) = entry.enclosed_name() else {
                continue;
            };
            let name = name.to_string_lossy().replace('\\', "/");
            if let Some(relative) = name.strip_prefix(prefix) {
                if mrpack::safe_relative(relative).is_some() {
                    entries.insert(relative.to_owned(), i);
                }
            }
        }
    }
    Ok(entries)
}

/// Everything a pack version installs on a client.
pub async fn contents(pack: &Path, index: &PackIndex) -> Result<PackState> {
    let pack = pack.to_owned();
    let index = index.clone();
    mrpack::blocking(move || {
        let files = index
            .files
            .iter()
            .filter(|f| f.for_client())
            .map(|f| (f.path.clone(), f.hashes.sha1.to_lowercase()))
            .collect();
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&pack)?)?;
        let mut overrides = BTreeMap::new();
        for (path, i) in override_entries(&pack)? {
            let mut bytes = Vec::new();
            archive.by_index(i)?.read_to_end(&mut bytes)?;
            overrides.insert(path, sha1_bytes(&bytes));
        }
        Ok(PackState {
            version_id: index.version_id.clone(),
            files,
            overrides,
        })
    })
    .await
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReport {
    pub added: usize,
    pub replaced: usize,
    pub removed: usize,
    /// Files the player changed, kept as they are.
    pub kept: Vec<String>,
    /// `(from, to)` when the game version changed.
    pub game_version: Option<(String, String)>,
    pub worlds_backed_up: usize,
}

/// What a rollback needs to undo an update.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rollback {
    game_version: String,
    loader: Loader,
    loader_version: Option<String>,
    origin: Option<PackOrigin>,
    state: Option<PackState>,
    /// Files the update created (removed by a rollback).
    created: Vec<String>,
    /// Files the update replaced or removed, saved under `files/`.
    saved: Vec<String>,
}

/// The pack version a rollback would bring back, if one is possible.
pub fn rollback_version(game_dir: &Path) -> Option<String> {
    let bytes = std::fs::read(game_dir.join(ROLLBACK_DIR).join(ROLLBACK_MANIFEST)).ok()?;
    let rollback: Rollback = serde_json::from_slice(&bytes).ok()?;
    Some(rollback.origin.map(|o| o.version).unwrap_or_default())
}

/// What the previous version installed: saved at install time, else rebuilt from the
/// pack file of that version. `None` when unknown (then nothing is removed).
async fn previous_state(ctx: &Context, instance: &Instance) -> Option<PackState> {
    let game_dir = ctx.data.instance_dir(&instance.id);
    if let Some(state) = PackState::load(&game_dir) {
        return Some(state);
    }
    let (project, version) = (
        instance.pack_project_id.as_deref()?,
        instance.pack_version_id.as_deref()?,
    );
    let pack = mrpack::download(ctx, project, Some(version)).await.ok()?;
    let index = mrpack::read_index(&pack.path).await.ok()?;
    let state = contents(&pack.path, &index).await.ok();
    let _ = tokio::fs::remove_file(&pack.path).await;
    state
}

/// Updates the instance to the pack in `pack` (already downloaded). `origin` is the new
/// Modrinth version, `None` for a pack file.
pub async fn update<F>(
    ctx: &Context,
    instance: &Instance,
    pack: &Path,
    origin: Option<PackOrigin>,
    on_progress: F,
) -> Result<UpdateReport>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    let stage = |stage| {
        on_progress(InstallProgress {
            stage,
            download: Default::default(),
        })
    };
    stage(Stage::Metadata);
    let game_dir = ctx.data.instance_dir(&instance.id);
    let index = mrpack::read_index(pack).await?;
    let new = contents(pack, &index).await?;
    let old = previous_state(ctx, instance).await;
    let (game_version, loader, loader_version) = index.target()?;
    let mut report = UpdateReport::default();
    // Mods the player turned off stay off, even when their file name changes.
    let disabled: HashSet<String> = ctx
        .db
        .list_content(&instance.id)
        .await?
        .into_iter()
        .filter(|c| !c.enabled)
        .map(|c| c.project_id)
        .collect();

    // The game version changes: worlds get converted on first load, keep a copy first.
    if game_version != instance.game_version {
        report.game_version = Some((instance.game_version.clone(), game_version.clone()));
        let (data, id, dir) = (ctx.data.clone(), instance.id.clone(), game_dir.clone());
        report.worlds_backed_up =
            mrpack::blocking(move || Ok(backup_worlds(&data, &id, &dir))).await?;
    }

    // Downloads first: if one fails, the instance is still untouched.
    let mut tasks = Vec::new();
    for file in index.files.iter().filter(|f| f.for_client()) {
        mrpack::safe_relative(&file.path).ok_or_else(|| {
            Error::InvalidInput(format!("Chemin dangereux dans le modpack : {}", file.path))
        })?;
        let url = file
            .downloads
            .iter()
            .find(|u| mrpack::allowed_url(u))
            .ok_or_else(|| {
                Error::InvalidInput(format!(
                    "{} n'a pas d'adresse de téléchargement autorisée",
                    file.path
                ))
            })?;
        let sha1 = file.hashes.sha1.to_lowercase();
        tasks.push(DownloadTask {
            url: url.clone(),
            dest: store::path(&ctx.data, &sha1),
            sha1: Some(sha1),
            size: Some(file.file_size),
        });
    }
    download::download_all(&ctx.http, tasks, download::DEFAULT_CONCURRENCY, |p| {
        on_progress(InstallProgress {
            stage: Stage::Downloading,
            download: p,
        })
    })
    .await?;

    stage(Stage::Finalizing);
    let previous_origin = instance
        .pack_project_id
        .clone()
        .map(|project_id| PackOrigin {
            project_id,
            version_id: instance.pack_version_id.clone().unwrap_or_default(),
            version: instance.pack_version.clone().unwrap_or_default(),
            icon: instance.icon.clone(),
        });
    let mut rollback = Rollback {
        game_version: instance.game_version.clone(),
        loader: instance.loader,
        loader_version: instance.loader_version.clone(),
        origin: previous_origin,
        state: old.clone(),
        created: Vec::new(),
        saved: Vec::new(),
    };
    let (data, dir, pack_path, new_state) = (
        ctx.data.clone(),
        game_dir.clone(),
        pack.to_owned(),
        new.clone(),
    );
    let (report, rollback) = mrpack::blocking(move || {
        let mut report = report;
        apply(
            &data,
            &dir,
            &pack_path,
            old.as_ref(),
            &new_state,
            &mut report,
            &mut rollback,
        )?;
        Ok((report, rollback))
    })
    .await?;

    let rollback_dir = game_dir.join(ROLLBACK_DIR);
    std::fs::create_dir_all(&rollback_dir)?;
    std::fs::write(
        rollback_dir.join(ROLLBACK_MANIFEST),
        serde_json::to_vec_pretty(&rollback)?,
    )?;
    new.save(&game_dir)?;

    ctx.db
        .set_instance_target(
            &instance.id,
            &game_version,
            loader,
            loader_version.as_deref(),
        )
        .await?;
    if let Some(origin) = &origin {
        ctx.db.set_instance_pack(&instance.id, origin).await?;
    }
    retrack(ctx, instance, &new).await;
    if let Ok(rows) = ctx.db.list_content(&instance.id).await {
        for row in rows
            .into_iter()
            .filter(|r| r.enabled && disabled.contains(&r.project_id))
        {
            if let Err(err) = super::set_enabled(ctx, &instance.id, &row.project_id, false).await {
                tracing::warn!(project = %row.title, error = %err, "could not keep content disabled");
            }
        }
    }
    tracing::info!(instance = %instance.id, version = %index.version_id, added = report.added, replaced = report.replaced, removed = report.removed, kept = report.kept.len(), "modpack updated");
    Ok(report)
}

fn backup_worlds(data: &DataDir, instance_id: &str, game_dir: &Path) -> usize {
    worlds::list(game_dir)
        .iter()
        .filter(|w| {
            worlds::backup(
                data,
                instance_id,
                game_dir,
                &w.folder,
                BackupKind::BeforeUpdate,
            )
            .is_ok()
        })
        .count()
}

/// The file at `rel`, or its disabled twin when the player turned it off.
fn existing(game_dir: &Path, rel: &str) -> Option<PathBuf> {
    let path = game_dir.join(rel);
    if path.is_file() {
        return Some(path);
    }
    let disabled = game_dir.join(format!("{rel}{DISABLED}"));
    disabled.is_file().then_some(disabled)
}

fn relative(game_dir: &Path, path: &Path) -> String {
    path.strip_prefix(game_dir)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Moves a file into the rollback folder.
fn save(game_dir: &Path, path: &Path, rollback: &mut Rollback) -> Result<()> {
    let rel = relative(game_dir, path);
    let dest = game_dir.join(ROLLBACK_DIR).join("files").join(&rel);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if std::fs::rename(path, &dest).is_err() {
        std::fs::copy(path, &dest)?;
        std::fs::remove_file(path)?;
    }
    rollback.saved.push(rel);
    Ok(())
}

fn apply(
    data: &DataDir,
    game_dir: &Path,
    pack: &Path,
    old: Option<&PackState>,
    new: &PackState,
    report: &mut UpdateReport,
    rollback: &mut Rollback,
) -> Result<()> {
    let rollback_root = game_dir.join(ROLLBACK_DIR);
    if rollback_root.exists() {
        std::fs::remove_dir_all(&rollback_root)?;
    }

    // What the previous version installed and this one no longer ships.
    if let Some(old) = old {
        for (rel, sha) in old.files.iter().chain(&old.overrides) {
            if new.files.contains_key(rel) || new.overrides.contains_key(rel) {
                continue;
            }
            if let Some(path) = existing(game_dir, rel) {
                if sha1_file(&path).as_deref() == Some(sha.as_str()) {
                    save(game_dir, &path, rollback)?;
                    report.removed += 1;
                } else {
                    report.kept.push(rel.clone());
                }
            }
        }
    }

    // Downloaded files: the pack decides (a disabled mod stays disabled).
    for (rel, sha) in &new.files {
        let target = existing(game_dir, rel).unwrap_or_else(|| game_dir.join(rel));
        if target.is_file() {
            if sha1_file(&target).as_deref() == Some(sha.as_str()) {
                continue;
            }
            save(game_dir, &target, rollback)?;
            report.replaced += 1;
        } else {
            rollback.created.push(relative(game_dir, &target));
            report.added += 1;
        }
        store::link_blocking(&store::path(data, sha), &target)?;
    }

    // Overrides: replaced unless the player changed the previous version's file.
    let entries = override_entries(pack)?;
    let mut archive = zip::ZipArchive::new(std::fs::File::open(pack)?)?;
    for (rel, sha) in &new.overrides {
        let path = game_dir.join(rel);
        let current = path.is_file().then(|| sha1_file(&path)).flatten();
        if current.as_deref() == Some(sha.as_str()) {
            continue;
        }
        if current.is_some() {
            let player_file = PLAYER_FILES.contains(&rel.as_str());
            let changed_by_player = old
                .and_then(|o| o.overrides.get(rel))
                .is_some_and(|before| current.as_deref() != Some(before.as_str()));
            if player_file || changed_by_player {
                if !player_file {
                    report.kept.push(rel.clone());
                }
                continue;
            }
            save(game_dir, &path, rollback)?;
            report.replaced += 1;
        } else {
            rollback.created.push(rel.clone());
            report.added += 1;
        }
        let Some(&i) = entries.get(rel) else { continue };
        let mut bytes = Vec::new();
        archive.by_index(i)?.read_to_end(&mut bytes)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)?;
    }
    Ok(())
}

/// Content rows follow the files: rows of removed files go, pack files Modrinth knows
/// are recorded (in their current enabled state).
async fn retrack(ctx: &Context, instance: &Instance, state: &PackState) {
    let game_dir = ctx.data.instance_dir(&instance.id);
    if let Ok(rows) = ctx.db.list_content(&instance.id).await {
        for row in rows {
            let rel = format!(
                "{}/{}",
                row.kind.folder(),
                row.file_name.trim_end_matches(DISABLED)
            );
            if existing(&game_dir, &rel).is_none() {
                let _ = ctx.db.delete_content(&instance.id, &row.project_id).await;
            }
        }
    }
    let placements: Vec<(PathBuf, String)> = state
        .files
        .iter()
        .map(|(p, s)| (PathBuf::from(p), s.clone()))
        .collect();
    if let Err(err) = mrpack::track(ctx, instance, &placements).await {
        tracing::warn!(instance = %instance.id, error = %err, "could not identify modpack files");
    }
    // `track` records files as enabled: give disabled ones their state back.
    if let Ok(rows) = ctx.db.list_content(&instance.id).await {
        for row in rows {
            let rel = format!(
                "{}/{}",
                row.kind.folder(),
                row.file_name.trim_end_matches(DISABLED)
            );
            let disabled = !game_dir.join(&rel).is_file()
                && game_dir.join(format!("{rel}{DISABLED}")).is_file();
            if disabled == row.enabled {
                let _ = ctx
                    .db
                    .set_content_enabled(&instance.id, &row.project_id, !disabled)
                    .await;
            }
        }
    }
}

/// Undoes the last update: created files removed, saved files back, game version,
/// loader and pack version restored.
pub async fn rollback(ctx: &Context, instance: &Instance) -> Result<()> {
    let game_dir = ctx.data.instance_dir(&instance.id);
    let root = game_dir.join(ROLLBACK_DIR);
    let bytes = std::fs::read(root.join(ROLLBACK_MANIFEST))
        .map_err(|_| Error::InvalidInput("Aucune mise à jour à annuler".into()))?;
    let rollback: Rollback = serde_json::from_slice(&bytes)?;
    let dir = game_dir.clone();
    let manifest = mrpack::blocking(move || {
        for rel in &rollback.created {
            let path = dir.join(rel);
            if path.is_file() {
                std::fs::remove_file(path)?;
            }
        }
        for rel in &rollback.saved {
            let from = dir.join(ROLLBACK_DIR).join("files").join(rel);
            let to = dir.join(rel);
            // The update may have put a newer twin (enabled or disabled) in its place.
            for twin in [
                to.clone(),
                PathBuf::from(format!("{}{DISABLED}", to.display())),
            ] {
                if twin.is_file() && !rollback.created.iter().any(|c| dir.join(c) == twin) {
                    let _ = std::fs::remove_file(&twin);
                }
            }
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if std::fs::rename(&from, &to).is_err() {
                std::fs::copy(&from, &to)?;
            }
        }
        match &rollback.state {
            Some(state) => state.save(&dir)?,
            None => {
                let _ = std::fs::remove_file(dir.join(STATE_FILE));
            }
        }
        std::fs::remove_dir_all(dir.join(ROLLBACK_DIR))?;
        Ok(rollback)
    })
    .await?;

    ctx.db
        .set_instance_target(
            &instance.id,
            &manifest.game_version,
            manifest.loader,
            manifest.loader_version.as_deref(),
        )
        .await?;
    if let Some(origin) = &manifest.origin {
        ctx.db.set_instance_pack(&instance.id, origin).await?;
    }
    if let Some(state) = &manifest.state {
        retrack(ctx, instance, state).await;
    }
    tracing::info!(instance = %instance.id, "modpack update rolled back");
    Ok(())
}

/// Newer versions of the instance's pack Tandem can install, newest first.
pub async fn newer_versions(
    ctx: &Context,
    instance: &Instance,
) -> Result<Vec<mrpack::PackVersion>> {
    let (Some(project), Some(current)) = (&instance.pack_project_id, &instance.pack_version_id)
    else {
        return Ok(Vec::new());
    };
    let versions = mrpack::versions(ctx, project).await?;
    let published: HashMap<&str, &str> = versions
        .iter()
        .map(|v| (v.id.as_str(), v.date_published.as_str()))
        .collect();
    let Some(current_date) = published.get(current.as_str()).copied() else {
        // The installed version vanished from Modrinth: offer the recommended one.
        return Ok(versions.into_iter().filter(|v| v.recommended).collect());
    };
    let ids: HashSet<String> = versions
        .iter()
        .filter(|v| v.date_published.as_str() > current_date)
        .map(|v| v.id.clone())
        .collect();
    Ok(versions
        .into_iter()
        .filter(|v| ids.contains(&v.id))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, rel: &str, content: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    fn sha(content: &str) -> String {
        sha1_bytes(content.as_bytes())
    }

    /// A pack file with the given overrides.
    fn pack(dir: &Path, overrides: &[(&str, &str)]) -> PathBuf {
        let path = dir.join("pack.mrpack");
        let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        for (name, content) in overrides {
            writer
                .start_file(
                    format!("overrides/{name}"),
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            std::io::Write::write_all(&mut writer, content.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
        path
    }

    #[test]
    fn updates_only_what_the_player_left_alone() {
        let root = tempfile::tempdir().unwrap();
        let data = DataDir::new(root.path().join("data"));
        let game = root.path().join("game");
        // Store holds the new mod version.
        let new_mod = "mod v2";
        let store_path = store::path(&data, &sha(new_mod));
        std::fs::create_dir_all(store_path.parent().unwrap()).unwrap();
        std::fs::write(&store_path, new_mod).unwrap();

        // Installed by v1: two mods (one disabled by the player), configs, options.
        write(&game, "mods/a.jar", "mod v1");
        write(&game, "mods/gone.jar", "old mod");
        write(&game, "mods/kept.jar.disabled", "mod v1");
        write(&game, "config/tuned.toml", "player value");
        write(&game, "config/plain.toml", "v1 value");
        write(&game, "config/obsolete.toml", "v1 obsolete");
        write(&game, "options.txt", "player options");
        let old = PackState {
            version_id: "1".into(),
            files: BTreeMap::from([
                ("mods/a.jar".into(), sha("mod v1")),
                ("mods/gone.jar".into(), sha("old mod")),
                ("mods/kept.jar".into(), sha("mod v1")),
            ]),
            overrides: BTreeMap::from([
                ("config/tuned.toml".into(), sha("v1 value")),
                ("config/plain.toml".into(), sha("v1 value")),
                ("config/obsolete.toml".into(), sha("v1 obsolete")),
                ("options.txt".into(), sha("pack options")),
            ]),
        };
        let overrides = [
            ("config/tuned.toml", "v2 value"),
            ("config/plain.toml", "v2 value"),
            ("config/new.toml", "fresh"),
            ("options.txt", "pack options v2"),
        ];
        let pack_file = pack(root.path(), &overrides);
        let new = PackState {
            version_id: "2".into(),
            files: BTreeMap::from([
                ("mods/a.jar".into(), sha(new_mod)),
                ("mods/kept.jar".into(), sha(new_mod)),
            ]),
            overrides: overrides
                .iter()
                .map(|(p, c)| ((*p).to_owned(), sha(c)))
                .collect(),
        };
        let mut report = UpdateReport::default();
        let mut rollback = Rollback {
            game_version: "1.20.1".into(),
            loader: Loader::Fabric,
            loader_version: None,
            origin: None,
            state: Some(old.clone()),
            created: Vec::new(),
            saved: Vec::new(),
        };
        apply(
            &data,
            &game,
            &pack_file,
            Some(&old),
            &new,
            &mut report,
            &mut rollback,
        )
        .unwrap();

        let read = |rel: &str| std::fs::read_to_string(game.join(rel)).ok();
        assert_eq!(read("mods/a.jar").as_deref(), Some(new_mod));
        assert_eq!(read("mods/gone.jar"), None);
        assert_eq!(
            read("mods/kept.jar.disabled").as_deref(),
            Some(new_mod),
            "stays disabled"
        );
        assert_eq!(read("mods/kept.jar"), None);
        assert_eq!(read("config/tuned.toml").as_deref(), Some("player value"));
        assert_eq!(read("config/plain.toml").as_deref(), Some("v2 value"));
        assert_eq!(read("config/obsolete.toml"), None);
        assert_eq!(read("config/new.toml").as_deref(), Some("fresh"));
        assert_eq!(read("options.txt").as_deref(), Some("player options"));
        assert_eq!(report.kept, ["config/tuned.toml"]);
        assert_eq!((report.added, report.replaced, report.removed), (1, 3, 2));
        assert!(rollback.created.contains(&"config/new.toml".to_owned()));
        assert!(game
            .join(ROLLBACK_DIR)
            .join("files/mods/gone.jar")
            .is_file());
    }

    #[test]
    fn reads_pack_contents() {
        let root = tempfile::tempdir().unwrap();
        let file = pack(root.path(), &[("config/a.toml", "x")]);
        let entries = override_entries(&file).unwrap();
        assert!(entries.contains_key("config/a.toml"));
    }
}
