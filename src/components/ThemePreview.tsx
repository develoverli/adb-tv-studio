import Cable from "lucide-solid/icons/cable";
import Settings from "lucide-solid/icons/settings";
import Wifi from "lucide-solid/icons/wifi";
import { For, type JSX, Show, createSignal, onCleanup, onMount } from "solid-js";
import type { ThemeFile } from "../lib/themes";
import styles from "./ThemePreview.module.css";

/**
 * A faithful replica of tv-launcher's home screen, drawn on its own 960×540 dp canvas and scaled
 * to fit. Positions, sizes and texts come from tv-launcher (res/values/dimens.xml, strings.xml and
 * MainActivity's layout maths), so the preview matches the TV.
 */
const CANVAS_W = 960;
const CANVAS_H = 540;
const MARGIN = 56;
const HEADER_TOP = 26;
const PILLS_TOP = 58;
const PILL_H = 42;
const PILL_GAP = 10;
const DOCK_TOP = 362;
const DOCK_PADDING = 14;
const DOCK_GAP = 33;
const CARD_MARGIN_H = 16;
const CARD_MARGIN_V = 12;
const HERO_TITLE_GAP = 30;
const LABEL_GAP = 8;
const SIDEBAR_W = 300;
const GRID_DOCK_TOP = 150;
const SIDEBAR_DOCK_TOP = 56;
const DOCK_CAPACITY = 4;

/** tv-launcher's own texts (Spanish only, as on the TV). */
const LAUNCHER_TEXT = {
  greetings: ["Buenos días", "Buenas tardes", "Buenas noches"] as const,
  inputs: "Entradas",
  network: "WiFi",
  settings: "Ajustes",
};

const HEX = /^#[0-9a-fA-F]{6}$/;
/** Stand-in cards when the TV's apps are not loaded yet. */
const SAMPLE_APPS: PreviewApp[] = [
  { label: "Películas", color: "#B23A48" },
  { label: "Series", color: "#2F5FD0" },
  { label: "Música", color: "#1E8C5A" },
  { label: "Deportes", color: "#C27C1E" },
  { label: "Noticias", color: "#6A44C2" },
  { label: "Juegos", color: "#1F8EA3" },
  { label: "Fotos", color: "#A2416E" },
  { label: "Radio", color: "#4E7A2F" },
];

export interface PreviewApp {
  label: string;
  /** Object URL of the app's banner or icon. */
  image?: string;
  color?: string;
}

function color(value: string | undefined, fallback: string): string {
  return value !== undefined && HEX.test(value) ? value : fallback;
}

function alpha(hex: string, amount: number): string {
  return `${hex}${Math.round(amount * 255)
    .toString(16)
    .padStart(2, "0")}`;
}

/** Same blend as tv-launcher's Glow: the accent mixed towards black. */
function towardsBlack(hex: string, amount: number): string {
  const channel = (start: number) => Math.round(Number.parseInt(hex.slice(start, start + 2), 16) * (1 - amount));
  return `rgb(${channel(1)}, ${channel(3)}, ${channel(5)})`;
}

const FONTS: Record<string, string> = {
  outfit: "var(--font-sans)",
  sans: 'Roboto, system-ui, "Segoe UI", sans-serif',
  serif: '"Noto Serif", Georgia, "Times New Roman", serif',
  mono: '"Droid Sans Mono", "Roboto Mono", var(--font-mono)',
  condensed: '"Roboto Condensed", "Arial Narrow", sans-serif',
};

interface ThemePreviewProps {
  theme: ThemeFile;
  /** Object URL of the theme's own background image. */
  image?: string;
  /** The TV's launcher apps, in dock order; samples are used when empty. */
  apps?: PreviewApp[];
  /** Extra class for the 16:9 frame (e.g. to size it). */
  class?: string;
}

export function ThemePreview(props: ThemePreviewProps) {
  let frame: HTMLDivElement | undefined;
  const [scale, setScale] = createSignal(0.25);
  const [now, setNow] = createSignal(new Date());

  onMount(() => {
    if (!frame) return;
    const observer = new ResizeObserver(([entry]) => {
      if (entry) setScale(entry.contentRect.width / CANVAS_W);
    });
    observer.observe(frame);
    const timer = window.setInterval(() => setNow(new Date()), 30_000);
    onCleanup(() => {
      observer.disconnect();
      window.clearInterval(timer);
    });
  });

  const t = () => props.theme;
  const layout = () => t().layout;
  const background = () => color(t().colors?.background, "#000000");
  const surface = () => color(t().colors?.surface, "#151517");
  const text = () => color(t().colors?.text, "#FFFFFF");
  const accentAuto = () => (t().colors?.accent ?? "auto") === "auto";
  const apps = () => (props.apps && props.apps.length > 0 ? props.apps : SAMPLE_APPS);
  const focusApp = () => apps()[0];
  /** Glow colour: the accent, or for "auto" the focused app's colour (samples have one). */
  const glowColor = () => (accentAuto() ? color(focusApp()?.color, "#3A6FD8") : color(t().colors?.accent, "#FFFFFF"));
  /** Light pages get dark shadows and a text-coloured ring, as tv-launcher does (Theme.isLight). */
  const isLight = () => {
    const hex = background();
    const linear = (start: number) => {
      const value = Number.parseInt(hex.slice(start, start + 2), 16) / 255;
      return value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * linear(1) + 0.7152 * linear(3) + 0.0722 * linear(5) > 0.5;
  };
  const ringColor = () => {
    if (!accentAuto()) return color(t().colors?.accent, "#FFFFFF");
    return isLight() ? text() : "#F2F2F2";
  };

  const columns = () => Math.min(6, Math.max(3, t().columns ?? 4));
  const dockCount = () => Math.min(DOCK_CAPACITY, columns());
  const startX = () => (layout() === "sidebar" ? SIDEBAR_W : MARGIN);
  const cellW = () => (CANVAS_W - startX() - MARGIN) / columns();
  const cardW = () => cellW() - 2 * CARD_MARGIN_H;
  const cardH = () => (cardW() * 9) / 16;
  const dockTop = () => (layout() === "grid" ? GRID_DOCK_TOP : layout() === "sidebar" ? SIDEBAR_DOCK_TOP : DOCK_TOP);
  const rowH = () => cardH() + 2 * CARD_MARGIN_V;
  const hasHero = () => layout() === "classic" || layout() === "rows" || layout() === "minimal";
  const drawsTray = () => layout() === "classic" || layout() === "minimal";
  const dockLabels = () => !(layout() === "classic" || layout() === "rows");
  const labels = () => t().labels ?? "focus";

  const cardRadius = () => (t().shape === "square" ? 4 : t().shape === "round" ? 24 : 14);
  const dockRadius = () => (t().shape === "square" ? 8 : t().shape === "round" ? 38 : 27);
  const pillRadius = () => (t().shape === "square" ? 8 : PILL_H);

  const fill = () => {
    switch (t().material) {
      case "glass":
        return alpha(text(), 0.12);
      case "clear":
        return "transparent";
      default:
        return surface();
    }
  };
  const stroke = () => alpha(text(), t().material === "clear" ? 0.24 : t().material === "glass" ? 0.16 : 0.08);

  const clockSize = () => (t().clock?.size === "small" ? 40 : t().clock?.size === "large" ? 68 : 52);
  const heroClockSize = () => (t().clock?.size === "small" ? 80 : t().clock?.size === "large" ? 140 : 112);
  const clockVisible = () => t().clock?.visible ?? true;

  const greeting = () => {
    const hour = now().getHours();
    return LAUNCHER_TEXT.greetings[hour >= 5 && hour < 12 ? 0 : hour >= 12 && hour < 19 ? 1 : 2];
  };
  const time = () => {
    const hours = now().getHours();
    return { main: `${hours % 12 || 12}:${String(now().getMinutes()).padStart(2, "0")}`, suffix: hours < 12 ? "AM" : "PM" };
  };
  const date = () => now().toLocaleDateString("es", { weekday: "long", day: "numeric", month: "long" }).replace(",", "");

  const pageStyle = (): JSX.CSSProperties => {
    const kind = t().background?.type ?? "photos";
    const stops = t().background?.colors ?? [];
    const style: JSX.CSSProperties = {
      width: `${CANVAS_W}px`,
      height: `${CANVAS_H}px`,
      transform: `scale(${scale()})`,
      "background-color": background(),
      color: text(),
      "font-family": FONTS[t().font ?? "outfit"],
    };
    if (kind === "gradient") {
      style["background-image"] =
        `linear-gradient(${t().background?.angle ?? 180}deg, ${color(stops[0], background())}, ${color(stops[1], background())})`;
    } else if (kind === "photos") {
      // Stand-in for the user's own photos.
      style["background-image"] = `linear-gradient(160deg, ${alpha("#5A7FA8", 0.55)}, ${alpha("#1B2433", 0.9)})`;
    }
    return style;
  };

  const cardStyle = (app: PreviewApp | undefined, focused: boolean): JSX.CSSProperties => {
    const focus = t().focus ?? "lift";
    const style: JSX.CSSProperties = {
      width: `${cardW()}px`,
      height: `${cardH()}px`,
      "border-radius": `${cardRadius()}px`,
      "background-color": app?.color ?? "#2C2C2E",
    };
    if (app?.image) style["background-image"] = `url("${app.image}")`;
    if (focused) {
      const scaleBy = focus === "scale" ? 1.1 : focus === "glow" ? 1.06 : focus === "outline" ? 1 : 1.12;
      style.transform = `scale(${scaleBy})`;
      const ring = focus === "outline" ? `0 0 0 3px ${ringColor()}` : focus === "scale" ? "" : `0 0 0 2px ${ringColor()}`;
      const glow =
        focus === "lift"
          ? `0 12px 36px ${isLight() ? alpha("#000000", 0.35) : alpha("#FFFFFF", 0.28)}`
          : focus === "glow"
            ? `0 0 34px ${ringColor()}`
            : "";
      style["box-shadow"] = [ring, glow].filter(Boolean).join(", ") || "none";
      style["z-index"] = 2;
    }
    return style;
  };

  const showsLabel = (inDock: boolean, focused: boolean) => {
    if (labels() === "never" || (inDock && !dockLabels())) return false;
    return labels() === "always" || focused;
  };

  const card = (app: PreviewApp | undefined, focused: boolean, inDock: boolean) => (
    <div class={styles.cell} style={{ width: `${cellW()}px`, padding: `${CARD_MARGIN_V}px ${CARD_MARGIN_H}px` }}>
      <div class={styles.card} style={cardStyle(app, focused)}>
        <Show when={!app?.image && app}>{(shown) => <span class={styles.cardText}>{shown().label}</span>}</Show>
      </div>
      <Show when={showsLabel(inDock, focused)}>
        <span
          class={styles.cardLabel}
          style={{
            "margin-top": `${LABEL_GAP}px`,
            color: text(),
            "text-shadow": isLight() ? "none" : "0 1px 4px rgb(0 0 0 / 0.6)",
          }}
        >
          {app?.label}
        </span>
      </Show>
    </div>
  );

  const dockApps = () => apps().slice(0, dockCount());
  const gridApps = () => {
    const rest = apps().slice(dockCount());
    const wanted = columns() * 3;
    // Repeat samples (or the TV's apps) so the grid always looks filled.
    return Array.from({ length: wanted }, (_, index) => rest[index % Math.max(1, rest.length)] ?? apps()[index % apps().length]);
  };
  const heroBottom = () => dockTop() - DOCK_PADDING - HERO_TITLE_GAP;

  const pills = (vertical: boolean) => (
    <div
      class={styles.pills}
      style={
        vertical
          ? { left: `${MARGIN}px`, bottom: `${MARGIN}px`, "flex-direction": "column", "align-items": "flex-start" }
          : { right: `${MARGIN}px`, top: `${PILLS_TOP}px` }
      }
    >
      <For
        each={[
          { icon: <Cable />, label: LAUNCHER_TEXT.inputs },
          { icon: <Wifi />, label: LAUNCHER_TEXT.network },
          { icon: <Settings />, label: LAUNCHER_TEXT.settings },
        ]}
      >
        {(pill) => (
          <span
            class={styles.pill}
            style={{
              height: `${PILL_H}px`,
              "border-radius": `${pillRadius()}px`,
              "background-color": t().material === "solid" || !t().material ? surface() : fill(),
              "border-color": stroke(),
              margin: vertical ? `0 0 ${PILL_GAP}px 0` : `0 0 0 ${PILL_GAP}px`,
            }}
          >
            {pill.icon}
            {pill.label}
          </span>
        )}
      </For>
    </div>
  );

  const headerInfo = (vertical: boolean) => (
    <div class={styles.info} style={{ left: `${MARGIN}px`, top: `${HEADER_TOP}px` }}>
      <span class={styles.greeting} style={{ color: alpha(text(), 0.55) }}>
        {greeting()}
      </span>
      <Show when={clockVisible()}>
        <div class={styles.clockRow} style={{ "flex-direction": vertical ? "column" : "row", "align-items": vertical ? "flex-start" : "center" }}>
          <span class={styles.clock}>
            <span style={{ "font-size": `${clockSize()}px` }}>{time().main}</span>
            <span class={styles.suffix} style={{ color: alpha(text(), 0.7) }}>
              {time().suffix}
            </span>
          </span>
          <Show when={!vertical}>
            <span class={styles.divider} style={{ "background-color": alpha(text(), 0.2) }} />
          </Show>
          <span class={styles.date} style={{ color: alpha(text(), 0.7) }}>
            {date()}
          </span>
        </div>
      </Show>
    </div>
  );

  return (
    <div ref={frame} class={`${styles.frame} ${props.class ?? ""}`} aria-hidden="true">
      <div class={styles.canvas} style={pageStyle()}>
        <Show when={t().background?.type === "image" && props.image}>
          {(url) => (
            <>
              <div
                class={styles.layer}
                style={{
                  "background-image": `url("${url()}")`,
                  filter: (t().background?.blur ?? 0) > 0 ? `blur(${t().background?.blur ?? 0}px)` : "none",
                }}
              />
              <div class={styles.layer} style={{ "background-color": alpha(background(), t().background?.veil ?? 0.55) }} />
            </>
          )}
        </Show>

        <Show when={t().glow && hasHero()}>
          <div
            class={styles.layer}
            style={{
              "background-image": `radial-gradient(circle at 32% 42%, ${towardsBlack(glowColor(), 0.6)}, ${towardsBlack(glowColor(), 0.88)} 45%, transparent 80%)`,
            }}
          />
        </Show>

        <Show when={layout() === "rows" && focusApp()}>
          {(app) => (
            <div class={styles.backdrop} style={{ height: `${heroBottom() + HERO_TITLE_GAP}px` }}>
              <div
                class={styles.backdropImage}
                style={{
                  "background-image": app().image ? `url("${app().image}")` : "none",
                  "background-color": app().color ?? "#333",
                }}
              />
              <div
                class={styles.layer}
                style={{ "background-image": `linear-gradient(to top, ${background()}, ${alpha(background(), 0.4)}, transparent)` }}
              />
            </div>
          )}
        </Show>

        <Show when={layout() === "classic" || layout() === "rows"}>
          <span class={styles.heroTitle} style={{ left: `${MARGIN}px`, top: `${heroBottom() - 54}px` }}>
            {focusApp()?.label}
          </span>
        </Show>

        <Show when={layout() === "minimal" && clockVisible()}>
          <div class={styles.heroClock} style={{ height: `${heroBottom()}px` }}>
            <span class={styles.clock}>
              <span style={{ "font-size": `${heroClockSize()}px` }}>{time().main}</span>
              <span class={styles.suffix} style={{ color: alpha(text(), 0.7), "font-size": "22px" }}>
                {time().suffix}
              </span>
            </span>
            <span class={styles.date} style={{ color: alpha(text(), 0.7), "font-size": "18px" }}>
              {date()}
            </span>
          </div>
        </Show>

        <Show when={layout() === "sidebar"}>
          <div class={styles.sidebar} style={{ width: `${SIDEBAR_W}px`, "background-color": fill() }} />
          {headerInfo(true)}
          {pills(true)}
        </Show>
        <Show when={layout() !== "sidebar"}>
          <Show when={layout() !== "minimal"}>{headerInfo(false)}</Show>
          {pills(false)}
        </Show>

        <Show when={drawsTray()}>
          <div
            class={styles.tray}
            style={{
              left: `${startX() + CARD_MARGIN_H - DOCK_PADDING}px`,
              top: `${dockTop() - DOCK_PADDING}px`,
              width: `${dockCount() * cellW() - 2 * CARD_MARGIN_H + 2 * DOCK_PADDING}px`,
              height: `${cardH() + 2 * DOCK_PADDING}px`,
              "border-radius": `${dockRadius()}px`,
              "background-color": fill(),
              "border-color": stroke(),
            }}
          />
        </Show>

        <div class={styles.rows} style={{ left: `${startX()}px`, top: `${dockTop() - CARD_MARGIN_V}px` }}>
          <div class={styles.row} style={{ "margin-bottom": `${DOCK_GAP}px` }}>
            <For each={dockApps()}>{(app, index) => card(app, index() === 0, true)}</For>
          </div>
          <For each={[0, 1, 2]}>
            {(row) => (
              <div class={styles.row} style={{ height: `${rowH()}px` }}>
                <For each={gridApps().slice(row * columns(), (row + 1) * columns())}>
                  {(app) => card(app, false, false)}
                </For>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
}
