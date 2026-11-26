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
import { For, type JSX, Show, createEffect, createMemo, createSignal, on } from "solid-js";
import { Button } from "../../components/Button";
import { confirmAction } from "../../components/ConfirmDialog";
import { Pill } from "../../components/Pill";
import { t } from "../../i18n";
import { describeError } from "../../lib/adb";
import { type AppAction, type AppEntry, appName, runAppAction } from "../../lib/apps";
import { appsState, ensureApps, reloadApps, setAppsQuery, setSelectedApp } from "../../lib/appsStore";
import { findDevice } from "../../lib/devices";
import styles from "./Apps.module.css";

const SEARCH_ID = "apps-search";

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

  // State lives in a per-TV store, so leaving the tab keeps the list and the loading goes on.
  const current = () => appsState(serial());
  const apps = () => current().apps;
  const loading = () => current().loading;
  const error = () => current().error;
  const icons = () => current().icons;
  const progress = () => current().progress;
  const query = () => current().query;
  const setQuery = (value: string) => setAppsQuery(serial(), value);
  const selected = () => current().selected;
  const setSelected = (packageName: string | null) => setSelectedApp(serial(), packageName);
  const reload = () => reloadApps(serial());

  createEffect(
    on([serial, connected], ([currentSerial, isConnected]) => {
      if (isConnected) ensureApps(currentSerial);
    }),
  );

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
