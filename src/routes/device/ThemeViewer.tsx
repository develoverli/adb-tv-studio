import X from "lucide-solid/icons/x";
import { type JSX, createEffect, onCleanup } from "solid-js";
import { Button } from "../../components/Button";
import { type PreviewApp, ThemePreview } from "../../components/ThemePreview";
import { t } from "../../i18n";
import type { ThemeFile } from "../../lib/themes";
import styles from "./ThemeViewer.module.css";

interface ThemeViewerProps {
  theme: ThemeFile;
  image?: string;
  apps?: PreviewApp[];
  meta: string;
  /** Apply / download buttons for this theme. */
  actions: JSX.Element;
  onClose: () => void;
}

/**
 * Large preview of a theme, close to how the TV will look. Native <dialog>: focus trap, Esc and
 * the backdrop click come from the platform.
 */
export function ThemeViewer(props: ThemeViewerProps) {
  let dialog: HTMLDialogElement | undefined;

  createEffect(() => {
    if (dialog && !dialog.open) dialog.showModal();
  });
  onCleanup(() => dialog?.close());

  return (
    <dialog
      ref={dialog}
      class={styles.viewer}
      aria-labelledby="theme-viewer-title"
      onClose={() => props.onClose()}
      onClick={(event) => {
        // A click on the backdrop lands on the dialog element itself.
        if (event.target === dialog) dialog.close();
      }}
    >
      <div class={styles.content}>
        <ThemePreview theme={props.theme} image={props.image} apps={props.apps} class={styles.preview} />
        <div class={styles.footer}>
          <div class={styles.text}>
            <h2 id="theme-viewer-title" class={styles.name}>
              {props.theme.name}
            </h2>
            <p class={styles.meta}>{props.meta}</p>
            <p class={styles.description}>{props.theme.description}</p>
          </div>
          <div class={styles.actions}>
            {props.actions}
            <Button variant="ghost" onClick={() => dialog?.close()}>
              <X />
              {t("themes.close")}
            </Button>
          </div>
        </div>
      </div>
    </dialog>
  );
}
