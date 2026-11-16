## 10. Testing strategy

### Unit tests

- Syllable estimation and lead calculation, including clamping and negative
  first-call beats.
- Role-term replacement across all presets and custom terms.
- Alias classification and template rendering.
- Timeline construction for the default dance and synthetic edge cases.
- Call variants, collision-free scheduling, call rhythm slots, thinning, phrase
  validation, and the first-call fit (§17).
- ABC sequencing: keys, accidentals, ties, repeats, endings, parts, tuplets,
  chords, pickups.
- Chord extraction and inference.
- Arrangement: 8-, 16-, 24-, 32-, and 48-bar tunes; 64-beat alignment.
- ContraDB filter serialization and response parsing.
- ABC Session Tunes query building and response parsing.
- Cache hit/miss/eviction and TTL behaviour.
- Preferences round-trip and version migration.

### Fixtures

- Synthetic, minimal HTML/JSON fixtures for parser tests.
- One attributed real dance and one attributed real tune for integration tests.
- No test may hit the network.

### Snapshot/golden tests

- Call event timelines.
- Arranged MIDI note counts and bar alignment.
- Rendered audio buffer length and peak/RMS sanity checks.

### Manual QA

- Count-in, first call, loop seam, pause/resume.
- Call along to the default dance at 224, 232, and 240 BPM with each call-support
  level; compare the result with a recording and, where possible, an experienced
  caller (§17).
- Tempo changes and instrument changes.
- TTS timing and stop behaviour.
- Small-screen layout and soft keyboard.
- Offline start and cached content.
- Soundfont load/remove/fallback.
- Tab and screen animations, including with **Smooth animations** turned off.
- About section: sits at the bottom of Preferences, shows the author and
  project links, and neither blocks focus nor suspends transport shortcuts.
