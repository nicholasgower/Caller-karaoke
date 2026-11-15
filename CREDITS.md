# Credits

Contra Karaoke is MIT-licensed. This file credits the libraries, fonts, data,
and build tools used by the project.

## Rust libraries

| Crate | License | Use |
| --- | --- | --- |
| `eframe` / `egui` | MIT OR Apache-2.0 | Desktop, web, and Android UI |
| `cpal` | Apache-2.0 | Audio output (WASAPI, ALSA, AAudio, WebAudio) |
| `abc-parser` | MIT | ABC syntax parsing |
| `midly` | Unlicense | Standard MIDI File writing |
| `rustysynth` | MIT | Optional SoundFont synthesis |
| `scraper` | ISC | ContraDB HTML parsing |
| `ehttp` | MIT OR Apache-2.0 | HTTP on native and WASM |
| `serde` / `serde_json` | MIT OR Apache-2.0 | Serialization |
| `regex` | MIT OR Apache-2.0 | Role-term and alias matching |
| `sha2` | MIT OR Apache-2.0 | Cache key hashing |
| `url` | MIT OR Apache-2.0 | Source URL construction |
| `percent-encoding` | MIT OR Apache-2.0 | Proxy URL encoding |
| `directories` | MIT OR Apache-2.0 | Platform data/cache directories |
| `rfd` | MIT | Native file dialogs |
| `tts` | MIT | Optional text-to-speech |
| `arc-swap` | MIT OR Apache-2.0 | Real-time-safe audio buffer swaps |
| `env_logger` | MIT OR Apache-2.0 | Logging setup |
| `thiserror` | MIT OR Apache-2.0 | Error types |

Transitive dependencies retain their own licenses; `Cargo.lock` records exact
versions. Run `cargo deny check` before a release.

## Fonts

`egui` bundles its default fonts:

- Ubuntu Light — Ubuntu Font Licence 1.0
- Hack — MIT / public-domain style license
- Noto Emoji — SIL Open Font License 1.1

See the `egui`/`epaint` source for the exact font files and notices.

## Data sources

- **ContraDB** — <https://contradb.com>. Dance transcriptions are contributed
  by ContraDB users. The bundled default *Another Last Dance* (dance 2409) is by
  Erik Hoffman, transcribed on ContraDB by Koren A Wake.
- **ABC Session Tunes** — <https://abcsessiontunes.com>. Tune transcriptions
  remain the property of their transcribers. The bundled *Sandy Boys* ABC is
  credited to Josh Larios; the ABC notes chords from Sarah Comer.
- **ABC notation standard** — <https://abcnotation.com>.

Contra Karaoke does not redistribute the source databases. Online content is
fetched on demand and cached locally.

The `samples/` directory contains a small curated set of downloaded dance and
tune pages used for parser tests during development. It is not part of the
application binary or of a normal release artifact.

## Build and release tools

- Inno Setup (Windows installer) — <https://jrsoftware.org/isinfo.php>
- Trunk (web build) — <https://trunkrs.dev>
- `cargo-apk`, `cargo-deb`, `linuxdeploy`, `appimagetool`, `flatpak-builder`
- `git-cliff` (changelog), `cargo-deny`, `cargo-machete`
- GitHub Actions

These tools are used at build/release time and are not part of the application
binary unless stated above.
