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

The `.deb` is written to `target/debian/`; the AppImage to `dist/`.

For Flatpak, see `packaging/flatpak/README.md`.
