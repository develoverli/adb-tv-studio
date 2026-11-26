//! Screen view: PNG screenshots with `screencap`, and saving the latest one to disk.

use super::{AdbError, AdbState, run_raw};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::ipc::Response;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

/// Every PNG file starts with these 8 bytes.
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
/// How far into the output to look for the PNG. Some vendor `screencap` builds print debug text
/// first (a KTC TV prints 65 bytes); a few KB covers that without scanning a whole image.
const MAX_PNG_PREFIX: usize = 4096;

/// Start of the PNG in `screencap -p` output, skipping any text the TV printed before it.
fn png_start(output: &[u8]) -> Option<usize> {
    let window = output
        .get(..MAX_PNG_PREFIX + PNG_SIGNATURE.len())
        .unwrap_or(output);
    window
        .windows(PNG_SIGNATURE.len())
        .position(|candidate| candidate == PNG_SIGNATURE)
}

/// Latest capture per serial, kept so "Save" writes exactly what is on screen without
/// sending the image back from the webview.
#[derive(Default)]
pub struct ScreenState {
    frames: Mutex<HashMap<String, Vec<u8>>>,
}

impl ScreenState {
    fn frames(&self) -> MutexGuard<'_, HashMap<String, Vec<u8>>> {
        // A poisoned lock means a panic mid-update; the stored frames are still valid PNGs.
        self.frames.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Captures the TV screen. Returns raw PNG bytes (an `ArrayBuffer` in the webview, no base64).
#[tauri::command]
pub async fn screen_capture(
    app: AppHandle,
    adb: State<'_, AdbState>,
    screen: State<'_, ScreenState>,
    serial: String,
) -> Result<Response, AdbError> {
    adb.require_device(&serial)?;
    let output = run_raw(&app, &["-s", &serial, "exec-out", "screencap", "-p"]).await?;
    let start = png_start(&output.stdout).filter(|_| output.success);
    let Some(start) = start else {
        let reason = output.stderr.trim();
        return Err(AdbError::Command(if reason.is_empty() {
            "screencap did not return a PNG".to_owned()
        } else {
            reason.to_owned()
        }));
    };
    let mut png = output.stdout;
    png.drain(..start);
    screen.frames().insert(serial, png.clone());
    Ok(Response::new(png))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_start_should_find_clean_png() {
        assert_eq!(png_start(b"\x89PNG\r\n\x1a\nrest"), Some(0));
    }

    #[test]
    fn png_start_should_skip_vendor_debug_text() {
        // Real prefix printed by a KTC Android TV before the image.
        let mut output =
            b"Init wrapper sys mutex successful. Pid:27007\nopen mma dev failed\n".to_vec();
        output.extend_from_slice(b"\x89PNG\r\n\x1a\nrest");
        assert_eq!(png_start(&output), Some(65));
    }

    #[test]
    fn png_start_should_reject_text_only() {
        assert_eq!(png_start(b"error: closed"), None);
    }
}

/// Saves the latest capture with the native "Save as" dialog. Returns `false` when cancelled.
///
/// The dialog runs in Rust, so the webview never picks a path to write to.
#[tauri::command]
pub async fn screen_save(
    app: AppHandle,
    screen: State<'_, ScreenState>,
    serial: String,
) -> Result<bool, AdbError> {
    let frame = screen
        .frames()
        .get(&serial)
        .cloned()
        .ok_or(AdbError::NoCapture)?;
    let file_name = format!("adb-tv-studio-{}.png", serial.replace([':', '.'], "-"));
    let dialog_app = app.clone();
    let chosen = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .add_filter("PNG", &["png"])
            .set_file_name(file_name)
            .blocking_save_file()
    })
    .await
    .map_err(|error| AdbError::Save(error.to_string()))?;
    let Some(chosen) = chosen else {
        return Ok(false);
    };
    let path = chosen
        .into_path()
        .map_err(|error| AdbError::Save(error.to_string()))?;
    std::fs::write(&path, frame).map_err(|error| AdbError::Save(error.to_string()))?;
    Ok(true)
}
