---
title: Install on Android
description: Build and sideload the Android APK, and understand the current device-testing status.
sidebar_position: 4
---

Android support is configured with minSdk 26 and targetSdk 35. A nightly APK is
built by CI, but real-device audio and TTS testing is still in progress. Do not
treat Android as a polished release yet.

## Requirements

- Android 8.0 (API 26) or newer.
- An ARM device; the build targets arm64 and armv7.
- A network connection for the first search, or the bundled defaults.
- Optional: a system TTS engine and voice.

## Get the APK

Download the APK from the
[nightly release](https://github.com/storytold/Contra-karaoke/releases/tag/nightly),
or build it yourself:

<Steps>
  <CommandStep command="rustup target add aarch64-linux-android armv7-linux-androideabi">
    Install the Android Rust targets.
  </CommandStep>
  <CommandStep command="export ANDROID_HOME=... ANDROID_NDK_ROOT=...">
    Point the build at your Android SDK and NDK.
  </CommandStep>
  <CommandStep command="cargo apk build --release --lib -p contra-app">
    Build the APK. It is written under `target/release/apk/`.
  </CommandStep>
</Steps>

The package ID is `org.contrakaraoke.app`. Nightly CI signs with the debug
keystore unless release signing secrets are configured, so treat the artifact as
a test build.

## Sideload the APK

<Steps>
  <Step shot="install-android-package">
    Open the APK on the device. Android may ask you to allow installing unknown
    apps for the file manager or browser you used.
  </Step>
  <Step shot="install-android-app">
    Confirm the install and open Contra Karaoke.
  </Step>
</Steps>

With ADB, `adb install path/to/contra-karaoke.apk` performs the same install.

## First launch

The app opens with *Another Last Dance* and *Sandy Boys*. Press **Play** to
render the music. Android uses the AAudio backend through cpal; the first
gesture is a normal app interaction, not a browser restriction.

If audio or TTS misbehaves, include the device model, Android version, and
whether a SoundFont was loaded in the issue report.

## Current Android limitations

- Real-device audio and TTS testing is incomplete.
- Loading a SoundFont from a file is not available in this build yet; the
  bundled Fluid R3 GM subset is used instead.
- MIDI export is not available in this build yet.
- There is no Play Store release. Use the nightly APK or build from source.
- Preferences and the source cache use the app's private storage; clearing app
  data resets them.

## Uninstall

Use Android's **Settings → Apps → Contra Karaoke → Uninstall**. Uninstalling
removes the app's private preferences and cache.

## Build from source

The Android build requires the Rust targets, Android SDK/NDK, `cargo-apk`, and
minSdk 26. The full commands are in the repository's `packaging/android/README.md`
and [Build and run](/docs/getting-started/build-and-run).

Next: [Your first session](/docs/getting-started/first-session).
