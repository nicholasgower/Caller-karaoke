# Linux packaging

Install the native build dependencies first (Debian/Ubuntu):

```sh
sudo apt-get update
sudo apt-get install -y \
  libasound2-dev libxkbcommon-dev libssl-dev \
  libx11-dev libxcursor-dev libxi-dev libxrandr-dev \
  libgl1-mesa-dev libgtk-3-dev libspeechd-dev speech-dispatcher
```

Build and package:

```sh
cargo build --release -p contra-app
cargo install cargo-deb --locked
cargo deb -p contra-app
VERSION=0.1.0 bash packaging/linux/build-appimage.sh
```

ONNX Runtime is loaded dynamically on Linux; the app downloads the official
MIT-licensed runtime (about 11 MB) on first speech use and caches it, so the
`.deb`, AppImage, and Flatpak do not carry a separate runtime library. The Kokoro
voice model is also downloaded from Hugging Face on first use and cached; it is
not part of the packages. If either download is unavailable, the app falls back
to the platform voice.

The `.deb` is written to `target/debian/`; the AppImage to `dist/`.

## Media keys (MPRIS)

With a D-Bus session bus, the app registers the MPRIS player
`org.contrakaraoke.ContraKaraoke`. The desktop's media keys, its media widget
(including the Cinnamon sound applet on Linux Mint), and lock-screen controls
drive the session from any window; the player shows the loaded dance as the
title and the tune as the artist.

MPRIS is the correct integration on Cinnamon, GNOME, KDE Plasma, Xfce, and
MATE — no tray icon or desktop-specific code is involved. If the app starts
without a session bus (a bare TTY, or a test runner), it skips the media session
instead of failing; check `DBUS_SESSION_BUS_ADDRESS` or
`$XDG_RUNTIME_DIR/bus` when debugging.

`contra-karaoke.desktop` sets `StartupWMClass=contra-karaoke`. winit derives
`WM_CLASS` from the executable's file name, so the packaged binary at
`/usr/bin/contra-karaoke` reports `contra-karaoke` and the panel associates the
window with the desktop entry. Renaming the executable means updating `Exec=`,
the `StartupWMClass=` value, and the icon name together, or the desktop keeps a
second, anonymous window entry.

For Flatpak, see `packaging/flatpak/README.md`.
