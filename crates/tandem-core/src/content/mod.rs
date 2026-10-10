//! Instance content (mods, resource packs, shaders): installation and updates from
//! Modrinth with required dependencies, files kept in the shared store and hardlinked
//! into instances. Disabled content keeps its file, renamed with a `.disabled` suffix.

pub mod deps;
pub mod markdown;
pub mod modrinth;
pub mod mrpack;
pub mod pack_update;
pub mod perf;
pub mod project;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::db::Database;
use crate::download::{self, DownloadTask};
use crate::error::{Error, Result};
use crate::instance::Instance;
use crate::store;
use modrinth::{Project, Version};

/// Upper bound on projects pulled by one operation, guarding against dependency loops.
const MAX_PROJECTS_PER_INSTALL: usize = 64;

/// Suffix the game and the major launchers use to skip a file.
const DISABLED_SUFFIX: &str = ".disabled";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum ContentKind {
    #[default]
    Mod,
    ResourcePack,
    Shader,
}

impl ContentKind {
    pub fn modrinth_type(self) -> &'static str {
        match self {
            ContentKind::Mod => "mod",
            ContentKind::ResourcePack => "resourcepack",
            ContentKind::Shader => "shader",
        }
    }

    fn from_modrinth(project_type: &str) -> Option<Self> {
        match project_type {
            "mod" => Some(ContentKind::Mod),
            "resourcepack" => Some(ContentKind::ResourcePack),
            "shader" => Some(ContentKind::Shader),
            _ => None,
        }
    }

    /// Folder inside the instance's game directory.
    pub fn folder(self) -> &'static str {
        match self {
            ContentKind::Mod => "mods",
            ContentKind::ResourcePack => "resourcepacks",
            ContentKind::Shader => "shaderpacks",
        }
    }
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct InstalledContent {
    pub project_id: String,
    pub version_id: String,
    pub kind: ContentKind,
    pub title: String,
    pub version_number: String,
    /// Name as published, without the `.disabled` suffix.
    pub file_name: String,
    pub sha1: String,
    pub icon_url: Option<String>,
    pub is_dependency: bool,
    pub enabled: bool,
    pub installed_at: String,
}

/// A newer compatible version of installed content.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdate {
    pub project_id: String,
    pub title: String,
    pub current_version: String,
    pub new_version: String,
}

/// Where a file lives inside an instance, given its enabled state.
fn content_path(
    ctx: &Context,
    instance_id: &str,
    kind: ContentKind,
    file_name: &str,
    enabled: bool,
) -> PathBuf {
    let name = if enabled {
        file_name.to_owned()
    } else {
        format!("{file_name}{DISABLED_SUFFIX}")
    };
    ctx.data
        .instance_dir(instance_id)
        .join(kind.folder())
        .join(name)
}

impl InstalledContent {
    fn path(&self, ctx: &Context, instance_id: &str) -> PathBuf {
        content_path(ctx, instance_id, self.kind, &self.file_name, self.enabled)
    }
}

const COLUMNS: &str = "project_id, version_id, kind, title, version_number, file_name, sha1, \
                       icon_url, is_dependency, enabled, installed_at";

impl Database {
    pub async fn list_content(&self, instance_id: &str) -> Result<Vec<InstalledContent>> {
        Ok(sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM instance_content WHERE instance_id = ?
             ORDER BY kind, is_dependency, title COLLATE NOCASE"
        ))
        .bind(instance_id)
        .fetch_all(self.pool())
        .await?)
    }

    async fn get_content(&self, instance_id: &str, project_id: &str) -> Result<InstalledContent> {
        sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM instance_content WHERE instance_id = ? AND project_id = ?"
        ))
        .bind(instance_id)
        .bind(project_id)
        .fetch_optional(self.pool())
        .await?
        .ok_or_else(|| Error::ContentNotFound(project_id.to_owned()))
    }

    /// Inserts or updates a row; `installed_at` keeps its first value.
    async fn upsert_content(&self, instance_id: &str, item: &InstalledContent) -> Result<()> {
        sqlx::query(
            "INSERT INTO instance_content
             (instance_id, project_id, version_id, kind, title, version_number, file_name, sha1,
              icon_url, is_dependency, enabled)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT (instance_id, project_id) DO UPDATE SET
                version_id = excluded.version_id, kind = excluded.kind, title = excluded.title,
                version_number = excluded.version_number, file_name = excluded.file_name,
                sha1 = excluded.sha1, icon_url = excluded.icon_url,
                is_dependency = excluded.is_dependency, enabled = excluded.enabled",
        )
        .bind(instance_id)
        .bind(&item.project_id)
        .bind(&item.version_id)
        .bind(item.kind)
        .bind(&item.title)
        .bind(&item.version_number)
        .bind(&item.file_name)
        .bind(&item.sha1)
        .bind(&item.icon_url)
        .bind(item.is_dependency)
        .bind(item.enabled)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn set_content_enabled(
        &self,
        instance_id: &str,
        project_id: &str,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE instance_content SET enabled = ? WHERE instance_id = ? AND project_id = ?",
        )
        .bind(enabled)
        .bind(instance_id)
        .bind(project_id)
        .execute(self.pool())
        .await?;
        Ok(())
    }

    async fn delete_content(&self, instance_id: &str, project_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM instance_content WHERE instance_id = ? AND project_id = ?")
            .bind(instance_id)
            .bind(project_id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

struct Pending {
    project_id: Option<String>,
    version_id: Option<String>,
    is_dependency: bool,
}

struct Planned {
    project: Project,
    kind: ContentKind,
    version: Version,
    is_dependency: bool,
}

/// Projects to download and place, built from roots then their required dependencies.
#[derive(Default)]
struct Plan {
    items: Vec<Planned>,
    queue: VecDeque<Pending>,
}

impl Plan {
    fn push(&mut self, project: Project, kind: ContentKind, version: Version, is_dependency: bool) {
        self.queue.extend(
            version
                .dependencies
                .iter()
                .filter(|d| d.is_required())
                .map(|d| Pending {
                    project_id: d.project_id.clone(),
                    version_id: d.version_id.clone(),
                    is_dependency: true,
                }),
        );
        self.items.push(Planned {
            project,
            kind,
            version,
            is_dependency,
        });
    }

    /// Drains the queue, resolving each project not in `known` to a compatible version.
    async fn resolve(
        &mut self,
        ctx: &Context,
        instance: &Instance,
        known: &mut HashSet<String>,
    ) -> Result<()> {
        while let Some(pending) = self.queue.pop_front() {
            if self.items.len() >= MAX_PROJECTS_PER_INSTALL {
                return Err(Error::InvalidInput(
                    "Trop de dépendances à installer".into(),
                ));
            }
            let pinned = match &pending.version_id {
                Some(id) => Some(modrinth::version(ctx, id).await?),
                None => None,
            };
            let Some(project_id) = pinned
                .as_ref()
                .map(|v| v.project_id.clone())
                .or(pending.project_id)
            else {
                continue;
            };
            if !known.insert(project_id.clone()) {
                continue;
            }
            let project = modrinth::project(ctx, &project_id).await?;
            let kind = ContentKind::from_modrinth(&project.project_type).ok_or_else(|| {
                Error::InvalidInput(format!("{} ne peut pas être installé ici", project.title))
            })?;
            let version = match pinned {
                Some(version) => version,
                None => pick_version(ctx, instance, &project, kind).await?,
            };
            self.push(project, kind, version, pending.is_dependency);
        }
        Ok(())
    }

    /// Downloads every planned file into the store, then places it in the instance,
    /// replacing the previous file of content that was already installed.
    async fn apply(
        self,
        ctx: &Context,
        instance: &Instance,
        existing: &HashMap<String, InstalledContent>,
    ) -> Result<Vec<InstalledContent>> {
        let mut tasks = Vec::new();
        for item in &self.items {
            let file = item.version.primary_file().ok_or_else(|| {
                Error::InvalidInput(format!(
                    "{} n'a pas de fichier à télécharger",
                    item.project.title
                ))
            })?;
            tasks.push(DownloadTask {
                url: file.url.clone(),
                dest: store::path(&ctx.data, &file.hashes.sha1),
                sha1: Some(file.hashes.sha1.clone()),
                size: Some(file.size),
            });
        }
        download::download_all(&ctx.http, tasks, download::DEFAULT_CONCURRENCY, |_| {}).await?;

        let mut done = Vec::new();
        for item in self.items {
            let Some(file) = item.version.primary_file() else {
                continue;
            };
            let previous = existing.get(&item.project.id);
            let content = InstalledContent {
                project_id: item.project.id.clone(),
                version_id: item.version.id.clone(),
                kind: item.kind,
                title: item.project.title.clone(),
                version_number: item.version.version_number.clone(),
                file_name: file.filename.clone(),
                sha1: file.hashes.sha1.clone(),
                icon_url: item.project.icon_url.clone(),
                // An update keeps the player's choices.
                is_dependency: previous.map_or(item.is_dependency, |p| p.is_dependency),
                enabled: previous.is_none_or(|p| p.enabled),
                installed_at: previous.map(|p| p.installed_at.clone()).unwrap_or_default(),
            };
            let dest = content.path(ctx, &instance.id);
            store::link(&store::path(&ctx.data, &content.sha1), &dest).await?;
            if let Some(previous) = previous {
                let old = previous.path(ctx, &instance.id);
                if old != dest {
                    remove_file(&old).await?;
                }
            }
            ctx.db.upsert_content(&instance.id, &content).await?;
            tracing::info!(
                instance = %instance.id,
                project = %content.title,
                version = %content.version_number,
                dependency = content.is_dependency,
                updated = previous.is_some(),
                "content installed"
            );
            done.push(content);
        }
        Ok(done)
    }
}

async fn remove_file(path: &std::path::Path) -> Result<()> {
    match tokio::fs::remove_file(path).await {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => Err(err.into()),
        _ => Ok(()),
    }
}

fn by_project(items: Vec<InstalledContent>) -> HashMap<String, InstalledContent> {
    items
        .into_iter()
        .map(|c| (c.project_id.clone(), c))
        .collect()
}

/// Installs a Modrinth project (id or slug) and its missing required dependencies,
/// each in the version that fits the instance, unless `version_id` pins the project's
/// version. Returns what was installed.
pub async fn install(
    ctx: &Context,
    instance: &Instance,
    project: &str,
    version_id: Option<&str>,
) -> Result<Vec<InstalledContent>> {
    let existing = by_project(ctx.db.list_content(&instance.id).await?);
    let mut known: HashSet<String> = existing.keys().cloned().collect();

    let root = modrinth::project(ctx, project).await?;
    if known.contains(&root.id) {
        return Err(Error::InvalidInput(format!(
            "{} est déjà installé",
            root.title
        )));
    }
    let mut plan = Plan::default();
    plan.queue.push_back(Pending {
        project_id: Some(root.id),
        version_id: version_id.map(str::to_owned),
        is_dependency: false,
    });
    plan.resolve(ctx, instance, &mut known).await?;
    plan.apply(ctx, instance, &existing).await
}

/// Newest release compatible with the instance, else the newest beta/alpha.
async fn pick_version(
    ctx: &Context,
    instance: &Instance,
    project: &Project,
    kind: ContentKind,
) -> Result<Version> {
    let loaders = match kind {
        ContentKind::Mod => {
            let loaders = modrinth::mod_loaders(instance.loader);
            if loaders.is_empty() {
                return Err(Error::ModLoaderRequired);
            }
            loaders
        }
        ContentKind::ResourcePack | ContentKind::Shader => &[],
    };
    let versions =
        modrinth::project_versions(ctx, &project.id, &instance.game_version, loaders).await?;
    let release = versions.iter().position(|v| v.version_type == "release");
    versions
        .into_iter()
        .nth(release.unwrap_or(0))
        .ok_or_else(|| Error::ContentUnavailable {
            title: project.title.clone(),
            game_version: instance.game_version.clone(),
        })
}

/// Newer compatible versions of `items`, keyed by project id. Releases are preferred;
/// content with no compatible release falls back to betas. A candidate older than the
/// installed version (e.g. installed beta, latest release behind it) is not an update.
async fn newer_versions(
    ctx: &Context,
    instance: &Instance,
    items: &[InstalledContent],
) -> Result<HashMap<String, Version>> {
    let mut groups: HashMap<ContentKind, Vec<&InstalledContent>> = HashMap::new();
    for item in items {
        groups.entry(item.kind).or_default().push(item);
    }

    let mut latest: HashMap<String, Version> = HashMap::new();
    for (kind, group) in groups {
        let loaders = match kind {
            ContentKind::Mod => modrinth::mod_loaders(instance.loader),
            ContentKind::ResourcePack | ContentKind::Shader => &[],
        };
        let hashes: Vec<String> = group.iter().map(|c| c.sha1.clone()).collect();
        let mut found =
            modrinth::latest_versions(ctx, &hashes, &instance.game_version, loaders, true).await?;
        let missing: Vec<String> = hashes
            .into_iter()
            .filter(|h| !found.contains_key(h))
            .collect();
        if !missing.is_empty() {
            found.extend(
                modrinth::latest_versions(ctx, &missing, &instance.game_version, loaders, false)
                    .await?,
            );
        }
        for item in group {
            if let Some(version) = found.remove(&item.sha1) {
                if version.id != item.version_id && version.project_id == item.project_id {
                    latest.insert(item.project_id.clone(), version);
                }
            }
        }
    }
    if latest.is_empty() {
        return Ok(latest);
    }

    let current_ids: Vec<String> = items
        .iter()
        .filter(|c| latest.contains_key(&c.project_id))
        .map(|c| c.version_id.clone())
        .collect();
    let published: HashMap<String, String> = modrinth::versions(ctx, &current_ids)
        .await?
        .into_iter()
        .map(|v| (v.project_id, v.date_published))
        .collect();
    latest.retain(|project_id, candidate| {
        published
            .get(project_id)
            .is_none_or(|current| candidate.date_published > *current)
    });
    Ok(latest)
}

/// Lists installed content that has a newer compatible version.
pub async fn check_updates(ctx: &Context, instance: &Instance) -> Result<Vec<ContentUpdate>> {
    let installed = ctx.db.list_content(&instance.id).await?;
    let latest = newer_versions(ctx, instance, &installed).await?;
    let mut updates: Vec<ContentUpdate> = installed
        .into_iter()
        .filter_map(|item| {
            let version = latest.get(&item.project_id)?;
            Some(ContentUpdate {
                project_id: item.project_id,
                title: item.title,
                current_version: item.version_number,
                new_version: version.version_number.clone(),
            })
        })
        .collect();
    updates.sort_by_key(|u| u.title.to_lowercase());
    Ok(updates)
}

/// Updates the given projects (all updatable content when `None`), installing any
/// new required dependency. Returns what changed.
pub async fn update(
    ctx: &Context,
    instance: &Instance,
    project_ids: Option<&[String]>,
) -> Result<Vec<InstalledContent>> {
    let installed = ctx.db.list_content(&instance.id).await?;
    let targets: Vec<InstalledContent> = installed
        .iter()
        .filter(|c| project_ids.is_none_or(|ids| ids.contains(&c.project_id)))
        .cloned()
        .collect();
    let latest = newer_versions(ctx, instance, &targets).await?;
    let existing = by_project(installed);
    let mut known: HashSet<String> = existing.keys().cloned().collect();

    let mut plan = Plan::default();
    for (project_id, version) in latest {
        let Some(current) = existing.get(&project_id) else {
            continue;
        };
        let project = modrinth::project(ctx, &project_id).await?;
        plan.push(project, current.kind, version, current.is_dependency);
    }
    plan.resolve(ctx, instance, &mut known).await?;
    plan.apply(ctx, instance, &existing).await
}

/// Enables or disables content by renaming its file (`x.jar` ↔ `x.jar.disabled`).
pub async fn set_enabled(
    ctx: &Context,
    instance_id: &str,
    project_id: &str,
    enabled: bool,
) -> Result<InstalledContent> {
    let mut item = ctx.db.get_content(instance_id, project_id).await?;
    if item.enabled != enabled {
        let from = item.path(ctx, instance_id);
        item.enabled = enabled;
        tokio::fs::rename(&from, item.path(ctx, instance_id)).await?;
        ctx.db
            .set_content_enabled(instance_id, project_id, enabled)
            .await?;
        tracing::info!(instance = %instance_id, project = %item.title, enabled, "content toggled");
    }
    Ok(item)
}

#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;

    /// A content row pointing at `folder/file_name`, for tests elsewhere in the crate.
    pub async fn add_row(
        ctx: &Context,
        instance_id: &str,
        folder: &str,
        file_name: &str,
        sha1: &str,
    ) {
        let kind = [
            ContentKind::Mod,
            ContentKind::ResourcePack,
            ContentKind::Shader,
        ]
        .into_iter()
        .find(|k| k.folder() == folder)
        .unwrap();
        ctx.db
            .upsert_content(
                instance_id,
                &InstalledContent {
                    project_id: file_name.into(),
                    version_id: "v".into(),
                    kind,
                    title: file_name.into(),
                    version_number: "1".into(),
                    file_name: file_name.into(),
                    sha1: sha1.into(),
                    icon_url: None,
                    is_dependency: false,
                    enabled: true,
                    installed_at: String::new(),
                },
            )
            .await
            .unwrap();
    }
}

/// Copies the content rows of an instance to another (after its files were copied).
pub async fn copy_content(db: &Database, from: &str, to: &str) -> Result<()> {
    for item in db.list_content(from).await? {
        db.upsert_content(to, &item).await?;
    }
    Ok(())
}

/// Removes a project's file from the instance. Dependencies are left in place: other
/// content may still need them.
pub async fn remove(ctx: &Context, instance_id: &str, project_id: &str) -> Result<()> {
    let item = ctx.db.get_content(instance_id, project_id).await?;
    remove_file(&item.path(ctx, instance_id)).await?;
    ctx.db.delete_content(instance_id, project_id).await?;
    tracing::info!(instance = %instance_id, project = %item.title, "content removed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::NewInstance;
    use crate::paths::DataDir;

    fn sodium() -> InstalledContent {
        InstalledContent {
            project_id: "AANobbMI".into(),
            version_id: "v1".into(),
            kind: ContentKind::Mod,
            title: "Sodium".into(),
            version_number: "0.6.13".into(),
            file_name: "sodium.jar".into(),
            sha1: "abc".into(),
            icon_url: None,
            is_dependency: false,
            enabled: true,
            installed_at: String::new(),
        }
    }

    #[tokio::test]
    async fn content_rows() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Database::open(&tmp.path().join("t.db")).await.unwrap();
        let instance = db
            .create_instance(&NewInstance {
                name: "Modded".into(),
                game_version: "1.21.4".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        db.upsert_content(&instance.id, &sodium()).await.unwrap();
        let first = db.list_content(&instance.id).await.unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].kind, ContentKind::Mod);
        assert!(first[0].enabled);
        assert!(!first[0].installed_at.is_empty());

        // An update replaces the version but keeps the install date.
        let updated = InstalledContent {
            version_id: "v2".into(),
            ..sodium()
        };
        db.upsert_content(&instance.id, &updated).await.unwrap();
        db.set_content_enabled(&instance.id, "AANobbMI", false)
            .await
            .unwrap();
        let second = db.get_content(&instance.id, "AANobbMI").await.unwrap();
        assert_eq!(second.version_id, "v2");
        assert!(!second.enabled);
        assert_eq!(second.installed_at, first[0].installed_at);

        // Rows follow their instance.
        db.delete_instance(&instance.id).await.unwrap();
        assert!(db.list_content(&instance.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn toggling_renames_the_file() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = Context::init(DataDir::new(tmp.path())).await.unwrap();
        let instance = ctx
            .db
            .create_instance(&NewInstance {
                name: "Modded".into(),
                game_version: "1.21.4".into(),
                ..Default::default()
            })
            .await
            .unwrap();
        ctx.db
            .upsert_content(&instance.id, &sodium())
            .await
            .unwrap();
        let mods = ctx.data.instance_dir(&instance.id).join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("sodium.jar"), b"jar").unwrap();

        let off = set_enabled(&ctx, &instance.id, "AANobbMI", false)
            .await
            .unwrap();
        assert!(!off.enabled);
        assert!(!mods.join("sodium.jar").exists());
        assert!(mods.join("sodium.jar.disabled").exists());

        set_enabled(&ctx, &instance.id, "AANobbMI", true)
            .await
            .unwrap();
        assert!(mods.join("sodium.jar").exists());

        set_enabled(&ctx, &instance.id, "AANobbMI", false)
            .await
            .unwrap();
        remove(&ctx, &instance.id, "AANobbMI").await.unwrap();
        assert!(!mods.join("sodium.jar.disabled").exists());
    }
}
