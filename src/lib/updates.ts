import { isTauri } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { type Update, check } from "@tauri-apps/plugin-updater";
import { createRoot, createSignal } from "solid-js";

/**
 * App updates through the Tauri updater: `latest.json` of the newest GitHub release, signed with
 * the key whose public half is in tauri.conf.json. Checked quietly at launch and on demand.
 */
export type UpdateStatus = "idle" | "checking" | "upToDate" | "available" | "downloading" | "error";

export interface UpdateProgress {
  downloaded: number;
  /** null when the server does not send a size. */
  total: number | null;
}

/** Delay before the launch check, so it never competes with the first render and device sync. */
const LAUNCH_CHECK_DELAY_MS = 4000;

const state = createRoot(() => {
  const [status, setStatus] = createSignal<UpdateStatus>("idle");
  const [update, setUpdate] = createSignal<Update | null>(null);
  const [progress, setProgress] = createSignal<UpdateProgress | null>(null);
  return { status, setStatus, update, setUpdate, progress, setProgress };
});

export const updateStatus = state.status;
export const updateProgress = state.progress;

/** Version and notes of the available update, if any. */
export function availableUpdate(): { version: string; notes: string } | null {
  const current = state.update();
  return current ? { version: current.version, notes: current.body ?? "" } : null;
}

/** Looks for a newer release. Quiet mode keeps a failed check out of sight (no network at launch is normal). */
export async function checkForUpdates(quiet = false): Promise<void> {
  if (!isTauri() || state.status() === "checking" || state.status() === "downloading") return;
  state.setStatus("checking");
  try {
    const found = await check();
    state.setUpdate(found);
    state.setStatus(found ? "available" : "upToDate");
  } catch (error) {
    console.warn("update check failed", error);
    state.setStatus(quiet ? "idle" : "error");
  }
}

/** Quiet check shortly after launch. Skipped in development, where the version is not a release. */
export function scheduleLaunchCheck(): void {
  if (import.meta.env.DEV) return;
  window.setTimeout(() => void checkForUpdates(true), LAUNCH_CHECK_DELAY_MS);
}

/**
 * Downloads and installs the update, then restarts. On Windows the installer closes the app
 * itself; elsewhere the relaunch below starts the new version.
 */
export async function installUpdate(): Promise<void> {
  const current = state.update();
  if (!current) return;
  state.setStatus("downloading");
  state.setProgress({ downloaded: 0, total: null });
  try {
    await current.downloadAndInstall((event) => {
      if (event.event === "Started") {
        state.setProgress({ downloaded: 0, total: event.data.contentLength ?? null });
      } else if (event.event === "Progress") {
        const previous = state.progress();
        state.setProgress({
          downloaded: (previous?.downloaded ?? 0) + event.data.chunkLength,
          total: previous?.total ?? null,
        });
      }
    });
    await relaunch();
  } catch (error) {
    console.warn("update install failed", error);
    state.setStatus("error");
    state.setProgress(null);
  }
}
