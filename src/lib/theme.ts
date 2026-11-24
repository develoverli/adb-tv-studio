import { createRenderEffect, createRoot, createSignal } from "solid-js";
import { STORAGE_KEYS, readChoice, writeStored } from "./storage";

export const THEME_PREFERENCES = ["system", "dark", "light"] as const;
export type ThemePreference = (typeof THEME_PREFERENCES)[number];
export type ResolvedTheme = "dark" | "light";

const LIGHT_QUERY = "(prefers-color-scheme: light)";

const { themePreference, setThemePreference, resolvedTheme } = createRoot(() => {
  const [themePreference, setPreference] = createSignal<ThemePreference>(
    readChoice(STORAGE_KEYS.theme, THEME_PREFERENCES, "system"),
  );

  const media = window.matchMedia(LIGHT_QUERY);
  const [systemTheme, setSystemTheme] = createSignal<ResolvedTheme>(media.matches ? "light" : "dark");
  media.addEventListener("change", (event) => setSystemTheme(event.matches ? "light" : "dark"));

  const resolvedTheme = (): ResolvedTheme => {
    const preference = themePreference();
    return preference === "system" ? systemTheme() : preference;
  };

  createRenderEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme();
  });

  const setThemePreference = (preference: ThemePreference) => {
    setPreference(preference);
    writeStored(STORAGE_KEYS.theme, preference);
  };

  return { themePreference, setThemePreference, resolvedTheme };
});

export { themePreference, setThemePreference, resolvedTheme };
