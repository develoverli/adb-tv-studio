import { useParams } from "@solidjs/router";
import Gamepad2 from "lucide-solid/icons/gamepad-2";
import LayoutGrid from "lucide-solid/icons/layout-grid";
import Package from "lucide-solid/icons/package";
import Palette from "lucide-solid/icons/palette";
import { Show } from "solid-js";
import { EmptyState } from "../../components/EmptyState";
import { Pill } from "../../components/Pill";
import { t } from "../../i18n";
import { findDevice } from "../../lib/devices";
import styles from "./sections.module.css";

function InDevelopment() {
  return <Pill>{t("common.inDevelopment")}</Pill>;
}

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
        </section>
      )}
    </Show>
  );
}

export function Apps() {
  return (
    <EmptyState
      icon={<LayoutGrid />}
      badge={<InDevelopment />}
      title={t("sections.appsTitle")}
      body={t("sections.appsBody")}
    />
  );
}

export function Install() {
  return (
    <EmptyState
      icon={<Package />}
      badge={<InDevelopment />}
      title={t("sections.installTitle")}
      body={t("sections.installBody")}
    />
  );
}

export function Themes() {
  return (
    <EmptyState
      icon={<Palette />}
      badge={<InDevelopment />}
      title={t("sections.themesTitle")}
      body={t("sections.themesBody")}
    />
  );
}

export function Remote() {
  return (
    <EmptyState
      icon={<Gamepad2 />}
      badge={<InDevelopment />}
      title={t("sections.remoteTitle")}
      body={t("sections.remoteBody")}
    />
  );
}
