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
- Call timeline with syllable-based early calls, aliases, and role terms.
- Optional text-to-speech through the platform TTS backend.
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
- **Speak calls** — use platform TTS instead of reading the prompt.
- **Select dance** — search ContraDB, preview the card, load it.
- **Select Tune** — search ABC Session Tunes, preview the ABC, load it.
- **Preferences** — role terms, aliases, instruments, theme, lead settings,
  TTS, soundfont, proxy, and cache tools.

## How call timing works

One **dance beat** is half a bar. At 240 BPM it lasts 0.5 s, so a standard
64-beat dance lasts 32 s. The count-in is three dance beats.

For each rendered call the app estimates syllables and words, converts the
speech estimate to seconds, and starts the call that many beats before the
figure. The default range is clamped to 1–2 beats; the maximum is adjustable in
Preferences for learning or slower TTS voices. See `docs/INITIAL_PROMPT.md`,
`ROADMAP.md`, and `docs/call-timing.md` for the full design.

Example:

| Call | Lead | Starts |
| --- | ---: | ---: |
| `swing` | 1.0 beat | 0.5 s before the move |
| `circle left three` | 1.6 beats | 0.8 s before the move |
| `neighbors do-si-do once` | 2.0 beats | 1.0 s before the move |

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

Windows installer and portable archive are built from
`packaging/windows/`; Linux `.deb`, AppImage, and Flatpak files live under
`packaging/linux/` and `packaging/flatpak/`.

## Project documents

- `ROADMAP.md` — full product and milestone plan.
- `AGENTS.md` — architecture, code, testing, and release rules.
- `docs/INITIAL_PROMPT.md` — original project request.
- `CREDITS.md` — libraries, fonts, data, and tools.
- `CHANGELOG.md` — change history.

## License

MIT. See `LICENSE`. Third-party libraries and data are credited in
`CREDITS.md`. Dance transcriptions remain the property of their contributors;
ABC transcriptions remain the property of their transcribers.
