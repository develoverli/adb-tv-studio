import { invoke } from "@tauri-apps/api/core";
import { t } from "../i18n";
import { describeError, isAdbError } from "./adb";

/** Mirrors `InstallKind` in src-tauri/src/adb/install_parse.rs. */
export type InstallKind = "new" | "update" | "same" | "downgrade" | "replace";

/** Mirrors `InstallPreview` in src-tauri/src/adb/install.rs. */
export interface InstallPreview {
  path: string;
  fileName: string;
  sizeBytes: number;
  package: string;
  label: string | null;
  versionName: string | null;
  versionCode: string | null;
  split: boolean;
  minSdk: number | null;
  deviceSdk: number | null;
  compatible: boolean;
  kind: InstallKind;
  installedVersionName: string | null;
  hasIcon: boolean;
}

/** Same list as `EXTENSIONS` in src-tauri/src/adb/install_parse.rs. */
export const INSTALL_EXTENSIONS = ["apk", "xapk", "apks", "apkm"] as const;

export function isInstallable(path: string): boolean {
  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  return INSTALL_EXTENSIONS.some((known) => known === extension);
}

export function pickFiles(): Promise<string[]> {
  return invoke<string[]>("install_pick");
}

export function inspectFile(serial: string, path: string): Promise<InstallPreview> {
  return invoke<InstallPreview>("install_inspect", { serial, path });
}

export function fileIcon(path: string): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("install_icon", { path });
}

export function installFile(serial: string, path: string, allowDowngrade: boolean): Promise<void> {
  return invoke<void>("install_run", { serial, path, allowDowngrade });
}

export function openPackage(serial: string, packageName: string): Promise<void> {
  return invoke<void>("apps_action", { serial, package: packageName, action: "open" });
}

/** Android install failure codes with a dedicated explanation. */
const KNOWN_FAILURES = {
  INSTALL_FAILED_VERSION_DOWNGRADE: () => t("install.failures.downgrade"),
  INSTALL_FAILED_UPDATE_INCOMPATIBLE: () => t("install.failures.signature"),
  INSTALL_FAILED_INSUFFICIENT_STORAGE: () => t("install.failures.storage"),
  INSTALL_FAILED_NO_MATCHING_ABIS: () => t("install.failures.abi"),
  INSTALL_FAILED_OLDER_SDK: () => t("install.failures.sdk"),
  INSTALL_FAILED_USER_RESTRICTED: () => t("install.failures.restricted"),
  INSTALL_PARSE_FAILED_NO_CERTIFICATES: () => t("install.failures.unsigned"),
} as const;

function isKnownFailure(code: string): code is keyof typeof KNOWN_FAILURES {
  return code in KNOWN_FAILURES;
}

/** Translated reason for a failed install, with Android's code when it is not a known one. */
export function describeInstallError(error: unknown): string {
  if (isAdbError(error) && error.kind === "installFailed") {
    const code = error.message ?? "";
    return isKnownFailure(code) ? KNOWN_FAILURES[code]() : t("install.failures.other", { code });
  }
  return describeError(error);
}

export function formatSize(bytes: number): string {
  const megabytes = bytes / (1024 * 1024);
  return megabytes >= 1 ? `${megabytes.toFixed(1)} MB` : `${Math.max(1, Math.round(bytes / 1024))} KB`;
}
