//! Themes tab: the bundled tv-launcher themes, the ones imported (`.zip`) or downloaded from a
//! catalog, and applying one to a TV with `adb push` plus an intent to the launcher.

use super::themes_parse::{self, THEME_FILE, ThemeFile};
use super::{AdbError, AdbState, run, tv_launcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

/// tv-launcher's launch component and the intent extra that switches theme.
const LAUNCHER_ACTIVITY: &str = "lol.develover.tvlauncher/.MainActivity";
const THEME_EXTRA: &str = "lol.develover.tvlauncher.extra.THEME";
/// The launcher's external files dir; adb can write there without any storage permission.
const REMOTE_THEMES: &str = "/sdcard/Android/data/lol.develover.tvlauncher/files/themes";
/// File the launcher writes with the id of the theme it shows.
const REMOTE_ACTIVE: &str = "/sdcard/Android/data/lol.develover.tvlauncher/files/themes/active";
/// Id of the launcher's built-in look.
const DEFAULT_THEME: &str = "default";
/// Folder name for themes, under the app resources (bundled) and the app data dir (library).
const THEMES_FOLDER: &str = "themes";
/// Whole theme (JSON and images); also the largest zip accepted.
const MAX_THEME_BYTES: u64 = 10 * 1024 * 1024;
const MAX_JSON_BYTES: u64 = 64 * 1024;
const MAX_ZIP_ENTRIES: usize = 32;
const ZIP_EXTENSION: &str = "zip";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeSource {
    /// Shipped with the app.
    Bundled,
    /// Imported from a zip or downloaded from a catalog.
    Library,
}

/// Serialized to `ThemeEntry` in `src/lib/themes.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeEntry {
    pub source: ThemeSource,
    pub theme: ThemeFile,
}

/// What the TV has. Serialized to `LauncherThemes` in `src/lib/themes.ts`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherThemes {
    /// tv-launcher is installed.
    pub installed: bool,
    /// The installed tv-launcher is new enough to read themes.
    pub supports_themes: bool,
    /// Theme the launcher shows (`default` for its built-in look); `None` when unknown.
    pub active: Option<String>,
}

/// One file of a theme downloaded by the frontend from a catalog.
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadedFile {
    pub name: String,
    pub data: Vec<u8>,
}

fn invalid(problem: impl Into<String>) -> AdbError {
    AdbError::InvalidTheme(problem.into())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "used as a map_err callback, which hands over the error"
)]
fn storage(error: impl ToString) -> AdbError {
    AdbError::ThemeStorage(error.to_string())
}

fn bundled_dir(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .resource_dir()
        .ok()
        .map(|dir| dir.join(THEMES_FOLDER))
}

fn library_dir(app: &AppHandle) -> Result<PathBuf, AdbError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(storage)?
        .join(THEMES_FOLDER);
    fs::create_dir_all(&dir).map_err(storage)?;
    Ok(dir)
}

/// Reads at most `limit` bytes; more means the file is not what we accept.
fn read_limited(reader: impl Read, limit: u64) -> Result<Vec<u8>, AdbError> {
    let mut data = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(storage)?;
    if data.len() as u64 > limit {
        return Err(invalid("a file is too large"));
    }
    Ok(data)
}

/// Validates a whole theme: `theme.json` plus exactly the images it refers to, within the size limit.
fn validate_files(files: &HashMap<String, Vec<u8>>) -> Result<ThemeFile, AdbError> {
    let json = files
        .get(THEME_FILE)
        .ok_or_else(|| invalid("theme.json is missing"))?;
    let text = std::str::from_utf8(json).map_err(|_| invalid("theme.json is not UTF-8"))?;
    let theme = themes_parse::parse_theme(text).map_err(invalid)?;
    let assets = theme.assets();
    for name in files.keys() {
        if name != THEME_FILE && !assets.contains(&name.as_str()) {
            return Err(invalid(format!("unexpected file {name}")));
        }
    }
    for asset in &assets {
        if !files.contains_key(*asset) {
            return Err(invalid(format!("{asset} is missing")));
        }
    }
    let total: u64 = files.values().map(|data| data.len() as u64).sum();
    if total > MAX_THEME_BYTES {
        return Err(invalid("the theme is larger than 10 MB"));
    }
    Ok(theme)
}

/// Reads a theme folder (bundled or library). The folder name must be the theme id.
fn read_folder(dir: &Path) -> Result<(ThemeFile, HashMap<String, Vec<u8>>), AdbError> {
    let json = fs::File::open(dir.join(THEME_FILE)).map_err(storage)?;
    let mut files = HashMap::new();
    files.insert(THEME_FILE.to_owned(), read_limited(json, MAX_JSON_BYTES)?);
    let text =
        std::str::from_utf8(&files[THEME_FILE]).map_err(|_| invalid("theme.json is not UTF-8"))?;
    let theme = themes_parse::parse_theme(text).map_err(invalid)?;
    for asset in theme.assets() {
        let file = fs::File::open(dir.join(asset)).map_err(storage)?;
        files.insert(asset.to_owned(), read_limited(file, MAX_THEME_BYTES)?);
    }
    let theme = validate_files(&files)?;
    let folder_matches = dir
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == theme.id);
    if !folder_matches {
        return Err(invalid("the folder name differs from the theme id"));
    }
    Ok((theme, files))
}

fn list_folder(dir: Option<&Path>, source: ThemeSource) -> Vec<ThemeEntry> {
    let Some(entries) = dir.and_then(|dir| fs::read_dir(dir).ok()) else {
        return Vec::new();
    };
    let mut themes: Vec<ThemeEntry> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| match read_folder(&entry.path()) {
            Ok((theme, _)) => Some(ThemeEntry { source, theme }),
            Err(error) => {
                eprintln!("skipping theme {}: {error}", entry.path().display());
                None
            }
        })
        .collect();
    themes.sort_by(|left, right| left.theme.name.cmp(&right.theme.name));
    themes
}

fn is_bundled(app: &AppHandle, id: &str) -> bool {
    bundled_dir(app).is_some_and(|dir| dir.join(id).join(THEME_FILE).is_file())
}

/// Folder of a theme by id: bundled first, then the library.
fn theme_folder(app: &AppHandle, id: &str) -> Result<PathBuf, AdbError> {
    if !themes_parse::is_theme_id(id) {
        return Err(invalid("invalid id"));
    }
    if let Some(dir) = bundled_dir(app).map(|dir| dir.join(id))
        && dir.is_dir()
    {
        return Ok(dir);
    }
    let dir = library_dir(app)?.join(id);
    if dir.is_dir() {
        Ok(dir)
    } else {
        Err(invalid("unknown theme"))
    }
}

/// Validates and writes a theme into the library, replacing an older copy with the same id.
fn store(app: &AppHandle, files: &HashMap<String, Vec<u8>>) -> Result<ThemeEntry, AdbError> {
    let theme = validate_files(files)?;
    if is_bundled(app, &theme.id) {
        return Err(invalid("this id belongs to a bundled theme"));
    }
    let library = library_dir(app)?;
    let staging = library.join(format!(".{}.tmp", theme.id));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(storage)?;
    }
    fs::create_dir_all(&staging).map_err(storage)?;
    // Names were validated (theme.json or a plain image name), so they cannot leave the folder.
    for (name, data) in files {
        fs::write(staging.join(name), data).map_err(storage)?;
    }
    let target = library.join(&theme.id);
    if target.exists() {
        fs::remove_dir_all(&target).map_err(storage)?;
    }
    fs::rename(&staging, &target).map_err(storage)?;
    Ok(ThemeEntry {
        source: ThemeSource::Library,
        theme,
    })
}

/// Reads a theme zip. Entries may sit at the root or inside one folder; only the file names are
/// used, and only `theme.json` and image names are accepted, so nothing can escape the folder.
fn read_zip(path: &Path) -> Result<HashMap<String, Vec<u8>>, AdbError> {
    let file = fs::File::open(path).map_err(storage)?;
    if file.metadata().map_err(storage)?.len() > MAX_THEME_BYTES {
        return Err(invalid("the zip is larger than 10 MB"));
    }
    let mut archive = zip::ZipArchive::new(file).map_err(|error| invalid(error.to_string()))?;
    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(invalid("the zip has too many files"));
    }
    let mut files = HashMap::new();
    let mut total = 0u64;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| invalid(error.to_string()))?;
        if entry.is_dir() {
            continue;
        }
        let full_name = entry
            .name()
            .map_err(|error| invalid(error.to_string()))?
            .into_owned();
        let name = full_name.rsplit('/').next().unwrap_or_default().to_owned();
        // macOS adds these to zips made in Finder.
        if full_name.starts_with("__MACOSX/") || name.starts_with('.') {
            continue;
        }
        if name != THEME_FILE && !themes_parse::is_asset_name(&name) {
            return Err(invalid(format!("unexpected file {full_name}")));
        }
        let data = read_limited(entry, MAX_THEME_BYTES - total)?;
        total += data.len() as u64;
        if files.insert(name.clone(), data).is_some() {
            return Err(invalid(format!("{name} appears twice")));
        }
    }
    Ok(files)
}

/// Every theme the app has: bundled ones first, then the library.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take owned arguments"
)]
#[tauri::command]
pub fn themes_list(app: AppHandle) -> Vec<ThemeEntry> {
    let mut themes = list_folder(bundled_dir(&app).as_deref(), ThemeSource::Bundled);
    let library = library_dir(&app).ok();
    themes.extend(
        list_folder(library.as_deref(), ThemeSource::Library)
            .into_iter()
            .filter(|entry| !is_bundled(&app, &entry.theme.id)),
    );
    themes
}

/// Opens the file dialog for theme zips.
#[tauri::command]
pub async fn themes_pick(app: AppHandle) -> Result<Vec<String>, AdbError> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("tv-launcher themes", &[ZIP_EXTENSION])
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

/// Imports a theme zip into the library.
#[tauri::command]
pub async fn themes_import(app: AppHandle, path: String) -> Result<ThemeEntry, AdbError> {
    let path = PathBuf::from(path);
    let is_zip = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(ZIP_EXTENSION));
    if !is_zip || !path.is_file() {
        return Err(invalid("choose a .zip file"));
    }
    let files = tauri::async_runtime::spawn_blocking(move || read_zip(&path))
        .await
        .map_err(|error| AdbError::Command(error.to_string()))??;
    store(&app, &files)
}

/// Saves a theme the frontend downloaded from a catalog.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take owned arguments"
)]
#[tauri::command]
pub fn themes_save(app: AppHandle, files: Vec<DownloadedFile>) -> Result<ThemeEntry, AdbError> {
    if files.len() > MAX_ZIP_ENTRIES {
        return Err(invalid("the theme has too many files"));
    }
    let mut map = HashMap::new();
    for file in &files {
        if file.name != THEME_FILE && !themes_parse::is_asset_name(&file.name) {
            return Err(invalid(format!("unexpected file {}", file.name)));
        }
        map.insert(file.name.clone(), file.data.clone());
    }
    store(&app, &map)
}

/// Removes a theme from the library (bundled themes cannot be removed).
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take owned arguments"
)]
#[tauri::command]
pub fn themes_remove(app: AppHandle, id: String) -> Result<(), AdbError> {
    if !themes_parse::is_theme_id(&id) || is_bundled(&app, &id) {
        return Err(invalid("this theme cannot be removed"));
    }
    let dir = library_dir(&app)?.join(&id);
    if dir.is_dir() {
        fs::remove_dir_all(dir).map_err(storage)?;
    }
    Ok(())
}

/// Background image of a theme, for its preview.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take owned arguments"
)]
#[tauri::command]
pub fn themes_image(app: AppHandle, id: String) -> Result<Response, AdbError> {
    let (theme, mut files) = read_folder(&theme_folder(&app, &id)?)?;
    let image = theme
        .assets()
        .first()
        .and_then(|name| files.remove(*name))
        .ok_or_else(|| invalid("the theme has no image"))?;
    Ok(Response::new(image))
}

/// Whether tv-launcher is installed and which theme it shows.
#[tauri::command]
pub async fn themes_status(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
) -> Result<LauncherThemes, AdbError> {
    adb.require_device(&serial)?;
    let version = tv_launcher::installed_version(&app, &serial).await?;
    if !tv_launcher::supports_themes(version.as_ref()) {
        return Ok(LauncherThemes {
            installed: version.is_some(),
            supports_themes: false,
            active: None,
        });
    }
    let output = run(&app, &["-s", &serial, "shell", "cat", REMOTE_ACTIVE]).await?;
    let id = output.stdout.trim();
    let active = (output.success && (id == DEFAULT_THEME || themes_parse::is_theme_id(id)))
        .then(|| id.to_owned());
    Ok(LauncherThemes {
        installed: true,
        supports_themes: true,
        active,
    })
}

/// Sends a theme to the TV and switches the launcher to it. `default` restores the built-in look.
#[tauri::command]
pub async fn themes_apply(
    app: AppHandle,
    adb: State<'_, AdbState>,
    serial: String,
    id: String,
) -> Result<(), AdbError> {
    adb.require_device(&serial)?;
    let version = tv_launcher::installed_version(&app, &serial).await?;
    if version.is_none() {
        return Err(AdbError::LauncherMissing);
    }
    if !tv_launcher::supports_themes(version.as_ref()) {
        return Err(AdbError::LauncherOutdated);
    }
    if id != DEFAULT_THEME {
        let folder = theme_folder(&app, &id)?;
        let (theme, files) = read_folder(&folder)?;
        let remote = format!("{REMOTE_THEMES}/{}", theme.id);
        let output = run(&app, &["-s", &serial, "shell", "mkdir", "-p", &remote]).await?;
        if !output.success {
            return Err(AdbError::Command(output.text().trim().to_owned()));
        }
        // Images first and theme.json last: the launcher reloads when theme.json changes.
        let mut names: Vec<&String> = files.keys().collect();
        names.sort_by_key(|name| name.as_str() == THEME_FILE);
        for name in names {
            let local = folder.join(name);
            let local = local.to_string_lossy();
            let output = run(
                &app,
                &["-s", &serial, "push", &local, &format!("{remote}/{name}")],
            )
            .await?;
            if !output.success {
                return Err(AdbError::Command(output.text().trim().to_owned()));
            }
        }
    }
    let output = run(
        &app,
        &[
            "-s",
            &serial,
            "shell",
            "am",
            "start",
            "-n",
            LAUNCHER_ACTIVITY,
            "--es",
            THEME_EXTRA,
            &id,
        ],
    )
    .await?;
    if output.text().contains("Error") {
        return Err(AdbError::Command(output.text().trim().to_owned()));
    }
    Ok(())
}
