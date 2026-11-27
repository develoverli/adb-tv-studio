import { invoke } from "@tauri-apps/api/core";

/** Package of tv-launcher, offered as a one-click choice when it is installed. */
export const TV_LAUNCHER_PACKAGE = "lol.develover.tvlauncher";
export const TV_LAUNCHER_NAME = "tv-launcher";

/** Mirrors `LauncherApp` in src-tauri/src/adb/launcher.rs. */
export interface LauncherApp {
  package: string;
  system: boolean;
}

/** Mirrors `LauncherStatus` in src-tauri/src/adb/launcher.rs. */
export interface LauncherStatus {
  /** Package HOME opens now; null when Android would ask which one to use. */
  current: string | null;
  apps: LauncherApp[];
  /** Stock launchers this app disabled and can restore. */
  disabledStock: string[];
}

export function launcherStatus(serial: string): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("launcher_status", { serial });
}

export function setLauncher(serial: string, packageName: string): Promise<void> {
  return invoke<void>("launcher_set", { serial, package: packageName });
}

export function disableStockLauncher(serial: string): Promise<void> {
  return invoke<void>("launcher_disable_stock", { serial });
}

export function restoreStockLauncher(serial: string): Promise<void> {
  return invoke<void>("launcher_restore", { serial });
}
