//! Modrinth modpacks (`.mrpack`, https://support.modrinth.com/en/articles/8802351):
//! a zip holding `modrinth.index.json` (files to download) plus `overrides/` and
//! `client-overrides/` folders copied over the instance.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{modrinth, ContentKind, InstalledContent};
use crate::context::Context;
use crate::download::{self, DownloadTask};
use crate::error::{Error, Result};
use crate::install::{InstallProgress, Stage};
use crate::instance::{Instance, NewInstance};
use crate::meta::loader::Loader;
use crate::store;

const INDEX_FILE: &str = "modrinth.index.json";
const OVERRIDES: &str = "overrides/";
const CLIENT_OVERRIDES: &str = "client-overrides/";

/// Hosts a pack may download from, as required by the format specification.
const ALLOWED_HOSTS: &[&str] = &[
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
];

/// Instance folders never exported: worlds, logs and caches are personal or generated.
const EXPORT_EXCLUDED: &[&str] = &[
    "saves",
    "logs",
    "crash-reports",
    "screenshots",
    ".cache",
    ".fabric",
    ".quilt",
    "natives",
    "downloads",
    "debug",
    "usercache.json",
    "usernamecache.json",
    "command_history.txt",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackIndex {
    pub format_version: u32,
    pub game: String,
    pub version_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub files: Vec<PackFile>,
    /// `minecraft` plus one of `fabric-loader`, `quilt-loader`, `forge`, `neoforge`.
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    pub path: String,
    pub hashes: PackHashes,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<PackEnv>,
    pub downloads: Vec<String>,
    pub file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackHashes {
    pub sha1: String,
    pub sha512: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackEnv {
    pub client: String,
    pub server: String,
}

impl PackFile {
    fn for_client(&self) -> bool {
        self.env.as_ref().is_none_or(|e| e.client != "unsupported")
    }
}

impl PackIndex {
    /// Game version, loader and loader version the pack needs.
    pub fn target(&self) -> Result<(String, Loader, Option<String>)> {
        let game = self
            .dependencies
            .get("minecraft")
            .cloned()
            .ok_or_else(|| Error::InvalidInput("modpack without a Minecraft version".into()))?;
        for (key, loader) in [
            ("fabric-loader", Loader::Fabric),
            ("quilt-loader", Loader::Quilt),
            ("neoforge", Loader::NeoForge),
            ("forge", Loader::Forge),
        ] {
            if let Some(version) = self.dependencies.get(key) {
                if matches!(loader, Loader::Forge | Loader::NeoForge) {
                    return Err(Error::LoaderNotSupported(loader.to_string()));
                }
                return Ok((game, loader, Some(version.clone())));
            }
        }
        Ok((game, Loader::Vanilla, None))
    }

    /// The instance a pack creates (the name can be overridden, e.g. by the project title).
    pub fn new_instance(&self, name: Option<&str>) -> Result<NewInstance> {
        let (game_version, loader, loader_version) = self.target()?;
        Ok(NewInstance {
            name: name.unwrap_or(&self.name).chars().take(64).collect(),
            game_version,
            loader,
            loader_version,
        })
    }
}

/// Relative path made only of normal components (no `..`, root or drive prefix).
fn safe_relative(path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    let ok = !path.as_os_str().is_empty()
        && path.components().all(|c| matches!(c, Component::Normal(_)));
    ok.then(|| path.to_owned())
}

fn allowed_url(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https" && u.host_str().is_some_and(|h| ALLOWED_HOSTS.contains(&h))
    })
}

pub async fn read_index(pack: &Path) -> Result<PackIndex> {
    let pack = pack.to_owned();
    blocking(move || {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&pack)?)?;
        let mut entry = archive
            .by_name(INDEX_FILE)
            .map_err(|_| Error::InvalidInput(format!("{INDEX_FILE} missing: not a .mrpack")))?;
        let mut json = String::new();
        entry.read_to_string(&mut json)?;
        let index: PackIndex = serde_json::from_str(&json)?;
        if index.format_version != 1 || index.game != "minecraft" {
            return Err(Error::InvalidInput("unsupported modpack format".into()));
        }
        Ok(index)
    })
    .await
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::Io(io::Error::other(e)))?
}

/// A modpack version downloaded from Modrinth into the cache.
pub struct DownloadedPack {
    pub path: PathBuf,
    pub project: modrinth::Project,
    pub version: modrinth::Version,
}

/// Downloads the newest release (else newest version) of a Modrinth modpack that
/// Tandem can launch.
pub async fn download(ctx: &Context, project_id: &str) -> Result<DownloadedPack> {
    let project = modrinth::project(ctx, project_id).await?;
    if project.project_type != "modpack" {
        return Err(Error::InvalidInput(format!(
            "{} is not a modpack",
            project.title
        )));
    }
    let versions = modrinth::pack_versions(ctx, &project.id, modrinth::MODPACK_LOADERS).await?;
    let release = versions.iter().position(|v| v.version_type == "release");
    let version = versions
        .into_iter()
        .nth(release.unwrap_or(0))
        .ok_or_else(|| Error::LoaderNotSupported(format!("{} (Forge/NeoForge)", project.title)))?;
    let file = version
        .primary_file()
        .ok_or_else(|| Error::InvalidInput(format!("{} has no file", project.title)))?;
    let path = ctx
        .data
        .cache()
        .join("modpacks")
        .join(format!("{}.mrpack", file.hashes.sha1));
    download::download_all(
        &ctx.http,
        vec![DownloadTask {
            url: file.url.clone(),
            dest: path.clone(),
            sha1: Some(file.hashes.sha1.clone()),
            size: Some(file.size),
        }],
        1,
        |_| {},
    )
    .await?;
    Ok(DownloadedPack {
        path,
        project,
        version,
    })
}

/// Installs a pack's files and overrides into an (already created) instance, then
/// records the files Modrinth knows so they can be updated and toggled like any content.
pub async fn install<F>(
    ctx: &Context,
    instance: &Instance,
    pack: &Path,
    index: &PackIndex,
    on_progress: F,
) -> Result<()>
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
    let mut placements = Vec::new();
    let mut tasks = Vec::new();
    for file in index.files.iter().filter(|f| f.for_client()) {
        let relative = safe_relative(&file.path)
            .ok_or_else(|| Error::InvalidInput(format!("unsafe path in modpack: {}", file.path)))?;
        let url = file
            .downloads
            .iter()
            .find(|u| allowed_url(u))
            .ok_or_else(|| {
                Error::InvalidInput(format!("{} has no allowed download URL", file.path))
            })?;
        let sha1 = file.hashes.sha1.to_lowercase();
        tasks.push(DownloadTask {
            url: url.clone(),
            dest: store::path(&ctx.data, &sha1),
            sha1: Some(sha1.clone()),
            size: Some(file.file_size),
        });
        placements.push((relative, sha1));
    }
    download::download_all(&ctx.http, tasks, download::DEFAULT_CONCURRENCY, |p| {
        on_progress(InstallProgress {
            stage: Stage::Downloading,
            download: p,
        })
    })
    .await?;

    stage(Stage::Finalizing);
    for (relative, sha1) in &placements {
        store::link(&store::path(&ctx.data, sha1), &game_dir.join(relative)).await?;
    }
    let (pack, dir) = (pack.to_owned(), game_dir.clone());
    blocking(move || extract_overrides(&pack, &dir)).await?;

    // Best effort: an untracked file still works, it just cannot be updated from the UI.
    if let Err(err) = track(ctx, instance, &placements).await {
        tracing::warn!(instance = %instance.id, error = %err, "could not identify modpack files");
    }
    tracing::info!(instance = %instance.id, pack = %index.name, version = %index.version_id, files = placements.len(), "modpack installed");
    Ok(())
}

/// Copies `overrides/` then `client-overrides/` (which wins) into the game directory.
fn extract_overrides(pack: &Path, game_dir: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(pack)?)?;
    for prefix in [OVERRIDES, CLIENT_OVERRIDES] {
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if entry.is_dir() {
                continue;
            }
            // `enclosed_name` rejects absolute paths and `..` (zip slip).
            let Some(name) = entry.enclosed_name() else {
                continue;
            };
            let name = name.to_string_lossy().replace('\\', "/");
            let Some(relative) = name.strip_prefix(prefix).and_then(safe_relative) else {
                continue;
            };
            let out = game_dir.join(relative);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            io::copy(&mut entry, &mut std::fs::File::create(&out)?)?;
        }
    }
    Ok(())
}

fn kind_of(relative: &Path) -> Option<ContentKind> {
    let mut components = relative.components();
    let folder = components.next()?.as_os_str().to_str()?;
    // Only files directly inside the folder are content the launcher manages.
    let _file = components.next()?;
    if components.next().is_some() {
        return None;
    }
    [
        ContentKind::Mod,
        ContentKind::ResourcePack,
        ContentKind::Shader,
    ]
    .into_iter()
    .find(|k| k.folder() == folder)
}

/// Records pack files that are Modrinth projects as regular instance content.
async fn track(ctx: &Context, instance: &Instance, placements: &[(PathBuf, String)]) -> Result<()> {
    let candidates: Vec<(&PathBuf, &String, ContentKind)> = placements
        .iter()
        .filter_map(|(path, sha1)| Some((path, sha1, kind_of(path)?)))
        .collect();
    let hashes: Vec<String> = candidates.iter().map(|(_, h, _)| (*h).clone()).collect();
    let versions = modrinth::versions_by_hash(ctx, &hashes).await?;
    let project_ids: Vec<String> = versions
        .values()
        .map(|v| v.project_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let projects: HashMap<String, modrinth::Project> = modrinth::projects(ctx, &project_ids)
        .await?
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();

    for (path, sha1, kind) in candidates {
        let Some(version) = versions.get(sha1) else {
            continue;
        };
        let Some(project) = projects.get(&version.project_id) else {
            continue;
        };
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        ctx.db
            .upsert_content(
                &instance.id,
                &InstalledContent {
                    project_id: project.id.clone(),
                    version_id: version.id.clone(),
                    kind,
                    title: project.title.clone(),
                    version_number: version.version_number.clone(),
                    file_name,
                    sha1: sha1.clone(),
                    icon_url: project.icon_url.clone(),
                    is_dependency: false,
                    enabled: true,
                    installed_at: String::new(),
                },
            )
            .await?;
    }
    Ok(())
}

/// Writes an instance as a `.mrpack`: enabled Modrinth content is referenced by URL,
/// everything else worth sharing (configs, untracked files) goes into `overrides/`.
pub async fn export(ctx: &Context, instance: &Instance, dest: &Path, version: &str) -> Result<()> {
    let content: Vec<InstalledContent> = ctx
        .db
        .list_content(&instance.id)
        .await?
        .into_iter()
        .filter(|c| c.enabled)
        .collect();
    let ids: Vec<String> = content.iter().map(|c| c.version_id.clone()).collect();
    let files_by_sha1: HashMap<String, modrinth::VersionFile> = modrinth::versions(ctx, &ids)
        .await?
        .into_iter()
        .flat_map(|v| v.files)
        .map(|f| (f.hashes.sha1.clone(), f))
        .collect();

    let mut files = Vec::new();
    let mut referenced: HashSet<PathBuf> = HashSet::new();
    for item in &content {
        let Some(file) = files_by_sha1.get(&item.sha1) else {
            continue;
        };
        let Some(sha512) = file.hashes.sha512.clone() else {
            continue;
        };
        let path = format!("{}/{}", item.kind.folder(), item.file_name);
        referenced.insert(PathBuf::from(&path));
        files.push(PackFile {
            path,
            hashes: PackHashes {
                sha1: item.sha1.clone(),
                sha512,
            },
            env: None,
            downloads: vec![file.url.clone()],
            file_size: file.size,
        });
    }

    let mut dependencies =
        BTreeMap::from([("minecraft".to_owned(), instance.game_version.clone())]);
    match (instance.loader, &instance.loader_version) {
        (Loader::Fabric, Some(v)) => {
            dependencies.insert("fabric-loader".into(), v.clone());
        }
        (Loader::Quilt, Some(v)) => {
            dependencies.insert("quilt-loader".into(), v.clone());
        }
        (Loader::Vanilla, _) => {}
        (other, _) => return Err(Error::LoaderNotSupported(other.to_string())),
    }
    let index = PackIndex {
        format_version: 1,
        game: "minecraft".into(),
        version_id: version.to_owned(),
        name: instance.name.clone(),
        summary: None,
        files,
        dependencies,
    };

    let (game_dir, dest) = (ctx.data.instance_dir(&instance.id), dest.to_owned());
    let files_count = index.files.len();
    let overrides = blocking(move || write_pack(&index, &game_dir, &referenced, &dest)).await?;
    tracing::info!(instance = %instance.id, files = files_count, overrides, "modpack exported");
    Ok(())
}

/// Writes the zip; returns how many override files were included.
fn write_pack(
    index: &PackIndex,
    game_dir: &Path,
    referenced: &HashSet<PathBuf>,
    dest: &Path,
) -> Result<usize> {
    let tmp = dest.with_extension("mrpack.part");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&tmp)?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(INDEX_FILE, options)?;
    zip.write_all(&serde_json::to_vec_pretty(index)?)?;

    let mut count = 0;
    let mut stack = vec![game_dir.to_owned()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let Ok(relative) = path.strip_prefix(game_dir) else {
                continue;
            };
            let top = relative
                .components()
                .next()
                .and_then(|c| c.as_os_str().to_str())
                .unwrap_or_default();
            let name = relative.to_string_lossy().replace('\\', "/");
            if EXPORT_EXCLUDED.contains(&top) || name.ends_with(".disabled") {
                continue;
            }
            if entry.file_type()?.is_dir() {
                stack.push(path);
            } else if !referenced.contains(&PathBuf::from(&name)) {
                zip.start_file(format!("{OVERRIDES}{name}"), options)?;
                io::copy(&mut std::fs::File::open(&path)?, &mut zip)?;
                count += 1;
            }
        }
    }
    zip.finish()?;
    std::fs::rename(&tmp, dest)?;
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_paths_and_hosts() {
        assert!(safe_relative("mods/sodium.jar").is_some());
        assert!(safe_relative("../evil.jar").is_none());
        assert!(safe_relative("mods/../../evil.jar").is_none());
        assert!(safe_relative("/etc/passwd").is_none());
        assert!(safe_relative("C:/Windows/evil.dll").is_none());
        assert!(safe_relative("").is_none());

        assert!(allowed_url(
            "https://cdn.modrinth.com/data/x/versions/y/a.jar"
        ));
        assert!(allowed_url(
            "https://github.com/a/b/releases/download/1/a.jar"
        ));
        assert!(!allowed_url("http://cdn.modrinth.com/a.jar"));
        assert!(!allowed_url("https://evil.example/a.jar"));
        assert!(!allowed_url("https://cdn.modrinth.com.evil.example/a.jar"));
    }

    #[test]
    fn reads_targets() {
        let json = r#"{"formatVersion":1,"game":"minecraft","versionId":"1.0","name":"Pack",
            "files":[{"path":"mods/a.jar","hashes":{"sha1":"a","sha512":"b"},
                      "env":{"client":"unsupported","server":"required"},
                      "downloads":["https://cdn.modrinth.com/a.jar"],"fileSize":1}],
            "dependencies":{"minecraft":"1.21.4","fabric-loader":"0.16.10"}}"#;
        let index: PackIndex = serde_json::from_str(json).unwrap();
        let (game, loader, version) = index.target().unwrap();
        assert_eq!(
            (game.as_str(), loader, version.as_deref()),
            ("1.21.4", Loader::Fabric, Some("0.16.10"))
        );
        assert!(!index.files[0].for_client());

        let neo: PackIndex = serde_json::from_str(
            &json.replace(r#""fabric-loader":"0.16.10""#, r#""neoforge":"21.1.77""#),
        )
        .unwrap();
        assert!(matches!(neo.target(), Err(Error::LoaderNotSupported(_))));
    }

    #[test]
    fn classifies_content_paths() {
        assert_eq!(kind_of(Path::new("mods/a.jar")), Some(ContentKind::Mod));
        assert_eq!(
            kind_of(Path::new("shaderpacks/b.zip")),
            Some(ContentKind::Shader)
        );
        assert_eq!(kind_of(Path::new("mods/sub/a.jar")), None);
        assert_eq!(kind_of(Path::new("config/a.toml")), None);
    }

    #[tokio::test]
    async fn extracts_overrides_safely() {
        let tmp = tempfile::tempdir().unwrap();
        let pack = tmp.path().join("p.mrpack");
        {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&pack).unwrap());
            let options = zip::write::SimpleFileOptions::default();
            for (name, body) in [
                ("overrides/config/a.toml", "base"),
                ("overrides/options.txt", "base"),
                ("client-overrides/options.txt", "client"),
                ("overrides/../escape.txt", "evil"),
            ] {
                zip.start_file(name, options).unwrap();
                zip.write_all(body.as_bytes()).unwrap();
            }
            zip.finish().unwrap();
        }
        let game = tmp.path().join("game");
        extract_overrides(&pack, &game).unwrap();
        assert_eq!(
            std::fs::read_to_string(game.join("config/a.toml")).unwrap(),
            "base"
        );
        assert_eq!(
            std::fs::read_to_string(game.join("options.txt")).unwrap(),
            "client"
        );
        assert!(!tmp.path().join("escape.txt").exists());
    }
}
