import { useParams } from "@solidjs/router";
import { Show } from "solid-js";
import { t } from "../../i18n";
import { findDevice } from "../../lib/devices";
import { LauncherSection } from "./LauncherSection";
import styles from "./sections.module.css";

export function Overview() {
  const params = useParams<{ id: string }>();
  const device = () => findDevice(params.id);

  return (
    <Show when={device()}>
      {(current) => (
        <section class={styles.overview}>
          <h2 class={styles.heading}>{t("sections.overviewTitle")}</h2>
          <dl class={styles.facts}>
            <div class={styles.fact}>
              <dt>{t("device.facts.model")}</dt>
              <dd>{current().model ?? t("device.unknownValue")}</dd>
            </div>
            <div class={styles.fact}>
              <dt>{t("device.facts.android")}</dt>
              <dd>{current().androidVersion ?? t("device.unknownValue")}</dd>
            </div>
            <div class={styles.fact}>
              <dt>{t("device.facts.address")}</dt>
              <dd class="mono">{current().address}</dd>
            </div>
            <div class={styles.fact}>
              <dt>{t("device.facts.connection")}</dt>
              <dd>{t(`status.${current().status}`)}</dd>
            </div>
          </dl>
          <LauncherSection serial={current().id} connected={current().status === "connected"} />
        </section>
      )}
    </Show>
  );
}
