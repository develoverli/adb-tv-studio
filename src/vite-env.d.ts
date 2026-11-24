/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Set to "1" in development to load two sample TVs instead of the real adb device list. */
  readonly VITE_MOCK?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
