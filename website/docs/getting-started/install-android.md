---
title: Install on Android
description: Build and sideload the Android APK, and understand the current device-testing status.
sidebar_position: 4
---

Android support is configured with minSdk 26 and targetSdk 35. A release APK is
built by CI, but real-device audio and TTS testing is still in progress. Do not
treat Android as a polished release yet.

## Requirements

- Android 8.0 (API 26) or newer.
- An ARM device; the build targets arm64 and armv7.
- A network connection for the first search, or the bundled defaults.
- Optional: a system TTS engine and voice.

## Get the APK

<DownloadButton platform="android" />

The APK is a test build; see the signing and device-testing caveats below. To
build it yourself:

<Steps>
  <CommandStep command="rustup target add --toolchain 1.100.0 aarch64-linux-android armv7-linux-androideabi">
    Install the Android Rust targets for the pinned toolchain.
  </CommandStep>
  <CommandStep command="export ANDROID_HOME=... ANDROID_NDK_ROOT=...">
    Point the build at your Android SDK and NDK.
  </CommandStep>
  <CommandStep command="python packaging/android/build-apk.py">
    Build the APK, add the TTS Java bridge, and sign it. The result is
    `target/release/apk/caller-karaoke.apk`.
  </CommandStep>
</Steps>

The package ID is `org.callerkaraoke.app`. The script signs with the debug
keystore unless `CARGO_APK_RELEASE_KEYSTORE` and
`CARGO_APK_RELEASE_KEYSTORE_PASSWORD` are set, so treat the default artifact as
a test build. The platform TTS backend needs the `rs.tts.Bridge` class that the
script packages as `classes.dex`; a bare `cargo apk build` omits it and the app
crashes while loading the native library.

## Sideload the APK

<Steps>
  <Step shot="install-android-package">
    Open the APK on the device. Android may ask you to allow installing unknown
    apps for the file manager or browser you used.
  </Step>
  <Step shot="install-android-app">
    Confirm the install and open Caller Karaoke.
  </Step>
</Steps>

With ADB, `adb install path/to/caller-karaoke.apk` performs the same install.

## First launch

The app opens with *Another Last Dance* and *Sandy Boys*. Press **Play** to
start; the music is prepared in the background. Android uses the AAudio backend
through cpal; the first gesture is a normal app interaction, not a browser
restriction.

If audio or TTS misbehaves, include the device model, Android version, and
whether a SoundFont was loaded in the issue report.

## Media controls

Android delivers headset buttons, Bluetooth controls, the lock screen, and the
system media player to the app's media session. The session shows the loaded
dance as the title, the tune as the artist, and *Caller Karaoke* as the album,
with a Play/Pause control.

- **Play** starts or resumes the session; **Pause** pauses it and keeps the beat
  so the next Play resumes from the same place.
- **Stop** pauses. **Next** and **Previous** are ignored.
- Headset and Bluetooth buttons, the lock screen, and the playback notification
  all take the same path as the app's own **Play/Pause** button.

On Android 13 and newer the app asks for notification permission so the playback
notification can appear. Denying it only hides the notification: headset,
Bluetooth, lock-screen, and system media controls still work.

The notification is a media-style notification, not a foreground service. If
Android stops the app in the background, playback stops with it.

## Touch controls

- **Tap a tab** at the top to switch between Session, Dance, Tune, and Terms;
  the gear opens Prefs.
- **Swipe left or right** across empty content to move to the next or previous
  tab. The tab row wraps, so swiping right from Session opens Terms (or Tune
  while the glossary is off) and swiping left from Terms returns to Session. A
  swipe needs roughly 60 points of mostly horizontal travel and changes the tab
  once per gesture. A press that starts on a button, slider, folder row, or
  splitter stays with that control.
- **Tap a slider, then drag it sideways** to change its value. Every slider
  shows a padlock and a rounded outline until it is tapped, so a thumb that lands
  on the tempo or volume control while scrolling does not knock it off its
  setting. Tapping or swiping anywhere outside the slider puts the padlock
  back. While an unlocked slider is being dragged the page does not scroll; a
  swipe that starts on a locked slider still scrolls.
- **Tap a text field** to open the system keyboard. Backspace, Enter, and the
  arrow keys work in the focused field, and the page resizes so the field stays
  visible. Dismiss the keyboard with the system Back gesture; tap the field
  again to bring it back. Suggestions and autocorrect are off so aliases,
  searches, and ABC source arrive exactly as typed.
- **Type into a number box** (a tempo, volume, or timing value) the same way.
  The keyboard stays open while you edit the number, and the value applies when
  you press Enter or tap elsewhere, so a half-typed number is not clamped or
  swapped while you are still typing.
- Swipe tab changes are ignored while a text field has focus, while a drop-down
  is open, and while the **About** window is showing, so they cannot interrupt
  typing, a menu, or a dialog.
- With a hardware keyboard, **←** and **→** also move between tabs.

## Current Android limitations

- Real-device audio and TTS testing is incomplete.
- Playback stops when Android ends the app in the background; there is no
  foreground service or background-playback mode yet.
- Loading a SoundFont from a file is not available in this build yet; the
  bundled Fluid R3 GM subset is used instead.
- MIDI export is not available in this build yet.
- There is no Play Store release. Use the release APK or build from source.
- Preferences and the source cache use the app's private storage; clearing app
  data resets them.
- The on-screen keyboard is basic: suggestions and autocorrect are off, and an
  IME that depends on a candidate window (some CJK keyboards) may not behave as
  expected.

## Uninstall

Use Android's **Settings → Apps → Caller Karaoke → Uninstall**. Uninstalling
removes the app's private preferences and cache.

## Build from source

The Android build requires the Rust targets, Android SDK/NDK, `cargo-apk`, and
minSdk 26. The full commands are in the repository's `packaging/android/README.md`
and [Build and run](/docs/getting-started/build-and-run).

Next: [Your first session](/docs/getting-started/first-session).
