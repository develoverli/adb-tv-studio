import { useParams } from "@solidjs/router";
import ChevronDown from "lucide-solid/icons/chevron-down";
import ChevronLeft from "lucide-solid/icons/chevron-left";
import ChevronRight from "lucide-solid/icons/chevron-right";
import ChevronUp from "lucide-solid/icons/chevron-up";
import Download from "lucide-solid/icons/download";
import House from "lucide-solid/icons/house";
import Layers from "lucide-solid/icons/layers";
import Menu from "lucide-solid/icons/menu";
import Monitor from "lucide-solid/icons/monitor";
import Pause from "lucide-solid/icons/pause";
import Play from "lucide-solid/icons/play";
import Power from "lucide-solid/icons/power";
import Send from "lucide-solid/icons/send";
import Undo2 from "lucide-solid/icons/undo-2";
import Volume1 from "lucide-solid/icons/volume-1";
import Volume2 from "lucide-solid/icons/volume-2";
import VolumeX from "lucide-solid/icons/volume-x";
import { For, type JSX, Show, createEffect, createRoot, createSignal, on, onCleanup, onMount } from "solid-js";
import { Button } from "../../components/Button";
import { t } from "../../i18n";
import { describeError } from "../../lib/adb";
import { findDevice } from "../../lib/devices";
import {
  MAX_TEXT_LENGTH,
  type RemoteKey,
  captureScreen,
  closeRemote,
  isTypeable,
  openRemote,
  pressKey,
  saveCapture,
  tap,
  typeText,
} from "../../lib/remote";
import styles from "./Remote.module.css";

/** Pause between a finished capture and the next request, so the UI thread can breathe. */
const CAPTURE_GAP_MS = 60;
const TEXT_INPUT_ID = "remote-text";
const TEXT_MESSAGE_ID = "remote-text-message";

/** Keyboard shortcuts while the Control tab is open (and focus is not in a text field). */
const SHORTCUTS: Record<string, RemoteKey> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Enter: "ok",
  Escape: "back",
  Backspace: "back",
  h: "home",
  H: "home",
};

const SHORTCUT_HELP: { keys: string; action: () => string }[] = [
  { keys: "← ↑ → ↓", action: () => t("remote.keys.arrows") },
  { keys: "Enter", action: () => t("remote.keys.ok") },
  { keys: "Esc / ⌫", action: () => t("remote.keys.back") },
  { keys: "H", action: () => t("remote.keys.home") },
];

function isTextField(target: EventTarget | null): boolean {
  return target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
}

export function Remote() {
  const params = useParams<{ id: string }>();
  const serial = () => params.id;
  const connected = () => findDevice(serial())?.status === "connected";

  const [error, setError] = createSignal<string | null>(null);

  const send = async (key: RemoteKey) => {
    try {
      await pressKey(serial(), key);
      setError(null);
    } catch (failure) {
      setError(describeError(failure));
    }
  };

  // One shell session per TV while this tab is open; switching TVs moves it.
  createEffect(
    on(serial, (current, previous) => {
      if (previous) void closeRemote(previous).catch(() => undefined);
      openRemote(current).catch((failure: unknown) => setError(describeError(failure)));
    }),
  );
  onCleanup(() => {
    void closeRemote(serial()).catch(() => undefined);
  });

  onMount(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.ctrlKey || event.metaKey || event.altKey || isTextField(event.target)) return;
      const key = SHORTCUTS[event.key];
      if (!key) return;
      event.preventDefault();
      void send(key);
    };
    window.addEventListener("keydown", onKeyDown);
    onCleanup(() => window.removeEventListener("keydown", onKeyDown));
  });

  return (
    <section class={styles.remote}>
      <Screen serial={serial()} connected={connected()} />

      <div class={styles.controls}>
        <div class={styles.group} role="group" aria-label={t("remote.navigation")}>
          <div class={styles.dpad}>
            <KeyButton class={styles.up} label={t("remote.up")} onPress={() => send("up")}>
              <ChevronUp />
            </KeyButton>
            <KeyButton class={styles.left} label={t("remote.left")} onPress={() => send("left")}>
              <ChevronLeft />
            </KeyButton>
            <button type="button" class={`${styles.key} ${styles.ok}`} onClick={() => send("ok")}>
              OK
            </button>
            <KeyButton class={styles.right} label={t("remote.right")} onPress={() => send("right")}>
              <ChevronRight />
            </KeyButton>
            <KeyButton class={styles.down} label={t("remote.down")} onPress={() => send("down")}>
              <ChevronDown />
            </KeyButton>
          </div>
          <div class={styles.row}>
            <KeyButton label={t("remote.back")} onPress={() => send("back")}>
              <Undo2 />
            </KeyButton>
            <KeyButton label={t("remote.home")} onPress={() => send("home")}>
              <House />
            </KeyButton>
            <KeyButton label={t("remote.menu")} onPress={() => send("menu")}>
              <Menu />
            </KeyButton>
            <KeyButton label={t("remote.recents")} onPress={() => send("recents")}>
              <Layers />
            </KeyButton>
          </div>
        </div>

        <div class={styles.group} role="group" aria-label={t("remote.media")}>
          <div class={styles.row}>
            <KeyButton label={t("remote.playPause")} onPress={() => send("playPause")}>
              <Play />
            </KeyButton>
            <KeyButton label={t("remote.volumeDown")} onPress={() => send("volumeDown")}>
              <Volume1 />
            </KeyButton>
            <KeyButton label={t("remote.volumeUp")} onPress={() => send("volumeUp")}>
              <Volume2 />
            </KeyButton>
            <KeyButton label={t("remote.mute")} onPress={() => send("mute")}>
              <VolumeX />
            </KeyButton>
          </div>
        </div>

        <TextForm serial={serial()} />

        <div class={styles.power}>
          <Button variant="secondary" onClick={() => send("power")}>
            <Power />
            {t("remote.power")}
          </Button>
          <p class={styles.hint}>{t("remote.powerHint")}</p>
        </div>

        <p class={styles.error} role="alert">
          {error()}
        </p>

        <div class={styles.shortcuts}>
          <h3 class={styles.heading}>{t("remote.shortcuts")}</h3>
          <dl class={styles.shortcutList}>
            <For each={SHORTCUT_HELP}>
              {(item) => (
                <div class={styles.shortcut}>
                  <dt>
                    <kbd>{item.keys}</kbd>
                  </dt>
                  <dd>{item.action()}</dd>
                </div>
              )}
            </For>
          </dl>
        </div>
      </div>
    </section>
  );
}

interface KeyButtonProps {
  label: string;
  onPress: () => void;
  class?: string;
  children: JSX.Element;
}

/** Icon-only key: the label is its accessible name and its tooltip. */
function KeyButton(props: KeyButtonProps) {
  return (
    <button
      type="button"
      class={`${styles.key} ${props.class ?? ""}`}
      aria-label={props.label}
      title={props.label}
      onClick={() => props.onPress()}
    >
      {props.children}
    </button>
  );
}

/** Live view: requests the next capture as soon as the previous one arrives, while visible. */
function Screen(props: { serial: string; connected: boolean }) {
  const [playing, setPlaying] = createSignal(true);
  const [pageVisible, setPageVisible] = createSignal(!document.hidden);
  const [frame, setFrame] = createSignal<string | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [saved, setSaved] = createSignal(false);
  let image: HTMLImageElement | undefined;
  let inFlight = false;
  let disposed = false;

  const shouldRun = () => playing() && pageVisible() && props.connected;

  const replaceFrame = (url: string | null) => {
    const old = frame();
    if (old) URL.revokeObjectURL(old);
    setFrame(url);
  };

  const tick = async () => {
    if (inFlight || disposed || !shouldRun()) return;
    inFlight = true;
    const serial = props.serial;
    try {
      const bytes = await captureScreen(serial);
      // Ignore a late frame from a TV we already switched away from.
      if (!disposed && serial === props.serial) {
        replaceFrame(URL.createObjectURL(new Blob([bytes], { type: "image/png" })));
        setError(null);
      }
    } catch (failure) {
      setError(describeError(failure));
      setPlaying(false);
    } finally {
      inFlight = false;
    }
    if (shouldRun()) window.setTimeout(() => void tick(), CAPTURE_GAP_MS);
  };

  createEffect(() => {
    if (shouldRun()) void tick();
  });

  // A new TV starts with a clean view.
  createEffect(
    on(
      () => props.serial,
      () => {
        replaceFrame(null);
        setSaved(false);
        setPlaying(true);
      },
      { defer: true },
    ),
  );

  onMount(() => {
    const onVisibility = () => setPageVisible(!document.hidden);
    document.addEventListener("visibilitychange", onVisibility);
    onCleanup(() => document.removeEventListener("visibilitychange", onVisibility));
  });

  onCleanup(() => {
    disposed = true;
    replaceFrame(null);
  });

  const onImageClick = (event: MouseEvent) => {
    if (!image || image.naturalWidth === 0) return;
    // object-fit: contain letterboxes the frame; map the click into the drawn picture only.
    const bounds = image.getBoundingClientRect();
    const scale = Math.min(bounds.width / image.naturalWidth, bounds.height / image.naturalHeight);
    const offsetX = (bounds.width - image.naturalWidth * scale) / 2;
    const offsetY = (bounds.height - image.naturalHeight * scale) / 2;
    const x = Math.round((event.clientX - bounds.left - offsetX) / scale);
    const y = Math.round((event.clientY - bounds.top - offsetY) / scale);
    if (x < 0 || y < 0 || x >= image.naturalWidth || y >= image.naturalHeight) return;
    tap(props.serial, x, y).catch((failure: unknown) => setError(describeError(failure)));
  };

  const save = async () => {
    setSaved(false);
    try {
      setSaved(await saveCapture(props.serial));
      setError(null);
    } catch (failure) {
      setError(describeError(failure));
    }
  };

  return (
    <div class={styles.screen}>
      <div class={styles.viewport} aria-busy={shouldRun() && !frame()}>
        <Show
          when={frame()}
          fallback={
            <div class={styles.placeholder}>
              <Monitor />
              <p>{props.connected ? t("remote.loadingScreen") : t("remote.notConnected")}</p>
            </div>
          }
        >
          {(url) => (
            // Taps are a pointer-only extra: the D-pad and keyboard cover every action.
            <img
              ref={image}
              class={styles.frame}
              src={url()}
              alt={t("remote.screenAlt")}
              title={t("remote.tapHint")}
              onClick={onImageClick}
              draggable={false}
            />
          )}
        </Show>
      </div>
      <div class={styles.screenBar}>
        <Button variant="ghost" onClick={() => setPlaying(!playing())} disabled={!props.connected}>
          <Show when={playing()} fallback={<Play />}>
            <Pause />
          </Show>
          {playing() ? t("remote.pause") : t("remote.resume")}
        </Button>
        <Button variant="ghost" onClick={save} disabled={!frame()}>
          <Download />
          {t("remote.save")}
        </Button>
        <p class={styles.screenStatus} role="status">
          {error() ?? (saved() ? t("remote.saved") : "")}
        </p>
      </div>
    </div>
  );
}

/** Unsent text per TV, kept when leaving the Control tab. */
const drafts = createRoot(() => {
  const [all, setAll] = createSignal<Record<string, string>>({});
  return { all, setAll };
});

function TextForm(props: { serial: string }) {
  const text = () => drafts.all()[props.serial] ?? "";
  const setText = (value: string) => drafts.setAll((current) => ({ ...current, [props.serial]: value }));
  const [pending, setPending] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const valid = () => text() === "" || isTypeable(text());

  const submit = async (event: SubmitEvent) => {
    event.preventDefault();
    if (!isTypeable(text())) return;
    setPending(true);
    try {
      await typeText(props.serial, text());
      setText("");
      setError(null);
    } catch (failure) {
      setError(describeError(failure));
    } finally {
      setPending(false);
    }
  };

  return (
    <form class={styles.textForm} onSubmit={submit} novalidate>
      <label class={styles.label} for={TEXT_INPUT_ID}>
        {t("remote.typeLabel")}
      </label>
      <div class={styles.textRow}>
        <input
          id={TEXT_INPUT_ID}
          class={styles.input}
          autocomplete="off"
          spellcheck={false}
          maxlength={MAX_TEXT_LENGTH}
          placeholder={t("remote.typePlaceholder")}
          value={text()}
          onInput={(event) => setText(event.currentTarget.value)}
          aria-invalid={!valid()}
          aria-describedby={TEXT_MESSAGE_ID}
          disabled={pending()}
        />
        <Button type="submit" disabled={pending() || text() === "" || !valid()} aria-label={t("remote.send")}>
          <Send />
        </Button>
      </div>
      <p id={TEXT_MESSAGE_ID} class={valid() && !error() ? styles.hint : styles.error}>
        {error() ?? (valid() ? t("remote.typeHint") : t("errors.invalidText"))}
      </p>
    </form>
  );
}
