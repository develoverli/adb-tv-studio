import { A } from "@solidjs/router";
import Plus from "lucide-solid/icons/plus";
import Settings from "lucide-solid/icons/settings";
import { For, Show } from "solid-js";
import { t } from "../i18n";
import { devices } from "../lib/devices";
import { isMac } from "../lib/platform";
import { Logo } from "./Logo";
import styles from "./Sidebar.module.css";
import { StatusDot } from "./StatusDot";

export function Sidebar() {
  return (
    <nav class={styles.sidebar} aria-label={t("app.name")}>
      <div class={styles.brand} classList={{ [styles.brandMac]: isMac }} data-tauri-drag-region>
        <Logo size={20} class={styles.logo} />
        <span class={styles.brandName} data-tauri-drag-region>
          {t("app.name")}
        </span>
      </div>

      <div class={styles.section}>
        <h2 class={styles.sectionLabel}>
          {t("sidebar.devices")}
          <Show when={devices.length > 0}>
            <span class={styles.count}>{devices.length}</span>
          </Show>
        </h2>

        <Show when={devices.length > 0} fallback={<p class={styles.empty}>{t("sidebar.noDevices")}</p>}>
          <ul class={styles.list}>
            <For each={devices}>
              {(device) => (
                <li>
                  <A href={`/device/${device.id}`} class={styles.item} activeClass={styles.itemActive}>
                    <StatusDot status={device.status} />
                    <span class={styles.itemText}>
                      <span class={styles.itemName}>{device.name}</span>
                      <span class={styles.itemMeta}>{t(`status.${device.status}`)}</span>
                    </span>
                  </A>
                </li>
              )}
            </For>
          </ul>
        </Show>

        <A href="/add" class={`${styles.item} ${styles.add}`} activeClass={styles.itemActive}>
          <Plus class={styles.icon} />
          <span>{t("sidebar.addDevice")}</span>
        </A>
      </div>

      <div class={styles.footer}>
        <A href="/settings" class={styles.item} activeClass={styles.itemActive}>
          <Settings class={styles.icon} />
          <span>{t("sidebar.settings")}</span>
        </A>
      </div>
    </nav>
  );
}
