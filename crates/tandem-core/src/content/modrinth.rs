//! Modrinth API v2 client (https://docs.modrinth.com/api/).

use std::collections::HashMap;

use reqwest::Url;
use serde::{Deserialize, Serialize};

use super::ContentKind;
use crate::context::Context;
use crate::download::fetch_bytes;
use crate::error::{Error, Result};
use crate::meta::loader::Loader;

const API: &str = "https://api.modrinth.com/v2";

/// Modpack loaders Tandem can launch.
pub const MODPACK_LOADERS: &[&str] = &["fabric", "quilt", "forge", "neoforge"];

/// Searchable project types: instance content plus modpacks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    #[default]
    Mod,
    ResourcePack,
    Shader,
    Modpack,
}

impl ProjectType {
    fn as_str(self) -> &'static str {
        match self {
            ProjectType::Mod => "mod",
            ProjectType::ResourcePack => "resourcepack",
            ProjectType::Shader => "shader",
            ProjectType::Modpack => "modpack",
        }
    }
}

impl From<ContentKind> for ProjectType {
    fn from(kind: ContentKind) -> Self {
        match kind {
            ContentKind::Mod => ProjectType::Mod,
            ContentKind::ResourcePack => ProjectType::ResourcePack,
            ContentKind::Shader => ProjectType::Shader,
        }
    }
}

/// What the player searches for, already narrowed to an instance when there is one.
#[derive(Debug, Clone, Default)]
pub struct SearchFilter<'a> {
    pub query: &'a str,
    pub kind: ProjectType,
    pub game_version: Option<&'a str>,
    pub loader: Option<Loader>,
    pub offset: u32,
    pub limit: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub downloads: u64,
    pub follows: u64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub display_categories: Vec<String>,
    pub date_modified: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub project_type: String,
    #[serde(default)]
    pub icon_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub version_number: String,
    pub version_type: String,
    /// RFC 3339 in UTC, so string order is chronological.
    #[serde(default)]
    pub date_published: String,
    #[serde(default)]
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

impl Version {
    /// The file to install: the one flagged primary, else the first.
    pub fn primary_file(&self) -> Option<&VersionFile> {
        self.files.iter().find(|f| f.primary).or(self.files.first())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    pub size: u64,
    pub hashes: FileHashes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileHashes {
    pub sha1: String,
    #[serde(default)]
    pub sha512: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    pub dependency_type: String,
}

impl Dependency {
    pub fn is_required(&self) -> bool {
        self.dependency_type == "required"
    }
}

/// Modrinth loader names whose mods run on `loader` (Quilt also runs Fabric mods).
pub fn mod_loaders(loader: Loader) -> &'static [&'static str] {
    match loader {
        Loader::Vanilla => &[],
        Loader::Fabric => &["fabric"],
        Loader::Quilt => &["quilt", "fabric"],
        Loader::Forge => &["forge"],
        Loader::NeoForge => &["neoforge"],
    }
}

fn url(segments: &[&str]) -> Result<Url> {
    let mut url = Url::parse(API).map_err(|e| Error::InvalidInput(e.to_string()))?;
    url.path_segments_mut()
        .map_err(|()| Error::InvalidInput("bad API URL".into()))?
        .extend(segments);
    Ok(url)
}

async fn get<T: for<'de> Deserialize<'de>>(ctx: &Context, url: Url) -> Result<T> {
    let bytes = fetch_bytes(&ctx.http, url.as_str(), None).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Search facets: AND between the outer arrays, OR inside each one.
fn facets(filter: &SearchFilter<'_>) -> String {
    let mut groups: Vec<Vec<String>> = vec![vec![format!("project_type:{}", filter.kind.as_str())]];
    if let Some(version) = filter.game_version {
        groups.push(vec![format!("versions:{version}")]);
    }
    let loaders = match (filter.kind, filter.loader) {
        (ProjectType::Mod, Some(loader)) => mod_loaders(loader),
        (ProjectType::Modpack, _) => MODPACK_LOADERS,
        _ => &[],
    };
    if !loaders.is_empty() {
        groups.push(loaders.iter().map(|l| format!("categories:{l}")).collect());
    }
    serde_json::to_string(&groups).unwrap_or_default()
}

pub async fn search(ctx: &Context, filter: &SearchFilter<'_>) -> Result<SearchResults> {
    let mut url = url(&["search"])?;
    url.query_pairs_mut()
        .append_pair("query", filter.query.trim())
        .append_pair("facets", &facets(filter))
        .append_pair(
            "index",
            if filter.query.trim().is_empty() {
                "downloads"
            } else {
                "relevance"
            },
        )
        .append_pair("offset", &filter.offset.to_string())
        .append_pair("limit", &filter.limit.clamp(1, 100).to_string());
    get(ctx, url).await
}

pub async fn project(ctx: &Context, id_or_slug: &str) -> Result<Project> {
    get(ctx, url(&["project", id_or_slug])?).await
}

pub async fn version(ctx: &Context, id: &str) -> Result<Version> {
    get(ctx, url(&["version", id])?).await
}

/// Versions of a project usable on a game version / loader set, newest first.
pub async fn project_versions(
    ctx: &Context,
    project_id: &str,
    game_version: &str,
    loaders: &[&str],
) -> Result<Vec<Version>> {
    let mut url = url(&["project", project_id, "version"])?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("game_versions", &serde_json::to_string(&[game_version])?);
        if !loaders.is_empty() {
            query.append_pair("loaders", &serde_json::to_string(loaders)?);
        }
    }
    get(ctx, url).await
}

/// Versions of a project for any of `loaders`, whatever the game version, newest first.
pub async fn pack_versions(
    ctx: &Context,
    project_id: &str,
    loaders: &[&str],
) -> Result<Vec<Version>> {
    let mut url = url(&["project", project_id, "version"])?;
    url.query_pairs_mut()
        .append_pair("loaders", &serde_json::to_string(loaders)?);
    get(ctx, url).await
}

/// Several versions at once (unknown ids are skipped by the API).
pub async fn versions(ctx: &Context, ids: &[String]) -> Result<Vec<Version>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut url = url(&["versions"])?;
    url.query_pairs_mut()
        .append_pair("ids", &serde_json::to_string(ids)?);
    get(ctx, url).await
}

#[derive(Serialize)]
struct UpdateQuery<'a> {
    hashes: &'a [String],
    algorithm: &'static str,
    game_versions: [&'a str; 1],
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    loaders: &'a [&'a str],
    #[serde(skip_serializing_if = "Option::is_none")]
    version_types: Option<[&'static str; 1]>,
}

/// Latest compatible version for each file SHA-1, in one request. Files Modrinth does
/// not know, or with nothing compatible, are absent from the result.
pub async fn latest_versions(
    ctx: &Context,
    sha1s: &[String],
    game_version: &str,
    loaders: &[&str],
    releases_only: bool,
) -> Result<HashMap<String, Version>> {
    if sha1s.is_empty() {
        return Ok(HashMap::new());
    }
    post(
        ctx,
        url(&["version_files", "update"])?,
        &UpdateQuery {
            hashes: sha1s,
            algorithm: "sha1",
            game_versions: [game_version],
            loaders,
            version_types: releases_only.then_some(["release"]),
        },
    )
    .await
}

#[derive(Serialize)]
struct HashQuery<'a> {
    hashes: &'a [String],
    algorithm: &'static str,
}

/// The version each file SHA-1 belongs to; files unknown to Modrinth are absent.
pub async fn versions_by_hash(ctx: &Context, sha1s: &[String]) -> Result<HashMap<String, Version>> {
    if sha1s.is_empty() {
        return Ok(HashMap::new());
    }
    post(
        ctx,
        url(&["version_files"])?,
        &HashQuery {
            hashes: sha1s,
            algorithm: "sha1",
        },
    )
    .await
}

/// Several projects at once (unknown ids are skipped by the API).
pub async fn projects(ctx: &Context, ids: &[String]) -> Result<Vec<Project>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut url = url(&["projects"])?;
    url.query_pairs_mut()
        .append_pair("ids", &serde_json::to_string(ids)?);
    get(ctx, url).await
}

async fn post<T: for<'de> Deserialize<'de>>(
    ctx: &Context,
    url: Url,
    body: &impl Serialize,
) -> Result<T> {
    let response = ctx.http.post(url.as_str()).json(body).send().await?;
    if !response.status().is_success() {
        return Err(Error::HttpStatus {
            url: url.to_string(),
            status: response.status().as_u16(),
        });
    }
    Ok(response.json().await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_facets() {
        let filter = SearchFilter {
            kind: ProjectType::Mod,
            game_version: Some("1.21.4"),
            loader: Some(Loader::Quilt),
            ..Default::default()
        };
        assert_eq!(
            facets(&filter),
            r#"[["project_type:mod"],["versions:1.21.4"],["categories:quilt","categories:fabric"]]"#
        );
        let packs = SearchFilter {
            kind: ProjectType::ResourcePack,
            loader: Some(Loader::Fabric),
            ..Default::default()
        };
        assert_eq!(facets(&packs), r#"[["project_type:resourcepack"]]"#);
        let modpacks = SearchFilter {
            kind: ProjectType::Modpack,
            ..Default::default()
        };
        assert_eq!(
            facets(&modpacks),
            r#"[["project_type:modpack"],["categories:fabric","categories:quilt","categories:forge","categories:neoforge"]]"#
        );
    }

    #[test]
    fn parses_version() {
        let json = r#"{"id":"v1","project_id":"p1","version_number":"1.0","version_type":"release",
            "files":[{"url":"u1","filename":"a.jar","primary":false,"size":1,"hashes":{"sha1":"x","sha512":"y"}},
                     {"url":"u2","filename":"b.jar","primary":true,"size":2,"hashes":{"sha1":"z"}}],
            "dependencies":[{"version_id":null,"project_id":"p2","file_name":null,"dependency_type":"required"},
                            {"project_id":"p3","dependency_type":"optional"}]}"#;
        let v: Version = serde_json::from_str(json).unwrap();
        assert_eq!(v.primary_file().unwrap().filename, "b.jar");
        let required: Vec<_> = v.dependencies.iter().filter(|d| d.is_required()).collect();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].project_id.as_deref(), Some("p2"));
    }
}
