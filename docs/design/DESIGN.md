# DESIGN

Tokens live in `src/styles/tokens.css` (primitives + semantic layer). Components use semantic tokens only.

## Theme
Dark by default (couch, low light); light follows the OS. User override in Settings: System / Dark / Light.
The resolved theme is written to `<html data-theme>` before first render.

## Color (Restrained)
OKLCH throughout, every neutral tinted toward the accent hue (160). No `#000` / `#fff`.
- Accent "signal green" `#3DDC97` ≈ `oklch(0.80 0.16 160)`. Used only for: connected state, primary action,
  current selection, focus ring. Text on accent is near-black (`--on-accent`), never white.
- Dark surfaces step up in lightness for elevation: canvas 0.155, sidebar 0.185, raised 0.215, hover 0.255.
- Light theme: canvas 0.985, sidebar 0.955 (a second neutral layer), accent text darkened to 0.50 for contrast.
- Semantic: danger (hue 25), warning (hue 80), info (hue 240). Never reuse the accent for warnings.

## Typography
Outfit Variable (OFL, bundled via `@fontsource-variable/outfit`). One family. System monospace for IPs,
serials and package names.
Fixed rem scale, ~1.2 ratio: 12 / 13 / 14 (body) / 17 / 21 / 26 / 32 px. Weights 400, 500, 600.
Small caps labels: 12px, 500, tracking 0.08em.

## Space & shape
4px base: 4, 8, 12, 16, 20, 24, 32, 40, 48, 64. Radii: 6 (controls), 10 (inputs, rows), 14 (panels), 20 (hero).

## Layout
- Custom title bar: no native decorations on Windows/Linux (own min/max/close); macOS keeps native traffic lights
  over a transparent title bar.
- Sidebar (248px, second neutral): brand, saved TVs with status, "Add a TV", Settings pinned to the bottom.
- Main: device header (name, model, Android, address, status) + tabs: Overview, Apps, Install, Themes, Remote.

## Components
- Button: primary (accent fill), secondary (raised surface + border), ghost. Heights 32 / 40.
- Status dot: connected = accent with a slow pulse ring; offline = hollow ring; unauthorized = warning.
- Empty state: icon, title, one sentence, numbered steps when there is a procedure, one primary action.
- "In development" pill marks features that are not built yet.

## Motion
150–200 ms, `cubic-bezier(0.25, 1, 0.5, 1)` (ease-out-quart) for hover, focus, tab change.
Only continuous motion: the connected pulse. `prefers-reduced-motion` turns all of it off.
