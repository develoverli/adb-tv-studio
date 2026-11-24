import { A, Navigate } from "@solidjs/router";
import Plus from "lucide-solid/icons/plus";
import { Show } from "solid-js";
import { buttonClass } from "../components/Button";
import { Logo } from "../components/Logo";
import { Steps } from "../components/Steps";
import { t } from "../i18n";
import { devices } from "../lib/devices";
import page from "./Page.module.css";
import styles from "./Home.module.css";

/** With saved TVs, jump to the first one; otherwise show the first-run guide. */
export function Home() {
  return (
    <Show when={devices[0]} fallback={<Welcome />}>
      {(first) => <Navigate href={`/device/${first().id}`} />}
    </Show>
  );
}

function Welcome() {
  return (
    <div class={page.page}>
      <header class={page.header}>
        <Logo size={44} class={styles.mark} />
        <h1 class={page.title}>{t("welcome.title")}</h1>
        <p class={page.lead}>{t("welcome.lead")}</p>
      </header>

      <Steps
        steps={[
          { title: t("welcome.step1Title"), body: t("welcome.step1Body") },
          { title: t("welcome.step2Title"), body: t("welcome.step2Body") },
          { title: t("welcome.step3Title"), body: t("welcome.step3Body") },
        ]}
      />

      <div>
        <A href="/add" class={buttonClass("primary", "lg")}>
          <Plus />
          {t("welcome.cta")}
        </A>
      </div>
    </div>
  );
}
