import type { Device } from "../lib/devices";

/** Development-only sample data, loaded with VITE_MOCK=1. Never shipped: see `useMock` in src/lib/devices.ts. */
export const MOCK_DEVICES: Device[] = [
  {
    id: "192.168.1.50:5555",
    name: "Living room",
    model: "Chromecast with Google TV",
    androidVersion: "14",
    address: "192.168.1.50:5555",
    status: "connected",
  },
  {
    id: "192.168.1.73:5555",
    name: "Bedroom",
    model: "Xiaomi TV Box S",
    androidVersion: "12",
    address: "192.168.1.73:5555",
    status: "offline",
  },
];
