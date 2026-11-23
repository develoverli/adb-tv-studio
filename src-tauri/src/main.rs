// Prevents an additional console window on Windows in release. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = adb_tv_studio_lib::run() {
        eprintln!("adb-tv-studio failed to start: {error}");
        std::process::exit(1);
    }
}
