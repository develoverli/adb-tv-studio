import House from "lucide-solid/icons/house";
import LoaderCircle from "lucide-solid/icons/loader-circle";
import RotateCcw from "lucide-solid/icons/rotate-ccw";
import { For, Show, createResource, createSignal } from "solid-js";
import { Button } from "../../components/Button";
import { confirmAction } from "../../components/ConfirmDialog";
import { Pill } from "../../components/Pill";
import { t } from "../../i18n";
import { describeError } from "../../lib/adb";
import { readablePackage } from "../../lib/apps";
import { appsState } from "../../lib/appsStore";
import {
  type LauncherApp,
  TV_LAUNCHER_NAME,
  TV_LAUNCHER_PACKAGE,
  disableStockLauncher,
  launcherStatus,
  restoreStockLauncher,
  setLauncher,
} from "../../lib/launcher";
import styles from "./LauncherSection.module.css";
import { TvLauncherSetup } from "./TvLauncherSetup";

interface Feedback {
  tone: "success" | "error";
  text: string;
}

type Busy = { kind: "set"; package: string } | { kind: "disable" } | { kind: "restore" };

/** Default launcher: which app HOME opens, switching it, and disabling or restoring the stock one. */
export function LauncherSection(props: { serial: string; connected: boolean }) {
  const [status, { refetch }] = createResource(
    () => (props.connected ? props.serial : null),
    (serial) => launcherStatus(serial),
  );
  const [busy, setBusy] = createSignal<Busy | null>(null);
  const [feedback, setFeedback] = createSignal<Feedback | null>(null);

  /** Real app name when the Apps tab already read it; tv-launcher by name; otherwise from the package. */
  const nameOf = (packageName: string) => {
    if (packageName === TV_LAUNCHER_PACKAGE) return TV_LAUNCHER_NAME;
    const known = appsState(props.serial).apps?.find((app) => app.package === packageName);
    return known?.details?.label ?? readablePackage(packageName);
  };

  // tv-launcher first, then the rest by name.
  const apps = () =>
    [...(status()?.apps ?? [])].sort((left, right) => {
      if (left.package === TV_LAUNCHER_PACKAGE) return -1;
      if (right.package === TV_LAUNCHER_PACKAGE) return 1;
      return nameOf(left.package).localeCompare(nameOf(right.package));
    });

  const current = () => status()?.current ?? null;
  const currentApp = () => status()?.apps.find((app) => app.package === current());
  /** A user-installed launcher is set and stock ones are still enabled: offer to disable them. */
  const canDisableStock = () =>
    currentApp()?.system === false && (status()?.apps ?? []).some((app) => app.system);

  const act = async (next: Busy, action: () => Promise<void>, success: string) => {
    setBusy(next);
    setFeedback(null);
    try {
      await action();
      setFeedback({ tone: "success", text: success });
    } catch (failure) {
      setFeedback({ tone: "error", text: describeError(failure) });
    } finally {
      setBusy(null);
      void refetch();
    }
  };

  const choose = (app: LauncherApp) =>
    act({ kind: "set", package: app.package }, () => setLauncher(props.serial, app.package), t("launcher.set"));

  const disableStock = async () => {
    const confirmed = await confirmAction({
      title: t("launcher.disableTitle"),
      body: t("launcher.disableBody"),
      confirmLabel: t("launcher.disable"),
      danger: true,
    });
    if (!confirmed) return;
    await act({ kind: "disable" }, () => disableStockLauncher(props.serial), t("launcher.disabled"));
  };

  const restore = () => act({ kind: "restore" }, () => restoreStockLauncher(props.serial), t("launcher.restored"));

  const settingThis = (packageName: string) => {
    const state = busy();
    return state?.kind === "set" && state.package === packageName;
  };

  return (
    <section class={styles.launcher} aria-labelledby="launcher-heading">
      <h2 id="launcher-heading" class={styles.heading}>
        {t("launcher.title")}
      </h2>

      <TvLauncherSetup serial={props.serial} connected={props.connected} onChanged={() => void refetch()} />
      <Show when={props.connected} fallback={<p class={styles.note}>{t("launcher.notConnected")}</p>}>
        <Show when={!status.error} fallback={<p class={styles.error}>{describeError(status.error)}</p>}>
          <Show when={status()} fallback={<p class={styles.note}>{t("launcher.loading")}</p>}>
            <ul class={styles.list}>
              <For each={apps()}>
                {(app) => (
                  <li class={styles.row}>
                    <House class={styles.rowIcon} aria-hidden="true" />
                    <span class={styles.rowText}>
                      <span class={styles.rowName}>{nameOf(app.package)}</span>
                      <span class={`${styles.rowMeta} mono`}>{app.package}</span>
                    </span>
                    <Show when={app.system}>
                      <Pill>{t("launcher.stock")}</Pill>
                    </Show>
                    <Show
                      when={app.package !== current()}
                      fallback={<Pill tone="accent">{t("launcher.current")}</Pill>}
                    >
                      <Button
                        variant={app.package === TV_LAUNCHER_PACKAGE ? "primary" : "secondary"}
                        onClick={() => choose(app)}
                        disabled={busy() !== null}
                      >
                        <Show when={settingThis(app.package)}>
                          <LoaderCircle class={styles.spin} />
                        </Show>
                        {t("launcher.use")}
                      </Button>
                    </Show>
                  </li>
                )}
              </For>
            </ul>

            <Show when={canDisableStock()}>
              <div class={styles.extra}>
                <p class={styles.note}>{t("launcher.stockWinsHint")}</p>
                <Button
                  variant="secondary"
                  class={styles.dangerButton}
                  onClick={disableStock}
                  disabled={busy() !== null}
                >
                  <Show when={busy()?.kind === "disable"}>
                    <LoaderCircle class={styles.spin} />
                  </Show>
                  {t("launcher.disable")}
                </Button>
              </div>
            </Show>

            <Show when={(status()?.disabledStock.length ?? 0) > 0}>
              <div class={styles.extra}>
                <p class={styles.note}>
                  {t("launcher.disabledList", {
                    names: (status()?.disabledStock ?? []).map(nameOf).join(", "),
                  })}
                </p>
                <Button variant="secondary" onClick={restore} disabled={busy() !== null}>
                  <Show when={busy()?.kind === "restore"} fallback={<RotateCcw />}>
                    <LoaderCircle class={styles.spin} />
                  </Show>
                  {t("launcher.restore")}
                </Button>
              </div>
            </Show>
          </Show>
        </Show>
      </Show>

      <p
        class={feedback()?.tone === "error" ? styles.error : styles.success}
        role={feedback()?.tone === "error" ? "alert" : "status"}
      >
        {feedback()?.text}
      </p>
    </section>
  );
}
