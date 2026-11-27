//! tv-launcher on the TV: which version is installed, whether it can show themes, and installing
//! or updating it from the APK bundled with the app (`launcher/tv-launcher.apk`).

use super::install_parse::{self, InstalledVersion};
use super::{AdbError, AdbState, launcher, run};
use apk_info::Apk;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

pub const PACKAGE: &str = "lol.develover.tvlauncher";
/// First tv-launcher version that reads themes.
pub const MIN_THEMES_VERSION_CODE: u64 = 2;
const BUNDLED_APK: &str = "launcher/tv-launcher.apk";
/// `adb install` codes meaning the installed copy is signed with another key.
const SIGNATURE_MISMATCH: [&str; 2] = [
    "INSTALL_FAILED_UPDATE_INCOMPATIBLE",
    "INSTALL_PARSE_FAILED_INCONSISTENT_CERTIFICATES",
];

/// A tv-launcher version. Serialized to `LauncherVersion` in `src/lib/tvLauncher.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherVersion {
    pub code: Option<u64>,
    pub name: Option<String>,
}

/// Serialized to `TvLauncherStatus` in `src/lib/tvLauncher.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TvLauncherStatus {
    /// The APK this app can install; `None` when this build does not include one.
    pub bundled: Option<LauncherVersion>,
    /// What the TV has; `None` when tv-launcher is not installed.
    pub installed: Option<LauncherVersion>,
    /// The installed copy reads themes.
    pub supports_themes: bool,
    /// The bundled APK is newer than the installed one (or it is not installed).
    pub can_install: bool,
}

fn bundled_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join(BUNDLED_APK))
        .filter(|path| path.is_file())
}

fn bundled_version(app: &AppHandle) -> Option<LauncherVersion> {
    let apk = Apk::new(bundled_path(app)?).ok()?;
    if apk.get_package_name().as_deref() != Some(PACKAGE) {
        return None;
    }
    Some(LauncherVersion {
        code: apk.get_version_code().and_then(|code| code.parse().ok()),
        name: apk.get_version_name(),
    })
}

/// Version installed on the TV, `None` when tv-launcher is not installed.
pub(super) async fn installed_version(
    app: &AppHandle,
    serial: &str,
) -> Result<Option<InstalledVersion>, AdbError> {
    let output = run(app, &["-s", serial, "shell", "dumpsys", "package", PACKAGE]).await?;
    Ok(install_parse::installed_version(&output.stdout, PACKAGE))
}

pub(super) fn supports_themes(version: Option<&InstalledVersion>) -> bool {
    version
        .and_then(|version| version.code)
        .is_some_and(|code| code >= MIN_THEMES_VERSION_CODE)
}

async fn status(app: &AppHandle, serial: &str) -> Result<TvLauncherStatus, AdbError> {
    let installed = installed_version(app, serial).await?;
    let bundled = bundled_version(app);
    let can_install = match (&bundled, &installed) {
        (Some(_), None) => true,
        (Some(bundled), Some(installed)) => match (bundled.code, installed.code) {
            (Some(bundled), Some(installed)) => bundled > installed,
            _ => false,
        },
        (None, _) => false,
    };
    Ok(TvLauncherStatus {
        supports_themes: supports_themes(installed.as_ref()),
        installed: installed.map(|version| LauncherVersion {
            code: version.code,
            name: version.name,
        }),
        bundled,
        can_install,
    })
}

/// What tv-launcher the TV has and what this app can install.
#[tauri::command]
pub async fn tv_launcher_status(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
) -> Result<TvLauncherStatus, AdbError> {
    adb.require_device(&serial)?;
    status(&app, &serial).await
}

/// Installs or updates tv-launcher from the bundled APK and makes it the TV's launcher.
///
/// `replace` uninstalls a copy signed with another key first (its dock and hidden apps are lost);
/// without it, that case fails with `LauncherSignatureMismatch` so the UI can ask.
#[tauri::command]
pub async fn tv_launcher_install(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
    replace: bool,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    let apk = bundled_path(&app).ok_or(AdbError::LauncherNotBundled)?;
    if replace {
        let output = run(&app, &["-s", &serial, "uninstall", PACKAGE]).await?;
        // "Unknown package" only means there was nothing to remove.
        if !output.success && !output.text().contains("Unknown package") {
            eprintln!("uninstall before reinstall: {}", output.text().trim());
        }
    }
    let apk = apk.to_string_lossy().into_owned();
    let output = run(&app, &["-s", &serial, "install", "-r", &apk]).await?;
    match install_parse::install_outcome(&output.text()) {
        Err(AdbError::InstallFailed(code)) if SIGNATURE_MISMATCH.contains(&code.as_str()) => {
            return Err(AdbError::LauncherSignatureMismatch);
        }
        result => result?,
    }
    // Installed: make it the launcher. If the stock one still wins, the Launcher section explains.
    match launcher::set_home(&app, &serial, PACKAGE).await {
        Ok(()) | Err(AdbError::LauncherNotApplied) => Ok(()),
        Err(error) => Err(error),
    }
}
