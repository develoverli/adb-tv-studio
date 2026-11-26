import { createRoot, createSignal } from "solid-js";
import { createStore } from "solid-js/store";
import { confirmAction } from "../components/ConfirmDialog";
import { t } from "../i18n";
import { describeError } from "./adb";
import {
  type InstallPreview,
  describeInstallError,
  fileIcon,
  inspectFile,
  installFile,
  isInstallable,
} from "./install";

export type InstallStatus = "inspecting" | "ready" | "installing" | "installed" | "error";

export interface QueueItem {
  path: string;
  status: InstallStatus;
  preview: InstallPreview | null;
  icon: string | null;
  message: string | null;
}

/**
 * Install queue per TV. Lives outside the Install tab, so leaving it keeps the list and
 * installs keep running; the tab shows a dot while one is in progress.
 */
const { queues, setQueues, rejected, setRejected } = createRoot(() => {
  const [queues, setQueues] = createStore<Record<string, QueueItem[]>>({});
  const [rejected, setRejected] = createSignal<Record<string, boolean>>({});
  return { queues, setQueues, rejected, setRejected };
});

/** TVs with an "Install all" loop running, so a second click does not start another. */
const installingAll = new Set<string>();

export function installQueue(serial: string): QueueItem[] {
  return queues[serial] ?? [];
}

/** True when the last drop or pick included files that cannot be installed. */
export function hadRejectedFiles(serial: string): boolean {
  return rejected()[serial] ?? false;
}

export function isInstalling(serial: string): boolean {
  return installQueue(serial).some((item) => item.status === "installing");
}

function update(serial: string, path: string, change: Partial<QueueItem>): void {
  setQueues(serial, (items) => (items ?? []).map((item) => (item.path === path ? { ...item, ...change } : item)));
}

/** Adds files (dropped or picked) and reads a preview of each, one at a time. */
export async function addToQueue(serial: string, paths: string[]): Promise<void> {
  const accepted = paths.filter(isInstallable);
  setRejected((current) => ({ ...current, [serial]: accepted.length < paths.length }));
  const known = new Set(installQueue(serial).map((item) => item.path));
  const fresh = accepted.filter((path) => !known.has(path));
  setQueues(serial, (items) => [
    ...(items ?? []),
    ...fresh.map((path): QueueItem => ({ path, status: "inspecting", preview: null, icon: null, message: null })),
  ]);
  for (const path of fresh) {
    try {
      const preview = await inspectFile(serial, path);
      let icon: string | null = null;
      if (preview.hasIcon) {
        try {
          icon = URL.createObjectURL(new Blob([await fileIcon(path)]));
        } catch {
          // Keep the generic icon.
        }
      }
      update(serial, path, { status: "ready", preview, icon });
    } catch (failure) {
      update(serial, path, { status: "error", message: describeError(failure) });
    }
  }
}

export function removeFromQueue(serial: string, path: string): void {
  const item = installQueue(serial).find((entry) => entry.path === path);
  if (item?.icon) URL.revokeObjectURL(item.icon);
  setQueues(serial, (items) => (items ?? []).filter((entry) => entry.path !== path));
}

/** Installs one file; returns false when the user cancelled a downgrade. */
export async function installItem(serial: string, path: string): Promise<boolean> {
  const item = installQueue(serial).find((entry) => entry.path === path);
  const preview = item?.preview;
  if (!preview || !preview.compatible) return true;
  const downgrade = preview.kind === "downgrade";
  if (downgrade) {
    const confirmed = await confirmAction({
      title: t("install.confirmDowngradeTitle", { name: preview.label ?? preview.package }),
      body: t("install.confirmDowngradeBody", {
        installed: preview.installedVersionName ?? "?",
        file: preview.versionName ?? "?",
      }),
      confirmLabel: t("install.installOlder"),
      danger: true,
    });
    if (!confirmed) return false;
  }
  update(serial, path, { status: "installing", message: null });
  try {
    await installFile(serial, path, downgrade);
    update(serial, path, { status: "installed" });
  } catch (failure) {
    update(serial, path, { status: "error", message: describeInstallError(failure) });
  }
  return true;
}

/** Installs every ready, compatible file in order. */
export async function installAll(serial: string): Promise<void> {
  if (installingAll.has(serial)) return;
  installingAll.add(serial);
  try {
    for (const path of installQueue(serial).map((item) => item.path)) {
      const current = installQueue(serial).find((entry) => entry.path === path);
      if (current?.status === "ready" && current.preview?.compatible) await installItem(serial, path);
    }
  } finally {
    installingAll.delete(serial);
  }
}
