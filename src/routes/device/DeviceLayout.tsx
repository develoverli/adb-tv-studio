import { A, type RouteSectionProps, useNavigate, useParams } from "@solidjs/router";
import LoaderCircle from "lucide-solid/icons/loader-circle";
import Unplug from "lucide-solid/icons/unplug";
import { For, Show, createSignal } from "solid-js";
import { Button } from "../../components/Button";
import { EmptyState } from "../../components/EmptyState";
import { StatusDot } from "../../components/StatusDot";
import { t } from "../../i18n";
import { describeError, disconnect } from "../../lib/adb";
import { findDevice } from "../../lib/devices";
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
  const [error, setError] = createSignal<string | null>(null);

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
              <Button
                variant="ghost"
                onClick={() => disconnectDevice(current().id)}
                disabled={disconnecting()}
                aria-describedby="device-disconnect-error"
              >
                <Show when={disconnecting()} fallback={<Unplug />}>
                  <LoaderCircle class={styles.spin} />
                </Show>
                {disconnecting() ? t("device.disconnecting") : t("device.disconnect")}
              </Button>
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
