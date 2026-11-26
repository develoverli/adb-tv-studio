import { A, type RouteSectionProps, useNavigate, useParams } from "@solidjs/router";
import LoaderCircle from "lucide-solid/icons/loader-circle";
import RefreshCw from "lucide-solid/icons/refresh-cw";
import Unplug from "lucide-solid/icons/unplug";
import { For, Show, createSignal } from "solid-js";
import { Button } from "../../components/Button";
import { EmptyState } from "../../components/EmptyState";
import { StatusDot } from "../../components/StatusDot";
import { t } from "../../i18n";
import { describeError, disconnect, reconnect } from "../../lib/adb";
import { findDevice } from "../../lib/devices";
import { isInstalling } from "../../lib/installQueue";
import styles from "./DeviceLayout.module.css";

const TABS = [
  { path: "", key: "overview" },
  { path: "/apps", key: "apps" },
  { path: "/install", key: "install" },
  { path: "/themes", key: "themes" },
  { path: "/remote", key: "remote" },
] as const;

export function DeviceLayout(props: RouteSectionProps) {
  const params = useParams<{ id: string }>();
  const device = () => findDevice(params.id);
  const navigate = useNavigate();
  const [disconnecting, setDisconnecting] = createSignal(false);
  const [reconnecting, setReconnecting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const busy = () => disconnecting() || reconnecting();

  const reconnectDevice = async (serial: string) => {
    setReconnecting(true);
    setError(null);
    try {
      const status = await reconnect(serial);
      if (status === "unauthorized") setError(t("add.unauthorized"));
    } catch (failure) {
      setError(describeError(failure));
    } finally {
      setReconnecting(false);
    }
  };

  const disconnectDevice = async (serial: string) => {
    setDisconnecting(true);
    setError(null);
    try {
      await disconnect(serial);
      // Without saved TVs (next roadmap step) a disconnected TV leaves the list, so leave its page too.
      navigate("/");
    } catch (failure) {
      setError(describeError(failure));
    } finally {
      setDisconnecting(false);
    }
  };

  return (
    <Show
      when={device()}
      fallback={<EmptyState icon={<Unplug />} title={t("device.notFoundTitle")} body={t("device.notFoundBody")} />}
    >
      {(current) => (
        <div class={styles.device}>
          <header class={styles.header}>
            <div class={styles.identity}>
              <h1 class={styles.name}>{current().name}</h1>
              <p class={styles.meta}>
                <Show when={current().model}>
                  {(model) => (
                    <>
                      <span>{model()}</span>
                      <span aria-hidden="true">·</span>
                    </>
                  )}
                </Show>
                <Show when={current().androidVersion}>
                  {(version) => (
                    <>
                      <span>{t("device.androidVersion", { version: version() })}</span>
                      <span aria-hidden="true">·</span>
                    </>
                  )}
                </Show>
                <span class="mono">{current().address}</span>
              </p>
            </div>
            <div class={styles.actions}>
              <p class={styles.status} classList={{ [styles.statusConnected]: current().status === "connected" }}>
                <StatusDot status={current().status} />
                {t(`status.${current().status}`)}
              </p>
              <div class={styles.actionButtons}>
                <Show when={current().status === "offline"}>
                  <Button
                    variant="primary"
                    onClick={() => reconnectDevice(current().id)}
                    disabled={busy()}
                    aria-describedby="device-disconnect-error"
                  >
                    <Show when={reconnecting()} fallback={<RefreshCw />}>
                      <LoaderCircle class={styles.spin} />
                    </Show>
                    {reconnecting() ? t("device.reconnecting") : t("device.reconnect")}
                  </Button>
                </Show>
                <Button
                  variant="ghost"
                  onClick={() => disconnectDevice(current().id)}
                  disabled={busy()}
                  aria-describedby="device-disconnect-error"
                >
                  <Show when={disconnecting()} fallback={<Unplug />}>
                    <LoaderCircle class={styles.spin} />
                  </Show>
                  {disconnecting() ? t("device.disconnecting") : t("device.disconnect")}
                </Button>
              </div>
              <p id="device-disconnect-error" class={styles.actionError} role="alert">
                {error()}
              </p>
            </div>
          </header>

          <nav class={styles.tabs} aria-label={current().name}>
            <For each={TABS}>
              {(tab) => (
                <A
                  href={`/device/${current().id}${tab.path}`}
                  end={tab.path === ""}
                  class={styles.tab}
                  activeClass={styles.tabActive}
                >
                  {t(`device.tabs.${tab.key}`)}
                  <Show when={tab.key === "install" && isInstalling(current().id)}>
                    <span class={styles.tabBadge} title={t("install.installing")}>
                      <span class="visually-hidden">{t("install.installing")}</span>
                    </span>
                  </Show>
                </A>
              )}
            </For>
          </nav>

          <div class={styles.panel}>{props.children}</div>
        </div>
      )}
    </Show>
  );
}
