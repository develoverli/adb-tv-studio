//! Tauri backend for adb-tv-studio.

mod adb;

/// Builds and runs the Tauri application.
///
/// # Errors
///
/// Returns an error when the Tauri runtime fails to start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            adb::init(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            adb::commands::adb_list,
            adb::commands::adb_scan,
            adb::commands::adb_connect,
            adb::commands::adb_pair,
            adb::commands::adb_disconnect,
        ])
        .build(tauri::generate_context!())?;

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            adb::shutdown(handle);
        }
    });
    Ok(())
}
