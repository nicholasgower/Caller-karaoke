# Flatpak

The manifest expects a `cargo-sources.json` in this directory. Generate it from
the workspace lockfile in CI or locally:

```sh
curl -L --fail -o flatpak-cargo-generator.py \
  https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
python3 flatpak-cargo-generator.py ../../Cargo.lock -o cargo-sources.json
```

Then build:

```sh
flatpak-builder --force-clean --repo=repo build-dir \
  packaging/flatpak/org.contrakaraoke.ContraKaraoke.yml
flatpak build-bundle repo \
  dist/ContraKaraoke-0.1.0.flatpak \
  org.contrakaraoke.ContraKaraoke
```

The manifest runs Cargo offline against the generated sources. The desktop voice
downloads ONNX Runtime (about 11 MB) and the Kokoro model on first use; the
manifest already has network access for dance and tune sources. Speech
Dispatcher is the platform-TTS fallback; the finish args expose the service.
