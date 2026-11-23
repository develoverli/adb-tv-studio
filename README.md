# adb-tv-studio

[![CI](https://github.com/develoverli/adb-tv-studio/actions/workflows/ci.yml/badge.svg)](https://github.com/develoverli/adb-tv-studio/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/develoverli/adb-tv-studio)](https://github.com/develoverli/adb-tv-studio/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Open-source desktop app to discover, connect to and manage Android TV devices over ADB, without touching a terminal.

> **Status: early development.** The project scaffold, CI and release pipeline are in place; features are being built. See the [roadmap](#roadmap).

## Features (planned for v1)

- **Discover TVs** on your network via mDNS (Android 11+ wireless debugging) and a `:5555` subnet scan, or add one by IP.
- **Pair** with Android 11+ devices using the 6-digit wireless debugging code.
- **Install apps** from a local `.apk` or from the [F-Droid](https://f-droid.org) catalog.
- **Manage apps**: list, launch, uninstall, clear data.
- **Set the default launcher** (HOME activity) in one click.
- **Themes** for [tv-launcher](https://github.com/develoverli/tv-launcher): wallpapers plus a `theme.json` colour scheme.
- **Remote control** from your PC: D-pad, keyboard and text input.
- **Saved devices**: keep several TVs and switch between them.

ADB is bundled: you do not need the Android SDK installed.

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
- [ ] Device discovery (mDNS, subnet scan, manual IP) and pairing
- [ ] Saved devices
- [ ] APK install and app management
- [ ] F-Droid catalog
- [ ] Default launcher
- [ ] Remote control
- [ ] tv-launcher themes (`theme.json` schema)
- [ ] English and Spanish UI

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) to set up the project, and follow the [Code of Conduct](CODE_OF_CONDUCT.md).

Found a security issue? Do not open a public issue; see [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE). `adb` is part of the Android SDK Platform-Tools, downloaded from Google at build time and distributed under its own terms.
