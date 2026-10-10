//! Moving an instance to another game version and/or loader. Each Modrinth project
//! gets the version that fits the new target; content with none is disabled, never
//! deleted, so it can come back when its authors catch up.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::{by_project, modrinth, ContentKind, InstalledContent, Plan};
use crate::context::Context;
use crate::error::{Error, Result};
use crate::instance::{self, Instance};
use crate::meta::loader::Loader;
use crate::worlds::{self, BackupKind};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub game_version: String,
    pub loader: Loader,
    /// Latest stable when omitted.
    pub loader_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub title: String,
    pub from: String,
    pub to: String,
}

/// What moving to a target does to the instance's content.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetargetPlan {
    /// The loader version that will be used.
    pub loader_version: Option<String>,
    /// Content whose current file already fits.
    pub kept: usize,
    pub updated: Vec<Change>,
    /// Content with no version for the target: disabled.
    pub unavailable: Vec<String>,
    /// Jar files Tandem does not know (added by hand): left as they are.
    pub unknown: Vec<String>,
    pub game_version_changes: bool,
    /// The instance comes from a modpack and will stop following its updates.
    pub leaves_modpack: bool,
    pub worlds_backed_up: usize,
}

/// The instance as it would be on `target` (for picking versions).
fn retargeted(instance: &Instance, target: &Target, loader_version: Option<String>) -> Instance {
    let mut moved = instance.clone();
    moved.game_version = target.game_version.clone();
    moved.loader = target.loader;
    moved.loader_version = loader_version;
    moved
}

/// For each installed item, the version that fits the target (absent when none does).
async fn fitting_versions(
    ctx: &Context,
    target: &Instance,
    items: &[InstalledContent],
) -> Result<HashMap<String, modrinth::Version>> {
    let mut by_kind: HashMap<ContentKind, Vec<&InstalledContent>> = HashMap::new();
    for item in items {
        by_kind.entry(item.kind).or_default().push(item);
    }
    let mut fitting = HashMap::new();
    for (kind, group) in by_kind {
        let loaders = match kind {
            ContentKind::Mod => {
                let loaders = modrinth::mod_loaders(target.loader);
                if loaders.is_empty() {
                    // A vanilla target runs no mods.
                    continue;
                }
                loaders
            }
            ContentKind::ResourcePack | ContentKind::Shader => &[],
        };
        let hashes: Vec<String> = group.iter().map(|c| c.sha1.clone()).collect();
        // Releases first, betas for what has no release.
        let mut found =
            modrinth::latest_versions(ctx, &hashes, &target.game_version, loaders, true).await?;
        let missing: Vec<String> = hashes
            .iter()
            .filter(|h| !found.contains_key(*h))
            .cloned()
            .collect();
        if !missing.is_empty() {
            found.extend(
                modrinth::latest_versions(ctx, &missing, &target.game_version, loaders, false)
                    .await?,
            );
        }
        for item in group {
            if let Some(version) = found
                .remove(&item.sha1)
                .filter(|v| v.project_id == item.project_id)
            {
                fitting.insert(item.project_id.clone(), version);
            }
        }
    }
    Ok(fitting)
}

/// What moving to `target` would do, without touching anything.
pub async fn plan(ctx: &Context, instance: &Instance, target: &Target) -> Result<RetargetPlan> {
    let loader_version = instance::resolve_loader_version(
        ctx,
        target.loader,
        &target.game_version,
        target.loader_version.clone(),
    )
    .await?;
    let moved = retargeted(instance, target, loader_version.clone());
    let installed = ctx.db.list_content(&instance.id).await?;
    let fitting = fitting_versions(ctx, &moved, &installed).await?;

    let mut plan = RetargetPlan {
        loader_version,
        game_version_changes: target.game_version != instance.game_version,
        leaves_modpack: instance.pack_project_id.is_some(),
        ..Default::default()
    };
    for item in &installed {
        match fitting.get(&item.project_id) {
            Some(version) if version.id == item.version_id => plan.kept += 1,
            Some(version) => plan.updated.push(Change {
                title: item.title.clone(),
                from: item.version_number.clone(),
                to: version.version_number.clone(),
            }),
            // Already disabled content stays as it is.
            None if item.enabled => plan.unavailable.push(item.title.clone()),
            None => {}
        }
    }
    let tracked: HashSet<String> = installed
        .iter()
        .map(|c| c.file_name.trim_end_matches(".disabled").to_owned())
        .collect();
    if let Ok(entries) = std::fs::read_dir(ctx.data.instance_dir(&instance.id).join("mods")) {
        plan.unknown = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".jar") && !tracked.contains(n))
            .collect();
        plan.unknown.sort();
    }
    plan.updated.sort_by_key(|c| c.title.to_lowercase());
    plan.unavailable.sort_by_key(|t| t.to_lowercase());
    Ok(plan)
}

/// Moves the instance to `target`: content updated or disabled, worlds backed up when
/// the game version changes, modpack origin dropped.
pub async fn apply(ctx: &Context, instance: &Instance, target: &Target) -> Result<RetargetPlan> {
    let mut plan = plan(ctx, instance, target).await?;
    let moved = retargeted(instance, target, plan.loader_version.clone());
    let game_dir = ctx.data.instance_dir(&instance.id);

    if plan.game_version_changes {
        let (data, id, dir) = (ctx.data.clone(), instance.id.clone(), game_dir.clone());
        plan.worlds_backed_up = tokio::task::spawn_blocking(move || {
            worlds::list(&dir)
                .iter()
                .filter(|w| {
                    worlds::backup(&data, &id, &dir, &w.folder, BackupKind::BeforeUpdate).is_ok()
                })
                .count()
        })
        .await
        .map_err(|e| Error::Io(std::io::Error::other(e)))?;
    }

    let installed = ctx.db.list_content(&instance.id).await?;
    let fitting = fitting_versions(ctx, &moved, &installed).await?;
    let existing = by_project(installed.clone());
    let mut known: HashSet<String> = existing.keys().cloned().collect();
    let mut content_plan = Plan::default();
    for item in &installed {
        if let Some(version) = fitting
            .get(&item.project_id)
            .filter(|v| v.id != item.version_id)
        {
            let project = modrinth::project(ctx, &item.project_id).await?;
            content_plan.push(project, item.kind, version.clone(), item.is_dependency);
        }
    }
    // New required dependencies are resolved against the target.
    content_plan.resolve(ctx, &moved, &mut known).await?;
    content_plan.apply(ctx, &moved, &existing).await?;

    for item in installed
        .iter()
        .filter(|c| c.enabled && !fitting.contains_key(&c.project_id))
    {
        if let Err(err) = super::set_enabled(ctx, &instance.id, &item.project_id, false).await {
            tracing::warn!(project = %item.title, error = %err, "could not disable content");
        }
    }

    ctx.db
        .set_instance_target(
            &instance.id,
            &target.game_version,
            target.loader,
            plan.loader_version.as_deref(),
        )
        .await?;
    if plan.leaves_modpack {
        ctx.db.clear_instance_pack(&instance.id).await?;
    }
    tracing::info!(instance = %instance.id, version = %target.game_version, loader = %target.loader, updated = plan.updated.len(), disabled = plan.unavailable.len(), "instance version changed");
    Ok(plan)
}
