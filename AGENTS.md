# AGENTS.md — Project guidelines

This file is for anyone (human or AI) changing Contra Karaoke. Read it together
with `ROADMAP.md` before starting. If the workspace has not been scaffolded yet,
start at Milestone M0 in the roadmap.

---

## 1. Project facts

- **Product:** Contra Karaoke — a contra-dance caller trainer.
- **What it does:** plays ABC-sourced music in sync with a dance card and shows
  each call early enough that dancers hear it before executing the move.
- **Targets:** Windows, Linux, web (WASM), Android.
- **Language/UI:** Rust, `eframe`/`egui`.
- **Version:** **0.1.0** until the project owner explicitly says otherwise.
- **License:** MIT. The existing `LICENSE` copyright line must not be changed.
- **Bundle identity:** `org.contrakaraoke.app` (Android), package prefix
  `contra-karaoke` / `contra-*`.
- **Default dance:** Another Last Dance (ContraDB 2409).
- **Default tune:** Sandy Boys (ABC Session Tunes 75815).

Do not rename the app, binary, Android ID, or default content without an
explicit instruction.

---

## 2. Non-negotiables

1. **Calls precede moves.** The lead is computed from the rendered call text
   (syllables, words, speech rate) and clamped to the configured 1–2 beat
   default. Do not change this to a fixed one-beat delay or to "on the beat".
2. **One dance beat is half a bar.** At BPM 240, one dance beat is 0.5 s, a
   standard 64-beat dance is 32 s, and a jig bar is 1.0 s. Keep reels and jigs
   equally danceable at the same BPM setting.
3. **The count-in is three dance beats of accompaniment**, built from the
   tune's first chord, before the loop.
4. **The music loop and dance share a 64-beat grid.** A standard 32-bar tune is
   one loop. Do not let them drift.
5. **Parser separation.** Dance and tune fetching/parsing lives behind
   `DanceProvider` / `TuneProvider`. The UI must not contain site-specific URLs,
   JSON shapes, or HTML selectors.
6. **Cache everything fetched.** A repeat request should not hit the network
   unless the cache is stale or the user refreshes.
7. **Permissive dependencies only.** MIT, Apache-2.0, BSD, ISC, Zlib, Unlicense,
   CC0, or equivalent. No GPL/AGPL/LGPL in shipped binaries without the owner's
   explicit decision.
8. **Do not redistribute third-party content.** Only the two attributed default
   items are bundled. Everything else is fetched and cached locally. Do not
   commit a soundfont, a dance database, or a tune database.
9. **Keep 0.1.0.** Nightly artifacts use dates and SHAs, not a new version.
10. **No secrets in the repository.** Proxy tokens, signing keys, and store
    credentials belong in CI secrets or local environment variables.

---

## 3. Repository layout and dependency rules

```
crates/core      domain, timing, aliases, roles, preferences
crates/music     ABC parsing/sequencing, arrangement, synth, MIDI
crates/sources   provider traits, ContraDB, ABC Session Tunes, HTTP, cache
crates/app       egui UI, transport, TTS, platform glue
xtask            packaging/changelog helpers
scripts          Scripts to aid agents in development
assets           bundled defaults and icons only
proxy            Cloudflare Worker for web CORS
fixtures         minimal attributed parser fixtures
docs             architecture, timing, sources, packaging
```

Allowed dependencies:

```
app -> sources -> core
app -> music   -> core
app -> core
```

Rules:

- `core` must not depend on `egui`, `cpal`, `reqwest`/`ehttp`, `tts`, `std::fs`,
  or any platform API. It is pure domain logic.
- `music` must not depend on `egui`, HTTP, or platform APIs. It is deterministic
  and testable offline.
- `sources` may depend on HTTP abstractions but must not depend on `egui`.
  Parsers are pure functions.
- `app` is the only crate that performs I/O, opens windows, plays audio, or
  speaks.
- Prefer small modules with explicit public types over large "util" modules.
- Do not add a crate if the standard library or an existing dependency is a
  reasonable fit.
- Add `#![forbid(unsafe_code)]` to `core`, `music`, and `sources`. `unsafe` is
  allowed only in narrow platform glue in `app`, with a safety comment.

---

## 4. Toolchain and commands

Use a recent stable Rust toolchain. Expected MSRV is **1.95** (driven by
`eframe`/`egui` 0.37 and `cpal` 0.18); confirm the exact floor during M0 and
update `rust-version` in `Cargo.toml` and CI when a dependency raises it.

Edition: **2024** (or resolver 2 where required by a dependency).

Once the workspace exists, the standard commands are:

```sh
cargo fmt --all
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace
cargo run -p contra-app
```

Web:

```sh
rustup target add wasm32-unknown-unknown
trunk serve
trunk build --release
```

Android:

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo apk build --release -p contra-app --lib
```

Dependency hygiene:

```sh
cargo deny check
cargo machete
```

Do not commit if `fmt`, `clippy`, or tests fail. Do not disable a lint globally
to silence one call site; fix the cause or use a narrowly scoped `allow` with a
comment.

---

## 5. Rust coding standards

- Use `thiserror` for library errors and `anyhow`/`color-eyre` at the app
  boundary. No `unwrap()` or `expect()` in UI or real-time paths unless the
  invariant is documented and tested.
- No panics in parsers. Return `Result` with a useful context (source URL,
  field, row).
- The audio callback must not allocate, lock, log, or block. It reads from a
  pre-rendered buffer and updates atomics.
- The UI thread must not perform network I/O or long audio rendering. Use a
  worker thread (native) or an async/chunked path (WASM), report progress, and
  repaint.
- Time units are explicit. Use `Beats(f64)`, `Seconds(f64)`, and
  `Samples(u64)` newtypes (or clearly named aliases) and convert in one place.
  Never pass a bare `f64` between modules without saying which unit it is.
- Prefer `f64` for musical time computations; convert to samples at the render
  boundary. Use integer samples for the transport position.
- Serialize preferences with `serde` and include a schema version. Add a
  migration when a field changes.
- Public items in library crates get doc comments. Explain "why" in comments,
  not "what".
- Keep functions small enough to test; extract pure helpers from UI code.
- Use `tracing`/`log` for diagnostics. Do not log ABC bodies, tokens, or user
  file contents.
- Do not use `println!` for production diagnostics.

---

## 6. Timing invariants and tests

These must remain true:

```
seconds_per_dance_beat = 120 / bpm
count_in_beats          = 3
music_loop_beats        = 64
lead_beats             in [min_lead, max_lead]   // default 1..2
call_start_beat         = execute_beat - lead_beats
```

- Lead is computed from the rendered alias, not the original figure.
- The first call may be negative; do not clamp it to 0.
- Calls must stay sorted and must not duplicate.
- A figure with an explicit sub-move separator may yield more than one call;
  "balance & swing" stays one call.
- When the dance total is not 64, use cumulative figure beats for calls and keep
  the 64-beat music grid. Show a warning; do not silently rescale.
- Any change to timing or aliases needs unit tests with:
  - a one-syllable call,
  - a long multi-word call,
  - a negative first-call beat,
  - a 32-beat and a 64-beat dance,
  - a non-4/4 meter.
- Add a snapshot/golden test when the call timeline output changes.

---

## 7. Music and audio rules

- ABC parsing is deterministic and offline; never fetch inside `music`.
- Normalize messy ABC (chord symbols, inline fields, Unicode fractions) before
  sequencing.
- Prefer the tune's quoted chords. If they are missing, infer and label the
  result as inferred.
- The arranger produces exactly 64 dance beats. Pad/repeat/trim at bar
  boundaries; never cut mid-note without a fade.
- The count-in is accompaniment-only and uses the first chord.
- Built-in synth is the default. A user-supplied SF2 is optional; do not bundle
  one.
- Map instruments to GM programs when an SF2 is active and fall back to the
  built-in voice when a program is missing.
- Render offline and loop the PCM buffer. The UI clock derives from the audio
  playback position, not wall-clock time.
- Keep MIDI export deterministic and musically identical to the arranged loop.
- Add tests that assert loop length, bar alignment, and non-silent output.
- Do not allow clipping; leave headroom and apply a gentle limiter.

---

## 8. Data sources, HTTP, and caching

- All source-specific code lives in `crates/sources`.
- Providers expose generic filter specs so the UI can render search forms
  without knowing a provider's API.
- Use a descriptive `User-Agent` and respect rate limits. Do not crawl whole
  sites in the app.
- Fetch only what the user requests; do not prefetch the whole catalog.
- Cache raw and parsed responses. Include the provider ID in cache keys.
- TTL: 30 days for details, 7 days for searches. Manual refresh bypasses TTL.
- Never cache an error page as a valid result.
- On network failure, fall back to cache and show a non-blocking banner.
- URL parsing must accept the canonical source URL and extract the ID.
- Web builds use the configured proxy. The proxy is allowlisted to the two
  upstream hosts and the specific API paths; never turn it into an open proxy.
- Tests must use fixtures and must not make network calls.

Current source endpoints are documented in `ROADMAP.md` §7. If a site changes,
update the provider and its fixtures, not the session engine.

---

## 9. UI and theme rules

- Default theme is dark grey. Use the tokens in `ROADMAP.md` §8.1; do not
  hard-code colours in widgets.
- No accent colours. Distinguish active calls with contrast, weight, and
  border.
- Minimum 44 px touch targets; no hover-only controls.
- The app must be usable at phone width and with a soft keyboard open.
- Tabs at the top; session controls below; dance information occupies the
  remaining space.
- The pickers are modal/sheets; they must not discard the current session.
- Show explicit states: idle, preparing, count-in, playing, paused, finished,
  error.
- Space = play/pause, R = restart, Esc = close sheet.
- All controls need labels/accessibility names.
- Avoid flashing faster than 3 Hz.
- Do not use images for text or icons that could be rendered as text.

---

## 10. Preferences and aliases

- Preference order in the UI: role terms, aliases, instruments, theme.
- Persist preferences with a schema version and safe defaults.
- Role presets: Larks/Robins (default), Ladies/Gentlemen, Women/Men,
  Leads/Follows, Lefts/Rights, custom.
- Replace role terms in both the card and the call text, with word-boundary
  matching. Do not replace "shadow", "neighbor", or "partner".
- Alias templates support `{actor}`, `{direction}`, and `{count}`. Document
  placeholders in the UI.
- Every alias can be reset to its default. Custom aliases can be added for
  unmatched figure text.
- Rebuild the timeline when aliases, roles, or lead settings change.
- If TTS is enabled, stop speech on pause/stop and do not queue stale calls.

---

## 11. Platforms

- Windows: portable ZIP must not write to the registry; installer is Inno Setup.
- Linux: `.deb`, AppImage, and Flatpak are all required. Test on a clean
  container where possible.
- Web: WebGL2 baseline; no threads requirement; use the proxy for source data;
  degrade gracefully when a proxy is not configured.
- Android: minSdk 26 (AAudio/TTS). Use the native activity backend and a
  universal APK with arm64 + armv7. The SF2 picker must not block the rest of the
  app; fall back to the built-in synth.
- Keep platform-specific code in `app`, behind `cfg` or small traits. Do not
  sprinkle `cfg` through domain logic.
- macOS/iOS are not release targets yet, but avoid choices that make them
  impossible.

---

## 12. Testing requirements

- Add tests with every behavior change.
- Pure logic (`core`, `music`, parsers) needs unit tests; UI needs at least
  state-machine tests and a manual QA note.
- Use small, attributed fixtures. Do not commit full site dumps.
- No network in tests, including doctests.
- Test failure paths: malformed HTML, missing ABC fields, unsupported ABC,
  cache miss, SF2 program missing.
- Audio tests assert buffer length, peak/RMS, no NaN, and loop seam.
- Run `cargo fmt`, `clippy`, and `test` before declaring work done.
- If a test is flaky, fix or quarantine it with an issue; do not delete it.

---

## 13. Commits, changelog, and releases

Use Conventional Commits:

```
feat(timing): add syllable-based call lead
fix(sources): handle missing choreographer on contradb
docs(roadmap): record proxy decision
build(android): bump minSdk to 26
ci(nightly): attach checksums
test(music): cover 6/8 arrangement
```

- Keep commits focused. Do not mix refactors with behavior changes.
- Do not bump the version from 0.1.0 unless explicitly instructed.
- `CHANGELOG.md` is generated by `git-cliff`; do not hand-edit release sections
  unless fixing a generation error.
- Nightly releases build on every push to `main`, tagged
  `nightly-YYYYMMDD-<short-sha>` and refreshed under the rolling `nightly`
  release.
- Release artifacts: Windows installer + portable ZIP; Linux `.deb`, AppImage,
  Flatpak; web ZIP; Android APK.
- Do not publish a release, tag, or deploy the proxy without the owner's
  approval.

---

## 14. Licensing and attribution

- MIT for the application code.
- Update `CREDITS.md` whenever adding a dependency, font, tool, or data source.
  Include name, version, license, and URL.
- Permissive licenses only; run `cargo deny check`.
- Do not copy code from GPL projects or from the source websites without a
  compatible license.
- Bundled defaults must include source URL and attribution in `CREDITS.md` and
  in the app's about/credits view.
- The user is responsible for the license of any SF2 they load; say so in the
  UI.
- Do not strip license headers from vendored assets.

---

## 15. Security and privacy

- No telemetry, analytics, or crash reporting unless the owner explicitly adds
  it.
- No accounts; do not collect email addresses.
- Do not log tokens or full response bodies.
- Proxy tokens are user-supplied and stored locally; redact them in logs.
- Validate and cap response sizes before parsing.
- Treat HTML/JSON from the network as untrusted. No script execution, no
  `eval`, no shell.
- Keep dependencies minimal and run `cargo deny`/`cargo audit` in CI.

---

## 16. Definition of done

A change is done when:

- [ ] It matches `ROADMAP.md` and does not silently expand scope.
- [ ] Timing invariants still hold and are covered by tests.
- [ ] Parser/source logic stays behind the provider traits.
- [ ] No new network call in tests.
- [ ] `cargo fmt --check`, `clippy -D warnings`, and tests pass.
- [ ] New settings are persisted with a schema version.
- [ ] New UI follows the grey theme and is usable at phone width.
- [ ] `CREDITS.md` is updated if dependencies/assets changed.
- [ ] Docs are updated for user-visible behavior.
- [ ] The 0.1.0 version is unchanged unless instructed.
- [ ] No third-party content or soundfont was added to the repository.

---

## 17. When in doubt

- Prefer the simpler implementation that preserves the timing invariants.
- Prefer a pure function plus a test over a platform-specific shortcut.
- Prefer cached/default content and a clear error over a blocking network
  dependency.
- Ask before changing the app name, version, licensing, default content, or
  release process.
