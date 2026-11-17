---
title: Build and run
description: Build Contra Karaoke from source on Windows or Linux and start the desktop app.
sidebar_position: 5
---

Contra Karaoke is a Rust workspace with five crates: `core` (domain and
timing), `music` (ABC, arrangement, synthesis), `sources` (ContraDB and ABC
Session Tunes), `app` (the shared `eframe`/`egui` application), and `web` (the
WASM entry point and Trunk page).

## Requirements

- A recent stable Rust toolchain. The workspace declares **Rust 1.95** as its
  minimum.
- Windows or Linux for the desktop build.
- On Linux: ALSA, X11/Wayland, OpenGL, GTK file dialogs, OpenSSL, and Speech
  Dispatcher. A typical Debian/Ubuntu install is:

<Steps>
  <CommandStep command="sudo apt-get install libasound2-dev libxkbcommon-dev libssl-dev libx11-dev libxcursor-dev libxi-dev libxrandr-dev libgl1-mesa-dev libgtk-3-dev libspeechd-dev speech-dispatcher">
    Install the native build dependencies.
  </CommandStep>
</Steps>

Windows needs no extra system packages for the default build; the release build
statically links the MSVC runtime. The app uses the system audio device and the
installed Windows voices.
## Run a development build

<Steps>
  <CommandStep command="git clone https://github.com/nicholasgower/Contra-karaoke.git">
    Clone the repository.
  </CommandStep>
  <CommandStep command="cd Contra-karaoke">
    Enter the workspace.
  </CommandStep>
  <CommandStep command="cargo run -p contra-app">
    Build and launch the desktop app.
  </CommandStep>
</Steps>

The first build compiles the UI, audio, and synthesis crates and takes a few
minutes. Later builds are incremental.

## Build a release binary

<Steps>
  <CommandStep command="cargo build --release -p contra-app">
    Build with optimizations.
  </CommandStep>
  <CommandStep command="target/release/contra-karaoke">
    Run the release binary on Linux. On Windows use `target\release\contra-karaoke.exe`.
  </CommandStep>
</Steps>

## First launch

The app opens on the **Session** tab with **Another Last Dance** and **Sandy
Boys** already loaded. The prompt says **READY** and the line above it says
`Press Play to prepare the music with these settings.` The arrangement is
prepared in the background while the session is idle; the first **Play** installs
the prepared loop.

The app stores preferences and the source cache in the standard per-user
configuration and cache directories. Searches and detail pages are fetched on
demand; the bundled defaults need no network.

## Build the web app

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd crates/web && trunk build --release
```

The bundle is written to `dist/web`. It must be served over HTTP. See
[Run in the browser](/docs/getting-started/install-web).

## Build the Android APK

Android requires the SDK/NDK, `cargo-apk`, and the arm64/armv7 targets:

```sh
rustup target add --toolchain 1.100.0 \
  aarch64-linux-android armv7-linux-androideabi
python packaging/android/build-apk.py
```

The script wraps `cargo apk`, adds the `rs.tts.Bridge` class used by the Android
TTS backend, aligns the archive, and signs it. The output is
`target/release/apk/contra-karaoke.apk`.

See [Install on Android](/docs/getting-started/install-android) for the device-testing status and
limitations.

## Run the tests

<Steps>
  <CommandStep command="cargo test --workspace">
    Run the core, music, sources, app, and web tests.
  </CommandStep>
  <CommandStep command="cargo fmt --all --check">
    Check formatting.
  </CommandStep>
  <CommandStep command="cargo clippy --workspace --all-targets -- -D warnings">
    Check lints.
  </CommandStep>
</Steps>

## Packaging status

There is no stable signed release yet, but nightly builds are produced on every
push to `dev`. Feature work merges into `dev` first; `main` only advances when a
release is ready. The workflow attaches a Windows installer and portable ZIP, a
Linux `.deb` and AppImage, a web ZIP, and an Android APK to the rolling
`nightly` release. Flatpak remains a source-tree packaging path. Use the
[download page](/download) for the current artifact list.

Do not follow a third-party install guide for 0.1.0; use this repository's
release artifacts or build from source.

Next: [Your first session](/docs/getting-started/first-session).
