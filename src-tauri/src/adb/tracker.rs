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
                    refresh(app, &frame).await;
                }
            }
            CommandEvent::Error(message) => return Err(AdbError::Command(message)),
            CommandEvent::Terminated(_) => break,
            _ => {}
        }
    }
    Ok(())
}

/// Rebuilds the device list from one frame, reading info for newly connected TVs, and emits it.
async fn refresh(app: &AppHandle, frame: &str) {
    let state = app.state::<AdbState>();
    let entries: Vec<(String, DeviceStatus)> = parse::device_list(frame)
        .into_iter()
        .filter(|(serial, _)| parse::is_network_serial(serial))
        .collect();

    for (serial, status) in &entries {
        let known = state.info().contains_key(serial);
        if *status == DeviceStatus::Connected && !known {
            let info = read_info(app, serial).await;
            state.info().insert(serial.clone(), info);
        }
    }

    let devices: Vec<Device> = {
        let info = state.info();
        entries
            .into_iter()
            .map(|(serial, status)| {
                let found = info.get(serial.as_str());
                to_device(serial, status, found)
            })
            .collect()
    };
    state.devices().clone_from(&devices);
    if let Err(error) = app.emit(DEVICES_CHANGED, devices) {
        eprintln!("could not emit {DEVICES_CHANGED}: {error}");
    }
}

/// Reads name, model and Android version. Failures leave the fields empty instead of hiding the TV.
async fn read_info(app: &AppHandle, serial: &str) -> DeviceInfo {
    match run(app, &["-s", serial, "shell", parse::INFO_SCRIPT]).await {
        Ok(output) if output.success => parse::device_info(&output.stdout),
        Ok(output) => {
            eprintln!(
                "could not read info from {serial}: {}",
                output.text().trim()
            );
            DeviceInfo::default()
        }
        Err(error) => {
            eprintln!("could not read info from {serial}: {error}");
            DeviceInfo::default()
        }
    }
}

/// Builds the frontend device. Name falls back to "Manufacturer Model", then to the address.
fn to_device(serial: String, status: DeviceStatus, info: Option<&DeviceInfo>) -> Device {
    let info = info.cloned().unwrap_or_default();
    let name = info
        .name
        .clone()
        .or_else(|| match (&info.manufacturer, &info.model) {
            (Some(manufacturer), Some(model)) => Some(format!("{manufacturer} {model}")),
            (None, Some(model)) => Some(model.clone()),
            _ => None,
        })
        .unwrap_or_else(|| serial.clone());
    Device {
        address: serial.clone(),
        id: serial,
        name,
        model: info.model,
        android_version: info.android_version,
        status,
    }
}
