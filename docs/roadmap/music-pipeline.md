## 6. Music pipeline

### 6.1 ABC to events

1. Normalize the ABC text.
2. Extract quoted chord symbols and attach them to note positions.
3. Parse headers and body with `abc-parser`.
4. Build a linear event stream:
   - note pitch and duration,
   - rests,
   - chords/unisons,
   - ties,
   - accidentals and key signatures,
   - repeats, first/second endings, and `P:` parts,
   - broken rhythm, tuplets, grace notes, decorations.
5. Convert ABC lengths to dance beats:

```
quarters_per_bar        = numerator * 4 / denominator
dance_beats_per_quarter = 2 / quarters_per_bar
```

This makes one bar = 2 dance beats in every supported meter.

6. Produce a `Score` of `NoteEvent`s plus a `ChordTrack`.

If `abc-parser` cannot handle a tune, report a clear error with the source URL
and keep the previous tune loaded. Do not panic.

### 6.2 Arrangement

- The session loop is **32 bars = 64 dance beats**.
- Linearize the tune. If it is shorter than 32 bars, repeat it at bar
  boundaries. If it is longer, take the first 32 bars, preferring complete
  parts.
- Align the loop to bar boundaries.
- Preserve pickup notes separately: they play during the count-in only and are
  **not** included when the music loop repeats. The tune body itself must still
  fill the 64-beat grid; fix a short B ending in the ABC rather than hiding it
  by repeating the opening notes.
- Apply a short fade at loop boundaries to avoid clicks.
- Chords: use quoted symbols; when absent, infer a simple diatonic progression
  from the key and strong-beat melody notes, and tell the user.
- Compare the dance's total beats with the tune body length (excluding the
  pickup). Show a warning when they differ; do not silently rescale either one.
- Arrangement warnings (extended/trimmed tune) appear in the tune picker.

### 6.3 Accompaniment

Generate a backing track from the chord track:

- Reels/4/4: bass note and chord alternation, with guitar strums and piano
  block chords.
- Jigs/6/8: compound-meter pattern with bass on the strong pulse and chord on
  the weak pulse.
- 2/2: half-note bass/chord pattern.
- 3/4: one chord per bar with a bass note on beat 1.
- Respect melody/accompaniment volume settings.

### 6.4 Count-in

- Three dance beats before the loop.
- Accompaniment only.
- First chord from the chord track at beat 0; fall back to the tonic triad.
- Visual 3-2-1 and optional "Get ready: <first call>" text.

### 6.5 Synthesis

Rendering uses `rustysynth` (MIT) with one of two voice sets:

1. **Bundled Fluid R3 GM subset (default):**
   - `assets/defaults/fluid-r3-gm-subset.sf2` — FluidR3_GM by Frank Wen, MIT.
   - Trimmed with `scripts/soundfont-subset.py` to the programs the
     preferences offer: Violin (GM 40), Accordion (GM 21), Acoustic Grand
     Piano (GM 0) and Acoustic Guitar steel (GM 25).
   - ~15 MB. The subset keeps the selected presets' generators, modulators and
     sample data unchanged, so it renders identically to the full 141 MB bank
     for those programs (`crates/music/tests/bundled_soundfont.rs`).
   - Attribution lives in `CREDITS.md` and the About window.
2. **User-supplied SoundFont (`.sf2`):**
   - Loaded from Preferences; overrides the bundled subset for the session.
   - The user is responsible for the file and its license.
3. **Built-in procedural synth (fallback):**
   - Fiddle: band-limited bowed saw, vibrato, body filter, bowed envelope.
   - Accordion: detuned reeds, tremolo, fast attack.
   - Piano: additive partials, inharmonicity, exponential decay, hammer noise.
   - Guitar: Karplus-Strong pluck and strum.
   - Used when neither font is available, so the app still plays.

Do not add a full soundfont bank to the repository. A new bundled soundfont
must be permissively licensed, trimmed to the programs used, and attributed.

Soundfont file selection:

- Windows/Linux: `rfd` file dialog.
- Web: hidden file input; keep in memory for the session and persist to
  IndexedDB when size/quota permit.
- Android: system document picker via a small JNI intent; fallback is scanning
  `files/soundfonts/` in the app data directory.

### 6.6 Playback and clock

- Render the count-in plus loop offline into a stereo PCM buffer.
- Play through `cpal`; the audio callback only reads from the buffer and updates
  atomics (no allocation, locks, or logging).
- The UI clock comes from the audio playback position, with latency
  compensation. Target visual accuracy: ±20 ms.
- Pause outputs silence but keeps the stream alive; restart resets the sample
  position.
- Tempo, instrument, or soundfont changes re-render and resume at the same
  beat instead of restarting the loop.
- Show a "Preparing music…" state while rendering.

### 6.7 Slow ending

- The rendered buffer appends a once-only ending after the loop passes, so the
  transport can request it without another render.
- Requesting the ending splices into it at the start of the tune's last bar;
  that bar is stretched with a falling tempo (a ritardando) and the final
  harmony is held for eight dance beats before a short fade.
- The ending leaves the 64-beat grid. The UI reports the loop end, stops calls,
  and shows a slow-finish state while it plays.
- With looping off, the ending replaces the abrupt stop at the loop boundary.

### 6.8 Tune rotation

- A session may keep an ordered set of saved tunes and change at a 64-beat
  boundary after a configurable number of passes (default 8).
- The next tune is parsed and rendered on a worker thread during the current
  tune's last pass, then installed at the boundary without a count-in.
- The call timeline does not change with the tune; calls keep their positions
  on the 64-beat grid.
- Rotation requires looping. The web build has no render worker and prepares
  the next tune at the boundary instead.

### 6.9 MIDI export

- Export the arranged 64-beat loop as a Standard MIDI File using `midly`.
- The once-only slow ending is a live transport action and is not exported.
- Two tracks: melody and accompaniment.
- Include tempo, meter, key, and program changes.
- The export must match what the synth plays.
