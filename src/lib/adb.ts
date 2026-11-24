import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import { t } from "../i18n";
import type { Device } from "./devices";

/** Event emitted by the Rust tracker (src-tauri/src/adb/tracker.rs) with the full device list. */
const DEVICES_CHANGED = "devices-changed";

export type ConnectStatus = "connected" | "unauthorized";

/** wireless = mDNS wireless debugging, pairing = pairing dialog open on the TV, network = port 5555 open. */
export type DiscoveryKind = "wireless" | "pairing" | "network";

export interface Discovered {
  address: string;
  name: string | null;
  kind: DiscoveryKind;
}

export interface PairResult {
  /** null: paired, but the connect port was not announced; connect with the IP and port shown on the TV. */
  connected: ConnectStatus | null;
}

export const ADB_ERROR_KINDS = [
  "spawn",
  "invalidAddress",
  "invalidCode",
  "connectFailed",
  "pairFailed",
  "unknownDevice",
  "command",
] as const;

export type AdbErrorKind = (typeof ADB_ERROR_KINDS)[number];

/** Mirrors `AdbError` in src-tauri/src/adb/error.rs. */
export interface AdbError {
  kind: AdbErrorKind;
  message?: string;
}

export function isAdbError(value: unknown): value is AdbError {
  if (typeof value !== "object" || value === null || !("kind" in value)) return false;
  const { kind } = value;
  return ADB_ERROR_KINDS.some((known) => known === kind);
}

/** Translated text for a rejected adb command. */
export function describeError(error: unknown): string {
  return isAdbError(error) ? t(`errors.${error.kind}`) : t("errors.unknown");
}

export function listDevices(): Promise<Device[]> {
  return invoke<Device[]>("adb_list");
}

export function onDevicesChanged(handler: (devices: Device[]) => void): Promise<UnlistenFn> {
  return listen<Device[]>(DEVICES_CHANGED, (event) => handler(event.payload));
}

export function scan(): Promise<Discovered[]> {
  return invoke<Discovered[]>("adb_scan");
}

export function connect(address: string): Promise<ConnectStatus> {
  return invoke<ConnectStatus>("adb_connect", { address });
}

export function pair(address: string, code: string): Promise<PairResult> {
  return invoke<PairResult>("adb_pair", { address, code });
}

export function disconnect(serial: string): Promise<void> {
  return invoke<void>("adb_disconnect", { serial });
}
