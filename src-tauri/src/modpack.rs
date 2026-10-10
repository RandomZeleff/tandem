//! Modpack installs: create the instance up front so the UI can show its progress,
//! then roll it back if anything fails.

use std::path::Path;

use tandem_core::content::curseforge::{self, Manifest};
use tandem_core::content::mrpack::{self, PackIndex};
use tandem_core::content::pack_update::{self, UpdateReport};
use tandem_core::instance::NewInstance;
use tandem_core::instance::{self, Instance, PackOrigin};
use tandem_core::Context;
use tauri::{AppHandle, Emitter};

use crate::error::{CommandError, CommandResult};
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
    version_id: Option<&str>,
) -> CommandResult<Instance> {
    let pack = mrpack::download(ctx, project_id, version_id).await?;
    let index = mrpack::read_index(&pack.path).await?;
    let origin = PackOrigin {
        project_id: pack.project.id.clone(),
        version_id: pack.version.id.clone(),
        version: pack.version.version_number.clone(),
        icon: pack.project.icon_url.clone(),
    };
    let new = index.new_instance(Some(&pack.project.title))?;
    let installed = install(
        app,
        ctx,
        games,
        &pack.path,
        new,
        Source::Modrinth(&index),
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

/// Moves an instance to another version of its Modrinth modpack.
pub async fn update_from_modrinth(
    app: &AppHandle,
    ctx: &Context,
    games: &Games,
    instance_id: &str,
    version_id: &str,
) -> CommandResult<UpdateReport> {
    let instance = ctx.db.get_instance(instance_id).await?;
    let project = instance
        .pack_project_id
        .clone()
        .ok_or_else(|| CommandError::msg("Cette instance ne vient pas d'un modpack Modrinth"))?;
    if !games.begin(instance_id) {
        return Err(CommandError::msg(
            "Arrête le jeu avant de mettre à jour le modpack",
        ));
    }
    let result = async {
        let pack = mrpack::download(ctx, &project, Some(version_id)).await?;
        let origin = PackOrigin {
            project_id: pack.project.id.clone(),
            version_id: pack.version.id.clone(),
            version: pack.version.version_number.clone(),
            icon: pack.project.icon_url.clone(),
        };
        let report = pack_update::update(
            ctx,
            &instance,
            &pack.path,
            Some(origin),
            progress_emitter(app, instance_id),
        )
        .await;
        if report.is_ok() {
            let _ = tokio::fs::remove_file(&pack.path).await;
        }
        report
    }
    .await;
    games.end(instance_id);
    let _ = app.emit(INSTALL_FINISHED_EVENT, instance_id);
    let _ = app.emit(INSTANCES_CHANGED_EVENT, ());
    crate::translation::refresh_later(app, instance_id);
    Ok(result?)
}

/// Undoes the last modpack update of an instance.
pub async fn rollback(
    app: &AppHandle,
    ctx: &Context,
    games: &Games,
    instance_id: &str,
) -> CommandResult<()> {
    let instance = ctx.db.get_instance(instance_id).await?;
    if !games.begin(instance_id) {
        return Err(CommandError::msg(
            "Arrête le jeu avant d'annuler la mise à jour",
        ));
    }
    let result = pack_update::rollback(ctx, &instance).await;
    games.end(instance_id);
    let _ = app.emit(INSTALL_FINISHED_EVENT, instance_id);
    let _ = app.emit(INSTANCES_CHANGED_EVENT, ());
    crate::translation::refresh_later(app, instance_id);
    Ok(result?)
}

/// Installs a modpack file chosen by the player: a Modrinth `.mrpack` or a CurseForge `.zip`.
pub async fn import_file(
    app: &AppHandle,
    ctx: &Context,
    games: &Games,
    path: &Path,
) -> CommandResult<Instance> {
    if let Some(manifest) = curseforge::inspect(path).await? {
        let key = curseforge::stored_key()
            .await?
            .ok_or_else(|| CommandError::msg(CURSEFORGE_KEY_MISSING))?;
        let new = manifest.new_instance()?;
        return install(
            app,
            ctx,
            games,
            path,
            new,
            Source::CurseForge(&manifest, &key),
            None,
        )
        .await;
    }
    let index = mrpack::read_index(path).await?;
    let new = index.new_instance(None)?;
    install(app, ctx, games, path, new, Source::Modrinth(&index), None).await
}

/// Sent when a CurseForge pack is imported without a saved key, so the UI can ask for one.
pub const CURSEFORGE_KEY_MISSING: &str = "curseforge-key-missing";

/// Which kind of pack a file is: `modrinth` or `curseforge`.
pub async fn kind(path: &Path) -> CommandResult<&'static str> {
    Ok(if curseforge::inspect(path).await?.is_some() {
        "curseforge"
    } else {
        "modrinth"
    })
}

enum Source<'a> {
    Modrinth(&'a PackIndex),
    CurseForge(&'a Manifest, &'a str),
}

async fn install(
    app: &AppHandle,
    ctx: &Context,
    games: &Games,
    pack: &Path,
    new: NewInstance,
    source: Source<'_>,
    origin: Option<PackOrigin>,
) -> CommandResult<Instance> {
    let created = instance::create(ctx, new).await?;
    let id = created.id.clone();
    games.begin(&id);
    let _ = app.emit(INSTANCES_CHANGED_EVENT, ());

    let result = async {
        if let Some(origin) = &origin {
            ctx.db.set_instance_pack(&id, origin).await?;
        }
        let progress = progress_emitter(app, &id);
        match source {
            Source::Modrinth(index) => {
                mrpack::install(ctx, &created, pack, index, progress).await?;
            }
            Source::CurseForge(manifest, key) => {
                curseforge::install(ctx, &created, pack, manifest, key, progress).await?;
            }
        }
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
