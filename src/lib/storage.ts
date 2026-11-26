/** Keys for UI preferences kept in the webview's localStorage. */
export const STORAGE_KEYS = {
  theme: "adbtv.theme",
  locale: "adbtv.locale",
  remoteQuality: "adbtv.remoteQuality",
} as const;

/** Reads a value; storage can throw (blocked / private mode), so failures fall back to null. */
export function readStored(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

/** Writes a value; a failure only loses the preference for next launch, so it is ignored. */
export function writeStored(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Preference is still applied for this session.
  }
}

/** Returns the stored value if it is one of `allowed`, otherwise `fallback`. */
export function readChoice<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  const stored = readStored(key);
  return allowed.find((option) => option === stored) ?? fallback;
}
