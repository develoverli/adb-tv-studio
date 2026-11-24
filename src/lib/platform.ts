/**
 * Coarse OS detection for window chrome only. macOS keeps native traffic lights
 * (see tauri.macos.conf.json); Windows and Linux get our own controls.
 */
export const isMac = /Macintosh|Mac OS X/.test(navigator.userAgent);
