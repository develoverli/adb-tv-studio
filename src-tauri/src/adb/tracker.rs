//! Keeps the device list live with a long-running `adb track-devices` and emits `devices-changed`.

use super::parse::{self, DeviceInfo, DeviceStatus, FrameDecoder};
use super::{AdbError, AdbState, Device, run, sidecar};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;

/// Event the frontend listens to; payload is `Device[]`.
pub const DEVICES_CHANGED: &str = "devices-changed";
/// Wait before restarting `track-devices` after it exits (server restart, crash).
const RESTART_DELAY: Duration = Duration::from_secs(2);

/// Spawns the tracking loop. It restarts `track-devices` until the app shuts down.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Err(error) = track(&app).await {
                eprintln!("adb track-devices stopped: {error}");
            }
            if app.state::<AdbState>().is_shutting_down() {
                break;
            }
            // No async timer without a direct tokio dependency; a blocking sleep on the blocking pool is fine here.
            let _ =
                tauri::async_runtime::spawn_blocking(|| std::thread::sleep(RESTART_DELAY)).await;
        }
    });
}

/// Runs one `track-devices` process until it exits. It also starts the adb server if needed.
async fn track(app: &AppHandle) -> Result<(), AdbError> {
    let (mut events, _child) = sidecar(app)?
        .args(["track-devices"])
        .set_raw_out(true)
        .spawn()
        .map_err(|error| AdbError::Spawn(error.to_string()))?;
    let mut decoder = FrameDecoder::default();
    while let Some(event) = events.recv().await {
        match event {
            CommandEvent::Stdout(chunk) => {
                for frame in decoder.push(&chunk) {
                    refresh(app, &frame);
                }
            }
            CommandEvent::Error(message) => return Err(AdbError::Command(message)),
            CommandEvent::Terminated(_) => break,
            _ => {}
        }
    }
    Ok(())
}

/// Stores the entries from one frame, emits the list right away and reads info for newly
/// connected TVs in the background, so a slow or hung TV never holds up the list.
fn refresh(app: &AppHandle, frame: &str) {
    let state = app.state::<AdbState>();
    let entries: Vec<(String, DeviceStatus)> = parse::device_list(frame)
        .into_iter()
        .filter(|(serial, _)| parse::is_network_serial(serial))
        .collect();

    let to_read: Vec<String> = {
        let info = state.info();
        let mut pending = state.pending();
        entries
            .iter()
            .filter(|(serial, status)| {
                *status == DeviceStatus::Connected && !info.contains_key(serial)
            })
            .filter(|(serial, _)| pending.insert(serial.clone()))
            .map(|(serial, _)| serial.clone())
            .collect()
    };
    *state.entries() = entries;
    emit_devices(app);

    for serial in to_read {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let info = read_info(&app, &serial).await;
            let state = app.state::<AdbState>();
            state.pending().remove(&serial);
            if let Some(info) = info {
                state.info().insert(serial, info);
                emit_devices(&app);
            }
        });
    }
}

/// Rebuilds the list from the latest entries and info, stores it and emits it.
///
/// The `devices` lock is held through the emit, so concurrent callers (tracker and info reads)
/// emit in the same order they build, and an older list never arrives after a newer one.
fn emit_devices(app: &AppHandle) {
    let state = app.state::<AdbState>();
    let mut current = state.devices();
    let devices: Vec<Device> = {
        let entries = state.entries();
        let info = state.info();
        entries
            .iter()
            .map(|(serial, status)| to_device(serial, *status, info.get(serial)))
            .collect()
    };
    current.clone_from(&devices);
    if let Err(error) = app.emit(DEVICES_CHANGED, devices) {
        eprintln!("could not emit {DEVICES_CHANGED}: {error}");
    }
}

/// Reads name, model and Android version. `None` on failure, so the read is retried later;
/// the TV itself stays in the list either way.
async fn read_info(app: &AppHandle, serial: &str) -> Option<DeviceInfo> {
    match run(app, &["-s", serial, "shell", parse::INFO_SCRIPT]).await {
        Ok(output) if output.success => Some(parse::device_info(&output.stdout)),
        Ok(output) => {
            eprintln!(
                "could not read info from {serial}: {}",
                output.text().trim()
            );
            None
        }
        Err(error) => {
            eprintln!("could not read info from {serial}: {error}");
            None
        }
    }
}

/// Builds the frontend device. Name falls back to "Manufacturer Model", then to the address.
fn to_device(serial: &str, status: DeviceStatus, info: Option<&DeviceInfo>) -> Device {
    let info = info.cloned().unwrap_or_default();
    let name = info
        .name
        .clone()
        .or_else(|| match (&info.manufacturer, &info.model) {
            (Some(manufacturer), Some(model)) => Some(format!("{manufacturer} {model}")),
            (None, Some(model)) => Some(model.clone()),
            _ => None,
        })
        .unwrap_or_else(|| serial.to_owned());
    Device {
        id: serial.to_owned(),
        address: serial.to_owned(),
        name,
        model: info.model,
        android_version: info.android_version,
        status,
    }
}
