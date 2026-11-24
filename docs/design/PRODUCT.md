# PRODUCT

register: product

## Product Purpose
Desktop companion for Android TV devices. Finds TVs on the local network, connects over ADB, installs apps
(local APK or F-Droid), manages installed apps, sets the default launcher, applies themes to
[tv-launcher](https://github.com/develoverli/tv-launcher), and works as a remote control. It replaces typing
`adb` commands in a terminal.

## Users
Hobbyists and tinkerers who own one or more Android TV / Google TV devices and want to sideload apps or replace
the stock launcher. Comfortable following a guide, not necessarily comfortable with a terminal. Typical scene:
laptop on the couch in the evening, living-room light low, the TV they are configuring right in front of them.
English and Spanish speakers.

## Tone
Calm, precise, trustworthy. A tool that feels as quiet and premium as the launcher it pairs with, but with clear
state: you always know whether a TV is connected and what an action will do to it.

## Family
Shares DNA with tv-launcher (Outfit typeface, dark tinted canvas, restraint) and adds one accent of its own,
signal green, because a desktop tool needs to mark actions, selection and live state.

## Anti-references
- Generic Electron "admin dashboard" look: card grids of stats, gradient headers.
- Sketchy sideloading tools: cluttered toolbars, warnings everywhere, raw console output as UI.
- Terminal-cosplay dark UIs (monospace everything, neon on black).

## Principles
- State is always explicit: connected / offline / needs authorization, never implied.
- Empty states teach the next step; nothing says only "nothing here".
- Destructive actions on a TV (uninstall, clear data, disable launcher) are confirmed with an in-app dialog, never a native one.
- Honest UI: features not built yet are labelled as such, never faked.
