mod commands;
mod error;
mod game;
mod modpack;

use tandem_core::logging::{self, LogBuffer, LogEntry, WorkerGuard};
use tandem_core::paths::DataDir;
use tandem_core::Context;
use tauri::{async_runtime, Emitter, Manager};

/// Event carrying each new [`LogEntry`] to the frontend.
pub const LOG_EVENT: &str = "log://entry";
const LOG_BUFFER_CAPACITY: usize = 2000;

pub struct AppState {
    pub ctx: Context,
    pub logs: LogBuffer,
    pub games: game::Games,
    _log_guard: WorkerGuard,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = DataDir::from_env()?;
            async_runtime::block_on(data_dir.ensure())?;

            let logs = LogBuffer::new(LOG_BUFFER_CAPACITY);
            let handle = app.handle().clone();
            let log_guard = logging::init(
                &data_dir.logs(),
                logs.clone(),
                Some(Box::new(move |entry: &LogEntry| {
                    let _ = handle.emit(LOG_EVENT, entry);
                })),
            )?;

            let ctx = async_runtime::block_on(Context::init(data_dir))?;
            tracing::info!(
                version = tandem_core::version(),
                data_dir = %ctx.data.root().display(),
                "Tandem started"
            );

            app.manage(AppState {
                ctx,
                logs,
                games: game::Games::default(),
                _log_guard: log_guard,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_logs,
            commands::get_setting,
            commands::set_setting,
            commands::list_versions,
            commands::list_loader_versions,
            commands::list_instances,
            commands::create_instance,
            commands::delete_instance,
            commands::memory_info,
            commands::set_instance_memory,
            commands::search_content,
            commands::list_content,
            commands::install_content,
            commands::remove_content,
            commands::check_content_updates,
            commands::perf_suggestions,
            commands::update_content,
            commands::set_content_enabled,
            commands::install_modpack,
            commands::import_modpack,
            commands::export_modpack,
            commands::open_instance_folder,
            commands::launch_instance,
            commands::stop_instance,
            commands::install_rosetta,
            commands::running_instances,
            commands::list_accounts,
            commands::add_offline_account,
            commands::set_active_account,
            commands::remove_account,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
