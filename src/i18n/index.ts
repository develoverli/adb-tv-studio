import * as i18n from "@solid-primitives/i18n";
import { createMemo, createRenderEffect, createRoot, createSignal } from "solid-js";
import { STORAGE_KEYS, readChoice, writeStored } from "../lib/storage";
import { en } from "./en";
import { es } from "./es";

export const LOCALES = ["en", "es"] as const;
export type Locale = (typeof LOCALES)[number];

export const LOCALE_PREFERENCES = ["system", ...LOCALES] as const;
export type LocalePreference = (typeof LOCALE_PREFERENCES)[number];

/** Native names, shown untranslated so people can always find their own language. */
export const LOCALE_NAMES: Record<Locale, string> = {
  en: "English",
  es: "Español",
};

const dictionaries = { en, es };

function systemLocale(): Locale {
  const language = navigator.language.toLowerCase();
  return LOCALES.find((locale) => language.startsWith(locale)) ?? "en";
}

const { localePreference, setLocalePreference, locale, t } = createRoot(() => {
  const [localePreference, setPreference] = createSignal<LocalePreference>(
    readChoice(STORAGE_KEYS.locale, LOCALE_PREFERENCES, "system"),
  );

  const locale = (): Locale => {
    const preference = localePreference();
    return preference === "system" ? systemLocale() : preference;
  };

  const flatDictionary = createMemo(() => i18n.flatten(dictionaries[locale()]));
  const t = i18n.translator(flatDictionary, i18n.resolveTemplate);

  createRenderEffect(() => {
    document.documentElement.lang = locale();
  });

  const setLocalePreference = (preference: LocalePreference) => {
    setPreference(preference);
    writeStored(STORAGE_KEYS.locale, preference);
  };

  return { localePreference, setLocalePreference, locale, t };
});

export { localePreference, setLocalePreference, locale, t };
