import { invoke } from "@tauri-apps/api/core";

/** Official catalog: the tv-launcher-themes repo on GitHub (raw files). */
export const CATALOG_BASE = "https://raw.githubusercontent.com/develoverli/tv-launcher-themes/main";
/** Id of tv-launcher's built-in look. */
export const DEFAULT_THEME = "default";

export type ThemeLayout = "classic" | "rows" | "grid" | "minimal" | "sidebar";
export type ThemeMaterial = "solid" | "glass" | "clear";
export type ThemeShape = "square" | "rounded" | "round";
export type ThemeFocus = "scale" | "lift" | "glow" | "outline";
export type ThemeFont = "outfit" | "sans" | "serif" | "mono" | "condensed";
export type ThemeBackgroundType = "solid" | "gradient" | "image" | "photos";

/** Mirrors `ThemeFile` in src-tauri/src/adb/themes_parse.rs (the theme.json contract). */
export interface ThemeFile {
  schemaVersion: number;
  id: string;
  name: string;
  author?: string;
  version?: string;
  description?: string;
  layout: ThemeLayout;
  colors?: { background?: string; surface?: string; text?: string; accent?: string };
  background?: {
    type: ThemeBackgroundType;
    colors?: string[];
    angle?: number;
    image?: string;
    blur?: number;
    veil?: number;
  };
  material?: ThemeMaterial;
  shape?: ThemeShape;
  focus?: ThemeFocus;
  font?: ThemeFont;
  motion?: "none" | "smooth" | "springy";
  labels?: "focus" | "always" | "never";
  clock?: { visible?: boolean; size?: "small" | "normal" | "large" };
  columns?: number;
  glow?: boolean;
}

/** Mirrors `ThemeEntry` in src-tauri/src/adb/themes.rs. */
export interface ThemeEntry {
  source: "bundled" | "library";
  theme: ThemeFile;
}

/** Mirrors `LauncherThemes` in src-tauri/src/adb/themes.rs. */
export interface LauncherThemes {
  installed: boolean;
  /** The installed tv-launcher is new enough to read themes. */
  supportsThemes: boolean;
  active: string | null;
}

/** One theme listed in the catalog's index.json. */
export interface CatalogTheme {
  id: string;
  name: string;
  author: string;
  version: string;
  description: string;
  layout: ThemeLayout;
  path: string;
  files: string[];
}

export function listThemes(): Promise<ThemeEntry[]> {
  return invoke<ThemeEntry[]>("themes_list");
}

export function pickThemeZips(): Promise<string[]> {
  return invoke<string[]>("themes_pick");
}

export function importTheme(path: string): Promise<ThemeEntry> {
  return invoke<ThemeEntry>("themes_import", { path });
}

export function removeTheme(id: string): Promise<void> {
  return invoke<void>("themes_remove", { id });
}

export function themeImage(id: string): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>("themes_image", { id });
}

export function launcherThemes(serial: string): Promise<LauncherThemes> {
  return invoke<LauncherThemes>("themes_status", { serial });
}

export function applyTheme(serial: string, id: string): Promise<void> {
  return invoke<void>("themes_apply", { serial, id });
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

const LAYOUTS: readonly ThemeLayout[] = ["classic", "rows", "grid", "minimal", "sidebar"];

/** Same rule as `is_theme_id` in themes_parse.rs. */
const THEME_ID = /^[a-z0-9][a-z0-9-]{0,39}$/;
/** theme.json or an image name, as `is_asset_name` accepts. */
const THEME_FILE_NAME = /^(theme\.json|[a-z0-9_-]{1,40}\.(png|jpg|jpeg|webp))$/;
/** Relative folder inside the catalog repo: no "..", no leading slash. */
const CATALOG_PATH = /^[a-z0-9-]+(\/[a-z0-9-]+)*$/;

function toCatalogTheme(value: unknown): CatalogTheme | null {
  if (!isRecord(value)) return null;
  const { id, name, author, version, description, layout, path, files } = value;
  const layoutValue = LAYOUTS.find((known) => known === layout);
  const fileList = Array.isArray(files) ? files.filter(isString) : [];
  const valid =
    isString(id) &&
    THEME_ID.test(id) &&
    isString(name) &&
    isString(path) &&
    CATALOG_PATH.test(path) &&
    layoutValue !== undefined &&
    fileList.includes("theme.json") &&
    fileList.every((file) => THEME_FILE_NAME.test(file));
  if (!valid) return null;
  return {
    id,
    name,
    author: isString(author) ? author : "",
    version: isString(version) ? version : "",
    description: isString(description) ? description : "",
    layout: layoutValue,
    path,
    files: fileList,
  };
}

/** The catalog repo answered 404: it is not published yet (or moved). */
export class CatalogUnpublishedError extends Error {}

/** Reads the catalog index. Entries that do not look right are skipped, not fatal. */
export async function fetchCatalog(): Promise<CatalogTheme[]> {
  const response = await fetch(`${CATALOG_BASE}/index.json`, { cache: "no-cache" });
  if (response.status === 404) throw new CatalogUnpublishedError("catalog: not published");
  if (!response.ok) throw new Error(`catalog: HTTP ${response.status}`);
  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.themes)) throw new Error("catalog: unexpected index");
  return body.themes.map(toCatalogTheme).filter((theme): theme is CatalogTheme => theme !== null);
}

/** theme.json of a catalog theme, for its preview. Validated again by Rust when downloaded. */
export async function fetchCatalogTheme(entry: CatalogTheme): Promise<ThemeFile> {
  const response = await fetch(`${CATALOG_BASE}/${entry.path}/theme.json`, { cache: "no-cache" });
  if (!response.ok) throw new Error(`theme: HTTP ${response.status}`);
  const body: unknown = await response.json();
  if (!isRecord(body) || body.id !== entry.id || !isString(body.name)) throw new Error("theme: unexpected file");
  // Shape is checked by `themes_save` before anything is stored; here it only feeds a preview.
  return body as unknown as ThemeFile;
}

/** Downloads every file of a catalog theme and stores it in the library (Rust validates it). */
export async function downloadTheme(entry: CatalogTheme): Promise<ThemeEntry> {
  const files = await Promise.all(
    entry.files.map(async (name) => {
      const response = await fetch(`${CATALOG_BASE}/${entry.path}/${name}`, { cache: "no-cache" });
      if (!response.ok) throw new Error(`${name}: HTTP ${response.status}`);
      return { name, data: Array.from(new Uint8Array(await response.arrayBuffer())) };
    }),
  );
  return invoke<ThemeEntry>("themes_save", { files });
}
