use serde::Serialize;
use serde_json::Value;
use tandem_core::account::Account;
use tandem_core::content::modrinth::{self, SearchFilter, SearchResults};
use tandem_core::content::{self, ContentKind, ContentUpdate, InstalledContent};
use tandem_core::instance::{self, Instance, NewInstance};
use tandem_core::logging::LogEntry;
use tandem_core::meta::loader::{self, Loader, LoaderVersion};
use tandem_core::meta::{self, Latest, ManifestEntry};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{CommandError, CommandResult};
use crate::{game, AppState};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    version: &'static str,
    data_dir: String,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        version: tandem_core::version(),
        data_dir: state.ctx.data.root().display().to_string(),
    }
}

#[tauri::command]
pub fn get_logs(state: State<'_, AppState>) -> Vec<LogEntry> {
    state.logs.snapshot()
}

#[tauri::command]
pub async fn get_setting(state: State<'_, AppState>, key: String) -> CommandResult<Option<Value>> {
    Ok(state.ctx.db.get_setting(&key).await?)
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: Value,
) -> CommandResult<()> {
    state.ctx.db.set_setting(&key, &value).await?;
    Ok(())
}

#[derive(Serialize)]
pub struct VersionList {
    latest: Latest,
    versions: Vec<ManifestEntry>,
}

#[tauri::command]
pub async fn list_versions(state: State<'_, AppState>) -> CommandResult<VersionList> {
    let manifest = meta::fetch_manifest(&state.ctx).await?;
    Ok(VersionList {
        latest: manifest.latest,
        versions: manifest.versions,
    })
}

#[tauri::command]
pub async fn list_loader_versions(
    state: State<'_, AppState>,
    loader: Loader,
    game_version: String,
) -> CommandResult<Vec<LoaderVersion>> {
    Ok(loader::list_versions(&state.ctx, loader, &game_version).await?)
}

#[tauri::command]
pub async fn list_instances(state: State<'_, AppState>) -> CommandResult<Vec<Instance>> {
    Ok(state.ctx.db.list_instances().await?)
}

#[tauri::command]
pub async fn create_instance(
    state: State<'_, AppState>,
    instance: NewInstance,
) -> CommandResult<Instance> {
    Ok(instance::create(&state.ctx, instance).await?)
}

#[tauri::command]
pub async fn delete_instance(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    if state.games.is_busy(&id) {
        return Err(CommandError::msg(
            "stop the game before deleting this instance",
        ));
    }
    state.ctx.db.delete_instance(&id).await?;
    match tokio::fs::remove_dir_all(state.ctx.data.instance_dir(&id)).await {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err.into()),
        _ => {}
    }
    tracing::info!(%id, "instance deleted");
    Ok(())
}

#[tauri::command]
pub async fn search_content(
    state: State<'_, AppState>,
    query: String,
    kind: ContentKind,
    instance_id: Option<String>,
    offset: u32,
) -> CommandResult<SearchResults> {
    let instance = match &instance_id {
        Some(id) => Some(state.ctx.db.get_instance(id).await?),
        None => None,
    };
    let filter = SearchFilter {
        query: &query,
        kind,
        game_version: instance.as_ref().map(|i| i.game_version.as_str()),
        loader: instance.as_ref().map(|i| i.loader),
        offset,
        limit: 20,
    };
    Ok(modrinth::search(&state.ctx, &filter).await?)
}

#[tauri::command]
pub async fn list_content(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<InstalledContent>> {
    Ok(state.ctx.db.list_content(&instance_id).await?)
}

#[tauri::command]
pub async fn install_content(
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
) -> CommandResult<Vec<InstalledContent>> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(content::install(&state.ctx, &instance, &project_id, None).await?)
}

#[tauri::command]
pub async fn check_content_updates(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<ContentUpdate>> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(content::check_updates(&state.ctx, &instance).await?)
}

/// Updates the given projects, or every outdated one when `project_ids` is absent.
#[tauri::command]
pub async fn update_content(
    state: State<'_, AppState>,
    instance_id: String,
    project_ids: Option<Vec<String>>,
) -> CommandResult<Vec<InstalledContent>> {
    if state.games.is_busy(&instance_id) {
        return Err(CommandError::msg("stop the game before updating content"));
    }
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(content::update(&state.ctx, &instance, project_ids.as_deref()).await?)
}

#[tauri::command]
pub async fn set_content_enabled(
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
    enabled: bool,
) -> CommandResult<InstalledContent> {
    if state.games.is_busy(&instance_id) {
        return Err(CommandError::msg(
            "stop the game before enabling or disabling content",
        ));
    }
    Ok(content::set_enabled(&state.ctx, &instance_id, &project_id, enabled).await?)
}

#[tauri::command]
pub async fn remove_content(
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
) -> CommandResult<()> {
    if state.games.is_busy(&instance_id) {
        return Err(CommandError::msg("stop the game before removing content"));
    }
    Ok(content::remove(&state.ctx, &instance_id, &project_id).await?)
}

#[tauri::command]
pub async fn open_instance_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> CommandResult<()> {
    let dir = state.ctx.data.instance_dir(&id);
    tokio::fs::create_dir_all(&dir).await?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| CommandError::msg(e.to_string()))
}

#[tauri::command]
pub async fn launch_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> CommandResult<()> {
    game::launch(app, state.ctx.clone(), state.games.clone(), id).await
}

#[tauri::command]
pub fn stop_instance(state: State<'_, AppState>, id: String) -> bool {
    state.games.stop(&id)
}

#[tauri::command]
pub fn running_instances(state: State<'_, AppState>) -> Vec<String> {
    state.games.running_ids()
}

#[tauri::command]
pub async fn list_accounts(state: State<'_, AppState>) -> CommandResult<Vec<Account>> {
    Ok(state.ctx.db.list_accounts().await?)
}

#[tauri::command]
pub async fn add_offline_account(
    state: State<'_, AppState>,
    username: String,
) -> CommandResult<Account> {
    Ok(state.ctx.db.add_offline_account(&username).await?)
}

#[tauri::command]
pub async fn set_active_account(state: State<'_, AppState>, id: String) -> CommandResult<Account> {
    Ok(state.ctx.db.set_active_account(&id).await?)
}

#[tauri::command]
pub async fn remove_account(state: State<'_, AppState>, id: String) -> CommandResult<()> {
    Ok(state.ctx.db.remove_account(&id).await?)
}
