import { createRoot } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { describeError } from "./adb";
import { type AppEntry, appName, listApps, loadAppDetails, loadAppIcon } from "./apps";

export interface AppsProgress {
  done: number;
  total: number;
  /** Name of the app whose APK is being read right now. */
  current: string;
}

/** Apps tab state for one TV. Lives outside the component, so leaving the tab keeps it (and the loading goes on). */
export interface AppsState {
  apps: AppEntry[] | null;
  loading: boolean;
  error: string | null;
  /** Object URLs of banners/icons, by package. */
  icons: Record<string, string>;
  progress: AppsProgress | null;
  query: string;
  selected: string | null;
}

const EMPTY: AppsState = {
  apps: null,
  loading: false,
  error: null,
  icons: {},
  progress: null,
  query: "",
  selected: null,
};

const { state, setState } = createRoot(() => {
  const [state, setState] = createStore<Record<string, AppsState>>({});
  return { state, setState };
});

/** Bumped on every reload of a TV, so its previous background loop stops. */
const generations = new Map<string, number>();

function ensure(serial: string): void {
  if (!state[serial]) setState(serial, { ...EMPTY, icons: {} });
}

function isCurrent(serial: string, run: number): boolean {
  return generations.get(serial) === run;
}

export function appsState(serial: string): AppsState {
  return state[serial] ?? EMPTY;
}

export function setAppsQuery(serial: string, query: string): void {
  ensure(serial);
  setState(serial, "query", query);
}

export function setSelectedApp(serial: string, packageName: string | null): void {
  ensure(serial);
  setState(serial, "selected", packageName);
}

function updateApp(serial: string, packageName: string, update: Partial<AppEntry>): void {
  setState(
    serial,
    "apps",
    produce((apps) => {
      const app = apps?.find((entry) => entry.package === packageName);
      if (app) Object.assign(app, update);
    }),
  );
}

async function showIcon(serial: string, app: AppEntry, run: number): Promise<void> {
  if (appsState(serial).icons[app.package]) return;
  try {
    const bytes = await loadAppIcon(app);
    if (!isCurrent(serial, run)) return;
    setState(serial, "icons", app.package, URL.createObjectURL(new Blob([bytes])));
  } catch {
    // No image: the card keeps its generic icon.
  }
}

/** Fills names and images one app at a time, from cache when possible. */
async function loadDetails(serial: string, list: AppEntry[], run: number): Promise<void> {
  const launcher = list.filter((app) => app.launcher);
  for (const app of launcher) {
    if (app.details?.hasIcon) void showIcon(serial, app, run);
  }
  const missing = launcher.filter((app) => !app.details);
  // Count every launcher app, so cached ones show as already done ("8/25", not "0/17").
  let done = launcher.length - missing.length;
  for (const app of missing) {
    if (!isCurrent(serial, run)) return;
    setState(serial, "progress", { done, total: launcher.length, current: appName(app) });
    try {
      const details = await loadAppDetails(serial, app);
      if (!isCurrent(serial, run)) return;
      updateApp(serial, app.package, { details });
      if (details.hasIcon) await showIcon(serial, { ...app, details }, run);
    } catch {
      // Unreadable APK (odd format, removed meanwhile): keep the package name.
    }
    done += 1;
  }
  if (isCurrent(serial, run)) setState(serial, "progress", null);
}

/** Loads the package list again; names and icons fill in the background. */
export async function reloadApps(serial: string): Promise<void> {
  ensure(serial);
  const run = (generations.get(serial) ?? 0) + 1;
  generations.set(serial, run);
  setState(serial, { loading: true, error: null });
  try {
    const list = await listApps(serial);
    if (!isCurrent(serial, run)) return;
    setState(serial, "apps", list);
    void loadDetails(serial, list, run);
  } catch (failure) {
    if (isCurrent(serial, run)) setState(serial, "error", describeError(failure));
  } finally {
    if (isCurrent(serial, run)) setState(serial, "loading", false);
  }
}

/** Loads the list the first time a TV's Apps tab is opened; later visits keep what is there. */
export function ensureApps(serial: string): void {
  const current = appsState(serial);
  if (current.apps === null && !current.loading) void reloadApps(serial);
}
