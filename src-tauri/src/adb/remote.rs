//! Remote control. Each TV gets one long-lived `adb shell` while the Control tab is open, and every
//! key, tap or text is a line written to it, which avoids starting a process per key press.

use super::parse;
use super::{AdbError, AdbState, sidecar};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};

/// Largest coordinate accepted for a tap; well above any TV resolution.
const MAX_COORDINATE: u32 = 10_000;

/// Keys the UI can send. A closed set, so the webview never chooses raw key codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RemoteKey {
    Up,
    Down,
    Left,
    Right,
    Ok,
    Back,
    Home,
    Menu,
    Recents,
    PlayPause,
    VolumeUp,
    VolumeDown,
    Mute,
    Power,
}

impl RemoteKey {
    /// Android `KeyEvent` code.
    const fn keycode(self) -> u16 {
        match self {
            Self::Home => 3,
            Self::Back => 4,
            Self::Up => 19,
            Self::Down => 20,
            Self::Left => 21,
            Self::Right => 22,
            Self::Ok => 23,
            Self::VolumeUp => 24,
            Self::VolumeDown => 25,
            Self::Power => 26,
            Self::Menu => 82,
            Self::PlayPause => 85,
            Self::Mute => 164,
            Self::Recents => 187,
        }
    }
}

/// Open shell sessions, keyed by serial.
#[derive(Default)]
pub struct RemoteState {
    sessions: Mutex<HashMap<String, CommandChild>>,
}

impl RemoteState {
    fn sessions(&self) -> MutexGuard<'_, HashMap<String, CommandChild>> {
        // A poisoned lock means a panic mid-update; the map itself is still usable.
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Starts `adb -s <serial> shell` reading commands from stdin.
fn open_session(app: &AppHandle, serial: &str) -> Result<CommandChild, AdbError> {
    let (mut events, child) = sidecar(app)?
        .args(["-s", serial, "shell"])
        .spawn()
        .map_err(|error| AdbError::Spawn(error.to_string()))?;
    let pid = child.pid();
    let app = app.clone();
    let serial = serial.to_owned();
    tauri::async_runtime::spawn(async move {
        // Drain output so the pipe never fills, and notice when the shell ends.
        while let Some(event) = events.recv().await {
            if matches!(event, CommandEvent::Terminated(_)) {
                break;
            }
        }
        // TV disconnected or shell exited: forget this session so the next command opens a new one.
        let state = app.state::<RemoteState>();
        let mut sessions = state.sessions();
        if sessions
            .get(&serial)
            .is_some_and(|child| child.pid() == pid)
        {
            sessions.remove(&serial);
        }
    });
    Ok(child)
}

/// Writes one command line to the TV's session, opening (or reopening) it when needed.
fn send(app: &AppHandle, serial: &str, command: &str) -> Result<(), AdbError> {
    let line = format!("{command}\n");
    let state = app.state::<RemoteState>();
    let mut sessions = state.sessions();
    if let Some(child) = sessions.get_mut(serial) {
        if child.write(line.as_bytes()).is_ok() {
            return Ok(());
        }
        sessions.remove(serial);
    }
    let mut child = open_session(app, serial)?;
    child
        .write(line.as_bytes())
        .map_err(|error| AdbError::Command(error.to_string()))?;
    sessions.insert(serial.to_owned(), child);
    Ok(())
}

/// Opens the session ahead of the first key, so it responds right away.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive owned arguments and State by value"
)]
#[tauri::command]
pub fn remote_open(
    app: AppHandle,
    adb: State<'_, AdbState>,
    remote: State<'_, RemoteState>,
    serial: String,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    if let std::collections::hash_map::Entry::Vacant(slot) = remote.sessions().entry(serial) {
        let child = open_session(&app, slot.key())?;
        slot.insert(child);
    }
    Ok(())
}

/// Closes the TV's session (leaving the Control tab).
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive owned arguments and State by value"
)]
#[tauri::command]
pub fn remote_close(remote: State<'_, RemoteState>, serial: String) {
    if let Some(child) = remote.sessions().remove(&serial)
        && let Err(error) = child.kill()
    {
        eprintln!("could not close the remote session for {serial}: {error}");
    }
}

/// Presses one key.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive owned arguments and State by value"
)]
#[tauri::command]
pub fn remote_key(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
    key: RemoteKey,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    send(&app, &serial, &format!("input keyevent {}", key.keycode()))
}

/// Types text into the focused field on the TV.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive owned arguments and State by value"
)]
#[tauri::command]
pub fn remote_text(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
    text: String,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    let argument = parse::input_text(&text)?;
    send(&app, &serial, &format!("input text {argument}"))
}

/// Taps the screen at TV pixel coordinates.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive owned arguments and State by value"
)]
#[tauri::command]
pub fn remote_tap(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
    x: u32,
    y: u32,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    if x > MAX_COORDINATE || y > MAX_COORDINATE {
        return Err(AdbError::Command(format!(
            "tap outside the screen: {x},{y}"
        )));
    }
    send(&app, &serial, &format!("input tap {x} {y}"))
}
