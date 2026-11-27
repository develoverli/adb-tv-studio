//! Default launcher: which app HOME opens, switching it, and disabling or restoring the stock one.

use super::apps_parse::{self, package_name};
use super::launcher_parse::{self, HomeStatus};
use super::{AdbError, AdbState, run};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

/// An app that can be the launcher. Serialized to `LauncherApp` in `src/lib/launcher.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherApp {
    pub package: String,
    pub system: bool,
}

/// Serialized to `LauncherStatus` in `src/lib/launcher.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherStatus {
    /// Package HOME opens now; `None` when Android would ask which one to use.
    pub current: Option<String>,
    pub apps: Vec<LauncherApp>,
    /// Stock launchers this app disabled and can restore.
    pub disabled_stock: Vec<String>,
}

async fn read_status(app: &AppHandle, serial: &str) -> Result<HomeStatus, AdbError> {
    let output = run(app, &["-s", serial, "shell", launcher_parse::STATUS_SCRIPT]).await?;
    if output.success {
        Ok(launcher_parse::home_status(&output.stdout))
    } else {
        Err(AdbError::Command(output.text().trim().to_owned()))
    }
}

fn disabled_stock(app: &AppHandle, serial: &str) -> Vec<String> {
    app.state::<AdbState>()
        .saved()
        .find(serial)
        .map_or_else(Vec::new, |tv| tv.disabled_launchers.clone())
}

/// Runs a shell command on the TV, logging a failure: callers check the result by reading the
/// launcher again, since `cmd role` and `set-home-activity` report errors inconsistently.
async fn try_shell(app: &AppHandle, serial: &str, command: &[&str]) {
    let args = [&["-s", serial, "shell"], command].concat();
    match run(app, &args).await {
        Ok(output) if output.success => {}
        Ok(output) => eprintln!("{}: {}", command.join(" "), output.text().trim()),
        Err(error) => eprintln!("{}: {error}", command.join(" ")),
    }
}

/// Makes `package` the launcher HOME opens. Also used by the Apps tab.
pub(super) async fn set_home(app: &AppHandle, serial: &str, package: &str) -> Result<(), AdbError> {
    let status = read_status(app, serial).await?;
    let Some(home) = status.apps.iter().find(|home| home.package == package) else {
        return Err(AdbError::NotALauncher(package.to_owned()));
    };
    if status.current.as_deref() == Some(package) {
        return Ok(());
    }
    // Android 10+ keeps the default launcher as the HOME role.
    try_shell(
        app,
        serial,
        &[
            "cmd",
            "role",
            "add-role-holder",
            "--user",
            "0",
            "android.app.role.HOME",
            package,
        ],
    )
    .await;
    if read_status(app, serial).await?.current.as_deref() == Some(package) {
        return Ok(());
    }
    // Android 7 to 9, or a TV whose role service refused.
    try_shell(
        app,
        serial,
        &[
            "cmd",
            "package",
            "set-home-activity",
            "--user",
            "0",
            &home.component,
        ],
    )
    .await;
    if read_status(app, serial).await?.current.as_deref() == Some(package) {
        Ok(())
    } else {
        Err(AdbError::LauncherNotApplied)
    }
}

/// Current launcher, the apps that can replace it and what can be restored.
#[tauri::command]
pub async fn launcher_status(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
) -> Result<LauncherStatus, AdbError> {
    adb.require_device(&serial)?;
    let status = read_status(&app, &serial).await?;
    Ok(LauncherStatus {
        current: status.current,
        apps: status
            .apps
            .into_iter()
            .map(|home| LauncherApp {
                package: home.package,
                system: home.system,
            })
            .collect(),
        disabled_stock: disabled_stock(&app, &serial),
    })
}

/// Makes `package` the default launcher.
#[tauri::command]
pub async fn launcher_set(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
    package: String,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    let package = package_name(&package)?;
    set_home(&app, &serial, package).await
}

/// Disables the other system launchers so the current, user-installed one is the only choice.
/// Each disabled package is recorded right away, so even a partial run can be restored.
#[tauri::command]
pub async fn launcher_disable_stock(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    if adb.saved().find(&serial).is_none() {
        return Err(AdbError::NotSaved);
    }
    let status = read_status(&app, &serial).await?;
    let chosen = status
        .current
        .as_deref()
        .filter(|current| {
            status
                .apps
                .iter()
                .any(|home| home.package == *current && !home.system)
        })
        .ok_or(AdbError::NoCustomLauncher)?;

    for package in launcher_parse::stock_launchers(&status, chosen) {
        let output = run(
            &app,
            &[
                "-s",
                &serial,
                "shell",
                "pm",
                "disable-user",
                "--user",
                "0",
                package,
            ],
        )
        .await?;
        apps_parse::command_outcome(&output.text())?;
        let mut saved = adb.saved();
        if let Some(tv) = saved.find_mut(&serial)
            && !tv.disabled_launchers.iter().any(|known| known == package)
        {
            tv.disabled_launchers.push(package.to_owned());
        }
        saved.save_or_report();
    }
    set_home(&app, &serial, chosen).await
}

/// Enables the stock launchers this app disabled and makes the first one the launcher again.
#[tauri::command]
pub async fn launcher_restore(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    let packages = disabled_stock(&app, &serial);
    for package in &packages {
        let output = run(&app, &["-s", &serial, "shell", "pm", "enable", package]).await?;
        apps_parse::command_outcome(&output.text())?;
        let mut saved = adb.saved();
        if let Some(tv) = saved.find_mut(&serial) {
            tv.disabled_launchers.retain(|known| known != package);
        }
        saved.save_or_report();
    }
    match packages.first() {
        Some(stock) => set_home(&app, &serial, stock).await,
        None => Ok(()),
    }
}
