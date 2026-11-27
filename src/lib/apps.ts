import { invoke } from "@tauri-apps/api/core";

/** Mirrors `AppDetails` in src-tauri/src/adb/apps.rs. */
export interface AppDetails {
  label: string | null;
  versionName: string | null;
  hasIcon: boolean;
}

/** Mirrors `AppEntry` in src-tauri/src/adb/apps.rs. */
export interface AppEntry {
  package: string;
  versionCode: string | null;
  system: boolean;
  disabled: boolean;
  launcher: boolean;
  /** Can be the default launcher (HOME). */
  home: boolean;
  details: AppDetails | null;
}

/** Mirrors `AppAction` in src-tauri/src/adb/apps.rs. */
export type AppAction = "open" | "forceStop" | "clearData" | "uninstall" | "disable" | "enable" | "setHome";

export function listApps(serial: string): Promise<AppEntry[]> {
  return invoke<AppEntry[]>("apps_list", { serial });
}

/** Name, version and image of one app; pulls its APK the first time for each version. */
export function loadAppDetails(serial: string, app: AppEntry): Promise<AppDetails> {
  return invoke<AppDetails>("apps_details", { serial, package: app.package, versionCode: app.versionCode });
}

/** Banner or icon bytes from the cache. */
export function loadAppIcon(app: AppEntry): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("apps_icon", { package: app.package, versionCode: app.versionCode });
}

export function runAppAction(serial: string, app: AppEntry, action: AppAction): Promise<void> {
  return invoke<void>("apps_action", { serial, package: app.package, action });
}

/** "com.netflix.ninja" → "netflix ninja": readable fallback while the real name is unknown. */
export function readablePackage(packageName: string): string {
  const parts = packageName.split(".");
  const meaningful = parts.length > 2 ? parts.slice(1) : parts;
  return meaningful.filter((part) => !["android", "com", "tv", "app"].includes(part)).join(" ") || packageName;
}

export function appName(app: AppEntry): string {
  return app.details?.label ?? readablePackage(app.package);
}
