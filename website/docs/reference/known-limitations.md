---
title: Known limitations
description: What is not finished or not available in the 0.1.0 development build.
sidebar_position: 5
---

Contra Karaoke is usable on desktop and in the browser, but 0.1.0 is still a
development release. These are the limitations to know before relying on it.

## Platforms

- **Android** is configured and buildable with minSdk 26, but real-device audio
  and TTS testing is incomplete. Nightly APKs use the debug keystore unless
  release signing secrets are configured.
- The **web build** needs the included proxy for online search and downloads.
  Without it, the bundled defaults and anything already in your saved library
  still work.
- There is **no stable signed release**. Nightly artifacts use version 0.1.0,
  may be replaced, and are not code-signed.
- Flatpak is a source-tree packaging path, not a nightly artifact yet.

## Features

- **SoundFont loading** is desktop-only; the browser and Android builds use the
  built-in synth.
- **MIDI export** is desktop-only; the browser and Android builds report that it
  is unavailable.
- A **SoundFont is not bundled**. You supply the file and are responsible for
  its license.
- **TTS** uses the local Kokoro neural voice on desktop. The model and selected
  voice are downloaded on first use and then cached; a first run without a
  network connection falls back to the platform voice. Web and Android always
  use the platform voice. Visual calls continue if no voice is available.
- **Page size** is fixed at ten results; there is no page-size selector.
- Folders are a flat list. There are no nested folders and no sync between
  devices.
- There is no offline “using cached search results” banner; stale cache is used
  silently. The saved-dance and saved-tune lists always work offline.
- There is no cache-size or library-size display, and no manual source refresh;
  clearing the cache and downloading an item again are the maintenance actions.
- The first **Play** renders the music synchronously. There is no progress bar
  or cancel control while it runs.

## Dance cards

The transport uses one 64-beat music loop. A card shorter than 64 beats leaves
music playing after its last call. A card longer than 64 beats is not fully
reached in 0.1.0 because the playhead resets at the loop boundary. The session
shows a **length check** warning when the dance and tune lengths differ, but it
does not rescale either one. Prefer 64-beat cards.

<Shot id="session-length-warning" />

## Accessibility

The app is keyboard-operable for Space, R, Esc, 1–4, and Enter, but the
accessibility tree is still being audited. Several search, alias, and slider
controls are visually labelled without a formal accessible-name association.
Screen-reader support is incomplete.

## Data and privacy

- Dance and tune data belong to their contributors. The app stores downloads
  locally but does not redistribute a database.
- There is no telemetry, account, or cloud sync.
- The docs describe the current `main`/nightly build; there are no versioned
  docs before 1.0.

If a limitation blocks you, open an issue with the platform, the screen, and
whether audio, TTS, or a SoundFont was involved.

Next: [Troubleshooting](/docs/reference/troubleshooting).
