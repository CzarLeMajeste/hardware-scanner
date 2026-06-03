#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[tauri::command]
fn run_live_hardware_scan() -> Result<String, String> {
    let report = hardware_scanner::generate_report();
    serde_json::to_string(&report).map_err(|err| format!("failed to serialize scan report: {err}"))
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![run_live_hardware_scan])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
