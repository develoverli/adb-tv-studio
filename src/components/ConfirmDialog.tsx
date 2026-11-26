import TriangleAlert from "lucide-solid/icons/triangle-exclamation-point";
import { Show, createEffect, createRoot, createSignal } from "solid-js";
import { t } from "../i18n";
import { Button } from "./Button";
import styles from "./ConfirmDialog.module.css";

export interface ConfirmOptions {
  title: string;
  body: string;
  confirmLabel: string;
  /** Destructive action: red confirm button and a warning icon. */
  danger?: boolean;
}

interface PendingConfirm extends ConfirmOptions {
  resolve: (confirmed: boolean) => void;
}

const { pending, setPending } = createRoot(() => {
  const [pending, setPending] = createSignal<PendingConfirm | null>(null);
  return { pending, setPending };
});

/**
 * Asks for confirmation with the app's own modal (never the browser's `confirm()`).
 * Resolves `true` on confirm, `false` on cancel, Esc or a newer request.
 */
export function confirmAction(options: ConfirmOptions): Promise<boolean> {
  pending()?.resolve(false);
  return new Promise((resolve) => setPending({ ...options, resolve }));
}

const TITLE_ID = "confirm-dialog-title";
const BODY_ID = "confirm-dialog-body";

/** Mount once in the app shell. Uses <dialog> so focus trapping, Esc and the backdrop are native. */
export function ConfirmHost() {
  let dialog: HTMLDialogElement | undefined;

  const close = (confirmed: boolean) => {
    const current = pending();
    setPending(null);
    current?.resolve(confirmed);
  };

  createEffect(() => {
    if (!dialog) return;
    if (pending() && !dialog.open) dialog.showModal();
    if (!pending() && dialog.open) dialog.close();
  });

  return (
    <dialog
      ref={dialog}
      class={styles.dialog}
      aria-labelledby={TITLE_ID}
      aria-describedby={BODY_ID}
      onCancel={(event) => {
        event.preventDefault();
        close(false);
      }}
    >
      <Show when={pending()}>
        {(current) => (
          <div class={styles.content}>
            <div class={styles.text}>
              <Show when={current().danger}>
                <span class={styles.icon} aria-hidden="true">
                  <TriangleAlert />
                </span>
              </Show>
              <h2 id={TITLE_ID} class={styles.title}>
                {current().title}
              </h2>
              <p id={BODY_ID} class={styles.body}>
                {current().body}
              </p>
            </div>
            <div class={styles.actions}>
              {/* Cancel first in the DOM so it gets initial focus: the safe default. */}
              <Button variant="secondary" onClick={() => close(false)} autofocus>
                {t("common.cancel")}
              </Button>
              <Button
                variant="primary"
                class={current().danger ? styles.danger : undefined}
                onClick={() => close(true)}
              >
                {current().confirmLabel}
              </Button>
            </div>
          </div>
        )}
      </Show>
    </dialog>
  );
}
