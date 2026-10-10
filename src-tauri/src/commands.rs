use serde::Serialize;
use serde_json::Value;
use tandem_core::account::Account;
use tandem_core::content::deps::{self, Dependent, Provider};
use tandem_core::content::modrinth::{self, ProjectType, SearchFilter, SearchResults};
use tandem_core::content::project::{self, DependencyItem, ProjectDetails, ProjectVersions};
use tandem_core::content::{self, mrpack, perf, ContentUpdate, InstalledContent};
use tandem_core::install;
use tandem_core::instance::{self, Instance, NewInstance};
use tandem_core::jvm;
use tandem_core::logging::LogEntry;
use tandem_core::meta::loader::{self, Loader, LoaderVersion};
use tandem_core::meta::{self, Latest, ManifestEntry};
use tandem_core::screenshots::{self, Screenshot};
use tandem_core::worlds::{self, Backup, BackupKind, World};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{CommandError, CommandResult, PROJECT_NOT_FOUND};
use crate::{game, modpack, translation, AppState};

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

/// Called by the frontend once its first screen is painted.
#[tauri::command]
pub fn ui_ready() {
    tracing::info!(ms = crate::uptime_ms(), "UI ready");
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
            "Arrête le jeu avant de supprimer cette instance",
        ));
    }
    Ok(instance::delete(&state.ctx, &id).await?)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    /// What "automatic" gives this instance right now.
    auto_mb: u32,
    total_mb: u64,
}

#[tauri::command]
pub fn memory_info(state: State<'_, AppState>, id: String) -> MemoryInfo {
    let total_mb = jvm::total_memory_mb();
    let mods = jvm::count_mods(&state.ctx.data.instance_dir(&id));
    MemoryInfo {
        auto_mb: jvm::auto_memory_mb(total_mb, mods),
        total_mb,
    }
}

#[tauri::command]
pub async fn set_instance_memory(
    state: State<'_, AppState>,
    id: String,
    memory_mb: Option<u32>,
) -> CommandResult<()> {
    if memory_mb.is_some_and(|mb| mb < 512) {
        return Err(CommandError::msg("Il faut au moins 512 Mo de mémoire"));
    }
    Ok(state.ctx.db.set_instance_memory(&id, memory_mb).await?)
}

#[tauri::command]
pub async fn list_worlds(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<World>> {
    let dir = state.ctx.data.instance_dir(&instance_id);
    blocking(move || Ok(worlds::list(&dir))).await
}

#[tauri::command]
pub async fn list_world_backups(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<Backup>> {
    let data = state.ctx.data.clone();
    blocking(move || Ok(worlds::list_backups(&data, &instance_id))).await
}

#[tauri::command]
pub async fn backup_world(
    state: State<'_, AppState>,
    instance_id: String,
    world: String,
) -> CommandResult<Backup> {
    if state.games.is_busy(&instance_id) {
        return Err(CommandError::msg(
            "Arrête le jeu avant de sauvegarder un monde",
        ));
    }
    let (data, dir) = (
        state.ctx.data.clone(),
        state.ctx.data.instance_dir(&instance_id),
    );
    blocking(move || worlds::backup(&data, &instance_id, &dir, &world, BackupKind::Manual)).await
}

#[tauri::command]
pub async fn restore_world_backup(
    state: State<'_, AppState>,
    instance_id: String,
    world: String,
    file_name: String,
) -> CommandResult<()> {
    if state.games.is_busy(&instance_id) {
        return Err(CommandError::msg(
            "Arrête le jeu avant de restaurer un monde",
        ));
    }
    let (data, dir) = (
        state.ctx.data.clone(),
        state.ctx.data.instance_dir(&instance_id),
    );
    blocking(move || worlds::restore(&data, &instance_id, &dir, &world, &file_name)).await
}

#[tauri::command]
pub async fn open_world_backups(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    world: String,
) -> CommandResult<()> {
    let dir = state.ctx.data.backups().join(&instance_id).join(&world);
    tokio::fs::create_dir_all(&dir).await?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| CommandError::msg(e.to_string()))
}

#[tauri::command]
pub async fn list_screenshots(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<Screenshot>> {
    let dir = state.ctx.data.instance_dir(&instance_id);
    blocking(move || Ok(screenshots::list(&dir))).await
}

#[tauri::command]
pub async fn delete_screenshot(
    state: State<'_, AppState>,
    instance_id: String,
    file_name: String,
) -> CommandResult<()> {
    let (data, dir) = (
        state.ctx.data.clone(),
        state.ctx.data.instance_dir(&instance_id),
    );
    blocking(move || screenshots::delete(&data, &instance_id, &dir, &file_name)).await
}

#[tauri::command]
pub async fn open_screenshots_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<()> {
    let dir = state
        .ctx
        .data
        .instance_dir(&instance_id)
        .join("screenshots");
    tokio::fs::create_dir_all(&dir).await?;
    app.opener()
        .open_path(dir.display().to_string(), None::<&str>)
        .map_err(|e| CommandError::msg(e.to_string()))
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> tandem_core::Result<T> + Send + 'static,
) -> CommandResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| CommandError::msg(e.to_string()))?
        .map_err(Into::into)
}

#[tauri::command]
pub async fn search_content(
    state: State<'_, AppState>,
    query: String,
    kind: ProjectType,
    instance_id: Option<String>,
    offset: u32,
) -> CommandResult<SearchResults> {
    // Modpacks create their own instance: the current one does not narrow the search.
    let instance = match (&instance_id, kind) {
        (Some(id), kind) if kind != ProjectType::Modpack => {
            Some(state.ctx.db.get_instance(id).await?)
        }
        _ => None,
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
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
    version_id: Option<String>,
) -> CommandResult<Vec<InstalledContent>> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    let installed =
        content::install(&state.ctx, &instance, &project_id, version_id.as_deref()).await?;
    translation::refresh_later(&app, &instance_id);
    Ok(installed)
}

/// Everything a project page shows (id or slug).
#[tauri::command]
pub async fn project_details(
    state: State<'_, AppState>,
    id: String,
) -> CommandResult<ProjectDetails> {
    match project::details(&state.ctx, &state.projects, &id).await {
        Ok(details) => Ok((*details).clone()),
        Err(tandem_core::Error::HttpStatus { status: 404, .. }) => {
            Err(CommandError::msg(PROJECT_NOT_FOUND))
        }
        Err(err) => Err(err.into()),
    }
}

/// Versions of a project, marked compatible with the instance when one is given.
#[tauri::command]
pub async fn project_versions(
    state: State<'_, AppState>,
    project_id: String,
    instance_id: Option<String>,
) -> CommandResult<ProjectVersions> {
    let instance = match &instance_id {
        Some(id) => Some(state.ctx.db.get_instance(id).await?),
        None => None,
    };
    let target = instance.as_ref().map(Into::into);
    Ok(project::versions(&state.ctx, &state.projects, &project_id, target).await?)
}

#[tauri::command]
pub async fn version_changelog(
    state: State<'_, AppState>,
    project_id: String,
    version_id: String,
) -> CommandResult<String> {
    Ok(project::changelog(&state.ctx, &state.projects, &project_id, &version_id).await?)
}

/// Dependencies of a version, or the content of a modpack version (`embedded`).
#[tauri::command]
pub async fn version_dependencies(
    state: State<'_, AppState>,
    project_id: String,
    version_id: String,
) -> CommandResult<Vec<DependencyItem>> {
    Ok(project::dependencies(&state.ctx, &state.projects, &project_id, &version_id).await?)
}

#[tauri::command]
pub async fn perf_suggestions(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<perf::PerfSuggestion>> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(perf::suggestions(&state.ctx, &instance).await?)
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
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    project_ids: Option<Vec<String>>,
) -> CommandResult<Vec<InstalledContent>> {
    if state.games.is_busy(&instance_id) {
        return Err(CommandError::msg(
            "Arrête le jeu avant de mettre à jour le contenu",
        ));
    }
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    let updated = content::update(&state.ctx, &instance, project_ids.as_deref()).await?;
    translation::refresh_later(&app, &instance_id);
    Ok(updated)
}

/// Reads the instance's mod metadata ahead of time, so the checks below are instant.
#[tauri::command]
pub async fn warm_mod_dependencies(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<()> {
    let (data, dir) = (
        state.ctx.data.clone(),
        state.ctx.data.instance_dir(&instance_id),
    );
    blocking(move || {
        deps::scan_cached(&data, &instance_id, &dir);
        Ok(())
    })
    .await
}

/// Enabled mods that need this content item and would stop the game from starting
/// without it.
#[tauri::command]
pub async fn content_dependents(
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
) -> CommandResult<Vec<Dependent>> {
    let Some(item) = state
        .ctx
        .db
        .list_content(&instance_id)
        .await?
        .into_iter()
        .find(|c| c.project_id == project_id)
    else {
        return Ok(Vec::new());
    };
    let (data, dir) = (
        state.ctx.data.clone(),
        state.ctx.data.instance_dir(&instance_id),
    );
    blocking(move || {
        let mods = deps::scan_cached(&data, &instance_id, &dir);
        Ok(deps::dependents(&mods, &item.file_name))
    })
    .await
}

/// Enabled mods that cannot run together.
#[tauri::command]
pub async fn content_conflicts(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<deps::Conflict>> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(content::conflicts(&state.ctx, &instance).await?)
}

/// The jar behind each mod id (to offer re-enabling a missing dependency).
#[tauri::command]
pub async fn mod_providers(
    state: State<'_, AppState>,
    instance_id: String,
    mod_ids: Vec<String>,
) -> CommandResult<Vec<Provider>> {
    let (data, dir) = (
        state.ctx.data.clone(),
        state.ctx.data.instance_dir(&instance_id),
    );
    blocking(move || {
        Ok(deps::providers(
            &deps::scan_cached(&data, &instance_id, &dir),
            &mod_ids,
        ))
    })
    .await
}

#[tauri::command]
pub async fn set_content_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
    enabled: bool,
) -> CommandResult<InstalledContent> {
    if state.games.is_busy(&instance_id) {
        return Err(CommandError::msg(
            "Arrête le jeu avant d'activer ou de désactiver du contenu",
        ));
    }
    let item = content::set_enabled(&state.ctx, &instance_id, &project_id, enabled).await?;
    translation::refresh_later(&app, &instance_id);
    Ok(item)
}

#[tauri::command]
pub async fn remove_content(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    project_id: String,
) -> CommandResult<()> {
    if state.games.is_busy(&instance_id) {
        return Err(CommandError::msg(
            "Arrête le jeu avant de retirer du contenu",
        ));
    }
    content::remove(&state.ctx, &instance_id, &project_id).await?;
    translation::refresh_later(&app, &instance_id);
    Ok(())
}

/// Creates an instance from the latest compatible version of a Modrinth modpack.
#[tauri::command]
pub async fn install_modpack(
    app: AppHandle,
    state: State<'_, AppState>,
    project_id: String,
    version_id: Option<String>,
) -> CommandResult<Instance> {
    modpack::install_from_modrinth(
        &app,
        &state.ctx,
        &state.games,
        &project_id,
        version_id.as_deref(),
    )
    .await
}

/// Saves the settings the player edits on an instance.
#[tauri::command]
pub async fn update_instance_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    settings: instance::InstanceSettings,
) -> CommandResult<Instance> {
    let updated = instance::update_settings(&state.ctx, &id, &settings).await?;
    let _ = app.emit(modpack::INSTANCES_CHANGED_EVENT, ());
    Ok(updated)
}

/// `image`: a file to use as the icon, or `null` to go back to a block (`block`).
#[tauri::command]
pub async fn set_instance_icon(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    image: Option<String>,
    block: Option<u32>,
) -> CommandResult<Instance> {
    let updated = instance::set_icon(
        &state.ctx,
        &id,
        image.as_deref().map(std::path::Path::new),
        block,
    )
    .await?;
    let _ = app.emit(modpack::INSTANCES_CHANGED_EVENT, ());
    Ok(updated)
}

#[tauri::command]
pub async fn duplicate_instance(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> CommandResult<Instance> {
    if state.games.is_busy(&id) {
        return Err(CommandError::msg(
            "Arrête le jeu avant de dupliquer cette instance",
        ));
    }
    let created = instance::duplicate(&state.ctx, &id, &name).await?;
    let _ = app.emit(modpack::INSTANCES_CHANGED_EVENT, ());
    Ok(created)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaInfo {
    /// Java major version the instance's game version asks for.
    required: u32,
    installs: Vec<tandem_core::java_detect::JavaInstall>,
}

/// Java installations found on the computer, and what the instance needs.
#[tauri::command]
pub async fn instance_java(state: State<'_, AppState>, id: String) -> CommandResult<JavaInfo> {
    let instance = state.ctx.db.get_instance(&id).await?;
    let required = match meta::load_version(&state.ctx, &instance.game_version).await {
        Ok(version) => tandem_core::java_detect::required_major(&version),
        Err(_) => 8,
    };
    let data = state.ctx.data.clone();
    let installs = blocking(move || Ok(tandem_core::java_detect::installed(&data))).await?;
    Ok(JavaInfo { required, installs })
}

#[tauri::command]
pub async fn list_datapacks(
    state: State<'_, AppState>,
    instance_id: String,
    world: String,
) -> CommandResult<Vec<tandem_core::datapacks::Datapack>> {
    Ok(tandem_core::datapacks::list(&state.ctx, &instance_id, &world).await?)
}

#[tauri::command]
pub async fn install_datapack(
    state: State<'_, AppState>,
    instance_id: String,
    world: String,
    project_id: String,
) -> CommandResult<tandem_core::datapacks::Datapack> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(tandem_core::datapacks::install(&state.ctx, &instance, &world, &project_id).await?)
}

#[tauri::command]
pub async fn remove_datapack(
    state: State<'_, AppState>,
    instance_id: String,
    world: String,
    file_name: String,
) -> CommandResult<()> {
    if state.games.is_busy(&instance_id) {
        return Err(CommandError::msg(
            "Arrête le jeu avant de retirer un datapack",
        ));
    }
    Ok(tandem_core::datapacks::remove(&state.ctx, &instance_id, &world, &file_name).await?)
}

/// What moving an instance to another version would do to its content.
#[tauri::command]
pub async fn plan_version_change(
    state: State<'_, AppState>,
    id: String,
    target: content::retarget::Target,
) -> CommandResult<content::retarget::RetargetPlan> {
    let instance = state.ctx.db.get_instance(&id).await?;
    Ok(content::retarget::plan(&state.ctx, &instance, &target).await?)
}

#[tauri::command]
pub async fn change_instance_version(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    target: content::retarget::Target,
) -> CommandResult<content::retarget::RetargetPlan> {
    if !state.games.begin(&id) {
        return Err(CommandError::msg(
            "Arrête le jeu avant de changer de version",
        ));
    }
    let result = async {
        let instance = state.ctx.db.get_instance(&id).await?;
        content::retarget::apply(&state.ctx, &instance, &target).await
    }
    .await;
    state.games.end(&id);
    let _ = app.emit(modpack::INSTANCES_CHANGED_EVENT, ());
    translation::refresh_later(&app, &id);
    Ok(result?)
}

/// Newer versions of the instance's modpack, newest first.
#[tauri::command]
pub async fn modpack_updates(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Vec<mrpack::PackVersion>> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(content::pack_update::newer_versions(&state.ctx, &instance).await?)
}

#[tauri::command]
pub async fn update_modpack(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    version_id: String,
) -> CommandResult<content::pack_update::UpdateReport> {
    modpack::update_from_modrinth(&app, &state.ctx, &state.games, &instance_id, &version_id).await
}

/// The pack version a rollback would bring back, if the last update can be undone.
#[tauri::command]
pub async fn modpack_rollback_version(
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<Option<String>> {
    Ok(content::pack_update::rollback_version(
        &state.ctx.data.instance_dir(&instance_id),
    ))
}

#[tauri::command]
pub async fn rollback_modpack(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
) -> CommandResult<()> {
    modpack::rollback(&app, &state.ctx, &state.games, &instance_id).await
}

/// Installable versions of a modpack, for the version picker.
#[tauri::command]
pub async fn modpack_versions(
    state: State<'_, AppState>,
    project_id: String,
) -> CommandResult<Vec<mrpack::PackVersion>> {
    Ok(mrpack::versions(&state.ctx, &project_id).await?)
}

/// Creates an instance from a `.mrpack` file.
#[tauri::command]
pub async fn import_modpack(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CommandResult<Instance> {
    modpack::import_file(&app, &state.ctx, &state.games, std::path::Path::new(&path)).await
}

#[tauri::command]
pub async fn export_modpack(
    state: State<'_, AppState>,
    instance_id: String,
    path: String,
    version: String,
) -> CommandResult<()> {
    let instance = state.ctx.db.get_instance(&instance_id).await?;
    Ok(mrpack::export(&state.ctx, &instance, std::path::Path::new(&path), &version).await?)
}

#[tauri::command]
pub async fn open_data_folder(app: AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    app.opener()
        .open_path(state.ctx.data.root().display().to_string(), None::<&str>)
        .map_err(|e| CommandError::msg(e.to_string()))
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

/// Installs Rosetta 2 behind macOS's administrator prompt. `false` if the user
/// cancels the prompt.
#[tauri::command]
pub async fn install_rosetta() -> CommandResult<bool> {
    if !cfg!(target_os = "macos") {
        return Err(CommandError::msg("Rosetta n'existe que sur macOS"));
    }
    let script = r#"do shell script "/usr/sbin/softwareupdate --install-rosetta --agree-to-license" with administrator privileges"#;
    let output = tokio::process::Command::new("/usr/bin/osascript")
        .args(["-e", script])
        .output()
        .await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // -128: the user dismissed the password prompt.
        if stderr.contains("(-128)") {
            return Ok(false);
        }
        tracing::error!(error = %stderr.trim(), "Rosetta install failed");
        return Err(CommandError::msg(format!(
            "L'installation de Rosetta a échoué : {}",
            stderr.trim()
        )));
    }
    if !install::rosetta_installed() {
        return Err(CommandError::msg(
            "Rosetta est toujours absent après l'installation",
        ));
    }
    tracing::info!("Rosetta installed");
    Ok(true)
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
