import { Channel, invoke } from "@tauri-apps/api/core";
import type { RemoteKey } from "./remote";

/** Mirrors `Quality` in src-tauri/src/adb/scrcpy.rs. */
export type Quality = "smooth" | "sharp" | "max";
export const QUALITIES: readonly Quality[] = ["smooth", "sharp", "max"];

/** Message kinds from `relay` in src-tauri/src/adb/scrcpy.rs (first byte of each message). */
const MESSAGE_SESSION = 0;
const MESSAGE_PACKET = 1;
const MESSAGE_END = 2;
const FLAG_CONFIG = 1;
const FLAG_KEY = 2;
/** Byte offset of the payload in a packet message: kind, flags, 8-byte PTS. */
const PAYLOAD_OFFSET = 10;
/** Frames waiting to decode before deltas are dropped to keep latency low; the next key frame catches up. */
const MAX_DECODE_QUEUE = 3;
/** Used when the stream's SPS cannot be read: High profile, level 4.0. */
const FALLBACK_CODEC = "avc1.640028";

/** True when this webview can decode H.264 with WebCodecs; otherwise the Control tab uses screenshots. */
export async function canPlayLive(): Promise<boolean> {
  if (typeof VideoDecoder !== "function" || typeof EncodedVideoChunk !== "function") return false;
  try {
    const support = await VideoDecoder.isConfigSupported({ codec: "avc1.42E01E" });
    return support.supported === true;
  } catch {
    return false;
  }
}

function hex(byte: number): string {
  return byte.toString(16).padStart(2, "0");
}

/** WebCodecs codec string ("avc1.PPCCLL") from the SPS inside an Annex B config packet. */
function codecFromConfig(config: Uint8Array): string {
  for (let index = 0; index + 4 < config.length; index += 1) {
    const startCode = config[index] === 0 && config[index + 1] === 0 && config[index + 2] === 1;
    const nal = config[index + 3];
    if (startCode && nal !== undefined && (nal & 0x1f) === 7) {
      const profile = config[index + 4];
      const compatibility = config[index + 5];
      const level = config[index + 6];
      if (profile === undefined || compatibility === undefined || level === undefined) break;
      return `avc1.${hex(profile)}${hex(compatibility)}${hex(level)}`;
    }
  }
  return FALLBACK_CODEC;
}

function concat(first: Uint8Array, second: Uint8Array): Uint8Array {
  const joined = new Uint8Array(first.length + second.length);
  joined.set(first, 0);
  joined.set(second, first.length);
  return joined;
}

export interface LiveHandlers {
  /** Frame size (also after the TV rotates or changes resolution). */
  onSize: (width: number, height: number) => void;
  /** First frame drawn. */
  onFirstFrame: () => void;
  /** Stream or decoder failed: the caller falls back to screenshots. */
  onFailure: (reason: string) => void;
}

/**
 * Starts live video on `canvas` and returns a function that stops it.
 * Packets arrive in Annex B form, so the decoder needs no `description`; each key frame is
 * prefixed with the last config packet (SPS/PPS) so decoding can (re)start from it.
 */
export async function startLive(
  serial: string,
  quality: Quality,
  canvas: HTMLCanvasElement,
  handlers: LiveHandlers,
): Promise<() => Promise<void>> {
  const context = canvas.getContext("2d");
  if (!context) throw new Error("canvas 2D context unavailable");

  let decoder: VideoDecoder | null = null;
  let config: Uint8Array | null = null;
  let waitingForKey = true;
  let drawnFirst = false;
  let stopped = false;

  const fail = (reason: string) => {
    if (stopped) return;
    stopped = true;
    handlers.onFailure(reason);
  };

  const configure = (packet: Uint8Array) => {
    if (decoder && decoder.state !== "closed") decoder.close();
    decoder = new VideoDecoder({
      output: (frame) => {
        context.drawImage(frame, 0, 0, canvas.width, canvas.height);
        frame.close();
        if (!drawnFirst) {
          drawnFirst = true;
          handlers.onFirstFrame();
        }
      },
      error: (error) => fail(error.message),
    });
    decoder.configure({ codec: codecFromConfig(packet), optimizeForLatency: true });
    waitingForKey = true;
  };

  const channel = new Channel<ArrayBuffer>();
  channel.onmessage = (data) => {
    if (stopped) return;
    const bytes = new Uint8Array(data);
    const view = new DataView(data);
    const kind = bytes[0];
    if (kind === MESSAGE_SESSION) {
      canvas.width = view.getUint32(1);
      canvas.height = view.getUint32(5);
      handlers.onSize(canvas.width, canvas.height);
    } else if (kind === MESSAGE_PACKET) {
      const flags = bytes[1] ?? 0;
      const payload = bytes.subarray(PAYLOAD_OFFSET);
      if (flags & FLAG_CONFIG) {
        config = payload.slice();
        configure(config);
        return;
      }
      const current: VideoDecoder | null = decoder;
      if (!current || current.state !== "configured") return;
      const key = (flags & FLAG_KEY) !== 0;
      if (!key && (waitingForKey || current.decodeQueueSize > MAX_DECODE_QUEUE)) {
        waitingForKey = true;
        return;
      }
      waitingForKey = false;
      try {
        current.decode(
          new EncodedVideoChunk({
            type: key ? "key" : "delta",
            timestamp: Number(view.getBigUint64(2)),
            data: key && config ? concat(config, payload) : payload,
          }),
        );
      } catch (error) {
        fail(error instanceof Error ? error.message : String(error));
      }
    } else if (kind === MESSAGE_END) {
      fail("stream ended");
    }
  };

  const info = await invoke<{ width: number; height: number }>("scrcpy_start", { serial, quality, frames: channel });
  canvas.width = info.width;
  canvas.height = info.height;
  handlers.onSize(info.width, info.height);

  return async () => {
    stopped = true;
    const current: VideoDecoder | null = decoder;
    if (current && current.state !== "closed") current.close();
    await invoke<void>("scrcpy_stop", { serial });
  };
}

export function liveKey(serial: string, key: RemoteKey): Promise<void> {
  return invoke<void>("scrcpy_key", { serial, key });
}

export function liveText(serial: string, text: string): Promise<void> {
  return invoke<void>("scrcpy_text", { serial, text });
}

export function liveTap(serial: string, x: number, y: number): Promise<void> {
  return invoke<void>("scrcpy_tap", { serial, x, y });
}

/** Same limit as `MAX_TEXT_BYTES` in src-tauri/src/adb/scrcpy_proto.rs (UTF-8 bytes). */
export const MAX_LIVE_TEXT_BYTES = 300;

export function fitsLiveText(text: string): boolean {
  const length = new TextEncoder().encode(text).length;
  return length > 0 && length <= MAX_LIVE_TEXT_BYTES;
}
