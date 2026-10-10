//! Mod loaders. Fabric and Quilt publish launcher profiles that inherit from the
//! vanilla version JSON: we only add their libraries, main class and arguments.
//! Forge and NeoForge go through their installer (see [`super::forge`]).

use std::cmp::Ordering;
use std::fmt;

use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::version::{Arguments, Library};
use super::write_file;
use crate::context::Context;
use crate::download::fetch_bytes;
use crate::error::{Error, Result};

const FABRIC_META: &str = "https://meta.fabricmc.net/v2";
const QUILT_META: &str = "https://meta.quiltmc.org/v3";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum Loader {
    #[default]
    Vanilla,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl Loader {
    pub fn as_str(self) -> &'static str {
        match self {
            Loader::Vanilla => "vanilla",
            Loader::Fabric => "fabric",
            Loader::Quilt => "quilt",
            Loader::Forge => "forge",
            Loader::NeoForge => "neoforge",
        }
    }

    fn meta_base(self) -> Result<&'static str> {
        match self {
            Loader::Fabric => Ok(FABRIC_META),
            Loader::Quilt => Ok(QUILT_META),
            other => Err(Error::LoaderNotSupported(other.as_str().to_owned())),
        }
    }
}

impl fmt::Display for Loader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
    /// Picked by the loader's team for this game version (Forge only).
    pub recommended: bool,
}

#[derive(Deserialize)]
struct LoaderEntry {
    loader: LoaderInfo,
}

#[derive(Deserialize)]
struct LoaderInfo {
    version: String,
    /// Fabric only; Quilt marks pre-releases in the version string (`0.20.0-beta.9`).
    stable: Option<bool>,
}

/// Launcher profile (`versions/<id>/<id>.json`) published by a loader's meta server.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderProfile {
    pub id: String,
    pub inherits_from: String,
    pub main_class: String,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// Pre-1.13 profiles (Forge 1.12.2) replace the whole legacy argument string.
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
}

fn meta_url(loader: Loader, segments: &[&str]) -> Result<Url> {
    let mut url = Url::parse(loader.meta_base()?)
        .map_err(|e| Error::InvalidInput(format!("Adresse invalide : {e}")))?;
    url.path_segments_mut()
        .map_err(|()| Error::InvalidInput("Adresse invalide".into()))?
        .extend(segments);
    Ok(url)
}

/// Loader versions available for a game version, newest first.
/// Empty when the loader does not support that version.
pub async fn list_versions(
    ctx: &Context,
    loader: Loader,
    game_version: &str,
) -> Result<Vec<LoaderVersion>> {
    match loader {
        Loader::Vanilla => return Ok(Vec::new()),
        Loader::Forge | Loader::NeoForge => {
            return super::forge::list_versions(ctx, loader, game_version).await
        }
        Loader::Fabric | Loader::Quilt => {}
    }
    let url = meta_url(loader, &["versions", "loader", game_version])?;
    let bytes = match fetch_bytes(&ctx.http, url.as_str(), None).await {
        Ok(bytes) => bytes,
        // Fabric answers 400 for game versions it does not know.
        Err(Error::HttpStatus { status: 400, .. }) => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    Ok(parse_versions(&bytes)?)
}

fn parse_versions(bytes: &[u8]) -> serde_json::Result<Vec<LoaderVersion>> {
    let entries: Vec<LoaderEntry> = serde_json::from_slice(bytes)?;
    let mut versions: Vec<_> = entries
        .into_iter()
        .map(|e| LoaderVersion {
            stable: e
                .loader
                .stable
                .unwrap_or_else(|| !e.loader.version.contains('-')),
            recommended: false,
            version: e.loader.version,
        })
        .collect();
    // Quilt returns versions in no particular order.
    versions.sort_by(|a, b| compare_versions(&b.version, &a.version));
    Ok(versions)
}

/// Semver-like ordering: `0.9.0 < 0.10.0-beta.2 < 0.10.0-beta.10 < 0.10.0`.
/// Build metadata (`+build.12`) is ignored.
pub(crate) fn compare_versions(a: &str, b: &str) -> Ordering {
    fn split(v: &str) -> (&str, Option<&str>) {
        let v = v.split('+').next().unwrap_or(v);
        match v.split_once('-') {
            Some((release, pre)) => (release, Some(pre)),
            None => (v, None),
        }
    }
    fn compare_parts(a: &str, b: &str) -> Ordering {
        let mut a = a.split('.');
        let mut b = b.split('.');
        loop {
            match (a.next(), b.next()) {
                (None, None) => return Ordering::Equal,
                (None, Some(_)) => return Ordering::Less,
                (Some(_), None) => return Ordering::Greater,
                (Some(x), Some(y)) => {
                    let order = match (x.parse::<u64>(), y.parse::<u64>()) {
                        (Ok(x), Ok(y)) => x.cmp(&y),
                        _ => x.cmp(y),
                    };
                    if order != Ordering::Equal {
                        return order;
                    }
                }
            }
        }
    }
    let (a_release, a_pre) = split(a);
    let (b_release, b_pre) = split(b);
    compare_parts(a_release, b_release).then_with(|| match (a_pre, b_pre) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => compare_parts(x, y),
    })
}

/// Loads a loader profile, downloading it once: a given loader + game version pair never changes.
pub async fn load_profile(
    ctx: &Context,
    loader: Loader,
    game_version: &str,
    loader_version: &str,
) -> Result<LoaderProfile> {
    let id = format!("{loader}-loader-{loader_version}-{game_version}");
    let path = ctx.data.version_dir(&id).join(format!("{id}.json"));
    if let Ok(bytes) = tokio::fs::read(&path).await {
        if let Ok(profile) = serde_json::from_slice(&bytes) {
            return Ok(profile);
        }
    }
    let url = meta_url(
        loader,
        &[
            "versions",
            "loader",
            game_version,
            loader_version,
            "profile",
            "json",
        ],
    )?;
    let bytes = fetch_bytes(&ctx.http, url.as_str(), None).await?;
    let profile: LoaderProfile = serde_json::from_slice(&bytes)?;
    if profile.inherits_from != game_version {
        return Err(Error::InvalidInput(format!(
            "Le profil {loader} vise {} au lieu de {game_version}",
            profile.inherits_from
        )));
    }
    write_file(&path, &bytes).await?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stability() {
        let fabric = br#"[{"loader":{"version":"0.16.10","stable":true}},
                          {"loader":{"version":"0.16.11","stable":false}}]"#;
        let quilt = br#"[{"loader":{"version":"0.20.0-beta.9"}},{"loader":{"version":"0.28.0"}}]"#;
        assert_eq!(
            parse_versions(fabric).unwrap(),
            [
                LoaderVersion {
                    version: "0.16.11".into(),
                    stable: false,
                    recommended: false
                },
                LoaderVersion {
                    version: "0.16.10".into(),
                    stable: true,
                    recommended: false
                }
            ]
        );
        let quilt = parse_versions(quilt).unwrap();
        assert_eq!(quilt[0].version, "0.28.0");
        assert!(quilt[0].stable);
        assert!(!quilt[1].stable);
    }

    #[test]
    fn orders_versions() {
        let mut versions = [
            "0.20.0-beta.9",
            "0.24.0",
            "0.20.0-beta.10",
            "0.9.1",
            "0.20.0",
            "0.7.2+build.175",
            "0.30.0-beta.2",
        ];
        versions.sort_by(|a, b| compare_versions(b, a));
        assert_eq!(
            versions,
            [
                "0.30.0-beta.2",
                "0.24.0",
                "0.20.0",
                "0.20.0-beta.10",
                "0.20.0-beta.9",
                "0.9.1",
                "0.7.2+build.175"
            ]
        );
    }

    #[test]
    fn encodes_version_segments() {
        let url = meta_url(
            Loader::Fabric,
            &["versions", "loader", "1.14 Pre-Release 1"],
        )
        .unwrap();
        assert_eq!(
            url.as_str(),
            "https://meta.fabricmc.net/v2/versions/loader/1.14%20Pre-Release%201"
        );
        assert!(meta_url(Loader::Forge, &[]).is_err());
    }
}
