import { getCurrentWindow } from "@tauri-apps/api/window";
import Copy from "lucide-solid/icons/copy";
import Minus from "lucide-solid/icons/minus";
import Square from "lucide-solid/icons/square";
import X from "lucide-solid/icons/x";
import { createSignal, onCleanup, onMount } from "solid-js";
import { t } from "../i18n";
import styles from "./WindowControls.module.css";

/** Minimize / maximize / close for Windows and Linux (macOS uses native traffic lights). */
export function WindowControls() {
  const appWindow = getCurrentWindow();
  const [maximized, setMaximized] = createSignal(false);

  const syncMaximized = async () => setMaximized(await appWindow.isMaximized());

  onMount(async () => {
    await syncMaximized();
    const unlisten = await appWindow.onResized(syncMaximized);
    onCleanup(unlisten);
  });

  return (
    <div class={styles.controls}>
      <button class={styles.control} type="button" aria-label={t("window.minimize")} onClick={() => appWindow.minimize()}>
        <Minus />
      </button>
      <button
        class={styles.control}
        type="button"
        aria-label={maximized() ? t("window.restore") : t("window.maximize")}
        onClick={() => appWindow.toggleMaximize()}
      >
        {maximized() ? <Copy class={styles.restore} /> : <Square />}
      </button>
      <button
        class={`${styles.control} ${styles.close}`}
        type="button"
        aria-label={t("window.close")}
        onClick={() => appWindow.close()}
      >
        <X />
      </button>
    </div>
  );
}
