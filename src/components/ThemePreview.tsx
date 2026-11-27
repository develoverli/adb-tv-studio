import { For, type JSX, Show } from "solid-js";
import type { ThemeFile } from "../lib/themes";
import styles from "./ThemePreview.module.css";

const HEX = /^#[0-9a-fA-F]{6}$/;
/** Placeholder card colours, standing in for app banners. */
const TILE_COLORS = ["#C2453A", "#2F6FD6", "#2E9E6A", "#D49B2A", "#7A4FD1", "#2A9CB0"];

/** Only well-formed #RRGGBB values reach a style; anything else falls back. */
function color(value: string | undefined, fallback: string): string {
  return value !== undefined && HEX.test(value) ? value : fallback;
}

function withAlpha(hex: string, alpha: number): string {
  const channel = Math.round(alpha * 255)
    .toString(16)
    .padStart(2, "0");
  return `${hex}${channel}`;
}

const FONTS: Record<string, string> = {
  outfit: 'var(--font-sans)',
  sans: 'system-ui, "Segoe UI", Roboto, sans-serif',
  serif: 'Georgia, "Times New Roman", serif',
  mono: "var(--font-mono)",
  condensed: '"Arial Narrow", "Roboto Condensed", sans-serif-condensed, sans-serif',
};

const RADIUS: Record<string, string> = { square: "1px", rounded: "3px", round: "6px" };

interface ThemePreviewProps {
  theme: ThemeFile;
  /** Object URL of the theme's background image, when it has one. */
  image?: string;
}

/**
 * A miniature of the TV home screen drawn from the theme's tokens, so every theme (bundled,
 * imported or from the catalog) gets a preview without screenshots.
 */
export function ThemePreview(props: ThemePreviewProps) {
  const background = () => color(props.theme.colors?.background, "#000000");
  const surface = () => color(props.theme.colors?.surface, "#151517");
  const text = () => color(props.theme.colors?.text, "#FFFFFF");
  const accentValue = () => props.theme.colors?.accent;
  const accent = () => (accentValue() === "auto" ? (TILE_COLORS[0] ?? "#FFFFFF") : color(accentValue(), "#FFFFFF"));
  const ring = () => (accentValue() === "auto" ? text() : accent());
  const radius = () => RADIUS[props.theme.shape ?? "rounded"] ?? "3px";
  const layout = () => props.theme.layout;
  const columns = () => Math.min(6, Math.max(3, props.theme.columns ?? 4));
  const showLabels = () => (props.theme.labels ?? "focus") === "always";

  const surfaceFill = () => {
    switch (props.theme.material) {
      case "glass":
        return withAlpha(text(), 0.12);
      case "clear":
        return "transparent";
      default:
        return surface();
    }
  };

  const pageStyle = (): JSX.CSSProperties => {
    const kind = props.theme.background?.type ?? "photos";
    const stops = props.theme.background?.colors ?? [];
    const base: JSX.CSSProperties = {
      "background-color": background(),
      color: text(),
      "font-family": FONTS[props.theme.font ?? "outfit"],
    };
    if (kind === "gradient") {
      const angle = props.theme.background?.angle ?? 180;
      base["background-image"] =
        `linear-gradient(${angle}deg, ${color(stops[0], background())}, ${color(stops[1], background())})`;
    } else if (kind === "image" && props.image) {
      base["background-image"] = `url("${props.image}")`;
      base["background-size"] = "cover";
      base["background-position"] = "center";
    }
    return base;
  };

  const tileStyle = (index: number): JSX.CSSProperties => {
    const focused = index === 0;
    const focus = props.theme.focus ?? "lift";
    const style: JSX.CSSProperties = {
      "background-color": TILE_COLORS[index % TILE_COLORS.length],
      "border-radius": radius(),
    };
    if (focused) {
      const scale = focus === "outline" ? 1 : focus === "glow" ? 1.06 : 1.12;
      style.transform = `scale(${scale})`;
      if (focus !== "scale") style["box-shadow"] = `0 0 0 1px ${ring()}`;
      if (focus === "lift") style["box-shadow"] = `0 0 0 1px ${ring()}, 0 3px 8px ${withAlpha("#FFFFFF", 0.25)}`;
      if (focus === "glow") style["box-shadow"] = `0 0 0 1px ${ring()}, 0 0 10px ${accent()}`;
    }
    return style;
  };

  const tiles = (count: number, offset = 0) => (
    <For each={Array.from({ length: count }, (_, index) => index + offset)}>
      {(index) => (
        <span class={styles.cell}>
          <span class={styles.tile} style={tileStyle(index)} />
          <Show when={showLabels()}>
            <span class={styles.label} style={{ "background-color": withAlpha(text(), 0.5) }} />
          </Show>
        </span>
      )}
    </For>
  );

  const pills = () => (
    <span class={styles.pills}>
      <For each={[0, 1, 2]}>
        {() => (
          <span
            class={styles.pill}
            style={{
              "background-color": props.theme.material === "solid" ? surface() : surfaceFill(),
              "border-color": withAlpha(text(), 0.2),
              "border-radius": props.theme.shape === "square" ? "1px" : "4px",
            }}
          />
        )}
      </For>
    </span>
  );

  const clock = (big = false) => (
    <Show when={props.theme.clock?.visible ?? true}>
      <span class={big ? styles.bigClock : styles.clock}>9:41</span>
    </Show>
  );

  return (
    <div class={styles.preview} style={pageStyle()} aria-hidden="true">
      <Show when={props.theme.glow && layout() !== "grid" && layout() !== "sidebar"}>
        <span class={styles.glow} style={{ "background-image": `radial-gradient(circle at 32% 55%, ${withAlpha(accent(), 0.45)}, transparent 60%)` }} />
      </Show>

      <Show when={layout() === "sidebar"}>
        <div class={styles.sidebar} style={{ "background-color": surfaceFill() }}>
          {clock()}
          <span class={styles.sidebarPills}>{pills()}</span>
        </div>
        <div class={styles.sideGrid} style={{ "grid-template-columns": `repeat(${columns()}, 1fr)` }}>
          {tiles(columns() * 2)}
        </div>
      </Show>

      <Show when={layout() !== "sidebar"}>
        <div class={styles.header}>
          <Show when={layout() !== "minimal"}>{clock()}</Show>
          {pills()}
        </div>

        <Show when={layout() === "rows"}>
          <span
            class={styles.backdrop}
            style={{ "background-image": `linear-gradient(to top, ${background()}, transparent), linear-gradient(120deg, ${TILE_COLORS[0]}, ${TILE_COLORS[4]})` }}
          />
        </Show>

        <Show when={layout() === "minimal"}>
          <div class={styles.minimalClock}>{clock(true)}</div>
        </Show>

        <Show when={layout() === "classic" || layout() === "rows"}>
          <span class={styles.title} style={{ "background-color": text() }} />
        </Show>

        <div
          class={styles.dock}
          classList={{ [styles.dockLow]: layout() !== "grid" }}
          style={{
            "background-color": layout() === "classic" || layout() === "minimal" ? surfaceFill() : "transparent",
            "border-radius": props.theme.shape === "square" ? "2px" : props.theme.shape === "round" ? "9px" : "6px",
            "grid-template-columns": `repeat(${columns()}, 1fr)`,
          }}
        >
          {tiles(columns())}
        </div>
        <Show when={layout() === "grid"}>
          <div class={styles.grid} style={{ "grid-template-columns": `repeat(${columns()}, 1fr)` }}>
            {tiles(columns(), columns())}
          </div>
        </Show>
      </Show>
    </div>
  );
}
