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
        .plugin(tauri_plugin_dialog::init())
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
            adb::commands::adb_forget,
            adb::commands::adb_reconnect,
            adb::remote::remote_open,
            adb::remote::remote_close,
            adb::remote::remote_key,
            adb::remote::remote_text,
            adb::remote::remote_tap,
            adb::screen::screen_capture,
            adb::screen::screen_save,
            adb::apps::apps_list,
            adb::apps::apps_details,
            adb::apps::apps_icon,
            adb::apps::apps_action,
            adb::launcher::launcher_status,
            adb::launcher::launcher_set,
            adb::launcher::launcher_disable_stock,
            adb::launcher::launcher_restore,
            adb::themes::themes_list,
            adb::themes::themes_pick,
            adb::themes::themes_import,
            adb::themes::themes_save,
            adb::themes::themes_remove,
            adb::themes::themes_image,
            adb::themes::themes_status,
            adb::themes::themes_apply,
            adb::tv_launcher::tv_launcher_status,
            adb::tv_launcher::tv_launcher_install,
            adb::install::install_pick,
            adb::install::install_inspect,
            adb::install::install_icon,
            adb::install::install_run,
            adb::scrcpy::scrcpy_start,
            adb::scrcpy::scrcpy_stop,
            adb::scrcpy::scrcpy_key,
            adb::scrcpy::scrcpy_text,
            adb::scrcpy::scrcpy_tap,
        ])
        .build(tauri::generate_context!())?;

    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            adb::shutdown(handle);
        }
    });
    Ok(())
}
