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

For Flatpak, see `packaging/flatpak/README.md`.
