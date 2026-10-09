//! Performance mods worth suggesting for an instance, checked live against Modrinth.

use std::collections::HashSet;
use std::path::Path;

use futures_util::future::try_join_all;
use serde::Serialize;

use super::modrinth;
use crate::context::Context;
use crate::error::Result;
use crate::instance::Instance;

/// Modrinth project ids, most impactful first.
const CATALOG: &[&str] = &[
    SODIUM, EMBEDDIUM, "gvQqBUqZ", // Lithium
    "uXXizFIs", // FerriteCore
    "nmDcB62a", // ModernFix
    "5ZwdcRci", // ImmediatelyFast
    "NNAgCjsB", // Entity Culling
    "LQ3K71Q1", // Dynamic FPS
];

const SODIUM: &str = "AANobbMI";
const EMBEDDIUM: &str = "sk9rgfiA";
const RUBIDIUM: &str = "4ZqxOvjD";
/// Rendering engines replace each other: an instance gets one at most.
const RENDERERS: &[&str] = &[SODIUM, EMBEDDIUM, RUBIDIUM];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfSuggestion {
    pub project_id: String,
    pub title: String,
    pub icon_url: Option<String>,
}

/// Catalog mods the instance lacks that have a build for its game version and loader.
/// Vanilla instances get nothing: these mods need a loader.
pub async fn suggestions(ctx: &Context, instance: &Instance) -> Result<Vec<PerfSuggestion>> {
    let loaders = modrinth::mod_loaders(instance.loader);
    if loaders.is_empty() {
        return Ok(Vec::new());
    }
    let installed: HashSet<String> = ctx
        .db
        .list_content(&instance.id)
        .await?
        .into_iter()
        .map(|c| c.project_id)
        .collect();
    let has_renderer = RENDERERS.iter().any(|id| installed.contains(*id))
        || has_optifine(&ctx.data.instance_dir(&instance.id));

    let candidates: Vec<&str> = CATALOG
        .iter()
        .copied()
        .filter(|id| !installed.contains(*id))
        .filter(|id| !(has_renderer && RENDERERS.contains(id)))
        .collect();
    let compatible = try_join_all(candidates.iter().map(|id| async move {
        let versions = modrinth::project_versions(ctx, id, &instance.game_version, loaders).await?;
        Ok::<_, crate::Error>(!versions.is_empty())
    }))
    .await?;

    let mut picked: Vec<String> = Vec::new();
    for (id, ok) in candidates.into_iter().zip(compatible) {
        let renderer_taken =
            RENDERERS.contains(&id) && picked.iter().any(|p| RENDERERS.contains(&p.as_str()));
        if ok && !renderer_taken {
            picked.push(id.to_owned());
        }
    }

    let projects = modrinth::projects(ctx, &picked).await?;
    Ok(picked
        .iter()
        .filter_map(|id| projects.iter().find(|p| &p.id == id))
        .map(|p| PerfSuggestion {
            project_id: p.id.clone(),
            title: p.title.clone(),
            icon_url: p.icon_url.clone(),
        })
        .collect())
}

/// OptiFine is not on Modrinth and clashes with Sodium and its forks.
fn has_optifine(game_dir: &Path) -> bool {
    std::fs::read_dir(game_dir.join("mods")).is_ok_and(|entries| {
        entries.filter_map(|e| e.ok()).any(|e| {
            e.file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("optifine")
        })
    })
}
