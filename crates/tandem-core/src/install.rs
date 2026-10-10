//! Makes a game version runnable: client jar, libraries, assets, natives and Java.

use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::context::Context;
use crate::download::{self, DownloadTask, Progress};
use crate::error::{Error, Result};
use crate::instance::Instance;
use crate::java;
use crate::meta::forge;
use crate::meta::loader::{self, Loader};
use crate::meta::rules::Environment;
use crate::meta::version::{Artifact, JavaVersion, LibraryDownloads, LibraryFile, VersionJson};
use crate::meta::{self, AssetIndex, ASSETS_BASE_URL};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Metadata,
    Downloading,
    Finalizing,
    /// Running the Forge / NeoForge installer steps (first launch only).
    Processing,
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
    /// Platform the game runs as (an Intel Mac under Rosetta 2 for old versions).
    pub env: Environment,
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

    let mut version = meta::load_version(ctx, target.game_version).await?;
    let mut forge_install = None;
    if target.loader != Loader::Vanilla {
        let loader_version = target.loader_version.ok_or_else(|| {
            Error::InvalidInput(format!("Aucune version de {} choisie", target.loader))
        })?;
        let profile = match target.loader {
            Loader::Forge | Loader::NeoForge => {
                let install =
                    forge::load(ctx, target.loader, target.game_version, loader_version).await?;
                let profile = install.profile.clone();
                forge_install = Some(install);
                profile
            }
            _ => {
                loader::load_profile(ctx, target.loader, target.game_version, loader_version)
                    .await?
            }
        };
        tracing::info!(profile = %profile.id, "applying loader profile");
        version.apply_loader(profile);
    }
    if needs_arm64_backport(&version) {
        backport_arm64(ctx, &mut version).await?;
    }
    let env = environment_for(&version);
    if env.os_name == "osx" {
        upgrade_jna(&mut version);
    }
    if env.is_rosetta() {
        tracing::info!(version = %version.id, "no arm64 natives, running through Rosetta 2");
        if !rosetta_installed() {
            return Err(Error::RosettaMissing);
        }
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
            // Forge leaves the URL empty for jars its processors generate.
            if !file.url.is_empty() {
                tasks.push(library_task(file, dest));
            }
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

    // Libraries the Forge installer's processors run with (not on the game classpath).
    if let Some(install) = &forge_install {
        for library in install
            .installer_libraries
            .iter()
            .filter(|l| l.is_allowed(&env))
        {
            if let Some(file) = library.artifact().filter(|f| !f.url.is_empty()) {
                let dest = libraries_dir.join(&file.path);
                tasks.push(library_task(file, dest));
            }
        }
    }

    let client = &version.downloads.client;
    let client_jar = version_dir.join(format!("{}.jar", version.id));
    classpath.push(client_jar.clone());
    tasks.push(artifact_task(
        &client.url,
        client_jar.clone(),
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
    let runtime = java::plan_runtime(ctx, component, &env).await?;
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
    if let Some(install) = &forge_install {
        if !install.is_processed().await {
            stage(Stage::Processing);
        }
        install
            .run_processors(&forge::ProcessorEnv {
                java: &runtime.java_executable,
                client_jar: &client_jar,
                game_version: target.game_version,
                libraries: &libraries_dir,
            })
            .await?;
    }

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
        env,
    })
}

fn has_arm64_natives(version: &VersionJson) -> bool {
    version
        .libraries
        .iter()
        .any(|l| l.name.ends_with(":natives-macos-arm64"))
}

fn is_apple_silicon() -> bool {
    let env = Environment::current();
    env.os_name == "osx" && env.arch == "arm64"
}

/// 1.18.x wants Java 17 and LWJGL 3.2, which Mojang only ships for Intel Macs. Java 17
/// arm64 exists, though, and LWJGL 3.3 runs these versions. Not 1.17: it sets a
/// window icon, which LWJGL 3.3 refuses on macOS with a fatal error.
fn needs_arm64_backport(version: &VersionJson) -> bool {
    is_apple_silicon()
        && !has_arm64_natives(version)
        && version
            .java_version
            .as_ref()
            .is_some_and(|j| j.component == "java-runtime-beta")
}

/// Runs the version natively: Java 17 arm64, with the LWJGL and objc-bridge
/// libraries of 1.19.2 (the first release shipping them for arm64).
async fn backport_arm64(ctx: &Context, version: &mut VersionJson) -> Result<()> {
    const DONOR: &str = "1.19.2";
    let swapped = |name: &str| {
        name.starts_with("org.lwjgl:") || name.starts_with("ca.weblite:java-objc-bridge:")
    };
    let donor = meta::load_version(ctx, DONOR).await?;
    version.libraries.retain(|l| !swapped(&l.name));
    // Intel natives left out: both native jars are the `org.lwjgl.natives` module, and
    // Forge's module layer keeps only one of them.
    version.libraries.extend(
        donor
            .libraries
            .into_iter()
            .filter(|l| swapped(&l.name) && !l.name.ends_with(":natives-macos")),
    );
    version.java_version = Some(JavaVersion {
        component: "java-runtime-gamma".into(),
        major_version: 17,
    });
    tracing::info!(version = %version.id, "running natively on Apple Silicon with LWJGL from {DONOR}");
    Ok(())
}

/// On Apple Silicon, versions with no arm64 LWJGL natives left (before 1.18) run as
/// an Intel Mac through Rosetta 2: Mojang ships no arm64 Java 8 either.
fn environment_for(version: &VersionJson) -> Environment {
    if is_apple_silicon() && !has_arm64_natives(version) {
        Environment::rosetta()
    } else {
        Environment::current()
    }
}

/// JNA before 5.13 aborts on recent macOS when a `dlopen` error message overflows its
/// buffer (oshi probing IOKit at startup), which kills 1.17 to 1.20.2 before the
/// first log line. Those versions get the JNA that Minecraft 1.20.4 ships.
fn upgrade_jna(version: &mut VersionJson) {
    const FIXED: &str = "5.13.0";
    const JARS: [(&str, &str, u64); 2] = [
        ("jna", "1200e7ebeedbe0d10062093f32925a912020e747", 1_879_325),
        (
            "jna-platform",
            "88e9a306715e9379f3122415ef4ae759a352640d",
            1_363_209,
        ),
    ];
    for library in &mut version.libraries {
        let mut parts = library.name.split(':');
        let (Some("net.java.dev.jna"), Some(artifact), Some(current), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let Some(&(artifact, sha1, size)) = JARS.iter().find(|(a, ..)| *a == artifact) else {
            continue;
        };
        if loader::compare_versions(current, FIXED).is_ge() {
            continue;
        }
        tracing::info!(
            from = current,
            to = FIXED,
            artifact,
            "upgrading JNA for macOS"
        );
        let path = format!("net/java/dev/jna/{artifact}/{FIXED}/{artifact}-{FIXED}.jar");
        library.name = format!("net.java.dev.jna:{artifact}:{FIXED}");
        library.downloads = Some(LibraryDownloads {
            artifact: Some(Artifact {
                url: format!("https://libraries.minecraft.net/{path}"),
                path: Some(path),
                sha1: sha1.to_owned(),
                size,
            }),
            classifiers: Default::default(),
        });
    }
}

pub fn rosetta_installed() -> bool {
    Path::new("/Library/Apple/usr/libexec/oah/libRosettaRuntime").exists()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrades_only_old_jna() {
        let mut version: VersionJson = serde_json::from_str(
            r#"{
                "id": "1.18.2", "type": "release", "mainClass": "M", "assets": "1.18",
                "assetIndex": {"id": "1.18", "sha1": "a", "size": 1, "url": "u"},
                "downloads": {"client": {"sha1": "b", "size": 2, "url": "c"}},
                "libraries": [
                    {"name": "net.java.dev.jna:jna:5.10.0"},
                    {"name": "net.java.dev.jna:jna-platform:5.15.0"},
                    {"name": "com.github.oshi:oshi-core:5.8.5"}
                ]
            }"#,
        )
        .unwrap();
        upgrade_jna(&mut version);
        let names: Vec<_> = version.libraries.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "net.java.dev.jna:jna:5.13.0",
                "net.java.dev.jna:jna-platform:5.15.0",
                "com.github.oshi:oshi-core:5.8.5"
            ]
        );
        let jar = version.libraries[0].artifact().unwrap();
        assert_eq!(jar.path, "net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar");
        assert_eq!(
            jar.url,
            "https://libraries.minecraft.net/net/java/dev/jna/jna/5.13.0/jna-5.13.0.jar"
        );
    }
}
