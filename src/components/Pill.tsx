import type { JSX } from "solid-js";
import styles from "./Pill.module.css";

interface PillProps {
  tone?: "neutral" | "accent" | "warning";
  children: JSX.Element;
}

export function Pill(props: PillProps) {
  return <span class={`${styles.pill} ${styles[props.tone ?? "neutral"]}`}>{props.children}</span>;
}
