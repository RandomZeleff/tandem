//! Mojang metadata: version manifest, version JSONs and asset indexes.

pub mod forge;
pub mod loader;
pub mod rules;
pub mod version;

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::download::fetch_bytes;
use crate::error::{Error, Result};
use version::VersionJson;

pub const VERSION_MANIFEST_URL: &str =
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const ASSETS_BASE_URL: &str = "https://resources.download.minecraft.net";

#[derive(Debug, Clone, Deserialize)]
pub struct VersionManifest {
    pub latest: Latest,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub release_time: String,
    pub sha1: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndex {
    pub objects: HashMap<String, AssetObject>,
    /// Pre-1.7 indexes: objects must be copied to `assets/virtual/<id>/<name>`.
    #[serde(default, rename = "virtual")]
    pub is_virtual: bool,
    /// Pre-1.6 indexes: objects must be copied to `<game dir>/resources/<name>`.
    #[serde(default)]
    pub map_to_resources: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetObject {
    pub hash: String,
    pub size: u64,
}

impl AssetObject {
    /// `ab/abcdef…`, relative to `assets/objects` and to the download base URL.
    pub fn relative_path(&self) -> String {
        format!("{}/{}", &self.hash[..2], self.hash)
    }
}

/// Fetches the version manifest, caching it so the launcher still works offline.
pub async fn fetch_manifest(ctx: &Context) -> Result<VersionManifest> {
    let cache = ctx.data.cache().join("version_manifest_v2.json");
    let request = fetch_bytes(&ctx.http, VERSION_MANIFEST_URL, None);
    let fetched = if tokio::fs::try_exists(&cache).await.unwrap_or(false) {
        ctx.optional(request).await
    } else {
        request.await
    };
    match fetched {
        Ok(bytes) => {
            let manifest = serde_json::from_slice(&bytes)?;
            tokio::fs::write(&cache, &bytes).await?;
            Ok(manifest)
        }
        Err(err) => {
            tracing::warn!(error = %err, "version manifest unreachable, using cache");
            let bytes = tokio::fs::read(&cache).await.map_err(|_| err)?;
            Ok(serde_json::from_slice(&bytes)?)
        }
    }
}

/// Loads a version JSON, downloading it when missing or outdated.
/// Works offline for versions already installed.
pub async fn load_version(ctx: &Context, id: &str) -> Result<VersionJson> {
    let path = ctx.data.version_dir(id).join(format!("{id}.json"));
    let manifest = fetch_manifest(ctx).await;
    let entry = match &manifest {
        Ok(m) => m.versions.iter().find(|v| v.id == id),
        Err(_) => None,
    };

    if let Some(entry) = entry {
        let up_to_date = tokio::fs::try_exists(&path).await.unwrap_or(false)
            && crate::download::sha1_file(&path).await? == entry.sha1;
        if !up_to_date {
            let bytes = fetch_bytes(&ctx.http, &entry.url, Some(&entry.sha1)).await?;
            write_file(&path, &bytes).await?;
        }
    } else if let Err(err) = manifest {
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Err(err);
        }
    } else if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(Error::VersionNotFound(id.to_owned()));
    }

    let bytes = tokio::fs::read(&path).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Loads the asset index referenced by a version, downloading it if needed.
pub async fn load_asset_index(ctx: &Context, version: &VersionJson) -> Result<AssetIndex> {
    let index = &version.asset_index;
    let path = ctx
        .data
        .assets()
        .join("indexes")
        .join(format!("{}.json", index.id));
    let valid = tokio::fs::try_exists(&path).await.unwrap_or(false)
        && crate::download::sha1_file(&path).await? == index.sha1;
    let bytes = if valid {
        tokio::fs::read(&path).await?
    } else {
        let bytes = fetch_bytes(&ctx.http, &index.url, Some(&index.sha1)).await?;
        write_file(&path, &bytes).await?;
        bytes
    };
    Ok(serde_json::from_slice(&bytes)?)
}

pub(crate) async fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(path, bytes).await?;
    Ok(())
}
