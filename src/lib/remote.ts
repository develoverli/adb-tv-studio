import { invoke } from "@tauri-apps/api/core";

/** Mirrors `RemoteKey` in src-tauri/src/adb/remote.rs. */
export type RemoteKey =
  | "up"
  | "down"
  | "left"
  | "right"
  | "ok"
  | "back"
  | "home"
  | "menu"
  | "recents"
  | "playPause"
  | "volumeUp"
  | "volumeDown"
  | "mute"
  | "power";

/** Same limit as `MAX_TEXT_LEN` in src-tauri/src/adb/parse.rs. */
export const MAX_TEXT_LENGTH = 200;

/** `input text` can only type printable ASCII and spaces. */
export function isTypeable(text: string): boolean {
  return /^[\x20-\x7E]+$/.test(text) && text.length <= MAX_TEXT_LENGTH;
}

export function openRemote(serial: string): Promise<void> {
  return invoke<void>("remote_open", { serial });
}

export function closeRemote(serial: string): Promise<void> {
  return invoke<void>("remote_close", { serial });
}

export function pressKey(serial: string, key: RemoteKey): Promise<void> {
  return invoke<void>("remote_key", { serial, key });
}

export function typeText(serial: string, text: string): Promise<void> {
  return invoke<void>("remote_text", { serial, text });
}

export function tap(serial: string, x: number, y: number): Promise<void> {
  return invoke<void>("remote_tap", { serial, x, y });
}

/** PNG bytes of the TV screen. */
export function captureScreen(serial: string): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("screen_capture", { serial });
}

/** Opens the native save dialog for the latest capture. false when cancelled. */
export function saveCapture(serial: string): Promise<boolean> {
  return invoke<boolean>("screen_save", { serial });
}
