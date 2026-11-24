import type { DeviceStatus } from "../lib/devices";
import styles from "./StatusDot.module.css";

interface StatusDotProps {
  status: DeviceStatus;
}

/** Decorative: always pair it with a visible or screen-reader status label. */
export function StatusDot(props: StatusDotProps) {
  return <span class={`${styles.dot} ${styles[props.status]}`} aria-hidden="true" />;
}
