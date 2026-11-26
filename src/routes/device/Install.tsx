import { useParams } from "@solidjs/router";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import CircleCheck from "lucide-solid/icons/circle-check";
import FolderOpen from "lucide-solid/icons/folder-open";
import LoaderCircle from "lucide-solid/icons/loader-circle";
import Package from "lucide-solid/icons/package";
import PackagePlus from "lucide-solid/icons/package-plus";
import Play from "lucide-solid/icons/play";
import TriangleAlert from "lucide-solid/icons/triangle-exclamation-point";
import X from "lucide-solid/icons/x";
import { For, Show, createSignal, onCleanup, onMount } from "solid-js";
import { Button } from "../../components/Button";
import { Pill } from "../../components/Pill";
import { t } from "../../i18n";
import { describeError } from "../../lib/adb";
import { findDevice } from "../../lib/devices";
import { type InstallPreview, formatSize, openPackage, pickFiles } from "../../lib/install";
import {
  type QueueItem,
  addToQueue,
  hadRejectedFiles,
  installAll as installEverything,
  installItem,
  installQueue,
  removeFromQueue,
} from "../../lib/installQueue";
import styles from "./Install.module.css";

function fileNameOf(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function kindLabel(preview: InstallPreview): string {
  switch (preview.kind) {
    case "new":
      return t("install.kind.new");
    case "update":
      return t("install.kind.update", {
        from: preview.installedVersionName ?? "?",
        to: preview.versionName ?? "?",
      });
    case "same":
      return t("install.kind.same");
    case "downgrade":
      return t("install.kind.downgrade", { installed: preview.installedVersionName ?? "?" });
    case "replace":
      return t("install.kind.replace");
  }
}

export function Install() {
  const params = useParams<{ id: string }>();
  const serial = () => params.id;
  const connected = () => findDevice(serial())?.status === "connected";

  // Queue lives in a per-TV store: leaving the tab keeps it and installs keep running.
  const queue = () => installQueue(serial());
  const rejected = () => hadRejectedFiles(serial());
  const [dragging, setDragging] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const addFiles = (paths: string[]) => addToQueue(serial(), paths);
  const remove = (path: string) => removeFromQueue(serial(), path);
  const install = (item: QueueItem) => installItem(serial(), item.path);
  const installAll = () => installEverything(serial());

  const choose = async () => {
    setError(null);
    try {
      await addFiles(await pickFiles());
    } catch (failure) {
      setError(describeError(failure));
    }
  };

  onMount(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "enter" || payload.type === "over") setDragging(true);
      else if (payload.type === "leave") setDragging(false);
      else {
        setDragging(false);
        if (connected()) void addFiles(payload.paths);
      }
    });
    onCleanup(() => {
      void unlisten.then((stop) => stop());
    });
  });

  const readyCount = () => queue().filter((item) => item.status === "ready" && item.preview?.compatible).length;
  const busy = () => queue().some((item) => item.status === "installing");

  return (
    <section class={styles.install}>
      <div class={styles.dropzone} classList={{ [styles.dragging]: dragging() }}>
        <PackagePlus class={styles.dropIcon} aria-hidden="true" />
        <p class={styles.dropTitle}>{dragging() ? t("install.dropNow") : t("install.dropTitle")}</p>
        <p class={styles.dropHint}>{t("install.dropHint")}</p>
        <Button variant="primary" onClick={choose} disabled={!connected()}>
          <FolderOpen />
          {t("install.choose")}
        </Button>
        <Show when={!connected()}>
          <p class={styles.dropHint}>{t("install.notConnected")}</p>
        </Show>
      </div>

      <Show when={rejected()}>
        <p class={styles.warning} role="status">
          {t("install.rejected")}
        </p>
      </Show>
      <p class={styles.error} role="alert">
        {error()}
      </p>

      <Show when={queue().length > 0}>
        <div class={styles.queueHeader}>
          <h2 class={styles.heading}>{t("install.queue")}</h2>
          <Show when={readyCount() > 1}>
            <Button variant="secondary" onClick={installAll} disabled={busy() || !connected()}>
              {t("install.installAll", { count: String(readyCount()) })}
            </Button>
          </Show>
        </div>
        <ul class={styles.queue}>
          <For each={queue()}>
            {(item) => (
              <li class={styles.item}>
                <span class={styles.art}>
                  <Show when={item.icon} fallback={<Package class={styles.genericIcon} />}>
                    {(url) => <img src={url()} alt="" class={styles.artImage} draggable={false} decoding="async" />}
                  </Show>
                </span>
                <div class={styles.itemText}>
                  <p class={styles.itemName}>{item.preview?.label ?? fileNameOf(item.path)}</p>
                  <p class={styles.itemMeta}>
                    <Show when={item.preview} fallback={<span>{t("install.reading")}</span>}>
                      {(preview) => (
                        <>
                          <span>{preview().versionName ?? preview().package}</span>
                          <span aria-hidden="true">·</span>
                          <span>{formatSize(preview().sizeBytes)}</span>
                          <Show when={preview().split}>
                            <Pill>{t("install.split")}</Pill>
                          </Show>
                        </>
                      )}
                    </Show>
                  </p>
                  <Show when={item.preview}>
                    {(preview) => (
                      <p class={styles.kind} classList={{ [styles.kindWarning]: preview().kind === "downgrade" }}>
                        {kindLabel(preview())}
                      </p>
                    )}
                  </Show>
                  <Show when={item.preview && !item.preview.compatible}>
                    <p class={styles.warning}>
                      <TriangleAlert aria-hidden="true" />
                      {t("install.incompatible", {
                        min: String(item.preview?.minSdk ?? "?"),
                        device: String(item.preview?.deviceSdk ?? "?"),
                      })}
                    </p>
                  </Show>
                  <Show when={item.message}>
                    <p class={styles.error}>{item.message}</p>
                  </Show>
                </div>
                <div class={styles.itemActions}>
                  <Show when={item.status === "inspecting"}>
                    <LoaderCircle class={styles.spin} aria-label={t("install.reading")} />
                  </Show>
                  <Show when={item.status === "ready"}>
                    <Button
                      variant="primary"
                      onClick={() => void install(item)}
                      disabled={!item.preview?.compatible || busy() || !connected()}
                    >
                      {t("install.install")}
                    </Button>
                  </Show>
                  <Show when={item.status === "installing"}>
                    <span class={styles.status} role="status">
                      <LoaderCircle class={styles.spin} />
                      {t("install.installing")}
                    </span>
                  </Show>
                  <Show when={item.status === "installed"}>
                    <span class={`${styles.status} ${styles.success}`} role="status">
                      <CircleCheck />
                      {t("install.installed")}
                    </span>
                    <Show when={item.preview}>
                      {(preview) => (
                        <Button variant="secondary" onClick={() => void openPackage(serial(), preview().package)}>
                          <Play />
                          {t("apps.open")}
                        </Button>
                      )}
                    </Show>
                  </Show>
                  <Show when={item.status === "error" && item.preview}>
                    <Button variant="secondary" onClick={() => void install(item)} disabled={busy() || !connected()}>
                      {t("install.retry")}
                    </Button>
                  </Show>
                  <Show when={item.status !== "installing"}>
                    <Button variant="ghost" onClick={() => remove(item.path)} aria-label={t("install.remove")}>
                      <X />
                    </Button>
                  </Show>
                </div>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </section>
  );
}
