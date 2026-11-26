# Contributing to adb-tv-studio

Thanks for your interest in improving adb-tv-studio. This guide covers local setup, the commit convention and how releases happen.

By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Ways to contribute

- Report bugs and request features through [issues](https://github.com/develoverli/adb-tv-studio/issues/new/choose).
- Test on TVs and boxes we do not own, and report what works.
- Translate the UI (once i18n lands).
- Pick an issue labelled `good first issue` or `help wanted`. Comment on it before starting so work is not duplicated.

For larger changes, open an issue first to agree on the approach.

## Prerequisites

- [Node.js](https://nodejs.org) 24+
- [pnpm](https://pnpm.io) (the version is pinned in `package.json` → `packageManager`; `corepack enable` picks it up)
- [Rust](https://rustup.rs) stable
- The [Tauri 2 system prerequisites](https://tauri.app/start/prerequisites/) for your OS (WebView2 on Windows, Xcode CLT on macOS, `webkit2gtk-4.1` on Linux)

## Setup

```bash
git clone https://github.com/develoverli/adb-tv-studio.git
cd adb-tv-studio
pnpm install
pnpm fetch-sidecars # downloads and verifies adb and scrcpy-server into src-tauri/binaries/
pnpm tauri dev      # runs the app with hot reload
```

`pnpm fetch-sidecars` must run once before the first Rust build: `tauri-build` refuses to compile if the `adb` sidecar or the scrcpy-server resource is missing.

The TV list comes from the bundled adb (its own server on port 5038, so it does not clash with another adb on 5037). To design screens without a TV, load two sample TVs (`src/dev/mockDevices.ts`) instead. They are never part of a release build:

```bash
VITE_MOCK=1 pnpm tauri dev
```

### Local release builds

Release bundles are signed for the auto-updater. Without the signing key, build an unbundled binary instead:

```bash
pnpm tauri build --no-bundle
```

## Checks

CI runs these on every pull request. Run them before pushing:

```bash
pnpm typecheck
pnpm build
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --locked
```

## Project layout

```
src/                 SolidJS + TypeScript frontend
src-tauri/           Rust backend (Tauri 2)
  binaries/          adb sidecar (generated, git-ignored)
  capabilities/      Tauri permission sets
scripts/fetch-sidecars.mjs  downloads platform-tools and scrcpy-server with pinned SHA-256s
```

## Coding guidelines

- **Language:** code, comments, docs, commits and issues are in English.
- **TypeScript:** `strict` mode, no `any`.
- **Rust:** no `unwrap()` / `expect()` outside tests; return `Result` and propagate with `?`. Clippy `pedantic` is enabled.
- **No hardcoded user-facing strings:** UI copy goes through the i18n layer.
- **Dependencies:** explain why in the PR when adding one.
- Keep pull requests focused: one change per PR, no unrelated refactors.

## Commit messages

We use [Conventional Commits](https://www.conventionalcommits.org). Release notes and version bumps are generated from them, so the type matters:

| Type | Use for | Version bump |
|---|---|---|
| `feat:` | a new user-facing feature | minor |
| `fix:` | a bug fix | patch |
| `feat!:` / `BREAKING CHANGE:` | a breaking change | major |
| `docs:`, `chore:`, `refactor:`, `test:`, `ci:`, `build:`, `perf:` | everything else | none |

A [gitmoji](https://gitmoji.dev) is welcome but optional. It goes **after** the type, never before it: release-please only parses messages that start with the type, and silently skips the rest.

```
feat: ✨ discover devices over mDNS
fix(adb): 🐛 handle unauthorized device state
✨ feat: discover devices over mDNS   ← wrong, ignored by release-please
```

Pull requests are squash-merged, so the **PR title** must follow the same format.

`pnpm install` enables a `commit-msg` hook (`.githooks/commit-msg`) that rejects subjects release-please cannot parse.

## Releases

Releases are automated with [release-please](https://github.com/googleapis/release-please):

1. A push to `main` with releasable commits (`feat:`, `fix:`, breaking changes) makes release-please open a release PR with the next version and `CHANGELOG.md`. A push with nothing releasable does nothing.
2. The release workflow merges that PR immediately, then tags the version and creates a GitHub Release. No manual step.
3. It then builds Windows, macOS and Linux bundles, uploads them, and publishes `latest.json` for the in-app updater.

Contributors never bump versions or create tags by hand. Each release adds a commit to `main`, so run `git pull --rebase` before your next push.

## Updating scrcpy-server

Bump `SCRCPY_VERSION` and `SCRCPY_SHA256` in `scripts/fetch-sidecars.mjs` and `SCRCPY_VERSION` in `src-tauri/src/adb/scrcpy.rs` together: the server refuses a client with another version string. The protocol changes between releases, so re-check `doc/develop.md` of the new tag.

## Updating adb

Bump `PLATFORM_TOOLS_VERSION` and the three SHA-256 values in `scripts/fetch-sidecars.mjs`. Cross-check the archives against the SHA-1 values Google publishes in `https://dl.google.com/android/repository/repository2-3.xml`.
