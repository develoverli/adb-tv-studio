import type { RouteSectionProps } from "@solidjs/router";
import { Show, onMount } from "solid-js";
import { isMac } from "../lib/platform";
import { scheduleLaunchCheck } from "../lib/updates";
import styles from "./AppShell.module.css";
import { ConfirmHost } from "./ConfirmDialog";
import { Sidebar } from "./Sidebar";
import { WindowControls } from "./WindowControls";

/** Window frame: sidebar + main column whose top strip is the draggable title bar. */
export function AppShell(props: RouteSectionProps) {
  onMount(scheduleLaunchCheck);

  return (
    <div class={styles.shell}>
      <Sidebar />
      <div class={styles.main}>
        <header class={styles.titlebar} data-tauri-drag-region>
          <Show when={!isMac}>
            <WindowControls />
          </Show>
        </header>
        <main class={styles.content}>{props.children}</main>
      </div>
      <ConfirmHost />
    </div>
  );
}
