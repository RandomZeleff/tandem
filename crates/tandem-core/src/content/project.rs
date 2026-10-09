//! What a project page shows: details, authors, versions, dependencies and the content
//! of a modpack, read from Modrinth and kept in memory for a few minutes so going back
//! to a page is instant.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::future::try_join_all;
use serde::Serialize;

use super::markdown::to_safe_html;
use super::modrinth::{self, ProjectPage, TeamMember, Version, MODPACK_LOADERS};
use crate::context::Context;
use crate::error::{Error, Result};
use crate::instance::Instance;
use crate::meta::loader::Loader;

/// How long a page, a version list or a project summary stays cached.
const TTL: Duration = Duration::from_secs(600);

/// Projects asked from Modrinth per request when resolving many ids (URL length).
const BATCH: usize = 100;

const SITE: &str = "https://modrinth.com";

/// Cached values by key, each forgotten after [`TTL`].
struct Ttl<T> {
    entries: Mutex<HashMap<String, (Instant, Arc<T>)>>,
}

impl<T> Default for Ttl<T> {
    fn default() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl<T> Ttl<T> {
    fn get(&self, key: &str) -> Option<Arc<T>> {
        let entries = self.entries.lock().ok()?;
        let (at, value) = entries.get(key)?;
        (at.elapsed() < TTL).then(|| value.clone())
    }

    /// Stores `value` under every key (a project is asked by id or by slug).
    fn put(&self, keys: &[&str], value: T) -> Arc<T> {
        let value = Arc::new(value);
        if let Ok(mut entries) = self.entries.lock() {
            entries.retain(|_, (at, _)| at.elapsed() < TTL);
            for key in keys {
                entries.insert((*key).to_owned(), (Instant::now(), value.clone()));
            }
        }
        value
    }
}

/// Modrinth answers kept for project pages. One per launcher.
#[derive(Default)]
pub struct ProjectCache {
    pages: Ttl<ProjectDetails>,
    versions: Ttl<Vec<Version>>,
    summaries: Ttl<ProjectSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDetails {
    pub id: String,
    pub slug: String,
    /// `mod`, `resourcepack`, `shader`, `modpack`, `datapack` or `plugin`.
    pub project_type: String,
    pub title: String,
    pub summary: String,
    /// The long description, sanitized.
    pub body_html: String,
    pub icon_url: Option<String>,
    /// Accent colour picked by Modrinth from the icon, `0xRRGGBB`.
    pub color: Option<u32>,
    pub downloads: u64,
    pub followers: u64,
    pub published: String,
    pub updated: String,
    pub categories: Vec<String>,
    pub loaders: Vec<String>,
    /// Oldest first, snapshots included.
    pub game_versions: Vec<String>,
    /// `required`, `optional`, `unsupported` or `unknown`.
    pub client_side: String,
    pub server_side: String,
    pub license: Option<License>,
    pub links: Vec<Link>,
    pub gallery: Vec<GalleryImage>,
    pub organization: Option<Organization>,
    pub authors: Vec<Author>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct License {
    /// SPDX id, or a `LicenseRef-…` for custom licenses.
    pub id: String,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// `source`, `issues`, `wiki`, `discord` or `donation`.
    pub kind: &'static str,
    /// Donation platform name, empty for the other kinds.
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryImage {
    pub thumb_url: String,
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Organization {
    pub name: String,
    pub icon_url: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    pub name: String,
    pub avatar_url: Option<String>,
    pub role: String,
    pub url: String,
}

/// A project in a list (dependencies, content of a modpack).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub project_type: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionSummary {
    pub id: String,
    pub name: String,
    pub version_number: String,
    /// `release`, `beta` or `alpha`.
    pub version_type: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub date_published: String,
    pub downloads: u64,
    pub file_name: Option<String>,
    pub size: u64,
    pub has_changelog: bool,
    /// Installable: in the given instance for content, by Tandem for a modpack.
    pub compatible: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectVersions {
    pub versions: Vec<VersionSummary>,
    /// What "Install" picks: the newest compatible release, else the newest compatible.
    pub recommended: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyItem {
    /// `required`, `optional`, `incompatible` or `embedded` (shipped inside, e.g. the
    /// mods of a modpack).
    pub kind: String,
    pub project: ProjectSummary,
}

/// The page of a project (id or slug).
pub async fn details(ctx: &Context, cache: &ProjectCache, id: &str) -> Result<Arc<ProjectDetails>> {
    if let Some(page) = cache.pages.get(id) {
        return Ok(page);
    }
    let (page, members) = tokio::join!(modrinth::project_page(ctx, id), modrinth::members(ctx, id));
    let page = page?;
    // Authors are a nice-to-have: the page shows without them.
    let members = members.unwrap_or_else(|err| {
        tracing::warn!(project = id, %err, "could not read project members");
        Vec::new()
    });
    let organization = match &page.organization {
        Some(org) => modrinth::organization(ctx, org)
            .await
            .inspect_err(|err| tracing::warn!(project = id, %err, "could not read organization"))
            .ok(),
        None => None,
    };
    let details = build_details(page, members, organization);
    let keys = [id.to_owned(), details.id.clone(), details.slug.clone()];
    let keys: Vec<&str> = keys.iter().map(String::as_str).collect();
    Ok(cache.pages.put(&keys, details))
}

fn build_details(
    page: ProjectPage,
    members: Vec<TeamMember>,
    organization: Option<modrinth::Organization>,
) -> ProjectDetails {
    let mut links = Vec::new();
    let mut push = |kind: &'static str, label: &str, url: Option<String>| {
        if let Some(url) = url.filter(|u| u.starts_with("http")) {
            links.push(Link {
                kind,
                label: label.to_owned(),
                url,
            });
        }
    };
    push("source", "", page.source_url);
    push("issues", "", page.issues_url);
    push("wiki", "", page.wiki_url);
    push("discord", "", page.discord_url);
    for donation in page.donation_urls {
        push("donation", &donation.platform, Some(donation.url));
    }

    let mut gallery = page.gallery;
    gallery.sort_by_key(|g| (!g.featured, g.ordering));
    let gallery = gallery
        .into_iter()
        .map(|g| GalleryImage {
            url: g.raw_url.clone().unwrap_or_else(|| g.url.clone()),
            thumb_url: g.url,
            title: g.title.filter(|t| !t.trim().is_empty()),
            description: g.description.filter(|d| !d.trim().is_empty()),
        })
        .collect();

    // Project members first, then organization members not already listed.
    let mut team = members;
    team.sort_by_key(|m| m.ordering);
    let mut org_members = organization
        .as_ref()
        .map(|o| o.members.clone())
        .unwrap_or_default();
    org_members.sort_by_key(|m| m.ordering);
    let mut seen = HashSet::new();
    let authors = team
        .into_iter()
        .chain(org_members)
        .filter(|m| m.accepted && seen.insert(m.user.username.clone()))
        .map(|m| Author {
            url: format!("{SITE}/user/{}", m.user.username),
            name: m.user.username,
            avatar_url: m.user.avatar_url,
            role: m.role,
        })
        .collect();

    let mut categories = page.categories;
    categories.extend(page.additional_categories);

    ProjectDetails {
        url: format!("{SITE}/{}/{}", page.project_type, page.slug),
        body_html: to_safe_html(&page.body),
        id: page.id,
        slug: page.slug,
        project_type: page.project_type,
        title: page.title,
        summary: page.description,
        icon_url: page.icon_url,
        color: page.color,
        downloads: page.downloads,
        followers: page.followers,
        published: page.published,
        updated: page.updated,
        categories,
        loaders: page.loaders,
        game_versions: page.game_versions,
        client_side: page.client_side,
        server_side: page.server_side,
        license: page.license.map(|l| License {
            name: license_name(&l.id, &l.name),
            id: l.id,
            url: l.url.filter(|u| u.starts_with("http")),
        }),
        links,
        gallery,
        organization: organization.map(|o| Organization {
            url: format!("{SITE}/organization/{}", o.slug),
            name: o.name,
            icon_url: o.icon_url,
        }),
        authors,
    }
}

/// Readable name of a license: Modrinth leaves it empty for custom ones.
fn license_name(id: &str, name: &str) -> String {
    if !name.trim().is_empty() {
        return name.to_owned();
    }
    match id {
        "LicenseRef-All-Rights-Reserved" => "Tous droits réservés".to_owned(),
        "LicenseRef-Custom" | "LicenseRef-Unknown" => "Licence personnalisée".to_owned(),
        _ => id
            .strip_prefix("LicenseRef-")
            .unwrap_or(id)
            .replace('-', " "),
    }
}

/// Every version of a project, with changelogs (cached).
async fn all_versions(
    ctx: &Context,
    cache: &ProjectCache,
    project_id: &str,
) -> Result<Arc<Vec<Version>>> {
    if let Some(versions) = cache.versions.get(project_id) {
        return Ok(versions);
    }
    let versions = modrinth::all_versions(ctx, project_id).await?;
    Ok(cache.versions.put(&[project_id], versions))
}

/// What content would be installed into.
#[derive(Debug, Clone, Copy)]
pub struct Target<'a> {
    pub game_version: &'a str,
    pub loader: Loader,
}

impl<'a> From<&'a Instance> for Target<'a> {
    fn from(instance: &'a Instance) -> Self {
        Self {
            game_version: &instance.game_version,
            loader: instance.loader,
        }
    }
}

/// The versions of a project, marked compatible with `target` (content) or with Tandem
/// (modpacks), newest first.
pub async fn versions(
    ctx: &Context,
    cache: &ProjectCache,
    project_id: &str,
    target: Option<Target<'_>>,
) -> Result<ProjectVersions> {
    let page = details(ctx, cache, project_id).await?;
    let all = all_versions(ctx, cache, &page.id).await?;
    Ok(summarize(&all, &page.project_type, target))
}

fn summarize(all: &[Version], project_type: &str, target: Option<Target<'_>>) -> ProjectVersions {
    let versions: Vec<VersionSummary> = all
        .iter()
        .map(|v| {
            let file = v.primary_file();
            VersionSummary {
                id: v.id.clone(),
                name: v.name.clone(),
                version_number: v.version_number.clone(),
                version_type: v.version_type.clone(),
                game_versions: v.game_versions.clone(),
                loaders: v.loaders.clone(),
                date_published: v.date_published.clone(),
                downloads: v.downloads,
                file_name: file.map(|f| f.filename.clone()),
                size: file.map_or(0, |f| f.size),
                has_changelog: v.changelog.as_deref().is_some_and(|c| !c.trim().is_empty()),
                compatible: compatible(v, project_type, target),
            }
        })
        .collect();
    let recommended = versions
        .iter()
        .find(|v| v.compatible && v.version_type == "release")
        .or_else(|| versions.iter().find(|v| v.compatible))
        .map(|v| v.id.clone());
    ProjectVersions {
        versions,
        recommended,
    }
}

/// Whether Tandem can install `version`: a modpack with a loader it launches, content
/// matching the instance's game version (and loader, for mods).
fn compatible(version: &Version, project_type: &str, target: Option<Target<'_>>) -> bool {
    let has_loader = |loaders: &[&str]| {
        version
            .loaders
            .iter()
            .any(|l| loaders.contains(&l.as_str()))
    };
    let fits_game = |t: Target<'_>| version.game_versions.iter().any(|g| g == t.game_version);
    match (project_type, target) {
        ("modpack", _) => has_loader(MODPACK_LOADERS),
        ("mod" | "resourcepack" | "shader", None) => true,
        ("mod", Some(t)) => fits_game(t) && has_loader(modrinth::mod_loaders(t.loader)),
        ("resourcepack" | "shader", Some(t)) => fits_game(t),
        _ => false,
    }
}

/// Changelog of a version, as sanitized HTML (empty when the author wrote none).
pub async fn changelog(
    ctx: &Context,
    cache: &ProjectCache,
    project_id: &str,
    version_id: &str,
) -> Result<String> {
    let page = details(ctx, cache, project_id).await?;
    let all = all_versions(ctx, cache, &page.id).await?;
    let version = all
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| Error::ContentNotFound(version_id.to_owned()))?;
    Ok(to_safe_html(
        version.changelog.as_deref().unwrap_or_default(),
    ))
}

/// Projects a version depends on, ships (modpack content) or conflicts with, sorted by
/// kind then title.
pub async fn dependencies(
    ctx: &Context,
    cache: &ProjectCache,
    project_id: &str,
    version_id: &str,
) -> Result<Vec<DependencyItem>> {
    let page = details(ctx, cache, project_id).await?;
    let all = all_versions(ctx, cache, &page.id).await?;
    let version = all
        .iter()
        .find(|v| v.id == version_id)
        .ok_or_else(|| Error::ContentNotFound(version_id.to_owned()))?;

    // Some dependencies only name a version: find its project.
    let orphans: Vec<String> = version
        .dependencies
        .iter()
        .filter(|d| d.project_id.is_none())
        .filter_map(|d| d.version_id.clone())
        .collect();
    let found = try_join_all(
        orphans
            .chunks(BATCH)
            .map(|chunk| modrinth::versions(ctx, chunk)),
    )
    .await?;
    let owners: HashMap<String, String> = found
        .into_iter()
        .flatten()
        .map(|v| (v.id, v.project_id))
        .collect();

    let mut kinds: HashMap<String, &str> = HashMap::new();
    for dep in &version.dependencies {
        let project = dep
            .project_id
            .clone()
            .or_else(|| dep.version_id.as_ref().and_then(|v| owners.get(v).cloned()));
        let Some(project) = project.filter(|p| *p != page.id) else {
            continue;
        };
        let kind = dep.dependency_type.as_str();
        kinds
            .entry(project)
            .and_modify(|k| {
                if rank(kind) < rank(k) {
                    *k = kind;
                }
            })
            .or_insert(kind);
    }

    let ids: Vec<String> = kinds.keys().cloned().collect();
    let mut items: Vec<DependencyItem> = summaries(ctx, cache, &ids)
        .await?
        .into_iter()
        .filter_map(|project| {
            let kind = kinds.get(&project.id)?;
            Some(DependencyItem {
                kind: (*kind).to_owned(),
                project,
            })
        })
        .collect();
    items.sort_by_cached_key(|item| (rank(&item.kind), sort_key(&item.project.title)));
    Ok(items)
}

/// Alphabetical order that skips tags like `[EMF]` in front of a title.
fn sort_key(title: &str) -> String {
    let title = title.trim_start_matches(|c: char| !c.is_alphanumeric());
    let title = match title.split_once(']') {
        Some((tag, rest)) if !tag.contains('[') && tag.len() < 16 => rest,
        _ => title,
    };
    title
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

fn rank(kind: &str) -> u8 {
    match kind {
        "required" => 0,
        "optional" => 1,
        "incompatible" => 2,
        "embedded" => 3,
        _ => 4,
    }
}

/// Short descriptions of projects, from the cache or in batches from Modrinth. Unknown
/// ids are left out.
async fn summaries(
    ctx: &Context,
    cache: &ProjectCache,
    ids: &[String],
) -> Result<Vec<ProjectSummary>> {
    let mut found = Vec::with_capacity(ids.len());
    let mut missing = Vec::new();
    for id in ids {
        match cache.summaries.get(id) {
            Some(summary) => found.push((*summary).clone()),
            None => missing.push(id.clone()),
        }
    }
    let batches = try_join_all(
        missing
            .chunks(BATCH)
            .map(|chunk| modrinth::projects(ctx, chunk)),
    )
    .await?;
    for project in batches.into_iter().flatten() {
        let summary = ProjectSummary {
            id: project.id,
            slug: project.slug,
            title: project.title.trim().to_owned(),
            summary: project.description,
            project_type: project.project_type,
            icon_url: project.icon_url,
            downloads: project.downloads,
        };
        cache.summaries.put(&[summary.id.as_str()], summary.clone());
        found.push(summary);
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(id: &str, kind: &str, games: &[&str], loaders: &[&str]) -> Version {
        serde_json::from_value(serde_json::json!({
            "id": id, "project_id": "p", "version_number": id, "version_type": kind,
            "game_versions": games, "loaders": loaders,
        }))
        .unwrap()
    }

    fn target(game_version: &str, loader: Loader) -> Option<Target<'_>> {
        Some(Target {
            game_version,
            loader,
        })
    }

    #[test]
    fn marks_compatible_versions_and_recommends_a_release() {
        let all = [
            version("beta", "beta", &["1.21.1"], &["fabric"]),
            version("forge", "release", &["1.21.1"], &["forge"]),
            version("old", "release", &["1.20.1"], &["fabric"]),
            version("good", "release", &["1.21.1"], &["fabric", "quilt"]),
        ];
        let result = summarize(&all, "mod", target("1.21.1", Loader::Quilt));
        let compatible: Vec<_> = result
            .versions
            .iter()
            .filter(|v| v.compatible)
            .map(|v| v.id.as_str())
            .collect();
        assert_eq!(compatible, ["beta", "good"]);
        assert_eq!(result.recommended.as_deref(), Some("good"));

        // Resource packs ignore the loader; without a release the newest fit is picked.
        let packs = [version("b", "beta", &["1.21.1"], &["minecraft"])];
        let result = summarize(&packs, "resourcepack", target("1.21.1", Loader::Vanilla));
        assert_eq!(result.recommended.as_deref(), Some("b"));

        // A vanilla instance takes no mods.
        let result = summarize(&all, "mod", target("1.21.1", Loader::Vanilla));
        assert_eq!(result.recommended, None);
    }

    #[test]
    fn modpacks_need_a_launchable_loader() {
        let all = [
            version("rift", "release", &["1.13"], &["rift"]),
            version("fabric", "beta", &["1.21.1"], &["fabric"]),
        ];
        let result = summarize(&all, "modpack", None);
        assert!(!result.versions[0].compatible);
        assert_eq!(result.recommended.as_deref(), Some("fabric"));
        assert!(!summarize(&all, "datapack", None).versions[1].compatible);
    }

    #[test]
    fn builds_page_details() {
        let page: ProjectPage = serde_json::from_value(serde_json::json!({
            "id": "AANobbMI", "slug": "sodium", "project_type": "mod", "title": "Sodium",
            "description": "Fast", "body": "# Hi\n<script>x</script>",
            "categories": ["optimization"], "additional_categories": ["utility"],
            "license": {"id": "LicenseRef-All-Rights-Reserved", "name": ""},
            "source_url": "https://github.com/x", "wiki_url": null, "discord_url": "not a url",
            "donation_urls": [{"id": "ko-fi", "platform": "Ko-fi", "url": "https://ko-fi.com/x"}],
            "gallery": [
                {"url": "a_350.webp", "raw_url": "a.png", "featured": false, "title": "", "ordering": 0},
                {"url": "b_350.webp", "featured": true, "title": "Main", "ordering": 5}
            ]
        }))
        .unwrap();
        let member = |name: &str, ordering: i64, accepted: bool| -> TeamMember {
            serde_json::from_value(serde_json::json!({
                "user": {"username": name, "avatar_url": null}, "role": "Dev",
                "ordering": ordering, "accepted": accepted,
            }))
            .unwrap()
        };
        let org: modrinth::Organization = serde_json::from_value(serde_json::json!({
            "slug": "caffeine", "name": "CaffeineMC", "members": [
                {"user": {"username": "jelly"}, "role": "Owner", "ordering": 0},
                {"user": {"username": "ima"}, "role": "Dev", "ordering": 1}
            ]
        }))
        .unwrap();
        let details = build_details(
            page,
            vec![member("jelly", 1, true), member("pending", 0, false)],
            Some(org),
        );
        assert_eq!(details.url, "https://modrinth.com/mod/sodium");
        assert!(details.body_html.contains("<h1>Hi</h1>"));
        assert!(!details.body_html.contains("script"));
        assert_eq!(details.categories, ["optimization", "utility"]);
        assert_eq!(details.license.unwrap().name, "Tous droits réservés");
        let kinds: Vec<_> = details.links.iter().map(|l| l.kind).collect();
        assert_eq!(kinds, ["source", "donation"]);
        assert_eq!(details.links[1].label, "Ko-fi");
        assert_eq!(details.gallery[0].title.as_deref(), Some("Main"));
        assert_eq!(details.gallery[0].url, "b_350.webp");
        assert_eq!(details.gallery[1].url, "a.png");
        assert_eq!(details.gallery[1].title, None);
        let authors: Vec<_> = details.authors.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(authors, ["jelly", "ima"]);
        assert_eq!(
            details.organization.unwrap().url,
            "https://modrinth.com/organization/caffeine"
        );
    }

    #[test]
    fn sorts_titles_without_tags() {
        assert_eq!(
            sort_key("[EMF] Entity Model Features"),
            "entity model features"
        );
        assert_eq!(sort_key(" 3D Skin Layers"), "3d skin layers");
        assert_eq!(
            sort_key("Create: Steam 'n' Rails"),
            "create: steam 'n' rails"
        );
    }

    #[test]
    fn cache_answers_every_key() {
        let cache: Ttl<u32> = Ttl::default();
        cache.put(&["id", "slug"], 7);
        assert_eq!(cache.get("slug").as_deref(), Some(&7));
        assert_eq!(cache.get("id").as_deref(), Some(&7));
        assert!(cache.get("other").is_none());
    }
}
