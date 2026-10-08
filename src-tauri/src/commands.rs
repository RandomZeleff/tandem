use serde::Serialize;
use serde_json::Value;
use tandem_core::logging::LogEntry;
use tauri::State;

use crate::error::CommandResult;
use crate::AppState;

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
        data_dir: state.data_dir.root().display().to_string(),
    }
}

#[tauri::command]
pub fn get_logs(state: State<'_, AppState>) -> Vec<LogEntry> {
    state.logs.snapshot()
}

#[tauri::command]
pub async fn get_setting(state: State<'_, AppState>, key: String) -> CommandResult<Option<Value>> {
    Ok(state.db.get_setting(&key).await?)
}

#[tauri::command]
pub async fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: Value,
) -> CommandResult<()> {
    state.db.set_setting(&key, &value).await?;
    Ok(())
}
