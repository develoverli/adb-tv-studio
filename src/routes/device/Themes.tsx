import { useParams } from "@solidjs/router";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import Download from "lucide-solid/icons/download";
import FolderOpen from "lucide-solid/icons/folder-open";
import LoaderCircle from "lucide-solid/icons/loader-circle";
import RotateCcw from "lucide-solid/icons/rotate-ccw";
import Trash from "lucide-solid/icons/trash";
import { For, Show, createResource, createSignal, onCleanup, onMount } from "solid-js";
import { Button } from "../../components/Button";
import { confirmAction } from "../../components/ConfirmDialog";
import { Pill } from "../../components/Pill";
import { ThemePreview } from "../../components/ThemePreview";
import { t } from "../../i18n";
import { describeError, isAdbError } from "../../lib/adb";
import { findDevice } from "../../lib/devices";
import {
  type CatalogTheme,
  CatalogUnpublishedError,
  DEFAULT_THEME,
  type ThemeEntry,
  type ThemeFile,
  applyTheme,
  downloadTheme,
  fetchCatalog,
  fetchCatalogTheme,
  importTheme,
  launcherThemes,
  listThemes,
  pickThemeZips,
  removeTheme,
  themeImage,
} from "../../lib/themes";
import styles from "./Themes.module.css";
import { TvLauncherSetup } from "./TvLauncherSetup";

interface Feedback {
  tone: "success" | "error";
  text: string;
}

/** Adds the validation detail for broken themes; other errors use the shared texts. */
function describeThemeError(failure: unknown): string {
  if (isAdbError(failure) && failure.kind === "invalidTheme" && failure.message) {
    return `${t("errors.invalidTheme")} (${failure.message})`;
  }
  return describeError(failure);
}

export function Themes() {
  const params = useParams<{ id: string }>();
  const device = () => findDevice(params.id);
  const connected = () => device()?.status === "connected";

  const [themes, { refetch: refetchThemes }] = createResource(listThemes);
  const [status, { refetch: refetchStatus }] = createResource(
    () => (connected() ? params.id : null),
    (serial) => launcherThemes(serial),
  );
  const [catalog] = createResource(fetchCatalog);
  const [catalogThemes, setCatalogThemes] = createSignal<Record<string, ThemeFile>>({});
  const [images, setImages] = createSignal<Record<string, string>>({});
  const [busy, setBusy] = createSignal<string | null>(null);
  const [feedback, setFeedback] = createSignal<Feedback | null>(null);
  const [dragging, setDragging] = createSignal(false);

  const installed = () => status()?.installed === true;
  const supportsThemes = () => status()?.supportsThemes === true;
  const activeId = () => status()?.active ?? null;
  const owned = (id: string) => themes()?.find((entry) => entry.theme.id === id);

  // Background images of local themes, as object URLs for their previews.
  const loadImages = async (entries: ThemeEntry[]) => {
    for (const entry of entries) {
      if (entry.theme.background?.type !== "image" || images()[entry.theme.id]) continue;
      try {
        const bytes = await themeImage(entry.theme.id);
        const url = URL.createObjectURL(new Blob([bytes]));
        setImages((current) => ({ ...current, [entry.theme.id]: url }));
      } catch {
        // The preview falls back to the background colour.
      }
    }
  };

  // Catalog previews need each theme.json; fetched one by one so a bad entry only hides itself.
  const loadCatalogThemes = async (entries: CatalogTheme[]) => {
    for (const entry of entries) {
      try {
        const theme = await fetchCatalogTheme(entry);
        setCatalogThemes((current) => ({ ...current, [entry.id]: theme }));
      } catch {
        // Listed without a preview.
      }
    }
  };

  onMount(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "enter" || payload.type === "over") setDragging(true);
      else if (payload.type === "leave") setDragging(false);
      else {
        setDragging(false);
        void importPaths(payload.paths);
      }
    });
    onCleanup(() => {
      void unlisten.then((stop) => stop());
    });
  });

  onCleanup(() => {
    for (const url of Object.values(images())) URL.revokeObjectURL(url);
  });

  const run = async (key: string, action: () => Promise<string>) => {
    setBusy(key);
    setFeedback(null);
    try {
      setFeedback({ tone: "success", text: await action() });
    } catch (failure) {
      setFeedback({ tone: "error", text: describeThemeError(failure) });
    } finally {
      setBusy(null);
    }
  };

  const importPaths = async (paths: string[]) => {
    const zips = paths.filter((path) => path.toLowerCase().endsWith(".zip"));
    if (zips.length === 0) {
      setFeedback({ tone: "error", text: t("themes.onlyZip") });
      return;
    }
    await run("import", async () => {
      const names: string[] = [];
      for (const path of zips) names.push((await importTheme(path)).theme.name);
      const next = await refetchThemes();
      if (next) void loadImages(next);
      return t("themes.imported", { names: names.join(", ") });
    });
  };

  const choose = async () => {
    const paths = await pickThemeZips();
    if (paths.length > 0) await importPaths(paths);
  };

  const apply = (id: string) =>
    run(`apply:${id}`, async () => {
      await applyTheme(params.id, id);
      await refetchStatus();
      return id === DEFAULT_THEME ? t("themes.restored") : t("themes.applied");
    });

  const remove = async (entry: ThemeEntry) => {
    const confirmed = await confirmAction({
      title: t("themes.removeTitle", { name: entry.theme.name }),
      body: t("themes.removeBody"),
      confirmLabel: t("themes.remove"),
      danger: true,
    });
    if (!confirmed) return;
    await run(`remove:${entry.theme.id}`, async () => {
      await removeTheme(entry.theme.id);
      await refetchThemes();
      return t("themes.removed", { name: entry.theme.name });
    });
  };

  const download = (entry: CatalogTheme) =>
    run(`download:${entry.id}`, async () => {
      const saved = await downloadTheme(entry);
      const next = await refetchThemes();
      if (next) void loadImages(next);
      return t("themes.downloaded", { name: saved.theme.name });
    });

  const statusText = () => {
    if (!connected()) return t("themes.notConnected");
    if (status.loading) return t("themes.checking");
    if (status.error) return describeError(status.error);
    if (!installed()) return t("themes.launcherMissing");
    if (!supportsThemes()) return t("themes.launcherOutdated");
    const active = activeId();
    if (active === null) return t("themes.activeUnknown");
    if (active === DEFAULT_THEME) return t("themes.activeDefault");
    return t("themes.active", { name: owned(active)?.theme.name ?? active });
  };

  const canApply = () => connected() && supportsThemes() && busy() === null;
  const spinnerFor = (key: string) => busy() === key;

  const meta = (theme: ThemeFile) =>
    [t(`themes.layouts.${theme.layout}`), theme.author ? t("themes.by", { author: theme.author }) : null]
      .filter((part): part is string => part !== null)
      .join(" · ");

  // Kick off image loading once the local list arrives, and previews once the catalog does.
  const watchThemes = () => {
    const list = themes();
    if (list) void loadImages(list);
    return list ?? [];
  };
  const watchCatalog = () => {
    const list = catalog();
    if (list && Object.keys(catalogThemes()).length === 0) void loadCatalogThemes(list);
    return list ?? [];
  };

  return (
    <section class={styles.themes} classList={{ [styles.dragging]: dragging() }}>
      <TvLauncherSetup serial={params.id} connected={connected()} onChanged={() => void refetchStatus()} />
      <div class={styles.bar}>
        <p class={styles.status} role="status">
          {dragging() ? t("themes.dropNow") : statusText()}
        </p>
        <div class={styles.barActions}>
          <Show when={supportsThemes() && activeId() !== DEFAULT_THEME}>
            <Button variant="ghost" onClick={() => apply(DEFAULT_THEME)} disabled={!canApply()}>
              <Show when={spinnerFor(`apply:${DEFAULT_THEME}`)} fallback={<RotateCcw />}>
                <LoaderCircle class={styles.spin} />
              </Show>
              {t("themes.restoreDefault")}
            </Button>
          </Show>
          <Button variant="secondary" onClick={choose} disabled={busy() !== null}>
            <Show when={spinnerFor("import")} fallback={<FolderOpen />}>
              <LoaderCircle class={styles.spin} />
            </Show>
            {t("themes.import")}
          </Button>
        </div>
      </div>

      <p class={feedback()?.tone === "error" ? styles.error : styles.success} role={feedback()?.tone === "error" ? "alert" : "status"}>
        {feedback()?.text}
      </p>

      <h2 class={styles.heading}>{t("themes.yours")}</h2>
      <ul class={styles.gallery}>
        <For each={watchThemes()}>
          {(entry) => (
            <li class={styles.tile}>
              <ThemePreview theme={entry.theme} image={images()[entry.theme.id]} />
              <div class={styles.tileBody}>
                <div class={styles.tileText}>
                  <span class={styles.name}>{entry.theme.name}</span>
                  <span class={styles.meta}>{meta(entry.theme)}</span>
                </div>
                <Show when={activeId() === entry.theme.id}>
                  <Pill tone="accent">{t("themes.onTv")}</Pill>
                </Show>
              </div>
              <Show when={entry.theme.description}>
                {(description) => <p class={styles.description}>{description()}</p>}
              </Show>
              <div class={styles.tileActions}>
                <Button
                  variant={activeId() === entry.theme.id ? "secondary" : "primary"}
                  onClick={() => apply(entry.theme.id)}
                  disabled={!canApply()}
                >
                  <Show when={spinnerFor(`apply:${entry.theme.id}`)}>
                    <LoaderCircle class={styles.spin} />
                  </Show>
                  {activeId() === entry.theme.id ? t("themes.reapply") : t("themes.apply")}
                </Button>
                <Show when={entry.source === "library"}>
                  <Button
                    variant="ghost"
                    class={styles.dangerButton}
                    onClick={() => remove(entry)}
                    disabled={busy() !== null}
                    aria-label={t("themes.removeTitle", { name: entry.theme.name })}
                  >
                    <Trash />
                    {t("themes.remove")}
                  </Button>
                </Show>
              </div>
            </li>
          )}
        </For>
      </ul>

      <h2 class={styles.heading}>{t("themes.catalog")}</h2>
      <Show
        when={!catalog.loading}
        fallback={<p class={styles.note}>{t("themes.catalogLoading")}</p>}
      >
        <Show
          when={!catalog.error}
          fallback={
            <p class={styles.note}>
              {catalog.error instanceof CatalogUnpublishedError ? t("themes.catalogUnpublished") : t("themes.catalogFailed")}
            </p>
          }
        >
          <Show when={watchCatalog().length > 0} fallback={<p class={styles.note}>{t("themes.catalogEmpty")}</p>}>
            <ul class={styles.gallery}>
              <For each={watchCatalog()}>
                {(entry) => {
                  const local = () => owned(entry.id);
                  const upToDate = () => local()?.theme.version === entry.version;
                  return (
                    <li class={styles.tile}>
                      <Show
                        when={catalogThemes()[entry.id]}
                        fallback={<div class={styles.previewPlaceholder} aria-hidden="true" />}
                      >
                        {(theme) => <ThemePreview theme={theme()} />}
                      </Show>
                      <div class={styles.tileBody}>
                        <div class={styles.tileText}>
                          <span class={styles.name}>{entry.name}</span>
                          <span class={styles.meta}>
                            {[t(`themes.layouts.${entry.layout}`), entry.author ? t("themes.by", { author: entry.author }) : null]
                              .filter((part): part is string => part !== null)
                              .join(" · ")}
                          </span>
                        </div>
                      </div>
                      <Show when={entry.description}>
                        <p class={styles.description}>{entry.description}</p>
                      </Show>
                      <div class={styles.tileActions}>
                        <Show
                          when={!(local() && upToDate())}
                          fallback={<Pill>{t("themes.inLibrary")}</Pill>}
                        >
                          <Button variant="secondary" onClick={() => download(entry)} disabled={busy() !== null}>
                            <Show when={spinnerFor(`download:${entry.id}`)} fallback={<Download />}>
                              <LoaderCircle class={styles.spin} />
                            </Show>
                            {local() ? t("themes.update") : t("themes.download")}
                          </Button>
                        </Show>
                      </div>
                    </li>
                  );
                }}
              </For>
            </ul>
          </Show>
        </Show>
      </Show>
    </section>
  );
}
