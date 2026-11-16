---
title: What is Contra Karaoke?
description: A caller trainer that shows each contra dance call before the dancers need to move, in sync with ABC-sourced music.
sidebar_position: 1
---

Contra Karaoke is a practice tool for contra dance callers. It behaves like a
karaoke machine for the caller: the dance card and a large moving prompt show
what to say, while a band-style arrangement of an ABC tune plays underneath.

The important difference from a metronome is **lead time**. Every call is placed
before the beat where the dancers execute the figure. The amount of lead comes
from the rendered words themselves, plus a safety padding (0.5 s by default): a
short `swing` starts about two beats early, a longer phrase such as
`neighbors do-si-do` starts about three and a half beats early.

<Shot id="main-idle" />

## How a session fits together

1. A dance card supplies the figure order and the beats for each figure.
2. A tune supplies the melody and chords. The arranger turns it into a
   64-dance-beat loop — 32 bars at one dance beat per half bar.
3. Four beats of accompaniment count in the tempo before the loop.
4. The timeline renders each figure into the caller's alias wording, estimates
   how long that wording takes to say, and places the call early.
5. The transport plays the rendered audio while the UI follows the playback
   position, not wall-clock time.

The result is one continuous practice loop: count-in, calls, music, and a
highlighted card row.

## What you need

- A desktop build (Windows or Linux), the browser build, or a development
  Android build. See the installation pages.
- An audio output device for the rendered music. In a browser, press Play once
  to satisfy the autoplay gesture. If no output device is available the app
  still opens with a silent engine and explains the problem.
- A network connection the first time you download a dance or tune from
  ContraDB or ABC Session Tunes. The bundled defaults and anything already in
  your saved library work offline.
- Optionally, the local Kokoro neural voice on desktop and Android (downloaded
  on first use) or a system text-to-speech voice (the only voice on web), and,
  on desktop, a user-supplied `.sf2` SoundFont. The bundled Fluid R3 GM subset
  needs no download.

## The defaults

The app is useful immediately. It starts with:

- **Another Last Dance** — ContraDB dance 2409, choreographed by Erik Hoffman.
- **Sandy Boys** — ABC Session Tunes tune 75815, with ABC transcription by
  Josh Larios and chords from Sarah Comer.

Both are bundled with the application and attributed on this site. Other dances
and tunes are downloaded on demand and saved locally; the project does not
redistribute a dance or tune database.

## Current status

These docs describe the 0.1.0 development build. The desktop app (Windows and
Linux) and the browser app (WASM) are implemented. Android is configured and
buildable, but still needs real-device testing. Release hardening and nightly
packaging are in progress.

Screenshots of the app are rendered by the real headless `contra-headless`
binary. A few OS installer dialogs remain labelled placeholders until they are
captured manually.

| Platform | Status |
| --- | --- |
| Windows x64 | Desktop build; installer and portable ZIP in the nightly release |
| Linux x86_64 | Desktop build; `.deb` and AppImage in the nightly release |
| Web (WASM) | Browser build; a hosted copy runs on this site, and the downloaded bundle needs a static server and a data proxy for online search and downloads |
| Android | Configured with minSdk 26; APK build and real-device testing still in progress |

Ready to try it? [Play in your browser](/play) without installing anything, or
continue to [Install on Windows](/docs/getting-started/install-windows),
[Install on Linux](/docs/getting-started/install-linux),
[Run in the browser](/docs/getting-started/install-web), or
[build from source](/docs/getting-started/build-and-run). If you just want the
short path, go to [Your first session](/docs/getting-started/first-session).
See [Known limitations](/docs/reference/known-limitations) before relying on a
nightly build.
