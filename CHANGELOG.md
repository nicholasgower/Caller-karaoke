# Changelog

All notable changes to Contra Karaoke are documented here. The format follows
Keep a Changelog and the project uses Conventional Commits.

## [Unreleased]

## [0.1.0] — 2026-11-14

### Added

- Rust workspace with domain, music, source, and application crates.
- Contra dance card model, role terms, call aliases, syllable estimation, and
  syllable-based early call timing, calibrated to a measured 2.25 s delivery of
  “larks allemande left once and a half”.
- ABC parsing (notes, chords, repeats, ties, keys, meters), chord inference,
  and a 64-beat arrangement that plays a pickup only during the count-in and
  never repeats it.
- Built-in fiddle/accordion/piano/guitar synthesizer and offline PCM rendering.
- Optional user-supplied SoundFont rendering through `rustysynth`.
- Standard MIDI File export.
- ContraDB search and HTML dance-card parser.
- ABC Session Tunes search and tune parser.
- Local source cache with TTLs and offline fallback.
- egui desktop interface: session screen, dance picker, tune picker,
  preferences, and alias editor.
- Optional platform text-to-speech.
- Bundled defaults: Another Last Dance and Sandy Boys.
- Browser (WASM) build with WebAudio playback, localStorage prefs/cache, and
  Trunk packaging.
- Cloudflare Worker CORS proxy for web source access.
- Windows/Linux packaging scaffolding, CI, and nightly release workflow.
