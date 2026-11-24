//! ADB integration. Every call goes through the bundled `adb` sidecar, started by Rust only;
//! the webview gets the `adb_*` commands below and never a shell permission.

pub mod commands;
mod error;
mod parse;
mod scan;
mod tracker;

pub use error::AdbError;

use parse::{DeviceInfo, DeviceStatus};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::Command;

/// Name of the sidecar in `bundle.externalBin` (`binaries/adb-<target-triple>`).
const SIDECAR: &str = "adb";
/// Our own adb server port, so we never fight another adb (Android Studio, platform-tools) on 5037.
const SERVER_PORT: &str = "5038";
/// "0" turns off adb's automatic connection to paired TVs found by mDNS. Connections are made
/// explicitly by the app, so serials stay `ip:port` and a TV never shows up twice.
const MDNS_AUTO_CONNECT: &str = "0";

/// A TV as the frontend sees it. Serialized to the `Device` type in `src/lib/devices.ts`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// adb serial (`ip:port`); also the route id.
    pub id: String,
    pub name: String,
    pub model: Option<String>,
    pub android_version: Option<String>,
    pub address: String,
    pub status: DeviceStatus,
}

/// Shared backend state, managed by Tauri.
#[derive(Default)]
pub struct AdbState {
    /// Latest device list from `track-devices`, already enriched with info.
    devices: Mutex<Vec<Device>>,
    /// Info read from each TV once connected, keyed by serial. Kept while it goes offline.
    info: Mutex<HashMap<String, DeviceInfo>>,
    /// Set on app exit so the tracker does not restart the server we are killing.
    shutting_down: AtomicBool,
}

impl AdbState {
    fn devices(&self) -> MutexGuard<'_, Vec<Device>> {
        // A poisoned lock means a panic mid-update; the list is still a valid list.
        self.devices.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn info(&self) -> MutexGuard<'_, HashMap<String, DeviceInfo>> {
        self.info.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::Relaxed)
    }
}

/// Output of a finished adb command, decoded as text.
struct CommandOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

impl CommandOutput {
    /// stdout and stderr together: adb prints results on either, depending on the command and version.
    fn text(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

/// The adb sidecar with our server environment applied.
fn sidecar(app: &AppHandle) -> Result<Command, AdbError> {
    app.shell()
        .sidecar(SIDECAR)
        .map(|command| {
            command
                .env("ADB_SERVER_PORT", SERVER_PORT)
                .env("ADB_MDNS_AUTO_CONNECT", MDNS_AUTO_CONNECT)
        })
        .map_err(|error| AdbError::Spawn(error.to_string()))
}

/// Runs adb to completion.
async fn run(app: &AppHandle, args: &[&str]) -> Result<CommandOutput, AdbError> {
    let output = sidecar(app)?
        .args(args)
        .output()
        .await
        .map_err(|error| AdbError::Spawn(error.to_string()))?;
    Ok(CommandOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// Registers the state and starts watching devices. Call from `setup`.
pub fn init(app: &AppHandle) {
    app.manage(AdbState::default());
    tracker::start(app.clone());
}

/// Stops our adb server on app exit. Blocking, so the process is gone before the app is.
pub fn shutdown(app: &AppHandle) {
    let Some(state) = app.try_state::<AdbState>() else {
        return;
    };
    state.shutting_down.store(true, Ordering::Relaxed);
    if let Err(error) = tauri::async_runtime::block_on(run(app, &["kill-server"])) {
        eprintln!("adb kill-server failed: {error}");
    }
}
