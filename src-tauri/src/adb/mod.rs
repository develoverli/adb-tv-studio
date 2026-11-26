//! ADB integration. Every call goes through the bundled `adb` sidecar, started by Rust only;
//! the webview gets the `adb_*`, `remote_*` and `screen_*` commands and never a shell permission.

pub mod commands;
mod error;
mod parse;
pub mod remote;
mod scan;
pub mod screen;
mod tracker;

pub use error::AdbError;

use parse::{DeviceInfo, DeviceStatus};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::{Command, CommandEvent};

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
    /// Latest device list sent to the frontend, already enriched with info.
    devices: Mutex<Vec<Device>>,
    /// Latest `serial, status` pairs from `track-devices`, so info arriving later can rebuild the list.
    entries: Mutex<Vec<(String, DeviceStatus)>>,
    /// Info read from each TV once connected, keyed by serial. Kept while it goes offline.
    /// Only successful reads are stored, so a failed read is retried on the next state change.
    info: Mutex<HashMap<String, DeviceInfo>>,
    /// Serials with an info read in flight, so a TV is never read twice at once.
    pending: Mutex<HashSet<String>>,
    /// Set on app exit so the tracker does not restart the server we are killing.
    shutting_down: AtomicBool,
}

impl AdbState {
    fn devices(&self) -> MutexGuard<'_, Vec<Device>> {
        // A poisoned lock means a panic mid-update; the list is still a valid list.
        self.devices.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn entries(&self) -> MutexGuard<'_, Vec<(String, DeviceStatus)>> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn info(&self) -> MutexGuard<'_, HashMap<String, DeviceInfo>> {
        self.info.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn pending(&self) -> MutexGuard<'_, HashSet<String>> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::Relaxed)
    }

    /// Fails with `UnknownDevice` unless `serial` is in the current list, so commands only
    /// ever target TVs adb already reports.
    fn require_device(&self, serial: &str) -> Result<(), AdbError> {
        if self.devices().iter().any(|device| device.id == serial) {
            Ok(())
        } else {
            Err(AdbError::UnknownDevice(serial.to_owned()))
        }
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
                .env("ANDROID_ADB_SERVER_PORT", SERVER_PORT)
                .env("ADB_MDNS_AUTO_CONNECT", MDNS_AUTO_CONNECT)
        })
        .map_err(|error| AdbError::Spawn(error.to_string()))
}

/// Output of a finished adb command with stdout kept as bytes (binary output such as PNG).
struct RawOutput {
    success: bool,
    stdout: Vec<u8>,
    stderr: String,
}

/// Runs adb to completion, keeping stdout as raw bytes.
///
/// Not `Command::output()`: it reads line by line, drops each CRLF/LF and appends a bare LF,
/// which corrupts binary output (the PNG signature itself contains CRLF). Raw mode passes the
/// bytes through untouched.
async fn run_raw(app: &AppHandle, args: &[&str]) -> Result<RawOutput, AdbError> {
    let (mut events, _child) = sidecar(app)?
        .args(args)
        .set_raw_out(true)
        .spawn()
        .map_err(|error| AdbError::Spawn(error.to_string()))?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut code = None;
    while let Some(event) = events.recv().await {
        match event {
            CommandEvent::Stdout(bytes) => stdout.extend(bytes),
            CommandEvent::Stderr(bytes) => stderr.extend(bytes),
            CommandEvent::Terminated(payload) => code = payload.code,
            CommandEvent::Error(message) => return Err(AdbError::Command(message)),
            _ => {}
        }
    }
    Ok(RawOutput {
        success: code == Some(0),
        stdout,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

/// Runs adb to completion, decoding its output as text.
async fn run(app: &AppHandle, args: &[&str]) -> Result<CommandOutput, AdbError> {
    let output = run_raw(app, args).await?;
    Ok(CommandOutput {
        success: output.success,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: output.stderr,
    })
}

/// Registers the state and starts watching devices. Call from `setup`.
pub fn init(app: &AppHandle) {
    app.manage(AdbState::default());
    app.manage(remote::RemoteState::default());
    app.manage(screen::ScreenState::default());
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
