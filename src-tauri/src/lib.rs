#[tauri::command]
fn core_version() -> &'static str {
    tandem_core::version()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![core_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
