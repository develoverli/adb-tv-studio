# Changelog

## [0.1.3](https://github.com/develoverli/adb-tv-studio/compare/v0.1.2...v0.1.3) (2026-11-27)


### Features

* **add:** ✨ open the TV's Overview right after connecting it ([84318b4](https://github.com/develoverli/adb-tv-studio/commit/84318b41548edc878ac2dad656609295953887f1))
* **themes:** ✨ preview themes as a faithful replica of the TV home screen, with a large view ([84318b4](https://github.com/develoverli/adb-tv-studio/commit/84318b41548edc878ac2dad656609295953887f1))
* **themes:** ✨ refine the bundled themes with contrast-checked palettes (noche, grafito, alto contraste) ([84318b4](https://github.com/develoverli/adb-tv-studio/commit/84318b41548edc878ac2dad656609295953887f1))
* **updates:** ✨ check for app updates at launch and install them from Settings ([84318b4](https://github.com/develoverli/adb-tv-studio/commit/84318b41548edc878ac2dad656609295953887f1))


### Bug Fixes

* **build:** 🐛 use the app icon for the Windows installer instead of the NSIS default ([84318b4](https://github.com/develoverli/adb-tv-studio/commit/84318b41548edc878ac2dad656609295953887f1))

## [0.1.2](https://github.com/develoverli/adb-tv-studio/compare/v0.1.1...v0.1.2) (2026-11-27)


### Features

* **adb:** ✨ add reconnect button for offline TVs ([35ac1cb](https://github.com/develoverli/adb-tv-studio/commit/35ac1cba708ce2b492c471b5857ad4bd6e891edd))
* **adb:** ✨ remember TVs and reconnect them automatically ([38e5bd4](https://github.com/develoverli/adb-tv-studio/commit/38e5bd420549564a225f9015346170da7f66de56))
* **install:** ✨ install local APK, XAPK, APKS and APKM files with a preview ([0e25b26](https://github.com/develoverli/adb-tv-studio/commit/0e25b26bb2b14a61a1958968bbe7227f0d3c6965))
* **launcher:** ✨ install and update tv-launcher from the app in one click ([38e5bd4](https://github.com/develoverli/adb-tv-studio/commit/38e5bd420549564a225f9015346170da7f66de56))
* **launcher:** ✨ set the default launcher and disable or restore the stock one ([38e5bd4](https://github.com/develoverli/adb-tv-studio/commit/38e5bd420549564a225f9015346170da7f66de56))
* **remote:** ✨ add remote control and live TV screen view ([35ac1cb](https://github.com/develoverli/adb-tv-studio/commit/35ac1cba708ce2b492c471b5857ad4bd6e891edd))
* **remote:** ✨ stream the TV screen live and send input instantly with scrcpy ([3466fd7](https://github.com/develoverli/adb-tv-studio/commit/3466fd7f4167b2c882652b6c96f68da3a0c8c142))
* **themes:** ✨ apply tv-launcher themes from bundled, imported and catalog sources ([38e5bd4](https://github.com/develoverli/adb-tv-studio/commit/38e5bd420549564a225f9015346170da7f66de56))


### Bug Fixes

* **adb:** 🐛 fold Windows line endings in track-devices so status updates arrive ([35ac1cb](https://github.com/develoverli/adb-tv-studio/commit/35ac1cba708ce2b492c471b5857ad4bd6e891edd))
* **adb:** 🐛 read TV info by key so values no longer shift ([35ac1cb](https://github.com/develoverli/adb-tv-studio/commit/35ac1cba708ce2b492c471b5857ad4bd6e891edd))
* **adb:** 🐛 use ANDROID_ADB_SERVER_PORT so the app runs its own adb server ([35ac1cb](https://github.com/develoverli/adb-tv-studio/commit/35ac1cba708ce2b492c471b5857ad4bd6e891edd))
* **apps:** 🐛 read names and icons from the base APK of split packages ([0e25b26](https://github.com/develoverli/adb-tv-studio/commit/0e25b26bb2b14a61a1958968bbe7227f0d3c6965))
* **install:** 🐛 show a clear message instead of a stack trace when the TV is out of space ([6e77f57](https://github.com/develoverli/adb-tv-studio/commit/6e77f57382fe0fdb3d42bf2be0cb1388e4ba8ff2))
* **screen:** 🐛 read adb output raw and skip vendor text before the PNG ([35ac1cb](https://github.com/develoverli/adb-tv-studio/commit/35ac1cba708ce2b492c471b5857ad4bd6e891edd))
* **ui:** 🐛 allow blob images in the CSP so app icons and screenshots show in release builds ([3466fd7](https://github.com/develoverli/adb-tv-studio/commit/3466fd7f4167b2c882652b6c96f68da3a0c8c142))
* **ui:** 🐛 keep installs, app loading and scan results when switching tabs ([6e77f57](https://github.com/develoverli/adb-tv-studio/commit/6e77f57382fe0fdb3d42bf2be0cb1388e4ba8ff2))

## [0.1.1](https://github.com/develoverli/adb-tv-studio/compare/v0.1.0...v0.1.1) (2026-11-26)


### Features

* ✨ add app shell with router, i18n and adb device discovery ([2fe2ac9](https://github.com/develoverli/adb-tv-studio/commit/2fe2ac99e4634db0b6801828bca27dd58acf69ae))


### Bug Fixes

* **adb:** � read TV info in ([2fe2ac9](https://github.com/develoverli/adb-tv-studio/commit/2fe2ac99e4634db0b6801828bca27dd58acf69ae))

## [0.1.0](https://github.com/develoverli/adb-tv-studio/compare/v0.1.0...v0.1.0) (2026-11-23)


### Documentation

* 📝 move gitmoji after commit type ([918db72](https://github.com/develoverli/adb-tv-studio/commit/918db7262c4e7f4e7c82cc2aea886f204c9590cc))
