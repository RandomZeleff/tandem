mod commands;
mod error;

use tandem_core::db::Database;
use tandem_core::logging::{self, LogBuffer, LogEntry, WorkerGuard};
use tandem_core::paths::DataDir;
use tauri::{async_runtime, Emitter, Manager};

/// Event carrying each new [`LogEntry`] to the frontend.
pub const LOG_EVENT: &str = "log://entry";
const LOG_BUFFER_CAPACITY: usize = 2000;

pub struct AppState {
    pub data_dir: DataDir,
    pub db: Database,
    pub logs: LogBuffer,
    _log_guard: WorkerGuard,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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

            let db = async_runtime::block_on(Database::open(&data_dir.database()))?;
            tracing::info!(
                version = tandem_core::version(),
                data_dir = %data_dir.root().display(),
                "Tandem started"
            );

            app.manage(AppState {
                data_dir,
                db,
                logs,
                _log_guard: log_guard,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_logs,
            commands::get_setting,
            commands::set_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
