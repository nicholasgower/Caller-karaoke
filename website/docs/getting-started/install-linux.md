---
title: Install on Linux
description: Install the Debian package or AppImage, or build Contra Karaoke from source.
sidebar_position: 2
---

Linux nightly builds are published as a Debian package and an AppImage. The
Flatpak manifest is included in the source tree but is not part of the nightly
artifact set yet.

## Requirements

- 64-bit Linux with X11 or Wayland.
- ALSA/PulseAudio-compatible audio output.
- Speech Dispatcher for optional spoken calls.
- WebGL2-capable graphics for the `glow` renderer.

The native build dependencies are:

```sh
sudo apt-get install libasound2-dev libxkbcommon-dev libssl-dev \
  libx11-dev libxcursor-dev libxi-dev libxrandr-dev \
  libgl1-mesa-dev libgtk-3-dev libspeechd-dev speech-dispatcher
```

## Debian package

<Steps>
  <Step shot="install-linux-deb">
    Download the `.deb` from the
    [nightly release](https://github.com/storytold/Contra-karaoke/releases/tag/nightly).
  </Step>
  <CommandStep command="sudo apt install ./contra-app_0.1.0_amd64.deb">
    Install the package with apt so its dependencies are resolved. Use the
    actual downloaded filename; the version and architecture may differ.
  </CommandStep>
  <CommandStep command="contra-karaoke">
    Start the app from the application menu or a terminal.
  </CommandStep>
</Steps>

The package installs the binary under `/usr/bin` and a desktop entry under
`/usr/share/applications`.

## AppImage

<Steps>
  <Step shot="install-linux-appimage">
    Download `ContraKaraoke-0.1.0-x86_64.AppImage` and make it executable.
  </Step>
  <CommandStep command="chmod +x ContraKaraoke-0.1.0-x86_64.AppImage && ./ContraKaraoke-0.1.0-x86_64.AppImage">
    Run the AppImage.
  </CommandStep>
</Steps>

If FUSE is unavailable, run it with
`./ContraKaraoke-0.1.0-x86_64.AppImage --appimage-extract-and-run`.

## Flatpak from source

The Flatpak manifest is
`packaging/flatpak/org.contrakaraoke.ContraKaraoke.yml`. It requires a generated
`cargo-sources.json` and `flatpak-builder`; see
`packaging/flatpak/README.md` for the exact commands. This is a developer path,
not a published nightly package.

## First launch and data

The app opens with the bundled defaults. Press **Play** to render the loop.
Preferences and the source cache live in the standard XDG per-user directories;
the AppImage and the `.deb` share them.

Linux TTS requires Speech Dispatcher to be running. If it is not available, the
visual calls still work and the TTS section explains the problem.

## Uninstall

```sh
sudo apt remove contra-app
```

For the AppImage, delete the file. Flatpak builds are removed with
`flatpak uninstall org.contrakaraoke.ContraKaraoke`.

Removing the app does not automatically remove the per-user source cache or
preferences.

## Build from source

See [Build and run](/docs/getting-started/build-and-run). For packaging, the source tree includes
`packaging/linux/` and `packaging/flatpak/`.

Next: [Your first session](/docs/getting-started/first-session).
