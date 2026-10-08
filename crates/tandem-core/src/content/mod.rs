//! Instance content (mods, resource packs, shaders): installation from Modrinth with
//! required dependencies, files kept in the shared store and hardlinked into instances.

pub mod modrinth;

use std::collections::{HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::db::Database;
use crate::download::{self, DownloadTask};
use crate::error::{Error, Result};
use crate::instance::Instance;
use crate::store;
use modrinth::{Project, Version};

/// Upper bound on projects pulled by one install, guarding against dependency loops.
const MAX_PROJECTS_PER_INSTALL: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
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
    pub file_name: String,
    pub sha1: String,
    pub icon_url: Option<String>,
    pub is_dependency: bool,
    pub installed_at: String,
}

const COLUMNS: &str = "project_id, version_id, kind, title, version_number, file_name, sha1, \
                       icon_url, is_dependency, installed_at";

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

    async fn insert_content(&self, instance_id: &str, item: &InstalledContent) -> Result<()> {
        sqlx::query(
            "INSERT OR REPLACE INTO instance_content
             (instance_id, project_id, version_id, kind, title, version_number, file_name, sha1,
              icon_url, is_dependency)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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

/// Installs a Modrinth project (id or slug) and its missing required dependencies,
/// each in the version that fits the instance. Returns what was installed.
pub async fn install(
    ctx: &Context,
    instance: &Instance,
    project: &str,
) -> Result<Vec<InstalledContent>> {
    let installed = ctx.db.list_content(&instance.id).await?;
    let mut known: HashSet<String> = installed.into_iter().map(|c| c.project_id).collect();

    let root = modrinth::project(ctx, project).await?;
    if known.contains(&root.id) {
        return Err(Error::InvalidInput(format!(
            "{} is already installed",
            root.title
        )));
    }

    let mut plan: Vec<Planned> = Vec::new();
    let mut queue = VecDeque::from([Pending {
        project_id: Some(root.id),
        version_id: None,
        is_dependency: false,
    }]);
    while let Some(pending) = queue.pop_front() {
        if plan.len() >= MAX_PROJECTS_PER_INSTALL {
            return Err(Error::InvalidInput("too many dependencies".into()));
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
        let kind = ContentKind::from_modrinth(&project.project_type)
            .ok_or_else(|| Error::InvalidInput(format!("{} cannot be installed", project.title)))?;
        let version = match pinned {
            Some(version) => version,
            None => pick_version(ctx, instance, &project, kind).await?,
        };
        queue.extend(
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
        plan.push(Planned {
            project,
            kind,
            version,
            is_dependency: pending.is_dependency,
        });
    }

    let mut tasks = Vec::new();
    for item in &plan {
        let file = item
            .version
            .primary_file()
            .ok_or_else(|| Error::InvalidInput(format!("{} has no file", item.project.title)))?;
        tasks.push(DownloadTask {
            url: file.url.clone(),
            dest: store::path(&ctx.data, &file.hashes.sha1),
            sha1: Some(file.hashes.sha1.clone()),
            size: Some(file.size),
        });
    }
    download::download_all(&ctx.http, tasks, download::DEFAULT_CONCURRENCY, |_| {}).await?;

    let game_dir = ctx.data.instance_dir(&instance.id);
    let mut done = Vec::new();
    for item in plan {
        let Some(file) = item.version.primary_file() else {
            continue;
        };
        let stored = store::path(&ctx.data, &file.hashes.sha1);
        store::link(
            &stored,
            &game_dir.join(item.kind.folder()).join(&file.filename),
        )
        .await?;
        let content = InstalledContent {
            project_id: item.project.id,
            version_id: item.version.id.clone(),
            kind: item.kind,
            title: item.project.title,
            version_number: item.version.version_number.clone(),
            file_name: file.filename.clone(),
            sha1: file.hashes.sha1.clone(),
            icon_url: item.project.icon_url,
            is_dependency: item.is_dependency,
            installed_at: String::new(),
        };
        ctx.db.insert_content(&instance.id, &content).await?;
        tracing::info!(
            instance = %instance.id,
            project = %content.title,
            version = %content.version_number,
            dependency = content.is_dependency,
            "content installed"
        );
        done.push(content);
    }
    Ok(done)
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

/// Removes a project's file from the instance. Dependencies are left in place: other
/// content may still need them.
pub async fn remove(ctx: &Context, instance_id: &str, project_id: &str) -> Result<()> {
    let item = ctx.db.get_content(instance_id, project_id).await?;
    let file = ctx
        .data
        .instance_dir(instance_id)
        .join(item.kind.folder())
        .join(&item.file_name);
    match tokio::fs::remove_file(&file).await {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err.into()),
        _ => {}
    }
    ctx.db.delete_content(instance_id, project_id).await?;
    tracing::info!(instance = %instance_id, project = %item.title, "content removed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instance::NewInstance;

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
        let item = InstalledContent {
            project_id: "AANobbMI".into(),
            version_id: "v1".into(),
            kind: ContentKind::Mod,
            title: "Sodium".into(),
            version_number: "0.6.13".into(),
            file_name: "sodium.jar".into(),
            sha1: "abc".into(),
            icon_url: None,
            is_dependency: false,
            installed_at: String::new(),
        };
        db.insert_content(&instance.id, &item).await.unwrap();
        let listed = db.list_content(&instance.id).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind, ContentKind::Mod);
        assert!(!listed[0].installed_at.is_empty());

        // Rows follow their instance.
        db.delete_instance(&instance.id).await.unwrap();
        assert!(db.list_content(&instance.id).await.unwrap().is_empty());
    }
}
