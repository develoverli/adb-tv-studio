import { For } from "solid-js";
import styles from "./Segmented.module.css";

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
}

interface SegmentedProps<T extends string> {
  name: string;
  label: string;
  value: T;
  options: SegmentedOption<T>[];
  onChange: (value: T) => void;
}

/** Single choice from 2-4 options, built on native radios for keyboard and screen readers. */
export function Segmented<T extends string>(props: SegmentedProps<T>) {
  return (
    <fieldset class={styles.group}>
      <legend class="visually-hidden">{props.label}</legend>
      <For each={props.options}>
        {(option) => (
          <label class={styles.option} classList={{ [styles.selected]: option.value === props.value }}>
            <input
              class={styles.radio}
              type="radio"
              name={props.name}
              value={option.value}
              checked={option.value === props.value}
              onChange={() => props.onChange(option.value)}
            />
            {option.label}
          </label>
        )}
      </For>
    </fieldset>
  );
}
