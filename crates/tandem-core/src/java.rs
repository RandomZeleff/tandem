//! Java runtimes downloaded from Mojang's runtime manifests (one folder per component).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::context::Context;
use crate::download::{fetch_bytes, DownloadTask};
use crate::error::{Error, Result};

const RUNTIME_INDEX_URL: &str = "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// Component used by versions whose JSON predates `javaVersion` (Java 8).
pub const LEGACY_COMPONENT: &str = "jre-legacy";

type RuntimeIndex = HashMap<String, HashMap<String, Vec<RuntimeEntry>>>;

#[derive(Debug, Deserialize)]
struct RuntimeEntry {
    manifest: ManifestRef,
}

#[derive(Debug, Deserialize)]
struct ManifestRef {
    sha1: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct RuntimeManifest {
    files: HashMap<String, RuntimeFile>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RuntimeFile {
    Directory,
    File {
        #[serde(default)]
        executable: bool,
        downloads: RuntimeDownloads,
    },
    Link {
        #[cfg_attr(windows, allow(dead_code))]
        target: String,
    },
}

#[derive(Debug, Deserialize)]
struct RuntimeDownloads {
    raw: RawDownload,
}

#[derive(Debug, Deserialize)]
struct RawDownload {
    sha1: String,
    size: u64,
    url: String,
}

/// What must happen to make a runtime usable.
#[derive(Debug)]
pub struct RuntimePlan {
    pub java_executable: PathBuf,
    pub downloads: Vec<DownloadTask>,
    /// Files to mark executable after download (Unix only).
    pub executables: Vec<PathBuf>,
    /// Symlinks to create after download (Unix only): (link, target).
    pub links: Vec<(PathBuf, String)>,
}

fn platform_key() -> Result<&'static str> {
    let key = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x64",
        ("windows", "x86") => "windows-x86",
        ("windows", "aarch64") => "windows-arm64",
        ("macos", "aarch64") => "mac-os-arm64",
        ("macos", _) => "mac-os",
        ("linux", "x86") => "linux-i386",
        ("linux", _) => "linux",
        (os, arch) => return Err(Error::UnsupportedPlatform(format!("{os}-{arch}"))),
    };
    Ok(key)
}

/// Path of the `java` binary inside a runtime folder. On Windows we use `java.exe`
/// (not `javaw.exe`) so stdout/stderr can be captured; the console is hidden at spawn.
pub fn java_executable(runtime_dir: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        runtime_dir.join("bin").join("java.exe")
    } else if cfg!(target_os = "macos") {
        runtime_dir.join("jre.bundle/Contents/Home/bin/java")
    } else {
        runtime_dir.join("bin").join("java")
    }
}

/// Lists the files needed for `component`. If Mojang is unreachable but the runtime is
/// already installed, returns an empty plan so the game can still start offline.
pub async fn plan_runtime(ctx: &Context, component: &str) -> Result<RuntimePlan> {
    let runtime_dir = ctx.data.java().join(component);
    let java_executable = java_executable(&runtime_dir);

    let manifest = match fetch_runtime_manifest(ctx, component).await {
        Ok(manifest) => manifest,
        Err(err) if java_executable.is_file() => {
            tracing::warn!(error = %err, component, "runtime manifest unreachable, using installed runtime");
            return Ok(RuntimePlan {
                java_executable,
                downloads: Vec::new(),
                executables: Vec::new(),
                links: Vec::new(),
            });
        }
        Err(err) => return Err(err),
    };

    let mut plan = RuntimePlan {
        java_executable,
        downloads: Vec::new(),
        executables: Vec::new(),
        links: Vec::new(),
    };
    for (relative, file) in manifest.files {
        let path = runtime_dir.join(&relative);
        match file {
            RuntimeFile::Directory => tokio::fs::create_dir_all(&path).await?,
            RuntimeFile::File {
                executable,
                downloads,
            } => {
                if executable {
                    plan.executables.push(path.clone());
                }
                plan.downloads.push(DownloadTask {
                    url: downloads.raw.url,
                    dest: path,
                    sha1: Some(downloads.raw.sha1),
                    size: Some(downloads.raw.size),
                });
            }
            #[cfg(unix)]
            RuntimeFile::Link { target } => plan.links.push((path, target)),
            #[cfg(not(unix))]
            RuntimeFile::Link { .. } => {}
        }
    }
    Ok(plan)
}

async fn fetch_runtime_manifest(ctx: &Context, component: &str) -> Result<RuntimeManifest> {
    let platform = platform_key()?;
    let index: RuntimeIndex =
        serde_json::from_slice(&fetch_bytes(&ctx.http, RUNTIME_INDEX_URL, None).await?)?;
    let entry = index
        .get(platform)
        .and_then(|components| components.get(component))
        .and_then(|entries| entries.first())
        .ok_or_else(|| Error::JavaUnavailable {
            component: component.to_owned(),
            platform: platform.to_owned(),
        })?;
    let bytes = fetch_bytes(&ctx.http, &entry.manifest.url, Some(&entry.manifest.sha1)).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Applies executable bits and symlinks once files are downloaded.
pub async fn finalize_runtime(plan: &RuntimePlan) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for path in &plan.executables {
            tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).await?;
        }
        for (link, target) in &plan.links {
            if tokio::fs::symlink_metadata(link).await.is_err() {
                tokio::fs::symlink(target, link).await?;
            }
        }
    }
    #[cfg(not(unix))]
    let _ = plan;
    Ok(())
}
