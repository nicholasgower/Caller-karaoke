# Contra Karaoke

A caller-training app for contra dancing. It behaves like a karaoke machine for
callers: the dance card and a moving call prompt show what to say, while
ABC-sourced music plays in sync. Calls are timed to land **before** the dancers
execute each figure, with lead time derived from the words the caller will say.

The same Rust codebase is intended to target Windows, Linux, the web, and
Android. Version **0.1.0** is the current development version.

## What works now

- Native desktop application (Windows and Linux) built with `eframe`/`egui`.
- Browser build (WASM) with cpal WebAudio playback, localStorage prefs/cache,
  and the same UI.
- ContraDB dance search and HTML dance-card parsing.
- ABC Session Tunes search and JSON tune parsing.
- ABC → score → 64-beat arrangement → built-in synth playback.
- Three-beat accompaniment count-in built from the tune's first chord.
- On by default: small accompaniment variations each time the loop repeats —
  extra emphasis notes and, now and then, a bar left to the melody alone.
- Call timeline with syllable-based early calls, aliases, and role terms.
- Local Kokoro-82M neural voice on desktop, with a platform TTS fallback on
  web/Android and a reviewed dance-term pronunciation lexicon.
- User-supplied `.sf2` soundfont support via `rustysynth`.
- MIDI export of the arranged loop.
- Source cache with TTLs and offline fallback.
- Bundled defaults: *Another Last Dance* and *Sandy Boys*.

## Quick start

```sh
cargo run -p contra-app
```

You need a recent stable Rust toolchain (expected MSRV 1.95). On Linux, install
the audio and windowing development packages listed in `AGENTS.md` /
`.github/workflows/ci.yml`.

The app opens with the default dance and tune. Press **Play** to build and play
the loop. The first three beats are the count-in; the first call may be spoken
during it.

On the web, the browser needs a user gesture (the Play button) before it will
start audio. Online dance/tune search requires the proxy described below.

Android support is configured (`cargo-apk`, minSdk 26) but the APK is still
being device-tested.

### Controls

- **Play / Pause / Restart** — control the session.
- **Tempo** — 120–320 BPM (default 240).
- **Loop** — repeat the 64-beat music and dance loop.
- **Count-in** — three accompaniment beats before the loop.
- **Speak calls** — use the local neural voice (platform TTS on web/Android).
- **Alternative call aliases** — off by default. When on, moves with more than
  one caller wording randomly use one of them (for example “partner swing”
  instead of “swing your partner”). The card and TTS use the chosen wording.
  Add your own alternatives per move type in Preferences → Aliases.
- **Select dance** — search saved dances, organise them into folders, and load
  one. **Download dance** searches ContraDB, previews the card, and saves it.
- **Select tune** — search saved tunes, organise them into folders, and load
  one. **Download tune** searches ABC Session Tunes and previews the sheet
  music (or the ABC source).
- **Preferences** — role terms, aliases, instruments, theme, animations, lead
  settings, TTS, soundfont, proxy, cache tools, and accompaniment variations.

### Keyboard shortcuts

- **Space** — play or pause.
- **R** — restart from the beginning.
- **Esc** — return from a download screen to its saved-content list, or leave a
  picker or preferences screen; from the alias editor it returns to preferences.
- **1 / 2 / 3 / 4** — switch to Session, Select dance, Select tune, or
  Preferences.
- **Enter** — start a dance or tune search from any search field, create a
  folder, add a custom alias, or add an alternative call alias.

Shortcuts are ignored while typing in a text field, so Space and R stay ordinary
characters in search boxes. Hovering a tab, Play/Pause, Restart, Search, or Add
alias control shows its shortcut.

### Voice

On Windows and Linux, **Speak calls** uses a local Kokoro-82M neural voice. The
default is **Heart** (US female); Preferences also offers **Bella** and
**Nicole** (US female), and **Michael** and **Fenrir** (US male). The first
use of a voice downloads the model and that voice’s style file into the app
cache; later runs use the cache. The shared model is about 92 MB and each voice
style is about 0.5 MB; Linux also fetches the ONNX Runtime library (about
11 MB). The voice is phonemized with Misaki's pure-Rust
G2P, and the dance terms that the model mispronounced are corrected with a
reviewed lexicon (see `docs/pronunciation.md`). If the model cannot be
downloaded or loaded, the app falls back to the platform TTS backend and shows
the reason in Preferences.

The web and Android builds use the platform voice. Speech is pre-synthesized on
a worker thread and mixed into the existing audio stream, so the UI and audio
callback stay responsive.

## How call timing works

One **dance beat** is half a bar. At 240 BPM it lasts 0.5 s, so a standard
64-beat dance lasts 32 s. The count-in is three dance beats.

For each rendered call the app estimates syllables, converts the speech
estimate to seconds, and starts the call that many beats before the figure. The
defaults are calibrated from a measured 2.25 s delivery of "larks allemande
left once and a half" (nine syllables = 4.5 beats at 240 BPM), then a flat
0.5 s safety padding is added to every call. The speech-lead range is 1-5
beats; the maximum is adjustable up to 6 beats for learning or slower TTS
voices. When the dance loops, the first call is scheduled at the end of the
loop so it still precedes the first move. See `docs/INITIAL_PROMPT.md`,
`ROADMAP.md`, and `docs/call-timing.md` for the full design.

Example:

| Call | Lead | Starts |
| --- | ---: | ---: |
| `swing` | 2.0 beats | 1.0 s before the move |
| `circle left three` | 3.0 beats | 1.5 s before the move |
| `neighbors do-si-do once` | 4.0 beats | 2.0 s before the move |
| `larks allemande left once and a half` | 5.5 beats | 2.75 s before the move |

## Data sources

### ContraDB

- Search: `POST https://contradb.com/api/v1/dances`
- Detail: `GET https://contradb.com/dances/{id}`
- Default dance: <https://contradb.com/dances/2409> (*Another Last Dance*)

### ABC Session Tunes

- Search/detail: `https://backend.abcsessiontunes.com/api/tunes`
- Default tune: <https://abcsessiontunes.com/tunes/75815> (*Sandy Boys*)

Both sites are fetched by provider modules in `crates/sources`; adding another
site means implementing the same traits. Responses are cached locally.

### Web proxy

ContraDB sends no CORS headers, and ABC Session Tunes only allows its own
origin. Native and Android builds fetch directly. The web build needs the
included Cloudflare Worker proxy in `proxy/`; set its URL under
**Preferences → Web data proxy**. Without a proxy, the web build still works
with the bundled defaults and cached content.

## Soundfont

The built-in synthesizer needs no assets. To use realistic samples, load an
`.sf2` under **Preferences → Soundfont**. The file is copied/remembered locally
and is never bundled or redistributed by Contra Karaoke. You are responsible
for its license.

## Building and testing

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p contra-app
```

Web:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd crates/web && trunk build --release
```

Android (requires the Android SDK/NDK, cargo-apk, and minSdk 26):

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo apk build --release --lib -p contra-app
```

Windows installer and portable archive are built from
`packaging/windows/`; Linux `.deb`, AppImage, and Flatpak files live under
`packaging/linux/` and `packaging/flatpak/`.

## Agent automation (MCP)

The desktop app can expose a loopback inspection endpoint so an MCP client can
read the widget tree, synthesize input, and take screenshots. This is a
development tool: the `inspection` feature is off by default, and release builds
do not carry the listener.

```sh
# bash
EGUI_INSPECTION=127.0.0.1:5719 cargo run -p contra-app --features inspection
cargo run -p contra-mcp
```

```powershell
# PowerShell
$env:EGUI_INSPECTION = "127.0.0.1:5719"
cargo run -p contra-app --features inspection
cargo run -p contra-mcp
```

`contra-mcp` speaks MCP over stdio. The repository's `.mcp.json` registers it
for clients that read that file. Call `attach` in the client, then use
`screenshot`, `widget_tree`, `click`, `type_text`, `press_key`, `wait_for`, and
the other tools. See `docs/automation.md` for client configuration, the tool
list, and safety notes.

Documentation screenshots can also be rendered without a window or audio device,
then annotated with PhotoCraft:

```sh
cd website
npm run screenshots
```

## Documentation

The user guide is a Docusaurus site in `website/`; run `npm install && npm start`
there. It is published to GitHub Pages by `.github/workflows/docs.yml`.
Development notes live in `docs/`.

## Project documents

- `ROADMAP.md` — full product and milestone plan.
- `AGENTS.md` — architecture, code, testing, and release rules.
- `docs/INITIAL_PROMPT.md` — original project request.
- `docs/automation.md` — driving a development build from an MCP agent.
- `CREDITS.md` — libraries, fonts, data, and tools.
- `CHANGELOG.md` — change history.

## License

MIT. See `LICENSE`. Third-party libraries and data are credited in
`CREDITS.md`. Dance transcriptions remain the property of their contributors;
ABC transcriptions remain the property of their transcribers.
