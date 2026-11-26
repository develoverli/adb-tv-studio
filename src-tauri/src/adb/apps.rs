//! Apps tab: list installed packages, read name and banner/icon from each launcher app's APK
//! (cached per version), and run actions (open, force stop, clear data, uninstall, disable).

use super::apps_parse::{self, PackageEntry};
use super::{AdbError, AdbState, run};
use apk_info::Apk;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};

/// Folder under the app cache dir holding `<package>-<versionCode>.json` and `.img` files.
const CACHE_FOLDER: &str = "apps";
/// Cache key part used when the TV does not report a version code.
const UNKNOWN_VERSION: &str = "0";

/// Name and image read from an APK. Serialized to `AppDetails` in `src/lib/apps.ts`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppDetails {
    pub label: Option<String>,
    pub version_name: Option<String>,
    /// Whether a banner or icon was found; fetch it with `apps_icon`.
    pub has_icon: bool,
}

/// One installed package. Serialized to `AppEntry` in `src/lib/apps.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEntry {
    pub package: String,
    pub version_code: Option<String>,
    pub system: bool,
    pub disabled: bool,
    /// Shown by the TV launcher; only these get a name and image.
    pub launcher: bool,
    /// Cached details, when this version was read before.
    pub details: Option<AppDetails>,
}

/// Actions the UI can run on a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppAction {
    Open,
    ForceStop,
    ClearData,
    Uninstall,
    Disable,
    Enable,
}

fn cache_dir(app: &AppHandle) -> Result<PathBuf, AdbError> {
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| AdbError::ApkRead(error.to_string()))?
        .join(CACHE_FOLDER);
    std::fs::create_dir_all(&dir).map_err(|error| AdbError::ApkRead(error.to_string()))?;
    Ok(dir)
}

/// Cache file path. Package and version are validated before, so they are safe file name parts.
fn cache_file(dir: &Path, package: &str, version: Option<&str>, extension: &str) -> PathBuf {
    dir.join(format!(
        "{package}-{}.{extension}",
        version.unwrap_or(UNKNOWN_VERSION)
    ))
}

fn read_cached(dir: &Path, package: &str, version: Option<&str>) -> Option<AppDetails> {
    let bytes = std::fs::read(cache_file(dir, package, version, "json")).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Lists every package, with cached details for launcher apps.
#[tauri::command]
pub async fn apps_list(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
) -> Result<Vec<AppEntry>, AdbError> {
    adb.require_device(&serial)?;
    let output = run(&app, &["-s", &serial, "shell", apps_parse::LIST_SCRIPT]).await?;
    if !output.success {
        return Err(AdbError::Command(output.text().trim().to_owned()));
    }
    // Without a cache dir the list still works; details just load again.
    let cache = cache_dir(&app).ok();
    let entries = apps_parse::package_list(&output.stdout)
        .into_iter()
        .map(|entry: PackageEntry| {
            let details = cache
                .as_deref()
                .filter(|_| entry.launcher)
                .and_then(|dir| read_cached(dir, &entry.package, entry.version_code.as_deref()));
            AppEntry {
                package: entry.package,
                version_code: entry.version_code,
                system: entry.system,
                disabled: entry.disabled,
                launcher: entry.launcher,
                details,
            }
        })
        .collect();
    Ok(entries)
}

/// Reads name, version name and banner/icon of one app: from cache, or by pulling its APK once.
#[tauri::command]
pub async fn apps_details(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
    package: String,
    version_code: Option<String>,
) -> Result<AppDetails, AdbError> {
    adb.require_device(&serial)?;
    let package = apps_parse::package_name(&package)?.to_owned();
    let version = version_code
        .as_deref()
        .and_then(apps_parse::version_code)
        .map(str::to_owned);
    let dir = cache_dir(&app)?;
    if let Some(details) = read_cached(&dir, &package, version.as_deref()) {
        return Ok(details);
    }

    let paths = run(&app, &["-s", &serial, "shell", "pm", "path", &package]).await?;
    let remote = apps_parse::base_apk_path(&paths.stdout)
        .ok_or_else(|| AdbError::ApkRead(paths.text().trim().to_owned()))?;
    let local = dir.join(format!("{package}.download.apk"));
    let local_text = local.to_string_lossy().into_owned();
    let pulled = run(&app, &["-s", &serial, "pull", &remote, &local_text]).await;
    let pulled = match pulled {
        Ok(output) if output.success => Ok(()),
        Ok(output) => Err(AdbError::ApkRead(output.text().trim().to_owned())),
        Err(error) => Err(error),
    };

    let parse_path = local.clone();
    let parsed = match pulled {
        Ok(()) => tauri::async_runtime::spawn_blocking(move || read_apk(&parse_path))
            .await
            .map_err(|error| AdbError::ApkRead(error.to_string()))
            .and_then(|result| result),
        Err(error) => Err(error),
    };
    // The APK can be large; never keep it, whatever happened.
    if local.exists()
        && let Err(error) = std::fs::remove_file(&local)
    {
        eprintln!("could not delete {}: {error}", local.display());
    }
    let (details, image) = parsed?;

    if let Some(image) = &image {
        std::fs::write(cache_file(&dir, &package, version.as_deref(), "img"), image)
            .map_err(|error| AdbError::ApkRead(error.to_string()))?;
    }
    let json =
        serde_json::to_vec(&details).map_err(|error| AdbError::ApkRead(error.to_string()))?;
    std::fs::write(cache_file(&dir, &package, version.as_deref(), "json"), json)
        .map_err(|error| AdbError::ApkRead(error.to_string()))?;
    Ok(details)
}

/// Parses an APK: label, version name, and the banner (TV) or icon as image bytes.
fn read_apk(path: &Path) -> Result<(AppDetails, Option<Vec<u8>>), AdbError> {
    let apk = Apk::new(path).map_err(|error| AdbError::ApkRead(error.to_string()))?;
    let (label, image) = label_and_image(&apk);
    let details = AppDetails {
        label,
        version_name: apk.get_version_name(),
        has_icon: image.is_some(),
    };
    Ok((details, image))
}

/// Display name and the banner (TV) or icon bytes of a parsed APK.
///
/// Images are read from the base APK, so split containers (`.xapk`, `.apks`, `.apkm`) work too.
pub(super) fn label_and_image(apk: &Apk) -> (Option<String>, Option<Vec<u8>>) {
    // An unresolved label comes back as a raw "@string/..." reference; that is not a name.
    let label = apk
        .get_application_label()
        .filter(|label| !label.is_empty() && !label.starts_with('@'));
    let image = apk.get_base_zip().ok().and_then(|zip| {
        let names: Vec<&str> = zip.namelist().collect();
        [
            apk.get_attribute_value("application", "banner"),
            apk.get_application_icon(),
        ]
        .into_iter()
        .flatten()
        .find_map(|resolved| apps_parse::best_raster(&resolved, &names).map(str::to_owned))
        .and_then(|name| zip.read(&name).ok())
        .map(|(bytes, _)| bytes)
    });
    (label, image)
}

/// Cached banner/icon bytes (PNG, WebP or JPEG).
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands receive owned arguments"
)]
#[tauri::command]
pub fn apps_icon(
    app: AppHandle,
    package: String,
    version_code: Option<String>,
) -> Result<Response, AdbError> {
    let package = apps_parse::package_name(&package)?;
    let version = version_code.as_deref().and_then(apps_parse::version_code);
    let file = cache_file(&cache_dir(&app)?, package, version, "img");
    std::fs::read(file)
        .map(Response::new)
        .map_err(|error| AdbError::ApkRead(error.to_string()))
}

/// Runs one action on a package.
#[tauri::command]
pub async fn apps_action(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
    package: String,
    action: AppAction,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    let package = apps_parse::package_name(&package)?;
    let shell = |command: &'static [&'static str]| {
        let mut args = vec!["-s", serial.as_str(), "shell"];
        args.extend_from_slice(command);
        args.push(package);
        args
    };
    match action {
        AppAction::Open => {
            let tv = shell(&[
                "monkey",
                "-c",
                "android.intent.category.LEANBACK_LAUNCHER",
                "-p",
            ]);
            let tv_output = run(&app, &[tv.as_slice(), &["1"]].concat()).await?;
            if apps_parse::command_outcome(&tv_output.text()).is_ok() {
                return Ok(());
            }
            // Apps without a TV launcher entry still open with the phone launcher category.
            let phone = shell(&["monkey", "-c", "android.intent.category.LAUNCHER", "-p"]);
            let output = run(&app, &[phone.as_slice(), &["1"]].concat()).await?;
            apps_parse::command_outcome(&output.text())
        }
        AppAction::ForceStop => run_action(&app, &shell(&["am", "force-stop"])).await,
        AppAction::ClearData => run_action(&app, &shell(&["pm", "clear"])).await,
        AppAction::Disable => {
            run_action(&app, &shell(&["pm", "disable-user", "--user", "0"])).await
        }
        AppAction::Enable => run_action(&app, &shell(&["pm", "enable"])).await,
        AppAction::Uninstall => {
            let system = run(
                &app,
                &["-s", &serial, "shell", "pm", "list", "packages", "-s"],
            )
            .await?;
            let is_system = system
                .stdout
                .lines()
                .any(|line| line.trim().strip_prefix("package:") == Some(package));
            // System apps cannot be removed from the device; `--user 0` removes them for this
            // user only, and `cmd package install-existing` brings them back.
            if is_system {
                run_action(&app, &shell(&["pm", "uninstall", "--user", "0"])).await
            } else {
                run_action(&app, &shell(&["pm", "uninstall"])).await
            }
        }
    }
}

async fn run_action(app: &AppHandle, args: &[&str]) -> Result<(), AdbError> {
    let output = run(app, args).await?;
    apps_parse::command_outcome(&output.text())
}
