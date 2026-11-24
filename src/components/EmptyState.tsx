import type { JSX } from "solid-js";
import { Show } from "solid-js";
import styles from "./EmptyState.module.css";

interface EmptyStateProps {
  icon: JSX.Element;
  title: string;
  body: string;
  badge?: JSX.Element;
  children?: JSX.Element;
}

/** Teaching empty state: what this place is for and, when possible, the next step. */
export function EmptyState(props: EmptyStateProps) {
  return (
    <section class={styles.empty}>
      <div class={styles.icon} aria-hidden="true">
        {props.icon}
      </div>
      <div class={styles.text}>
        <Show when={props.badge}>
          <div class={styles.badge}>{props.badge}</div>
        </Show>
        <h2 class={styles.title}>{props.title}</h2>
        <p class={styles.body}>{props.body}</p>
      </div>
      <Show when={props.children}>
        <div class={styles.actions}>{props.children}</div>
      </Show>
    </section>
  );
}
