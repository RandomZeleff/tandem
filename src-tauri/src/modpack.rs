//! Modpack installs: create the instance up front so the UI can show its progress,
//! then roll it back if anything fails.

use std::path::Path;

use tandem_core::content::mrpack::{self, PackIndex};
use tandem_core::instance::{self, Instance, PackOrigin};
use tandem_core::Context;
use tauri::{AppHandle, Emitter};

use crate::error::CommandResult;
use crate::game::{progress_emitter, Games};

/// The instance list changed outside of a direct user action (created or rolled back).
pub const INSTANCES_CHANGED_EVENT: &str = "instances://changed";
/// A non-game install task ended for an instance (payload: instance id).
pub const INSTALL_FINISHED_EVENT: &str = "install://finished";

/// Downloads the latest compatible version of a Modrinth modpack and installs it.
pub async fn install_from_modrinth(
    app: &AppHandle,
    ctx: &Context,
    games: &Games,
    project_id: &str,
) -> CommandResult<Instance> {
    let pack = mrpack::download(ctx, project_id).await?;
    let index = mrpack::read_index(&pack.path).await?;
    let origin = PackOrigin {
        project_id: pack.project.id.clone(),
        version_id: pack.version.id.clone(),
        version: pack.version.version_number.clone(),
        icon: pack.project.icon_url.clone(),
    };
    let installed = install(
        app,
        ctx,
        games,
        &pack.path,
        index,
        Some(&pack.project.title),
        Some(origin),
    )
    .await?;
    // Everything it holds now lives in the instance (some packs weigh 400+ MB). Kept after
    // a failure: named by its hash, a retry reuses it instead of downloading it again.
    if let Err(err) = tokio::fs::remove_file(&pack.path).await {
        tracing::warn!(path = %pack.path.display(), error = %err, "could not remove the downloaded pack");
    }
    Ok(installed)
}

/// Installs a `.mrpack` file chosen by the player.
pub async fn import_file(
    app: &AppHandle,
    ctx: &Context,
    games: &Games,
    path: &Path,
) -> CommandResult<Instance> {
    let index = mrpack::read_index(path).await?;
    install(app, ctx, games, path, index, None, None).await
}

async fn install(
    app: &AppHandle,
    ctx: &Context,
    games: &Games,
    pack: &Path,
    index: PackIndex,
    name: Option<&str>,
    origin: Option<PackOrigin>,
) -> CommandResult<Instance> {
    let created = instance::create(ctx, index.new_instance(name)?).await?;
    let id = created.id.clone();
    games.begin(&id);
    let _ = app.emit(INSTANCES_CHANGED_EVENT, ());

    let result = async {
        if let Some(origin) = &origin {
            ctx.db.set_instance_pack(&id, origin).await?;
        }
        mrpack::install(ctx, &created, pack, &index, progress_emitter(app, &id)).await?;
        ctx.db.get_instance(&id).await
    }
    .await;

    games.end(&id);
    let _ = app.emit(INSTALL_FINISHED_EVENT, &id);
    match result {
        Ok(instance) => Ok(instance),
        Err(err) => {
            tracing::error!(instance = %id, error = %err, "modpack install failed, rolling back");
            if let Err(cleanup) = instance::delete(ctx, &id).await {
                tracing::warn!(instance = %id, error = %cleanup, "rollback incomplete");
            }
            let _ = app.emit(INSTANCES_CHANGED_EVENT, ());
            Err(err.into())
        }
    }
}
