//! CurseForge modpacks (`.zip` with a `manifest.json`). Files are resolved through the
//! CurseForge API with the player's own key. Files whose authors forbid third-party
//! downloads are looked up on Modrinth by SHA-1 first; what is left must be downloaded
//! by hand, and is then picked up from the player's Downloads folder.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{modrinth, mrpack};
use crate::context::Context;
use crate::download::{self, DownloadTask};
use crate::error::{Error, Result};
use crate::install::{InstallProgress, Stage};
use crate::instance::{Instance, NewInstance};
use crate::meta::loader::Loader;
use crate::{secrets, store};

const API: &str = "https://api.curseforge.com/v1";
/// Secret name of the player's CurseForge key.
pub const KEY_SECRET: &str = "curseforge-key";
const MANUAL_FILE: &str = ".tandem/manual-downloads.json";

/// CurseForge "class" (project type) ids.
const CLASS_MODS: u32 = 6;
const CLASS_RESOURCE_PACKS: u32 = 12;
const CLASS_SHADERS: u32 = 6552;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub minecraft: ManifestMinecraft,
    #[serde(default)]
    pub manifest_type: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub files: Vec<ManifestFile>,
    #[serde(default = "default_overrides")]
    pub overrides: String,
}

fn default_overrides() -> String {
    "overrides".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestMinecraft {
    pub version: String,
    #[serde(default)]
    pub mod_loaders: Vec<ManifestLoader>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestLoader {
    pub id: String,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestFile {
    #[serde(rename = "projectID")]
    pub project_id: u64,
    #[serde(rename = "fileID")]
    pub file_id: u64,
    #[serde(default = "yes")]
    pub required: bool,
}

fn yes() -> bool {
    true
}

impl Manifest {
    /// The instance the pack needs: game version and loader from `forge-47.2.0`-style ids.
    pub fn new_instance(&self) -> Result<NewInstance> {
        let loader = self
            .minecraft
            .mod_loaders
            .iter()
            .find(|l| l.primary)
            .or(self.minecraft.mod_loaders.first());
        let (loader, loader_version) = match loader {
            Some(l) => crate::launchers::parse_loader_id(&l.id, &self.minecraft.version)?,
            None => (Loader::Vanilla, None),
        };
        Ok(NewInstance {
            name: if self.name.trim().is_empty() {
                "Modpack CurseForge".into()
            } else {
                self.name.chars().take(64).collect()
            },
            game_version: self.minecraft.version.clone(),
            loader,
            loader_version,
        })
    }
}

/// The player's saved CurseForge key, if any.
pub async fn stored_key() -> Result<Option<String>> {
    mrpack::blocking(|| secrets::get(KEY_SECRET)).await
}

/// Checks `key` against the API, then saves it. An empty key deletes the saved one.
pub async fn save_key(ctx: &Context, key: &str) -> Result<()> {
    let key = key.trim().to_owned();
    if !key.is_empty() {
        check_key(ctx, &key).await?;
    }
    mrpack::blocking(move || secrets::set(KEY_SECRET, &key)).await
}

/// Asks the API for Minecraft's game entry: fails when the key is refused.
pub async fn check_key(ctx: &Context, key: &str) -> Result<()> {
    let url = format!("{API}/games/432");
    let response = ctx.http.get(&url).header("x-api-key", key).send().await?;
    match response.status().as_u16() {
        200..=299 => Ok(()),
        401 | 403 => Err(Error::InvalidInput(
            "Clé CurseForge refusée : vérifie qu'elle est complète".into(),
        )),
        status => Err(Error::HttpStatus { url, status }),
    }
}

/// [`read_manifest`] off the async runtime.
pub async fn inspect(pack: &Path) -> Result<Option<Manifest>> {
    let pack = pack.to_owned();
    mrpack::blocking(move || read_manifest(&pack)).await
}

/// [`collect_manual`] from the player's Downloads folder, off the async runtime.
pub async fn collect_from_downloads(game_dir: &Path) -> Result<Vec<ManualFile>> {
    let game_dir = game_dir.to_owned();
    mrpack::blocking(move || match downloads_dir() {
        Some(downloads) => collect_manual(&game_dir, &downloads),
        None => Ok(manual_downloads(&game_dir)),
    })
    .await
}

/// Reads `manifest.json` from a CurseForge pack. `None` when the zip is not one.
pub fn read_manifest(pack: &Path) -> Result<Option<Manifest>> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(pack)?)?;
    let Ok(mut entry) = archive.by_name("manifest.json") else {
        return Ok(None);
    };
    let mut json = String::new();
    entry.read_to_string(&mut json)?;
    let manifest: Manifest = serde_json::from_str(json.trim_start_matches('\u{feff}'))?;
    Ok(
        (manifest.manifest_type.is_empty() || manifest.manifest_type == "minecraftModpack")
            .then_some(manifest),
    )
}

/// A file the player must download from CurseForge's website.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManualFile {
    /// Project name.
    pub name: String,
    pub file_name: String,
    pub sha1: Option<String>,
    pub size: u64,
    /// Download page of this exact file.
    pub url: String,
    /// `mods`, `resourcepacks` or `shaderpacks`.
    pub folder: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFile {
    id: u64,
    mod_id: u64,
    file_name: String,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    hashes: Vec<CfHash>,
    #[serde(default)]
    file_length: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct CfHash {
    value: String,
    /// 1 = SHA-1, 2 = MD5.
    algo: u32,
}

impl CfFile {
    fn sha1(&self) -> Option<String> {
        self.hashes
            .iter()
            .find(|h| h.algo == 1)
            .map(|h| h.value.to_lowercase())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMod {
    id: u64,
    name: String,
    #[serde(default)]
    class_id: Option<u32>,
    #[serde(default)]
    links: Option<CfLinks>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfLinks {
    #[serde(default)]
    website_url: Option<String>,
}

#[derive(Deserialize)]
struct Data<T> {
    data: T,
}

async fn post<T: for<'de> Deserialize<'de>>(
    ctx: &Context,
    key: &str,
    path: &str,
    body: &serde_json::Value,
) -> Result<T> {
    let response = ctx
        .http
        .post(format!("{API}{path}"))
        .header("x-api-key", key)
        .json(body)
        .send()
        .await?;
    match response.status().as_u16() {
        200..=299 => Ok(response.json::<Data<T>>().await?.data),
        401 | 403 => Err(Error::InvalidInput(
            "Clé CurseForge refusée : vérifie-la dans les réglages".into(),
        )),
        status => Err(Error::HttpStatus {
            url: format!("{API}{path}"),
            status,
        }),
    }
}

fn folder_of(class_id: Option<u32>, file_name: &str) -> &'static str {
    match class_id {
        Some(CLASS_RESOURCE_PACKS) => "resourcepacks",
        Some(CLASS_SHADERS) => "shaderpacks",
        Some(CLASS_MODS) => "mods",
        _ if file_name.ends_with(".zip") => "resourcepacks",
        _ => "mods",
    }
}

fn safe_file_name(name: &str) -> bool {
    !name.is_empty() && !name.contains(['/', '\\', ':']) && name != "." && name != ".."
}

/// Installs a CurseForge pack into an (already created) instance. Returns the files
/// left to download by hand, also saved in the instance.
pub async fn install<F>(
    ctx: &Context,
    instance: &Instance,
    pack: &Path,
    manifest: &Manifest,
    key: &str,
    on_progress: F,
) -> Result<Vec<ManualFile>>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    on_progress(InstallProgress {
        stage: Stage::Metadata,
        download: Default::default(),
    });
    let game_dir = ctx.data.instance_dir(&instance.id);
    let wanted: Vec<&ManifestFile> = manifest.files.iter().filter(|f| f.required).collect();
    let mut files: Vec<CfFile> = Vec::new();
    for chunk in wanted.chunks(500) {
        let ids: Vec<u64> = chunk.iter().map(|f| f.file_id).collect();
        files.extend(
            post::<Vec<CfFile>>(
                ctx,
                key,
                "/mods/files",
                &serde_json::json!({ "fileIds": ids }),
            )
            .await?,
        );
    }
    let mod_ids: Vec<u64> = files.iter().map(|f| f.mod_id).collect();
    let mut mods: HashMap<u64, CfMod> = HashMap::new();
    for chunk in mod_ids.chunks(500) {
        for m in post::<Vec<CfMod>>(
            ctx,
            key,
            "/mods",
            &serde_json::json!({ "modIds": chunk, "filterPcOnly": false }),
        )
        .await?
        {
            mods.insert(m.id, m);
        }
    }

    // Files CurseForge may not hand out: the same file on Modrinth, found by SHA-1.
    let blocked: Vec<String> = files
        .iter()
        .filter(|f| f.download_url.is_none())
        .filter_map(CfFile::sha1)
        .collect();
    let on_modrinth = modrinth::versions_by_hash(ctx, &blocked)
        .await
        .unwrap_or_default();

    let mut tasks = Vec::new();
    let mut placements: Vec<(PathBuf, String)> = Vec::new();
    let mut manual = Vec::new();
    for file in &files {
        if !safe_file_name(&file.file_name) {
            continue;
        }
        let project = mods.get(&file.mod_id);
        let folder = folder_of(project.and_then(|m| m.class_id), &file.file_name);
        let sha1 = file.sha1();
        let url = file.download_url.clone().or_else(|| {
            let sha1 = sha1.as_ref()?;
            let version = on_modrinth.get(sha1)?;
            version
                .files
                .iter()
                .find(|f| f.hashes.sha1 == *sha1)
                .map(|f| f.url.clone())
        });
        match (url, &sha1) {
            (Some(url), Some(sha1)) => {
                tasks.push(DownloadTask {
                    url,
                    dest: store::path(&ctx.data, sha1),
                    sha1: Some(sha1.clone()),
                    size: Some(file.file_length),
                });
                placements.push((PathBuf::from(folder).join(&file.file_name), sha1.clone()));
            }
            _ => manual.push(ManualFile {
                name: project.map_or_else(|| file.file_name.clone(), |m| m.name.clone()),
                file_name: file.file_name.clone(),
                sha1,
                size: file.file_length,
                url: project
                    .and_then(|m| m.links.as_ref()?.website_url.clone())
                    .map_or_else(
                        || format!("https://www.curseforge.com/projects/{}", file.mod_id),
                        |site| format!("{site}/files/{}", file.id),
                    ),
                folder: folder.to_owned(),
            }),
        }
    }
    download::download_all(&ctx.http, tasks, download::DEFAULT_CONCURRENCY, |p| {
        on_progress(InstallProgress {
            stage: Stage::Downloading,
            download: p,
        })
    })
    .await?;

    on_progress(InstallProgress {
        stage: Stage::Finalizing,
        download: Default::default(),
    });
    for (relative, sha1) in &placements {
        store::link(&store::path(&ctx.data, sha1), &game_dir.join(relative)).await?;
    }
    let (pack_path, dir, prefix) = (
        pack.to_owned(),
        game_dir.clone(),
        format!("{}/", manifest.overrides.trim_end_matches('/')),
    );
    mrpack::blocking(move || extract(&pack_path, &dir, &prefix)).await?;
    if let Err(err) = mrpack::track(ctx, instance, &placements).await {
        tracing::warn!(instance = %instance.id, error = %err, "could not identify CurseForge files on Modrinth");
    }
    save_manual(&game_dir, &manual)?;
    tracing::info!(instance = %instance.id, pack = %manifest.name, files = placements.len(), manual = manual.len(), "CurseForge pack installed");
    Ok(manual)
}

/// Copies the overrides folder of the pack over the instance.
fn extract(pack: &Path, game_dir: &Path, prefix: &str) -> Result<()> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(pack)?)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let name = name.to_string_lossy().replace('\\', "/");
        let Some(relative) = name.strip_prefix(prefix).and_then(mrpack::safe_relative) else {
            continue;
        };
        let out = game_dir.join(relative);
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::io::copy(&mut entry, &mut std::fs::File::create(&out)?)?;
    }
    Ok(())
}

fn save_manual(game_dir: &Path, manual: &[ManualFile]) -> Result<()> {
    let path = game_dir.join(MANUAL_FILE);
    if manual.is_empty() {
        let _ = std::fs::remove_file(path);
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(manual)?)?;
    Ok(())
}

/// Files of the instance still waiting to be downloaded by hand.
pub fn manual_downloads(game_dir: &Path) -> Vec<ManualFile> {
    std::fs::read(game_dir.join(MANUAL_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Looks for the awaited files in the Downloads folder (by SHA-1, else by name and
/// size), copies those found into the instance, and returns what is still missing.
pub fn collect_manual(game_dir: &Path, downloads: &Path) -> Result<Vec<ManualFile>> {
    let waiting = manual_downloads(game_dir);
    let candidates: Vec<PathBuf> = std::fs::read_dir(downloads)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .collect()
        })
        .unwrap_or_default();
    let mut missing = Vec::new();
    for file in waiting {
        let found = candidates.iter().find(|path| {
            let Ok(meta) = std::fs::metadata(path) else {
                return false;
            };
            if meta.len() != file.size && file.size > 0 {
                return false;
            }
            match &file.sha1 {
                Some(sha1) => std::fs::read(path)
                    .is_ok_and(|bytes| sha1_smol::Sha1::from(&bytes).digest().to_string() == *sha1),
                None => path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy() == file.file_name),
            }
        });
        match found {
            Some(path) => {
                let dest = game_dir.join(&file.folder).join(&file.file_name);
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(path, dest)?;
            }
            None => missing.push(file),
        }
    }
    save_manual(game_dir, &missing)?;
    Ok(missing)
}

/// The player's Downloads folder.
pub fn downloads_dir() -> Option<PathBuf> {
    dirs::download_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "minecraft": {"version": "1.20.1", "modLoaders": [{"id": "forge-47.2.0", "primary": true}]},
        "manifestType": "minecraftModpack", "manifestVersion": 1, "name": "All The Mods 9", "version": "0.2.60",
        "author": "ATMTeam", "files": [{"projectID": 238222, "fileID": 4712862, "required": true}, {"projectID": 1, "fileID": 2, "required": false}],
        "overrides": "overrides"
    }"#;

    #[test]
    fn reads_manifests() {
        let manifest: Manifest = serde_json::from_str(MANIFEST).unwrap();
        let new = manifest.new_instance().unwrap();
        assert_eq!(
            (
                new.game_version.as_str(),
                new.loader,
                new.loader_version.as_deref()
            ),
            ("1.20.1", Loader::Forge, Some("47.2.0"))
        );
        assert_eq!(new.name, "All The Mods 9");
        assert_eq!(manifest.files.iter().filter(|f| f.required).count(), 1);

        let neo: Manifest = serde_json::from_str(r#"{"minecraft": {"version": "1.20.1", "modLoaders": [{"id": "neoforge-1.20.1-47.1.99"}]}}"#).unwrap();
        assert_eq!(
            neo.new_instance().unwrap().loader_version.as_deref(),
            Some("47.1.99")
        );
        let fabric: Manifest = serde_json::from_str(
            r#"{"minecraft": {"version": "1.21.1", "modLoaders": [{"id": "fabric-0.16.5"}]}}"#,
        )
        .unwrap();
        assert_eq!(fabric.new_instance().unwrap().loader, Loader::Fabric);
        let odd: Manifest = serde_json::from_str(
            r#"{"minecraft": {"version": "1.7.10", "modLoaders": [{"id": "liteloader-1"}]}}"#,
        )
        .unwrap();
        assert!(odd.new_instance().is_err());
    }

    #[test]
    fn places_files_by_type() {
        assert_eq!(folder_of(Some(CLASS_SHADERS), "x.zip"), "shaderpacks");
        assert_eq!(folder_of(None, "pack.zip"), "resourcepacks");
        assert_eq!(folder_of(None, "mod.jar"), "mods");
        assert!(!safe_file_name("../evil.jar"));
    }

    #[test]
    fn collects_manual_downloads() {
        let dir = tempfile::tempdir().unwrap();
        let (game, downloads) = (dir.path().join("game"), dir.path().join("dl"));
        std::fs::create_dir_all(&downloads).unwrap();
        std::fs::write(downloads.join("renamed (1).jar"), "jar bytes").unwrap();
        let sha1 = sha1_smol::Sha1::from("jar bytes").digest().to_string();
        let waiting = vec![
            ManualFile {
                name: "A".into(),
                file_name: "a-1.0.jar".into(),
                sha1: Some(sha1),
                size: 9,
                url: "u".into(),
                folder: "mods".into(),
            },
            ManualFile {
                name: "B".into(),
                file_name: "b.jar".into(),
                sha1: Some("00".into()),
                size: 3,
                url: "u".into(),
                folder: "mods".into(),
            },
        ];
        save_manual(&game, &waiting).unwrap();
        let missing = collect_manual(&game, &downloads).unwrap();
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].name, "B");
        assert_eq!(
            std::fs::read_to_string(game.join("mods/a-1.0.jar")).unwrap(),
            "jar bytes"
        );
        assert_eq!(manual_downloads(&game).len(), 1);
    }
}
