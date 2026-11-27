# adb-tv-studio

[![CI](https://github.com/develoverli/adb-tv-studio/actions/workflows/ci.yml/badge.svg)](https://github.com/develoverli/adb-tv-studio/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/develoverli/adb-tv-studio)](https://github.com/develoverli/adb-tv-studio/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Open-source desktop app (Windows, macOS, Linux) to manage Android TVs over your network,
without touching a terminal: connect them, install and manage apps, control them from your PC,
and give them a clean home screen with themes.

## Features

- **Connect TVs** found on your network (mDNS for Android 11+ wireless debugging and a `:5555`
  scan), by IP, or by pairing with the 6-digit code. TVs are remembered and reconnected
  automatically.
- **Control** the TV from your PC: live screen (H.264 through scrcpy, screenshots as a fallback),
  D-pad, media keys, typing and clicks on the screen.
- **Apps**: real names and banners, open, force stop, clear data, disable, uninstall.
- **Install** local `.apk`, `.xapk`, `.apks` and `.apkm` files with a preview of what changes.
- **Launcher**: pick which app the Home button opens, disable the stock launcher (and restore it).
- **tv-launcher**: install a clean, ad-free home screen in one click (see below).
- **Themes** for tv-launcher: six built in, a catalog on GitHub, or your own `.zip`.
- English and Spanish.

`adb` is bundled: you do not need the Android SDK.

## The project

adb-tv-studio is one of three repositories that work together:

```
 adb-tv-studio  (this repo, the app you install on your computer)
     │
     ├── installs ──────▶  tv-launcher         home screen on the TV
     │                        ▲
     └── downloads ─▶  tv-launcher-themes ──┘  themes it applies to tv-launcher
```

| Repository | What it is |
|---|---|
| **adb-tv-studio** | The desktop app. The only thing you install. |
| [**tv-launcher**](https://github.com/develoverli/tv-launcher) | The TV home screen: just your apps. No ads or paid recommendations, and no internet permission at all, so it cannot track you or send data anywhere. MIT. The app bundles it and installs it from **Themes** or **Overview → Launcher**. |
| [**tv-launcher-themes**](https://github.com/develoverli/tv-launcher-themes) | The theme catalog and the `theme.json` contract (JSON Schema). Themes are data only: colours, background, layout (classic, rows, grid, minimal, sidebar), shapes, focus effect, fonts. Anyone can add one with a pull request. |

## Install

Download the latest build from [Releases](https://github.com/develoverli/adb-tv-studio/releases/latest):

| OS | File |
|---|---|
| Windows 10/11 | `adb-tv-studio_<version>_x64-setup.exe` |
| macOS (Apple Silicon / Intel) | `adb-tv-studio_<version>_aarch64.dmg` / `_x64.dmg` |
| Linux | `.AppImage` or `.deb` |

The app updates itself when a new release is published.

### Unsigned builds

Builds are not code-signed yet, so your OS will warn you the first time:

- **Windows (SmartScreen):** "Windows protected your PC" → **More info** → **Run anyway**.
- **macOS (Gatekeeper):** after moving the app to `/Applications`, run:
  ```bash
  xattr -dr com.apple.quarantine "/Applications/adb-tv-studio.app"
  ```
- **Linux (AppImage):** `chmod +x adb-tv-studio_*.AppImage`.

## Prepare your TV

1. **Settings → System → About** → press **Android TV OS build** 7 times to enable Developer options.
2. In **Developer options**, enable **USB debugging** and/or **Wireless debugging** (the name varies by vendor).
3. Keep the TV and your computer on the same network.
4. The first time you connect, accept the "Allow debugging?" prompt on the TV.

## Roadmap

- [x] Project scaffold, CI, automated releases, bundled `adb`
- [x] Device discovery (mDNS, subnet scan, manual IP) and pairing
- [x] Saved devices with automatic reconnect
- [x] APK install and app management
- [x] Remote control with live screen
- [x] Default launcher
- [x] tv-launcher install and themes (`theme.json` schema, catalog, zip import)
- [x] English and Spanish UI
- [ ] F-Droid catalog
- [ ] Theme editor and third-party theme catalogs

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) to set up the project, and follow the [Code of Conduct](CODE_OF_CONDUCT.md).
New themes go to [tv-launcher-themes](https://github.com/develoverli/tv-launcher-themes); launcher changes to [tv-launcher](https://github.com/develoverli/tv-launcher).

Found a security issue? Do not open a public issue; see [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE). The app bundles third-party software under its own terms, listed in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md): `adb` (Android SDK Platform-Tools, downloaded
from Google at build time), scrcpy-server (Apache-2.0) and tv-launcher (MIT).
