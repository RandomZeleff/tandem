//! Singleplayer worlds of an instance (`saves/`) and their zip backups, kept in
//! `backups/<instance>/<world>/<unix ms>-<kind>.zip`.

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::paths::DataDir;

/// Automatic backups kept per world; manual ones are never pruned.
pub const KEEP_AUTO_BACKUPS: usize = 5;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct World {
    /// Folder name in `saves/`, which identifies the world.
    pub folder: String,
    pub name: String,
    /// Unix milliseconds.
    pub last_played: Option<i64>,
    pub game_mode: GameMode,
    pub version: Option<String>,
    pub size_bytes: u64,
    /// `icon.png` as a data URL.
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Spectator,
    Hardcore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackupKind {
    Manual,
    /// Made after a play session.
    Auto,
    /// The world as it was right before a restore replaced it.
    BeforeRestore,
}

impl BackupKind {
    fn as_str(self) -> &'static str {
        match self {
            BackupKind::Manual => "manual",
            BackupKind::Auto => "auto",
            BackupKind::BeforeRestore => "before-restore",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        [Self::Manual, Self::Auto, Self::BeforeRestore]
            .into_iter()
            .find(|k| k.as_str() == s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub world: String,
    /// File name inside the world's backup folder.
    pub file_name: String,
    pub created_at: i64,
    pub kind: BackupKind,
    pub size_bytes: u64,
}

#[derive(Deserialize)]
struct LevelDat {
    #[serde(rename = "Data")]
    data: LevelData,
}

#[derive(Deserialize)]
struct LevelData {
    #[serde(rename = "LevelName")]
    level_name: Option<String>,
    #[serde(rename = "LastPlayed")]
    last_played: Option<i64>,
    #[serde(rename = "GameType")]
    game_type: Option<i32>,
    hardcore: Option<i8>,
    #[serde(rename = "Version")]
    version: Option<VersionTag>,
}

#[derive(Deserialize)]
struct VersionTag {
    #[serde(rename = "Name")]
    name: Option<String>,
}

fn saves(game_dir: &Path) -> PathBuf {
    game_dir.join("saves")
}

fn backups_of(data: &DataDir, instance_id: &str, world: &str) -> PathBuf {
    data.backups().join(instance_id).join(world)
}

/// A world folder name is a single plain path component.
fn check_world(world: &str) -> Result<()> {
    let mut components = Path::new(world).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => Err(Error::InvalidInput(format!(
            "Nom de monde invalide : {world}"
        ))),
    }
}

/// Worlds in `saves/`, most recently played first.
pub fn list(game_dir: &Path) -> Vec<World> {
    let Ok(entries) = std::fs::read_dir(saves(game_dir)) else {
        return Vec::new();
    };
    let mut worlds: Vec<World> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("level.dat").is_file())
        .filter_map(|p| read_world(&p))
        .collect();
    worlds.sort_by_key(|w| std::cmp::Reverse(w.last_played));
    worlds
}

fn read_world(dir: &Path) -> Option<World> {
    let folder = dir.file_name()?.to_string_lossy().into_owned();
    let level = read_level(&dir.join("level.dat"));
    let data = level.as_ref().map(|l| &l.data);
    let game_mode = match (
        data.and_then(|d| d.hardcore),
        data.and_then(|d| d.game_type),
    ) {
        (Some(h), _) if h != 0 => GameMode::Hardcore,
        (_, Some(1)) => GameMode::Creative,
        (_, Some(2)) => GameMode::Adventure,
        (_, Some(3)) => GameMode::Spectator,
        _ => GameMode::Survival,
    };
    let icon = std::fs::read(dir.join("icon.png")).ok().map(|png| {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        )
    });
    Some(World {
        name: data
            .and_then(|d| d.level_name.clone())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| folder.clone()),
        folder,
        last_played: data.and_then(|d| d.last_played),
        game_mode,
        version: data.and_then(|d| d.version.as_ref()?.name.clone()),
        size_bytes: dir_size(dir),
        icon,
    })
}

fn read_level(path: &Path) -> Option<LevelDat> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(file)
        .read_to_end(&mut bytes)
        .ok()?;
    fastnbt::from_bytes(&bytes).ok()
}

fn dir_size(dir: &Path) -> u64 {
    walk(dir)
        .iter()
        .filter_map(|p| p.metadata().ok())
        .map(|m| m.len())
        .sum()
}

/// Every file under `dir`, recursively.
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            match entry.file_type() {
                Ok(t) if t.is_dir() => pending.push(path),
                Ok(t) if t.is_file() => files.push(path),
                _ => {}
            }
        }
    }
    files
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// Zips a world into its backup folder.
pub fn backup(
    data: &DataDir,
    instance_id: &str,
    game_dir: &Path,
    world: &str,
    kind: BackupKind,
) -> Result<Backup> {
    check_world(world)?;
    let source = saves(game_dir).join(world);
    if !source.join("level.dat").is_file() {
        return Err(Error::InvalidInput(format!("Monde introuvable : {world}")));
    }
    let dir = backups_of(data, instance_id, world);
    std::fs::create_dir_all(&dir)?;
    let mut created_at = now_ms();
    // Two backups in the same millisecond would share a name.
    while dir.join(file_name(created_at, kind)).exists() {
        created_at += 1;
    }
    let name = file_name(created_at, kind);
    let tmp = dir.join(format!(".{name}.tmp"));

    let mut zip = zip::ZipWriter::new(std::fs::File::create(&tmp)?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for path in walk(&source) {
        // Held open by a running game, and meaningless in a backup.
        if path.file_name().is_some_and(|n| n == "session.lock") {
            continue;
        }
        let Ok(relative) = path.strip_prefix(&source) else {
            continue;
        };
        let entry = relative.to_string_lossy().replace('\\', "/");
        zip.start_file(entry, options)?;
        std::io::copy(&mut std::fs::File::open(&path)?, &mut zip)?;
    }
    zip.finish()?.flush()?;
    std::fs::rename(&tmp, dir.join(&name))?;

    let size_bytes = std::fs::metadata(dir.join(&name))?.len();
    tracing::info!(
        instance = instance_id,
        world,
        kind = kind.as_str(),
        size_bytes,
        "world backed up"
    );
    Ok(Backup {
        world: world.to_owned(),
        file_name: name,
        created_at,
        kind,
        size_bytes,
    })
}

fn file_name(created_at: i64, kind: BackupKind) -> String {
    format!("{created_at}-{}.zip", kind.as_str())
}

fn parse_file_name(name: &str) -> Option<(i64, BackupKind)> {
    let stem = name.strip_suffix(".zip")?;
    let (time, kind) = stem.split_once('-')?;
    Some((time.parse().ok()?, BackupKind::parse(kind)?))
}

/// Every backup of an instance, newest first.
pub fn list_backups(data: &DataDir, instance_id: &str) -> Vec<Backup> {
    let Ok(worlds) = std::fs::read_dir(data.backups().join(instance_id)) else {
        return Vec::new();
    };
    let mut backups = Vec::new();
    for world in worlds.filter_map(|e| e.ok()) {
        let world_name = world.file_name().to_string_lossy().into_owned();
        let Ok(files) = std::fs::read_dir(world.path()) else {
            continue;
        };
        for file in files.filter_map(|e| e.ok()) {
            let name = file.file_name().to_string_lossy().into_owned();
            let Some((created_at, kind)) = parse_file_name(&name) else {
                continue;
            };
            backups.push(Backup {
                world: world_name.clone(),
                file_name: name,
                created_at,
                kind,
                size_bytes: file.metadata().map_or(0, |m| m.len()),
            });
        }
    }
    backups.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    backups
}

/// Replaces a world with one of its backups. The current world is backed up first
/// (`BeforeRestore`), so a restore can always be undone.
pub fn restore(
    data: &DataDir,
    instance_id: &str,
    game_dir: &Path,
    world: &str,
    file_name: &str,
) -> Result<()> {
    check_world(world)?;
    if parse_file_name(file_name).is_none() {
        return Err(Error::InvalidInput(format!(
            "Sauvegarde invalide : {file_name}"
        )));
    }
    let archive_path = backups_of(data, instance_id, world).join(file_name);
    let target = saves(game_dir).join(world);
    if target.join("level.dat").is_file() {
        backup(
            data,
            instance_id,
            game_dir,
            world,
            BackupKind::BeforeRestore,
        )?;
    }

    let staging = saves(game_dir).join(format!(".{world}.restoring"));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    let mut archive = zip::ZipArchive::new(std::fs::File::open(&archive_path)?)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let out = staging.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::io::copy(&mut entry, &mut std::fs::File::create(&out)?)?;
    }
    if target.exists() {
        std::fs::remove_dir_all(&target)?;
    }
    std::fs::rename(&staging, &target)?;
    tracing::info!(instance = instance_id, world, file_name, "world restored");
    Ok(())
}

/// Backs up the worlds played since `since` (their `level.dat` changed), then keeps
/// only the newest [`KEEP_AUTO_BACKUPS`] automatic backups of each.
pub fn auto_backup(
    data: &DataDir,
    instance_id: &str,
    game_dir: &Path,
    since: SystemTime,
) -> Vec<Backup> {
    let mut made = Vec::new();
    for world in list(game_dir) {
        let level = saves(game_dir).join(&world.folder).join("level.dat");
        let played = level
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|t| t >= since);
        if !played {
            continue;
        }
        match backup(data, instance_id, game_dir, &world.folder, BackupKind::Auto) {
            Ok(b) => made.push(b),
            Err(err) => {
                tracing::warn!(instance = instance_id, world = %world.folder, error = %err, "auto backup failed");
                continue;
            }
        }
        prune_auto(data, instance_id, &world.folder);
    }
    made
}

fn prune_auto(data: &DataDir, instance_id: &str, world: &str) {
    let dir = backups_of(data, instance_id, world);
    let mut autos: Vec<Backup> = list_backups(data, instance_id)
        .into_iter()
        .filter(|b| b.world == world && b.kind == BackupKind::Auto)
        .collect();
    autos.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    for old in autos.iter().skip(KEEP_AUTO_BACKUPS) {
        let _ = std::fs::remove_file(dir.join(&old.file_name));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestLevel {
        #[serde(rename = "Data")]
        data: TestData,
    }

    #[derive(Serialize)]
    struct TestData {
        #[serde(rename = "LevelName")]
        level_name: String,
        #[serde(rename = "LastPlayed")]
        last_played: i64,
        #[serde(rename = "GameType")]
        game_type: i32,
        hardcore: i8,
    }

    fn make_world(game_dir: &Path, folder: &str, name: &str, game_type: i32) {
        let dir = saves(game_dir).join(folder);
        std::fs::create_dir_all(dir.join("region")).unwrap();
        let nbt = fastnbt::to_bytes(&TestLevel {
            data: TestData {
                level_name: name.into(),
                last_played: 1_700_000_000_000,
                game_type,
                hardcore: 0,
            },
        })
        .unwrap();
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&nbt).unwrap();
        std::fs::write(dir.join("level.dat"), gz.finish().unwrap()).unwrap();
        std::fs::write(dir.join("region/r.0.0.mca"), b"chunks").unwrap();
        std::fs::write(dir.join("session.lock"), b"lock").unwrap();
    }

    #[test]
    fn lists_backs_up_and_restores() {
        let tmp = tempfile::tempdir().unwrap();
        let data = DataDir::new(tmp.path().join("data"));
        let game_dir = tmp.path().join("instance");
        make_world(&game_dir, "New World", "Ma base", 1);

        let worlds = list(&game_dir);
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].name, "Ma base");
        assert_eq!(worlds[0].game_mode, GameMode::Creative);
        assert_eq!(worlds[0].last_played, Some(1_700_000_000_000));

        let b = backup(&data, "i", &game_dir, "New World", BackupKind::Manual).unwrap();
        assert_eq!(list_backups(&data, "i"), std::slice::from_ref(&b));

        // Break the world, then restore it.
        std::fs::write(
            saves(&game_dir).join("New World/region/r.0.0.mca"),
            b"griefed",
        )
        .unwrap();
        restore(&data, "i", &game_dir, "New World", &b.file_name).unwrap();
        let region = std::fs::read(saves(&game_dir).join("New World/region/r.0.0.mca")).unwrap();
        assert_eq!(region, b"chunks");
        assert!(!saves(&game_dir).join("New World/session.lock").exists());
        // The griefed state was kept as a before-restore backup.
        assert!(list_backups(&data, "i")
            .iter()
            .any(|x| x.kind == BackupKind::BeforeRestore));
    }

    #[test]
    fn auto_backups_are_pruned() {
        let tmp = tempfile::tempdir().unwrap();
        let data = DataDir::new(tmp.path().join("data"));
        let game_dir = tmp.path().join("instance");
        make_world(&game_dir, "w", "W", 0);
        backup(&data, "i", &game_dir, "w", BackupKind::Manual).unwrap();
        for _ in 0..KEEP_AUTO_BACKUPS + 2 {
            assert_eq!(auto_backup(&data, "i", &game_dir, UNIX_EPOCH).len(), 1);
        }
        let backups = list_backups(&data, "i");
        let autos = backups
            .iter()
            .filter(|b| b.kind == BackupKind::Auto)
            .count();
        assert_eq!(autos, KEEP_AUTO_BACKUPS);
        assert!(backups.iter().any(|b| b.kind == BackupKind::Manual));
        // Nothing played since now: no new backup.
        assert!(auto_backup(
            &data,
            "i",
            &game_dir,
            SystemTime::now() + std::time::Duration::from_secs(60)
        )
        .is_empty());
    }

    #[test]
    fn rejects_paths_outside_saves() {
        let tmp = tempfile::tempdir().unwrap();
        let data = DataDir::new(tmp.path().join("data"));
        assert!(backup(&data, "i", tmp.path(), "../x", BackupKind::Manual).is_err());
        assert!(restore(&data, "i", tmp.path(), "w", "../../etc.zip").is_err());
    }
}
