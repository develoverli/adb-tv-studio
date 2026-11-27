# tv-launcher

`pnpm fetch-sidecars` places `tv-launcher.apk` here, from the pinned tv-launcher GitHub release
(`TV_LAUNCHER_VERSION` and `TV_LAUNCHER_SHA256` in `scripts/fetch-sidecars.mjs`). The app bundles
this folder and installs the APK on TVs from the Themes and Launcher screens.

For a local build, run `TV_LAUNCHER_APK=/path/to/app-release.apk pnpm fetch-sidecars`.
Without an APK the app still builds; it then says that this version does not include tv-launcher.
