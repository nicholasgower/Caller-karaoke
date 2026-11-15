# Credits

Contra Karaoke is MIT-licensed. This file credits the libraries, fonts, data,
and build tools used by the project.

## Rust libraries

| Crate | License | Use |
| --- | --- | --- |
| `eframe` / `egui` | MIT OR Apache-2.0 | Desktop, web, and Android UI |
| `cpal` | Apache-2.0 | Audio output (WASAPI, ALSA, AAudio, WebAudio) |
| `abc-parser` | MIT | ABC syntax parsing |
| `acorde` | MIT OR Apache-2.0 | ABC-to-score engraving for the sheet-music preview |
| `acorde-render-svg` | MIT OR Apache-2.0 | Deterministic SVG sheet-music rendering |
| `resvg` / `usvg` / `tiny-skia` | MIT OR Apache-2.0 | SVG rasterization for the egui preview |
| `epaint_default_fonts` | MIT OR Apache-2.0 AND OFL-1.1 AND Ubuntu-font-1.0 | Shared egui font for sheet-music text |
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
| `tts` | MIT | Platform text-to-speech fallback (desktop, web, Android) |
| `misaki-rs` | MIT | Kokoro grapheme-to-phoneme (desktop neural voice) |
| `language-tokenizer` | WTFPL | Tokenizer used by `misaki-rs` |
| `ort` | MIT OR Apache-2.0 | ONNX Runtime bindings for the Kokoro voice |
| `ureq` | MIT OR Apache-2.0 | Voice model and runtime download |
| `flate2` | MIT OR Apache-2.0 | ONNX Runtime archive decompression (Linux) |
| `tar` | MIT OR Apache-2.0 | ONNX Runtime archive extraction (Linux) |
| `zip` | MIT | Build-time extraction of the ONNX Runtime DLL |
| `arc-swap` | MIT OR Apache-2.0 | Real-time-safe audio buffer swaps |
| `env_logger` | MIT OR Apache-2.0 | Logging setup |
| `thiserror` | MIT OR Apache-2.0 | Error types |
| `egui_inspection` | MIT OR Apache-2.0 | Development-only inspection plugin (eframe feature) |
| `egui_mcp` | MIT OR Apache-2.0 | MCP server for agent-driven UI testing |
| `rmcp` | Apache-2.0 | MCP protocol implementation (via `egui_mcp`) |
| `tokio` | MIT | Async runtime for the MCP server |
| `tracing-subscriber` | MIT | MCP server logging |

Transitive dependencies retain their own licenses; `Cargo.lock` records exact
versions. Run `cargo deny check` before a release.

## Fonts

`egui` bundles its default fonts:

- Ubuntu Light — Ubuntu Font Licence 1.0
- Hack — MIT / public-domain style license
- Noto Emoji — SIL Open Font License 1.1

See the `egui`/`epaint` source for the exact font files and notices.

## Voice model and runtime

- **Kokoro-82M** — <https://huggingface.co/hexgrad/Kokoro-82M>. Model weights and
  the voice styles (Heart, Bella, Nicole, Michael, Fenrir) are Apache-2.0. The
  desktop app downloads `model_quantized.onnx`, the selected voice `.bin`, and
  `config.json` from
  <https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX> on first use and
  caches them locally. They are not stored in this repository or in the source
  tree.
- **ONNX Runtime 1.31.0** — <https://onnxruntime.ai>. MIT licensed. The Windows
  build embeds the official `onnxruntime.dll` in the executable and unpacks it
  to the user cache on first use. Linux loads the official
  `libonnxruntime.so` dynamically; the app downloads and caches it on first use,
  so Flatpak and other offline package builds do not need a native runtime at
  build time.

  ```text
  MIT License

  Copyright (c) Microsoft Corporation. All rights reserved.

  Permission is hereby granted, free of charge, to any person obtaining a copy
  of this software and associated documentation files (the "Software"), to deal
  in the Software without restriction, including without limitation the rights
  to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
  copies of the Software, and to permit persons to whom the Software is
  furnished to do so, subject to the following conditions:

  The above copyright notice and this permission notice shall be included in all
  copies or substantial portions of the Software.

  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
  IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
  FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
  AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
  LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
  OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
  SOFTWARE.
  ```

- **Misaki G2P** — <https://github.com/hexgrad/misaki>. The Rust port
  (`misaki-rs`) is MIT licensed. Its embedded pronunciation dictionaries are
  derived from Misaki's Apache-2.0 data.
- **Pronunciation review** — see `docs/pronunciation.md` for the dance terms
  that needed an override and the tests that pin them.

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
