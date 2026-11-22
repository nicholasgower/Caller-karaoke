---
title: Install on Linux
description: Install the Debian package or AppImage, or build Caller Karaoke from source.
sidebar_position: 2
---

Linux release builds are published as a Debian package and an AppImage. The
Flatpak manifest is included in the source tree but is not part of the release
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

## Download

<DownloadButton platform="linux" />

The main button downloads the Debian package; the menu on the button offers the
AppImage.

## Debian package

<Steps>
  <Step shot="install-linux-deb">
    Download `caller-app_0.2.1-1_amd64.deb` with the button above.
  </Step>
  <CommandStep command="sudo apt install ./caller-app_0.2.1-1_amd64.deb">
    Install the package with apt so its dependencies are resolved. Use the
    actual downloaded filename; the version and architecture may differ.
  </CommandStep>
  <CommandStep command="caller-karaoke">
    Start the app from the application menu or a terminal.
  </CommandStep>
</Steps>

The package installs the binary under `/usr/bin` and a desktop entry under
`/usr/share/applications`.

## AppImage

<Steps>
  <Step shot="install-linux-appimage">
    Download `CallerKaraoke-0.2.1-x86_64.AppImage` and make it executable.
  </Step>
  <CommandStep command="chmod +x CallerKaraoke-0.2.1-x86_64.AppImage && ./CallerKaraoke-0.2.1-x86_64.AppImage">
    Run the AppImage.
  </CommandStep>
</Steps>

If FUSE is unavailable, run it with
`./CallerKaraoke-0.2.1-x86_64.AppImage --appimage-extract-and-run`.

## Flatpak from source

The Flatpak manifest is
`packaging/flatpak/org.callerkaraoke.CallerKaraoke.yml`. It requires a generated
`cargo-sources.json` and `flatpak-builder`; see
`packaging/flatpak/README.md` for the exact commands. This is a developer path,
not a published package.

## First launch and data

The app opens with the bundled defaults. Press **Play** to start; the loop is
prepared in the background. Preferences and the source cache live in the
standard XDG per-user directories; the AppImage and the `.deb` share them.

Linux TTS requires Speech Dispatcher to be running. If it is not available, the
visual calls still work and the TTS section explains the problem.

## Media keys and the Cinnamon sound applet

The app registers an MPRIS player, `org.callerkaraoke.CallerKaraoke`, on the
D-Bus session bus. The keyboard's **Play/Pause** key, the media widget in the
panel, and the lock-screen controls therefore drive the session from any window,
and the widget shows the loaded dance as the title and the tune as the artist.

On **Linux Mint (Cinnamon)**, the sound applet in the panel lists Caller Karaoke
as a player while it is active. Its Play/Pause button, the media key, and the
lock-screen media controls all go through MPRIS and drive the same transport as
the app's own **Play/Pause** button; the applet's own volume slider
controls the system output, not the app's **Volume** setting inside the session
tab. GNOME, KDE Plasma, and other MPRIS-aware panels work the same way.

MPRIS needs a session bus, which a normal desktop login provides. In a bare TTY,
or over SSH without a session bus, the app skips the media integration and the
in-app controls still work.

The desktop entry sets `StartupWMClass=caller-karaoke`, which matches the
`WM_CLASS` the window reports when the packaged binary is launched from
`/usr/bin/caller-karaoke`; that is what lets the panel associate the running
window with the app icon.

## Uninstall

```sh
sudo apt remove caller-app
```

For the AppImage, delete the file. Flatpak builds are removed with
`flatpak uninstall org.callerkaraoke.CallerKaraoke`.

Removing the app does not automatically remove the per-user source cache or
preferences.

## Build from source

See [Build and run](/docs/getting-started/build-and-run). For packaging, the source tree includes
`packaging/linux/` and `packaging/flatpak/`.

Next: [Your first session](/docs/getting-started/first-session).
