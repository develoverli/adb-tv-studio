import { invoke } from "@tauri-apps/api/core";

/** Mirrors `LauncherVersion` in src-tauri/src/adb/tv_launcher.rs. */
export interface LauncherVersion {
  code: number | null;
  name: string | null;
}

/** Mirrors `TvLauncherStatus` in src-tauri/src/adb/tv_launcher.rs. */
export interface TvLauncherStatus {
  /** APK this app can install; null when this build has none. */
  bundled: LauncherVersion | null;
  /** What the TV has; null when tv-launcher is not installed. */
  installed: LauncherVersion | null;
  supportsThemes: boolean;
  /** The bundled APK is newer than the installed one, or tv-launcher is missing. */
  canInstall: boolean;
}

export function tvLauncherStatus(serial: string): Promise<TvLauncherStatus> {
  return invoke<TvLauncherStatus>("tv_launcher_status", { serial });
}

/** Installs or updates tv-launcher and makes it the launcher. `replace` uninstalls a copy signed with another key first. */
export function installTvLauncher(serial: string, replace: boolean): Promise<void> {
  return invoke<void>("tv_launcher_install", { serial, replace });
}
