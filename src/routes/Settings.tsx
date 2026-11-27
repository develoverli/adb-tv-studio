import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import ArrowUpRight from "lucide-solid/icons/arrow-up-right";
import LoaderCircle from "lucide-solid/icons/loader-circle";
import RefreshCw from "lucide-solid/icons/refresh-cw";
import { createResource, For, type JSX, Show } from "solid-js";
import { Button } from "../components/Button";
import { Logo } from "../components/Logo";
import { Segmented } from "../components/Segmented";
import { LOCALE_NAMES, LOCALES, type LocalePreference, localePreference, setLocalePreference, t } from "../i18n";
import { LINKS } from "../lib/links";
import { setThemePreference, type ThemePreference, themePreference } from "../lib/theme";
import {
  availableUpdate,
  checkForUpdates,
  installUpdate,
  updateProgress,
  updateStatus,
} from "../lib/updates";
import page from "./Page.module.css";
import styles from "./Settings.module.css";

export function Settings() {
  const [version] = createResource(getVersion);

  const themeOptions = () => [
    { value: "system" as ThemePreference, label: t("settings.themeSystem") },
    { value: "dark" as ThemePreference, label: t("settings.themeDark") },
    { value: "light" as ThemePreference, label: t("settings.themeLight") },
  ];

  const localeOptions = () => [
    { value: "system" as LocalePreference, label: t("settings.languageSystem") },
    ...LOCALES.map((locale) => ({ value: locale as LocalePreference, label: LOCALE_NAMES[locale] })),
  ];

  return (
    <div class={page.page}>
      <header class={page.header}>
        <h1 class={page.title}>{t("settings.title")}</h1>
      </header>

      <section class={styles.group}>
        <h2 class={page.sectionTitle}>{t("settings.appearance")}</h2>
        <Row label={t("settings.theme")}>
          <Segmented
            name="theme"
            label={t("settings.theme")}
            value={themePreference()}
            options={themeOptions()}
            onChange={setThemePreference}
          />
        </Row>
        <Row label={t("settings.language")}>
          <Segmented
            name="locale"
            label={t("settings.language")}
            value={localePreference()}
            options={localeOptions()}
            onChange={setLocalePreference}
          />
        </Row>
      </section>

      <section class={styles.group}>
        <h2 class={page.sectionTitle}>{t("settings.about")}</h2>
        <div class={styles.about}>
          <Logo size={32} class={styles.logo} />
          <div>
            <p class={styles.appName}>{t("app.name")}</p>
            <Show when={version()}>
              {(current) => <p class={styles.version}>{t("settings.version", { version: current() })}</p>}
            </Show>
          </div>
        </div>
        <Updates />
        <ul class={styles.links}>
          <For
            each={[
              { label: t("settings.sourceCode"), url: LINKS.repository },
              { label: t("settings.reportIssue"), url: LINKS.issues },
              { label: t("settings.license"), url: LINKS.license },
            ]}
          >
            {(link) => (
              <li>
                <button type="button" class={styles.link} onClick={() => openUrl(link.url)}>
                  {link.label}
                  <ArrowUpRight />
                </button>
              </li>
            )}
          </For>
        </ul>
      </section>
    </div>
  );
}

/** Version check and install. The check also runs quietly at launch (see AppShell). */
function Updates() {
  const busy = () => updateStatus() === "checking" || updateStatus() === "downloading";
  const percent = () => {
    const progress = updateProgress();
    return progress?.total ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100)) : null;
  };

  const statusText = () => {
    switch (updateStatus()) {
      case "checking":
        return t("updates.checking");
      case "upToDate":
        return t("updates.upToDate");
      case "downloading": {
        const value = percent();
        return value === null ? t("updates.downloading") : t("updates.downloadingPercent", { percent: value });
      }
      case "error":
        return availableUpdate() ? t("updates.installFailed") : t("updates.checkFailed");
      default:
        return "";
    }
  };

  return (
    <div class={styles.updates}>
      <div class={styles.updatesBar}>
        <p class={styles.updatesStatus} role="status">
          {availableUpdate() && updateStatus() !== "downloading"
            ? t("updates.available", { version: availableUpdate()?.version ?? "" })
            : statusText()}
        </p>
        <Show
          when={availableUpdate()}
          fallback={
            <Button variant="secondary" onClick={() => checkForUpdates()} disabled={busy()}>
              <Show when={updateStatus() === "checking"} fallback={<RefreshCw />}>
                <LoaderCircle class={styles.spin} />
              </Show>
              {t("updates.check")}
            </Button>
          }
        >
          <Button variant="primary" onClick={() => installUpdate()} disabled={busy()}>
            <Show when={updateStatus() === "downloading"}>
              <LoaderCircle class={styles.spin} />
            </Show>
            {t("updates.install")}
          </Button>
        </Show>
      </div>
      <Show when={updateStatus() === "downloading"}>
        <progress class={styles.progress} max={100} value={percent() ?? undefined} aria-label={t("updates.downloading")} />
      </Show>
      <Show when={availableUpdate()?.notes}>
        {(notes) => (
          <div class={styles.notes}>
            <p class={styles.notesTitle}>{t("updates.notes")}</p>
            <p class={styles.notesBody}>{notes()}</p>
          </div>
        )}
      </Show>
    </div>
  );
}

function Row(props: { label: string; children: JSX.Element }) {
  return (
    <div class={styles.row}>
      <span class={styles.rowLabel}>{props.label}</span>
      {props.children}
    </div>
  );
}
