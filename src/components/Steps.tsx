import { For } from "solid-js";
import styles from "./Steps.module.css";

export interface Step {
  title: string;
  body: string;
}

/** Numbered procedure. Numbers carry the order, so no icons or stripes are needed. */
export function Steps(props: { steps: Step[] }) {
  return (
    <ol class={styles.steps}>
      <For each={props.steps}>
        {(step, index) => (
          <li class={styles.step}>
            <span class={styles.number} aria-hidden="true">
              {index() + 1}
            </span>
            <div class={styles.text}>
              <h3 class={styles.title}>{step.title}</h3>
              <p class={styles.body}>{step.body}</p>
            </div>
          </li>
        )}
      </For>
    </ol>
  );
}
