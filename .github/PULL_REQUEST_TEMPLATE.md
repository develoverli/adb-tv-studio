## What and why

<!-- What does this PR change, and why? Link the issue: "Closes #123". -->

## How to test

<!-- Steps a reviewer can follow. Mention the OS and TV/device you tested on. -->

## Checklist

- [ ] The PR title follows [Conventional Commits](https://www.conventionalcommits.org) (e.g. `feat: add mDNS discovery`)
- [ ] `pnpm typecheck` and `pnpm build` pass
- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings` and `cargo test` pass in `src-tauri/`
- [ ] User-facing strings go through i18n (no hardcoded copy)
- [ ] Docs updated if behavior changed
