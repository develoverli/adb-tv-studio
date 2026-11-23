# Security Policy

adb-tv-studio runs `adb` commands against devices on your network, so we take security reports seriously.

## Supported versions

Only the latest release receives security fixes. The app auto-updates, so please stay on the latest version.

## Reporting a vulnerability

**Do not open a public issue.**

Report privately through GitHub: go to the [Security tab](https://github.com/develoverli/adb-tv-studio/security) → **Report a vulnerability**.

Please include:

- What the issue is and its impact.
- Steps to reproduce, or a proof of concept.
- The affected version and OS.

## What to expect

- We acknowledge reports within **7 days**.
- We keep you updated while we investigate and fix the issue.
- Once a fix is released, we publish a GitHub Security Advisory and credit you, unless you prefer to stay anonymous.

## Scope

In scope:

- The adb-tv-studio desktop app and its update mechanism.
- Command injection through device names, package names, APK metadata, F-Droid data or theme files.
- The build and release pipeline, including the `adb` download and checksum verification.

Out of scope:

- Vulnerabilities in `adb` itself or in Android; report those to [Google](https://source.android.com/docs/security/overview/updates-resources#report-issues).
- The inherent risk of enabling ADB debugging on a device: anyone on your network who is authorized on the TV can control it.
