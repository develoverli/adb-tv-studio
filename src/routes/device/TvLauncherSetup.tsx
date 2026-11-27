import Check from "lucide-solid/icons/check";
import Download from "lucide-solid/icons/download";
import LoaderCircle from "lucide-solid/icons/loader-circle";
import { For, Show, createResource, createSignal } from "solid-js";
import { Button } from "../../components/Button";
import { confirmAction } from "../../components/ConfirmDialog";
import { t } from "../../i18n";
import { describeError, isAdbError } from "../../lib/adb";
import { installTvLauncher, tvLauncherStatus } from "../../lib/tvLauncher";
import styles from "./TvLauncherSetup.module.css";

interface TvLauncherSetupProps {
  serial: string;
  connected: boolean;
  /** tv-launcher was installed or updated. */
  onChanged: () => void;
}

/**
 * Installs or updates tv-launcher from the APK bundled with the app, in one click. Renders nothing
 * when the TV already has the bundled version (or a newer one).
 */
export function TvLauncherSetup(props: TvLauncherSetupProps) {
  const [status, { refetch }] = createResource(
    () => (props.connected ? props.serial : null),
    (serial) => tvLauncherStatus(serial),
  );
  const [installing, setInstalling] = createSignal(false);
  const [message, setMessage] = createSignal<{ tone: "success" | "error"; text: string } | null>(null);

  const bundledName = () => status()?.bundled?.name ?? "";
  const missing = () => status()?.installed === null;
  const needsAction = () => {
    const current = status();
    return current !== undefined && (current.canInstall || (!current.supportsThemes && current.bundled === null));
  };

  const [expanded, setExpanded] = createSignal(false);
  const benefits = () => [
    t("tvLauncher.benefitAds"),
    t("tvLauncher.benefitPrivacy"),
    t("tvLauncher.benefitOpen"),
    t("tvLauncher.benefitThemes"),
  ];

  const title = () => (missing() ? t("tvLauncher.missingTitle") : t("tvLauncher.outdatedTitle"));

  const body = () => {
    const current = status();
    if (!current) return "";
    if (current.bundled === null) return t("tvLauncher.notBundled");
    if (current.installed === null) return t("tvLauncher.missing");
    return t("tvLauncher.outdated", { version: bundledName() });
  };

  const install = async (replace: boolean) => {
    setInstalling(true);
    setMessage(null);
    try {
      await installTvLauncher(props.serial, replace);
      setMessage({ tone: "success", text: t("tvLauncher.installed") });
      await refetch();
      props.onChanged();
    } catch (failure) {
      if (!replace && isAdbError(failure) && failure.kind === "launcherSignatureMismatch") {
        setInstalling(false);
        const confirmed = await confirmAction({
          title: t("tvLauncher.reinstallTitle"),
          body: t("tvLauncher.reinstallBody"),
          confirmLabel: t("tvLauncher.reinstall"),
          danger: true,
        });
        if (confirmed) await install(true);
        return;
      }
      setMessage({ tone: "error", text: describeError(failure) });
    } finally {
      setInstalling(false);
    }
  };

  return (
    <Show when={props.connected && (needsAction() || message())}>
      <div class={styles.setup} role="region" aria-label="tv-launcher">
        <Show when={needsAction()}>
          <div class={styles.text}>
            <Show when={status()?.canInstall}>
              <h3 class={styles.title}>{title()}</h3>
            </Show>
            <p class={styles.body}>{body()}</p>
            <Show when={status()?.canInstall && missing()}>
              <ul class={styles.benefits}>
                <For each={benefits()}>
                  {(benefit) => (
                    <li>
                      <Check class={styles.check} aria-hidden="true" />
                      {benefit}
                    </li>
                  )}
                </For>
              </ul>
              <button
                type="button"
                class={styles.more}
                aria-expanded={expanded()}
                aria-controls="tv-launcher-details"
                onClick={() => setExpanded(!expanded())}
              >
                {expanded() ? t("tvLauncher.less") : t("tvLauncher.more")}
              </button>
              <Show when={expanded()}>
                <div id="tv-launcher-details" class={styles.details}>
                  <p>{t("tvLauncher.detailWhat")}</p>
                  <p>{t("tvLauncher.detailSafe")}</p>
                  <p>{t("tvLauncher.detailUndo")}</p>
                </div>
              </Show>
            </Show>
          </div>
          <Show when={status()?.canInstall}>
            <Button variant="primary" onClick={() => install(false)} disabled={installing()}>
              <Show when={installing()} fallback={<Download />}>
                <LoaderCircle class={styles.spin} />
              </Show>
              {installing()
                ? t("tvLauncher.installing")
                : missing()
                  ? t("tvLauncher.install")
                  : t("tvLauncher.update", { version: bundledName() })}
            </Button>
          </Show>
        </Show>
        <p class={message()?.tone === "error" ? styles.error : styles.success} role={message()?.tone === "error" ? "alert" : "status"}>
          {message()?.text}
        </p>
      </div>
    </Show>
  );
}
