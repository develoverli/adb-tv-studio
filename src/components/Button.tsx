import { type JSX, splitProps } from "solid-js";
import styles from "./Button.module.css";

type Variant = "primary" | "secondary" | "ghost";
type Size = "md" | "lg";

interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

export function Button(props: ButtonProps) {
  const [local, rest] = splitProps(props, ["variant", "size", "class", "children", "type"]);
  return (
    <button
      type={local.type ?? "button"}
      class={`${styles.button} ${styles[local.variant ?? "secondary"]} ${styles[local.size ?? "md"]} ${local.class ?? ""}`}
      {...rest}
    >
      {local.children}
    </button>
  );
}

/** Same look as Button, for navigation. Pass a router <A> or anchor via `class`. */
export function buttonClass(variant: Variant = "secondary", size: Size = "md"): string {
  return `${styles.button} ${styles[variant]} ${styles[size]}`;
}
