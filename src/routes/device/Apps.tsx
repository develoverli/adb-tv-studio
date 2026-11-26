import { useParams } from "@solidjs/router";
import Ban from "lucide-solid/icons/ban";
import CircleCheck from "lucide-solid/icons/circle-check";
import CircleStop from "lucide-solid/icons/circle-stop";
import Eraser from "lucide-solid/icons/eraser";
import LoaderCircle from "lucide-solid/icons/loader-circle";
import Package from "lucide-solid/icons/package";
import Play from "lucide-solid/icons/play";
import RefreshCw from "lucide-solid/icons/refresh-cw";
import Search from "lucide-solid/icons/search";
import Trash2 from "lucide-solid/icons/trash";
import X from "lucide-solid/icons/x";
import { For, type JSX, Show, createEffect, createMemo, createSignal, on, onCleanup } from "solid-js";
import { Button } from "../../components/Button";
import { confirmAction } from "../../components/ConfirmDialog";
import { Pill } from "../../components/Pill";
import { t } from "../../i18n";
import { describeError } from "../../lib/adb";
import {
  type AppAction,
  type AppEntry,
  appName,
  listApps,
  loadAppDetails,
  loadAppIcon,
  runAppAction,
} from "../../lib/apps";
import { findDevice } from "../../lib/devices";
import styles from "./Apps.module.css";

const SEARCH_ID = "apps-search";

interface Progress {
  done: number;
  total: number;
  /** Name of the app whose APK is being read right now. */
  current: string;
}

interface Feedback {
  tone: "success" | "error";
  text: string;
}

function matches(app: AppEntry, query: string): boolean {
  if (query === "") return true;
  const needle = query.toLowerCase();
  return appName(app).toLowerCase().includes(needle) || app.package.toLowerCase().includes(needle);
}

export function Apps() {
  const params = useParams<{ id: string }>();
  const serial = () => params.id;
  const connected = () => findDevice(serial())?.status === "connected";

  const [apps, setApps] = createSignal<AppEntry[] | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [query, setQuery] = createSignal("");
  const [selected, setSelected] = createSignal<string | null>(null);
  const [icons, setIcons] = createSignal<Record<string, string>>({});
  const [progress, setProgress] = createSignal<Progress | null>(null);
  // Bumped on every reload or TV switch, so a stale background loop stops.
  let generation = 0;

  const revokeIcons = () => {
    for (const url of Object.values(icons())) URL.revokeObjectURL(url);
    setIcons({});
  };
  onCleanup(() => {
    generation += 1;
    revokeIcons();
  });

  const updateApp = (packageName: string, update: Partial<AppEntry>) => {
    setApps((current) => current?.map((app) => (app.package === packageName ? { ...app, ...update } : app)) ?? null);
  };

  const showIcon = async (app: AppEntry, run: number) => {
    if (icons()[app.package]) return;
    try {
      const bytes = await loadAppIcon(app);
      if (run !== generation) return;
      const url = URL.createObjectURL(new Blob([bytes]));
      setIcons((current) => ({ ...current, [app.package]: url }));
    } catch {
      // No image: the card keeps its generic icon.
    }
  };

  /** Fills names and images one app at a time, from cache when possible. */
  const loadDetails = async (list: AppEntry[], run: number) => {
    const launcher = list.filter((app) => app.launcher);
    for (const app of launcher) {
      if (app.details?.hasIcon) void showIcon(app, run);
    }
    const missing = launcher.filter((app) => !app.details);
    // Count every launcher app, so cached ones show as already done ("8/25", not "0/17").
    let done = launcher.length - missing.length;
    for (const app of missing) {
      if (run !== generation) return;
      setProgress({ done, total: launcher.length, current: appName(app) });
      try {
        const details = await loadAppDetails(serial(), app);
        if (run !== generation) return;
        updateApp(app.package, { details });
        if (details.hasIcon) await showIcon({ ...app, details }, run);
      } catch {
        // Unreadable APK (odd format, removed meanwhile): keep the package name.
      }
      done += 1;
    }
    if (run === generation) setProgress(null);
  };

  const reload = async () => {
    generation += 1;
    const run = generation;
    setLoading(true);
    setError(null);
    try {
      const list = await listApps(serial());
      if (run !== generation) return;
      setApps(list);
      void loadDetails(list, run);
    } catch (failure) {
      if (run === generation) setError(describeError(failure));
    } finally {
      if (run === generation) setLoading(false);
    }
  };

  createEffect(
    on([serial, connected], ([, isConnected]) => {
      setSelected(null);
      if (isConnected) void reload();
    }),
  );
  createEffect(on(serial, revokeIcons, { defer: true }));

  const launcherApps = createMemo(() => (apps() ?? []).filter((app) => app.launcher && matches(app, query())));
  const otherApps = createMemo(() => (apps() ?? []).filter((app) => !app.launcher && matches(app, query())));
  const selectedApp = () => apps()?.find((app) => app.package === selected()) ?? null;

  return (
    <section class={styles.apps}>
      <div class={styles.toolbar}>
        <div class={styles.search}>
          <Search aria-hidden="true" />
          <label class="visually-hidden" for={SEARCH_ID}>
            {t("apps.search")}
          </label>
          <input
            id={SEARCH_ID}
            type="search"
            class={styles.searchInput}
            placeholder={t("apps.search")}
            autocomplete="off"
            spellcheck={false}
            value={query()}
            onInput={(event) => setQuery(event.currentTarget.value)}
          />
        </div>
        <Button variant="secondary" onClick={reload} disabled={loading() || !connected()}>
          <Show when={loading()} fallback={<RefreshCw />}>
            <LoaderCircle class={styles.spin} />
          </Show>
          {t("apps.refresh")}
        </Button>
        <p class={styles.progress} role="status">
          <Show when={progress()}>
            {(current) =>
              t("apps.loadingIcons", {
                done: String(current().done),
                total: String(current().total),
                name: current().current,
              })
            }
          </Show>
        </p>
      </div>

      <Show when={!connected()}>
        <p class={styles.notice}>{t("apps.notConnected")}</p>
      </Show>
      <p class={styles.error} role="alert">
        {error()}
      </p>

      <div class={styles.body} classList={{ [styles.withPanel]: selectedApp() !== null }}>
        <div class={styles.lists}>
          <Show when={apps() === null && loading()}>
            <p class={styles.notice}>{t("apps.loading")}</p>
          </Show>

          <Show when={apps()}>
            <h2 class={styles.heading}>
              {t("apps.launcherTitle")} <span class={styles.count}>{launcherApps().length}</span>
            </h2>
            <Show when={launcherApps().length > 0} fallback={<p class={styles.notice}>{t("apps.noMatches")}</p>}>
              <ul class={styles.grid}>
                <For each={launcherApps()}>
                  {(app) => (
                    <li>
                      <button
                        type="button"
                        class={styles.card}
                        classList={{ [styles.cardSelected]: selected() === app.package }}
                        onClick={() => setSelected(app.package)}
                        aria-pressed={selected() === app.package}
                      >
                        <span class={styles.art}>
                          <Show when={icons()[app.package]} fallback={<Package class={styles.genericIcon} />}>
                            {(url) => <img src={url()} alt="" class={styles.artImage} draggable={false} decoding="async" />}
                          </Show>
                        </span>
                        <span class={styles.cardName}>{appName(app)}</span>
                        <span class={styles.cardMeta}>
                          {app.details?.versionName ?? app.package}
                        </span>
                      </button>
                    </li>
                  )}
                </For>
              </ul>
            </Show>

            <details class={styles.system}>
              <summary class={styles.heading}>
                {t("apps.systemTitle")} <span class={styles.count}>{otherApps().length}</span>
              </summary>
              <ul class={styles.rows}>
                <For each={otherApps()}>
                  {(app) => (
                    <li>
                      <button
                        type="button"
                        class={styles.row}
                        classList={{ [styles.cardSelected]: selected() === app.package }}
                        onClick={() => setSelected(app.package)}
                        aria-pressed={selected() === app.package}
                      >
                        <span class={`${styles.rowPackage} mono`}>{app.package}</span>
                        <Show when={app.disabled}>
                          <Pill tone="warning">{t("apps.disabled")}</Pill>
                        </Show>
                      </button>
                    </li>
                  )}
                </For>
              </ul>
            </details>
          </Show>
        </div>

        <Show when={selectedApp()}>
          {(app) => (
            <AppPanel
              serial={serial()}
              app={app()}
              icon={icons()[app().package]}
              onClose={() => setSelected(null)}
              onChanged={reload}
            />
          )}
        </Show>
      </div>
    </section>
  );
}

interface AppPanelProps {
  serial: string;
  app: AppEntry;
  icon: string | undefined;
  onClose: () => void;
  /** The package list changed (uninstalled, disabled, enabled). */
  onChanged: () => void;
}

function AppPanel(props: AppPanelProps) {
  const [busy, setBusy] = createSignal<AppAction | null>(null);
  const [feedback, setFeedback] = createSignal<Feedback | null>(null);

  createEffect(on(() => props.app.package, () => setFeedback(null)));

  const run = async (action: AppAction) => {
    const name = appName(props.app);
    if (action === "clearData" || action === "uninstall") {
      const confirmed = await confirmAction(
        action === "clearData"
          ? {
              title: t("apps.confirmClearTitle", { name }),
              body: t("apps.confirmClearBody"),
              confirmLabel: t("apps.clearData"),
              danger: true,
            }
          : {
              title: t("apps.confirmUninstallTitle", { name }),
              body: props.app.system ? t("apps.confirmUninstallSystemBody") : t("apps.confirmUninstallBody"),
              confirmLabel: t("apps.uninstall"),
              danger: true,
            },
      );
      if (!confirmed) return;
    }
    setBusy(action);
    setFeedback(null);
    try {
      await runAppAction(props.serial, props.app, action);
      setFeedback({ tone: "success", text: t(`apps.done.${action}`) });
      if (action === "uninstall" || action === "disable" || action === "enable") props.onChanged();
    } catch (failure) {
      setFeedback({ tone: "error", text: describeError(failure) });
    } finally {
      setBusy(null);
    }
  };

  const actionButton = (action: AppAction, icon: JSX.Element, label: string, danger = false) => (
    <Button
      variant="secondary"
      class={danger ? styles.dangerButton : undefined}
      onClick={() => run(action)}
      disabled={busy() !== null}
    >
      <Show when={busy() === action} fallback={icon}>
        <LoaderCircle class={styles.spin} />
      </Show>
      {label}
    </Button>
  );

  return (
    <aside class={styles.panel} aria-label={appName(props.app)}>
      <div class={styles.panelHeader}>
        <span class={styles.panelArt}>
          <Show when={props.icon} fallback={<Package class={styles.genericIcon} />}>
            {(url) => <img src={url()} alt="" class={styles.artImage} draggable={false} decoding="async" />}
          </Show>
        </span>
        <Button variant="ghost" onClick={() => props.onClose()} aria-label={t("apps.close")} class={styles.close}>
          <X />
        </Button>
      </div>
      <h2 class={styles.panelName}>{appName(props.app)}</h2>
      <dl class={styles.facts}>
        <div>
          <dt>{t("apps.package")}</dt>
          <dd class="mono">{props.app.package}</dd>
        </div>
        <div>
          <dt>{t("apps.version")}</dt>
          <dd>{props.app.details?.versionName ?? props.app.versionCode ?? t("device.unknownValue")}</dd>
        </div>
        <div>
          <dt>{t("apps.type")}</dt>
          <dd>
            {props.app.disabled ? t("apps.disabled") : props.app.system ? t("apps.system") : t("apps.userInstalled")}
          </dd>
        </div>
      </dl>

      <div class={styles.actions}>
        {actionButton("open", <Play />, t("apps.open"))}
        {actionButton("forceStop", <CircleStop />, t("apps.forceStop"))}
        <Show
          when={props.app.disabled}
          fallback={actionButton("disable", <Ban />, t("apps.disable"))}
        >
          {actionButton("enable", <CircleCheck />, t("apps.enable"))}
        </Show>
      </div>
      <div class={styles.actions}>
        {actionButton("clearData", <Eraser />, t("apps.clearData"), true)}
        {actionButton("uninstall", <Trash2 />, t("apps.uninstall"), true)}
      </div>

      <p
        class={feedback()?.tone === "error" ? styles.error : styles.success}
        role={feedback()?.tone === "error" ? "alert" : "status"}
      >
        {feedback()?.text}
      </p>
    </aside>
  );
}
