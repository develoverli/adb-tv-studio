import { isTauri } from "@tauri-apps/api/core";
import { createRoot } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { MOCK_DEVICES } from "../dev/mockDevices";
import { listDevices, onDevicesChanged } from "./adb";

export type DeviceStatus = "connected" | "offline" | "unauthorized";

/** Mirrors `Device` in src-tauri/src/adb/mod.rs. */
export interface Device {
  /** adb serial (`ip:port`), also the route id. */
  id: string;
  name: string;
  /** null until the TV has been connected and authorized once. */
  model: string | null;
  androidVersion: string | null;
  address: string;
  status: DeviceStatus;
}

/**
 * Devices come from the Rust tracker. Set VITE_MOCK=1 while developing to load two sample TVs
 * instead (no adb needed). `import.meta.env.DEV` is `false` in production builds, so the mock never ships.
 */
const useMock = import.meta.env.DEV && import.meta.env.VITE_MOCK === "1";

const { devices, setDevices } = createRoot(() => {
  const [devices, setDevices] = createStore<Device[]>(useMock ? MOCK_DEVICES : []);
  return { devices, setDevices };
});

export { devices };

function applyDevices(next: Device[]): void {
  setDevices(reconcile(next, { key: "id" }));
}

/** Loads the current list and follows `devices-changed`. Call once at startup. */
export async function startDeviceSync(): Promise<void> {
  // Plain `vite` in a browser has no Tauri backend; keep the empty (or mock) list.
  if (useMock || !isTauri()) return;
  // Subscribe first so no change is missed, then load the snapshot. An event that arrives
  // while the snapshot is in flight is newer than it, so the snapshot is then dropped.
  let eventSeen = false;
  await onDevicesChanged((next) => {
    eventSeen = true;
    applyDevices(next);
  });
  const snapshot = await listDevices();
  if (!eventSeen) applyDevices(snapshot);
}

export function findDevice(id: string): Device | undefined {
  return devices.find((device) => device.id === id);
}
