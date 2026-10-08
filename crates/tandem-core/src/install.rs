//! Makes a game version runnable: client jar, libraries, assets, natives and Java.

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::context::Context;
use crate::download::{self, DownloadTask, Progress};
use crate::error::{Error, Result};
use crate::instance::Instance;
use crate::java;
use crate::meta::loader::{self, Loader};
use crate::meta::rules::Environment;
use crate::meta::version::{LibraryFile, VersionJson};
use crate::meta::{self, AssetIndex, ASSETS_BASE_URL};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Metadata,
    Downloading,
    Finalizing,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub stage: Stage,
    #[serde(flatten)]
    pub download: Progress,
}

/// Everything the launch step needs once files are in place.
#[derive(Debug, Clone)]
pub struct PreparedVersion {
    pub version: VersionJson,
    pub java: PathBuf,
    pub classpath: Vec<PathBuf>,
    pub natives_dir: PathBuf,
    /// Directory for `${game_assets}` (legacy virtual / resources layouts).
    pub game_assets: PathBuf,
}

/// What to install: a vanilla version, optionally with a mod loader on top.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub game_version: &'a str,
    pub loader: Loader,
    pub loader_version: Option<&'a str>,
}

impl<'a> Target<'a> {
    pub fn vanilla(game_version: &'a str) -> Self {
        Self {
            game_version,
            loader: Loader::Vanilla,
            loader_version: None,
        }
    }

    pub fn of(instance: &'a Instance) -> Self {
        Self {
            game_version: &instance.game_version,
            loader: instance.loader,
            loader_version: instance.loader_version.as_deref(),
        }
    }
}

pub async fn prepare<F>(
    ctx: &Context,
    target: Target<'_>,
    game_dir: &Path,
    on_progress: F,
) -> Result<PreparedVersion>
where
    F: Fn(InstallProgress) + Send + Sync,
{
    let stage = |stage| {
        on_progress(InstallProgress {
            stage,
            download: Progress::default(),
        })
    };
    stage(Stage::Metadata);

    let env = Environment::current();
    let mut version = meta::load_version(ctx, target.game_version).await?;
    if target.loader != Loader::Vanilla {
        let loader_version = target
            .loader_version
            .ok_or_else(|| Error::InvalidInput(format!("no {} version selected", target.loader)))?;
        let profile =
            loader::load_profile(ctx, target.loader, target.game_version, loader_version).await?;
        tracing::info!(profile = %profile.id, "applying loader profile");
        version.apply_loader(profile);
    }
    let version_dir = ctx.data.version_dir(&version.id);
    let libraries_dir = ctx.data.libraries();

    let mut tasks = Vec::new();
    let mut classpath = Vec::new();
    let mut native_jars = Vec::new();

    for library in version.libraries.iter().filter(|l| l.is_allowed(&env)) {
        if let Some(file) = library.artifact() {
            let dest = libraries_dir.join(&file.path);
            classpath.push(dest.clone());
            tasks.push(library_task(file, dest));
        }
        if let Some(file) = library.native_artifact(&env) {
            let dest = libraries_dir.join(&file.path);
            let exclude = library
                .extract
                .as_ref()
                .map(|e| e.exclude.clone())
                .unwrap_or_default();
            native_jars.push((dest.clone(), exclude));
            tasks.push(library_task(file, dest));
        }
    }

    let client = &version.downloads.client;
    let client_jar = version_dir.join(format!("{}.jar", version.id));
    classpath.push(client_jar.clone());
    tasks.push(artifact_task(
        &client.url,
        client_jar,
        &client.sha1,
        client.size,
    ));

    let assets = meta::load_asset_index(ctx, &version).await?;
    let objects_dir = ctx.data.assets().join("objects");
    for object in assets.objects.values() {
        let relative = object.relative_path();
        tasks.push(DownloadTask {
            url: format!("{ASSETS_BASE_URL}/{relative}"),
            dest: objects_dir.join(&relative),
            sha1: Some(object.hash.clone()),
            size: Some(object.size),
        });
    }

    let component = version
        .java_version
        .as_ref()
        .map_or(java::LEGACY_COMPONENT, |j| j.component.as_str());
    let runtime = java::plan_runtime(ctx, component).await?;
    tasks.extend(runtime.downloads.iter().cloned());

    download::download_all(&ctx.http, tasks, download::DEFAULT_CONCURRENCY, |p| {
        on_progress(InstallProgress {
            stage: Stage::Downloading,
            download: p,
        })
    })
    .await?;

    stage(Stage::Finalizing);
    java::finalize_runtime(&runtime).await?;

    let natives_dir = version_dir.join("natives");
    let game_assets = legacy_assets_dir(ctx, &assets, &version.assets, game_dir);
    {
        let natives_dir = natives_dir.clone();
        let objects_dir = objects_dir.clone();
        let game_assets = game_assets.clone();
        tokio::task::spawn_blocking(move || -> Result<()> {
            extract_natives(&native_jars, &natives_dir)?;
            if assets.is_virtual || assets.map_to_resources {
                copy_legacy_assets(&assets, &objects_dir, &game_assets)?;
            }
            Ok(())
        })
        .await
        .map_err(|e| Error::Io(io::Error::other(e)))??;
    }

    tracing::info!(version = %version.id, "version ready");
    Ok(PreparedVersion {
        version,
        java: runtime.java_executable,
        classpath,
        natives_dir,
        game_assets,
    })
}

fn artifact_task(url: &str, dest: PathBuf, sha1: &str, size: u64) -> DownloadTask {
    DownloadTask {
        url: url.to_owned(),
        dest,
        sha1: Some(sha1.to_owned()),
        size: Some(size),
    }
}

fn library_task(file: LibraryFile, dest: PathBuf) -> DownloadTask {
    DownloadTask {
        url: file.url,
        dest,
        sha1: file.sha1,
        size: file.size,
    }
}

fn legacy_assets_dir(
    ctx: &Context,
    assets: &AssetIndex,
    index_id: &str,
    game_dir: &Path,
) -> PathBuf {
    if assets.map_to_resources {
        game_dir.join("resources")
    } else if assets.is_virtual {
        ctx.data.assets().join("virtual").join(index_id)
    } else {
        ctx.data.assets()
    }
}

/// Extracts legacy native jars (LWJGL 2 era). Modern versions ship natives as
/// regular classpath jars that LWJGL unpacks itself, so this is often a no-op.
fn extract_natives(jars: &[(PathBuf, Vec<String>)], dest: &Path) -> Result<()> {
    if jars.is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(dest)?;
    for (jar, exclude) in jars {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(jar)?)?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let Some(relative) = entry.enclosed_name() else {
                continue;
            };
            let name = relative.to_string_lossy().replace('\\', "/");
            if entry.is_dir()
                || exclude
                    .iter()
                    .any(|prefix| name.starts_with(prefix.as_str()))
            {
                continue;
            }
            let out = dest.join(&relative);
            if out.is_file() {
                continue;
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            io::copy(&mut entry, &mut std::fs::File::create(&out)?)?;
        }
    }
    Ok(())
}

fn copy_legacy_assets(assets: &AssetIndex, objects_dir: &Path, dest: &Path) -> Result<()> {
    for (name, object) in &assets.objects {
        let target = dest.join(name);
        if target.is_file() {
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(objects_dir.join(object.relative_path()), &target)?;
    }
    Ok(())
}
