## 12. Milestones

### M0 — Platform spike

Goal: prove the four-target stack before writing product code.

- [ ] Cargo workspace with shared version `0.1.0` and MIT metadata.
- [ ] Minimal `eframe` window on Windows and in the browser.
- [ ] Minimal Android build with `cargo-apk` (minSdk 26).
- [ ] `cpal` tone playback on Windows and browser; Android smoke test.
- [ ] `tts` "hello" on Windows, browser, and Android.
- [ ] Decide final MSRV (expected 1.95, driven by eframe).
- [ ] Document build commands in `docs/architecture.md`.

Exit: a "hello" app with sound and speech on at least Windows, web, and
Android.

### M1 — Core domain and timing

- [ ] `Dance`, `Figure`, `Call`, `Tune`, `SourceRef` models.
- [ ] Role-term presets and replacement.
- [ ] Move classifier and alias templates with placeholders.
- [ ] Syllable estimator and lead calculator.
- [ ] Timeline builder with sub-call splitting and 64-beat grid handling.
- [ ] Preference schema, defaults, and migration hook.
- [ ] Unit tests for all of the above.

Exit: the default dance produces a correct, test-covered call timeline for
multiple tempo/alias settings.

### M2 — Music engine

- [ ] ABC normalization and chord-symbol extraction.
- [ ] `abc-parser` adapter and linear sequencer.
- [ ] 64-beat arranger with pickup, repeat, and trim handling.
- [ ] Chord track and accompaniment patterns.
- [ ] Built-in synth for fiddle/accordion/piano/guitar.
- [ ] User-supplied SF2 backend via `rustysynth`.
- [ ] Offline renderer and `cpal` transport with audio-clock sync.
- [ ] MIDI export.
- [ ] Golden tests for arrangement and audio sanity.

Exit: Sandy Boys plays as a 64-beat fiddle/piano loop with a three-beat
count-in, and the UI clock matches the audio.

### M3 — Sources, cache, defaults

- [ ] HTTP abstraction using `ehttp`.
- [ ] `DanceProvider`/`TuneProvider` traits and registry.
- [ ] ContraDB search and HTML detail parser.
- [ ] ABC Session Tunes search and detail parser.
- [ ] Filter specs and query serialization.
- [ ] File/localStorage cache with TTL and eviction.
- [ ] Bundled default dance and tune with attribution.
- [ ] Cloudflare Worker proxy and web settings.
- [ ] Parser fixtures and tests (no network).

Exit: desktop can search/select both sources; web works through the proxy and
falls back to bundled/cached content.

### M4 — UI, TTS, preferences

- [ ] Main session screen and responsive layout.
- [ ] Dark/light grey themes.
- [ ] Select dance sheet with filters, pagination, preview.
- [ ] Select tune sheet with filters, pagination, preview.
- [ ] Preferences in the specified order.
- [ ] Alias editor with live preview and reset.
- [ ] Role-term selector and custom terms.
- [ ] TTS toggle, voice/rate, and stop-on-pause.
- [ ] Loading, empty, error, and offline states.
- [ ] Keyboard shortcuts and accessibility labels.

Exit: the full training loop is usable on desktop and mobile widths, with no
placeholder screens.

### M5 — Packaging and automation

- [ ] Windows Inno Setup installer and portable ZIP.
- [ ] Linux `.deb`, AppImage, and Flatpak.
- [ ] Web bundle and hosted copy in the documentation site.
- [ ] Android APK with minSdk 26.
- [ ] `ci.yml`, `nightly.yml`, `changelog.yml`.
- [ ] `git-cliff` config and `CHANGELOG.md`.
- [ ] `CREDITS.md`, README, and docs.
- [ ] Artifact checksums and release notes.

Exit: a push to `dev` produces a complete nightly release.

### M6 — Release hardening

- [ ] Real-device Android audio and TTS test.
- [ ] Web test on Chrome and Firefox.
- [ ] SF2 load/remove test with a large soundfont.
- [ ] Timing validation against a metronome and a human caller.
- [ ] Performance: render time, memory, startup.
- [ ] Cache size and offline behaviour.
- [ ] Legal/attribution review.
- [ ] Tag 0.1.0 (only when the owner approves).

Exit: all release-checklist items pass.
