//! Install tab: inspect local APK files (plain or split containers), compare them with the TV
//! and install them with `adb install` / `install-multiple`.

use super::install_parse::{self, InstallKind};
use super::{AdbError, AdbState, apps, run};
use apk_info::Apk;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

/// Folder under the app cache dir where split containers are unpacked before installing.
const SPLITS_FOLDER: &str = "install";

/// Gives every install its own unpack folder.
static NEXT_UNPACK: AtomicU64 = AtomicU64::new(0);

/// What the UI shows before installing a file. Serialized to `InstallPreview` in `src/lib/install.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview {
    pub path: String,
    pub file_name: String,
    pub size_bytes: u64,
    pub package: String,
    pub label: Option<String>,
    pub version_name: Option<String>,
    pub version_code: Option<String>,
    /// Several APKs inside (`.xapk`, `.apks`, `.apkm`); installed with `install-multiple`.
    pub split: bool,
    pub min_sdk: Option<u32>,
    pub device_sdk: Option<u32>,
    /// False when the app needs a newer Android than the TV has.
    pub compatible: bool,
    pub kind: InstallKind,
    pub installed_version_name: Option<String>,
    pub has_icon: bool,
}

/// Images of inspected files, keyed by path, served by `install_icon`.
#[derive(Default)]
pub struct InstallState {
    icons: Mutex<HashMap<String, Vec<u8>>>,
}

impl InstallState {
    fn icons(&self) -> MutexGuard<'_, HashMap<String, Vec<u8>>> {
        // A poisoned lock means a panic mid-update; stored images are still valid.
        self.icons.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What the file says about itself.
struct FileInfo {
    package: String,
    label: Option<String>,
    version_name: Option<String>,
    version_code: Option<String>,
    min_sdk: Option<u32>,
    split: bool,
    image: Option<Vec<u8>>,
}

fn installable_path(path: &str) -> Result<PathBuf, AdbError> {
    let path = PathBuf::from(path);
    if path.is_file() && install_parse::is_installable(&path) {
        Ok(path)
    } else {
        Err(AdbError::ApkRead(format!(
            "not an APK file: {}",
            path.display()
        )))
    }
}

fn read_file_info(path: &Path) -> Result<FileInfo, AdbError> {
    let apk = Apk::new(path).map_err(|error| AdbError::ApkRead(error.to_string()))?;
    let package = apk
        .get_package_name()
        .ok_or_else(|| AdbError::ApkRead("the file has no package name".to_owned()))?;
    let (label, image) = apps::label_and_image(&apk);
    Ok(FileInfo {
        package,
        label,
        version_name: apk.get_version_name(),
        version_code: apk.get_version_code(),
        min_sdk: apk.get_min_sdk_version().and_then(|sdk| sdk.parse().ok()),
        split: apk.get_inner_zip().is_ok(),
        image,
    })
}

/// Opens the native file picker (in Rust, so the webview never picks paths on its own).
#[tauri::command]
pub async fn install_pick(app: AppHandle) -> Result<Vec<String>, AdbError> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("Android apps", &install_parse::EXTENSIONS)
            .blocking_pick_files()
    })
    .await
    .map_err(|error| AdbError::Command(error.to_string()))?;
    Ok(picked
        .unwrap_or_default()
        .into_iter()
        .filter_map(|file| file.into_path().ok())
        .map(|path| path.to_string_lossy().into_owned())
        .collect())
}

/// Reads a file and compares it with what the TV has installed.
#[tauri::command]
pub async fn install_inspect(
    app: AppHandle,
    adb: State<'_, AdbState>,
    install: State<'_, InstallState>,
    serial: String,
    path: String,
) -> Result<InstallPreview, AdbError> {
    adb.require_device(&serial)?;
    let file = installable_path(&path)?;
    let size_bytes = std::fs::metadata(&file).map_or(0, |metadata| metadata.len());
    let file_name = file
        .file_name()
        .map_or_else(|| path.clone(), |name| name.to_string_lossy().into_owned());
    let info = tauri::async_runtime::spawn_blocking(move || read_file_info(&file))
        .await
        .map_err(|error| AdbError::ApkRead(error.to_string()))??;

    let dumpsys = run(
        &app,
        &["-s", &serial, "shell", "dumpsys", "package", &info.package],
    )
    .await?;
    let installed = install_parse::installed_version(&dumpsys.stdout, &info.package);
    let sdk = run(
        &app,
        &["-s", &serial, "shell", "getprop", "ro.build.version.sdk"],
    )
    .await?;
    let device_sdk = sdk.stdout.trim().parse::<u32>().ok();
    let compatible = match (info.min_sdk, device_sdk) {
        (Some(min), Some(device)) => min <= device,
        _ => true,
    };
    let file_code = info
        .version_code
        .as_deref()
        .and_then(|code| code.parse().ok());

    let has_icon = info.image.is_some();
    if let Some(image) = info.image {
        install.icons().insert(path.clone(), image);
    }
    Ok(InstallPreview {
        path,
        file_name,
        size_bytes,
        package: info.package,
        label: info.label,
        version_name: info.version_name,
        version_code: info.version_code,
        split: info.split,
        min_sdk: info.min_sdk,
        device_sdk,
        compatible,
        kind: install_parse::install_kind(file_code, installed.as_ref()),
        installed_version_name: installed.and_then(|version| version.name),
        has_icon,
    })
}

/// Image of an inspected file.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive owned arguments and State by value"
)]
#[tauri::command]
pub fn install_icon(install: State<'_, InstallState>, path: String) -> Result<Response, AdbError> {
    install
        .icons()
        .get(&path)
        .cloned()
        .map(Response::new)
        .ok_or_else(|| AdbError::ApkRead("no image for this file".to_owned()))
}

/// Installs a file. Always replaces (`-r`, keeps data); `allow_downgrade` adds `-d`.
#[tauri::command]
pub async fn install_run(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
    path: String,
    allow_downgrade: bool,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    let file = installable_path(&path)?;
    let mut flags = vec!["-r"];
    if allow_downgrade {
        flags.push("-d");
    }

    let unpack_dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| AdbError::ApkRead(error.to_string()))?
        .join(SPLITS_FOLDER)
        .join(format!(
            "{}-{}",
            std::process::id(),
            NEXT_UNPACK.fetch_add(1, Ordering::Relaxed)
        ));
    let dir = unpack_dir.clone();
    let source = file.clone();
    let splits = tauri::async_runtime::spawn_blocking(move || unpack_splits(&source, &dir))
        .await
        .map_err(|error| AdbError::ApkRead(error.to_string()))?;

    let result = match splits {
        Ok(None) => {
            let file_text = file.to_string_lossy().into_owned();
            let mut args = vec!["-s", serial.as_str(), "install"];
            args.extend(&flags);
            args.push(&file_text);
            install(&app, &args).await
        }
        Ok(Some(parts)) => {
            let texts: Vec<String> = parts
                .iter()
                .map(|part| part.to_string_lossy().into_owned())
                .collect();
            let mut args = vec!["-s", serial.as_str(), "install-multiple"];
            args.extend(&flags);
            args.extend(texts.iter().map(String::as_str));
            install(&app, &args).await
        }
        Err(error) => Err(error),
    };
    // Unpacked splits can be large; always remove them.
    if unpack_dir.exists()
        && let Err(error) = std::fs::remove_dir_all(&unpack_dir)
    {
        eprintln!("could not delete {}: {error}", unpack_dir.display());
    }
    result
}

async fn install(app: &AppHandle, args: &[&str]) -> Result<(), AdbError> {
    let output = run(app, args).await?;
    install_parse::install_outcome(&output.text())
}

/// For a split container, writes every inner `.apk` to `dir` and returns their paths.
/// `None` for a plain APK, which is installed as is.
fn unpack_splits(file: &Path, dir: &Path) -> Result<Option<Vec<PathBuf>>, AdbError> {
    let apk = Apk::new(file).map_err(|error| AdbError::ApkRead(error.to_string()))?;
    if apk.get_inner_zip().is_err() {
        return Ok(None);
    }
    std::fs::create_dir_all(dir).map_err(|error| AdbError::ApkRead(error.to_string()))?;
    let names: Vec<String> = apk
        .namelist()
        .filter(|name| {
            Path::new(name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("apk"))
        })
        .map(str::to_owned)
        .collect();
    let mut parts = Vec::with_capacity(names.len());
    for (index, name) in names.iter().enumerate() {
        let (bytes, _) = apk
            .read(name)
            .map_err(|error| AdbError::ApkRead(error.to_string()))?;
        // Numbered names: entries come from the archive and must not choose paths on disk.
        let part = dir.join(format!("part-{index}.apk"));
        std::fs::write(&part, bytes).map_err(|error| AdbError::ApkRead(error.to_string()))?;
        parts.push(part);
    }
    if parts.is_empty() {
        return Err(AdbError::ApkRead("no APK inside the package".to_owned()));
    }
    Ok(Some(parts))
}
