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
- Respect the band's per-instrument volume settings; percussion has its own
  volume.
- Lift the melody above the accompaniment and accent it on the dance pulse.
- When variations are enabled, later passes add deterministic melody ornaments
  (diatonic grace notes, chord-tone doubles, occasional octave lifts) alongside
  the accompaniment variations. Each kind is a preference and can be turned off
  on its own: note-length articulation, double stops, grace notes, octave lifts,
  chord stabs, and the melody-alone bar. Existing note starts stay on the 64-beat
  grid.
- The melody-alone bar is only chosen where the tune itself has enough sounding
  notes to carry it. A variation must never turn a written rest into a silent
  measure.
- Keep the chordal accompaniment out of the melody's register: a chord tone
  closer than a minor third to the tune is moved down an octave. The bass and
  the countermelody already sit below the melody and are left alone.

The band is a user-editable list of slots. Each slot names a part (melody,
countermelody, accompaniment, or percussion), a voice, a player count, a level,
a side, and a depth. The part chooses the notes; the voice chooses the
timbre/General MIDI program. Two slots on the same part are independent
instruments, not a copy count: a second piano has its own level and place. The
default list is two melody fiddles, one piano, two countermelody fiddles, and
three foot taps.

A slot with more than one player is a section. Each player gets an independent
pan, a few milliseconds of constant latency, and a slight tuning or velocity
difference, so the section sounds wider rather than merely louder. The renderer
levels the section so its power matches a single player. The list must keep at
least one pitched slot; removing the last one falls back to a front fiddle.

The countermelody is derived from the tune rather than held as a pedal:

- One note per dance beat on the same 64-beat grid, so it introduces no new
  timing.
- Voice it a diatonic third below the melody note sounding at that beat, snapped
  to a nearby chord tone, so the line follows the tune instead of holding a
  pedal. Skip the beat rather than repeat a pitch, which leaves space.
- Rest in the same bar as the first accompaniment on a variation pass, so a
  "melody alone" bar really is alone.
- Derive it from the melody, key, and chord track only; the ABC needs no extra
  voice.

Percussion is optional and comes from the band list:

- **Foot taps** place a soft tap on every dance beat, accented at bar starts.
- **Drumset** adds kick, snare, and hi-hat. The hats subdivide each dance beat in
  half for simple meters and in thirds for compound meters, so jigs keep their
  lilt.
- Percussion starts with the count-in when enabled and does not play through
  the once-only slow ending.
- Render it with small procedural voices rather than a soundfont preset, so the
  bundled subset and user fonts both work.
- Give each percussion row its own volume, player count, side, and depth; it
  never changes the grid or the calls.
- Give it a rhythm menu like the accompaniment: match style, basic, on the beat,
  offbeats, half time, double time, shuffle, gallop, triplet, and train beat.
  Match style gives foot taps a gallop by default: one group per dance beat, an
  eighth note followed by two sixteenths. The count-in stays one clear note per
  beat.

Dancer sounds are a separate layer derived from the dance card:

- **The whole layer is temporarily disabled in 0.1.0.** The derived rhythms
  (claps on a balance's first two beats; one stomp at a figure's start and one
  on its last beat) do not match the sounds dancers actually make for the
  figures. `contra_core::dancer_sounds` returns no events; the switch
  `DANCER_SOUNDS_ENABLED` in `crates/core/src/dancer.rs` turns it back on once
  the rhythms are corrected (see §16). Foot stomps have their own
  `FOOT_STOMPS_ENABLED` because their timing is the known-wrong part.
- The derivation takes the dance card only (`contra_core::dancer_sounds`) and
  returns `DancerSoundEvent`s on the dance-beat grid. A plain dance produces no
  events.
- Render the sounds with short procedural voices after the effect chain, so
  reverb or echo cannot smear a clap past its beat. They never move the grid
  and add nothing to the count-in.
- The layer follows every loop pass and has its own level. It is mixed under
  the band and cannot create a silent bar.

### 6.4 Count-in

- Four dance beats before the loop.
- Accompaniment only.
- First chord from the chord track at beat 0; fall back to the tonic triad.
- Visual 4-3-2-1 and optional "Get ready: <first call>" text.

### 6.5 Synthesis

Rendering uses `rustysynth` (MIT) with one of two voice sets:

1. **Bundled Fluid R3 GM subset (default):**
   - `assets/defaults/fluid-r3-gm-subset.sf2` — FluidR3_GM by Frank Wen, MIT.
   - Regenerated with `scripts/rebuild-soundfont.py` from the pinned official
     bank, keeping the keep-list in `scripts/soundfont-keep.txt`: Acoustic Grand
     Piano (GM 0), Accordion (GM 21), Acoustic Guitar steel (GM 25), Violin
     (GM 40), Alto Sax (GM 65), Tenor Sax (GM 66) and Clarinet (GM 71).
   - ~22 MB. The subset keeps the selected presets' generators, modulators and
     sample data unchanged, so it renders identically to the full 141 MB bank
     for those programs (`crates/music/tests/bundled_soundfont.rs`).
   - Attribution lives in `CREDITS.md` and the About section of Preferences.
2. **User-supplied SoundFont (`.sf2`):**
   - Loaded from Preferences; overrides the bundled subset for the session.
   - The user is responsible for the file and its license.
3. **Built-in procedural synth (fallback):**
   - Fiddle: band-limited bowed saw, vibrato, body filter, bowed envelope.
   - Accordion: detuned reeds, tremolo, fast attack.
   - Sax (alto/tenor): square wave through a one-pole lowpass with vibrato and an
     octave edge.
   - Clarinet: hollow odd-harmonic square wave behind a darker lowpass.
   - Piano: additive partials, inharmonicity, exponential decay, hammer noise.
   - Guitar: Karplus-Strong pluck and strum.
   - Used when neither font is available, so the app still plays.

Percussion is always procedural, independent of the selected voice set: a soft
foot-tap thump, a pitch-dropping kick, a noise-and-body snare, and a high-passed
noise hi-hat. It is mixed into the rendered buffer after the melody and
accompaniment, so it works with the bundled subset and any user font.

After every voice and the percussion layer are mixed, a deterministic effect
chain applies chorus, echo, and reverb, then a peak limiter. Reverb defaults to
a medium room; echo defaults to a subtle slapback and chorus is off by default.
The settings are shared with MIDI export, where the same choices become General
MIDI sends. All-off leaves the dry buffer sample-for-sample unchanged.

Do not add a full soundfont bank to the repository. A new bundled soundfont
must be permissively licensed, trimmed to the programs used, and attributed.

Soundfont file selection:

- Windows/Linux: `rfd` file dialog.
- Web: hidden file input; keep in memory for the session and persist to
  IndexedDB when size/quota permit.
- Android: system document picker via a small JNI intent; fallback is scanning
  `files/soundfonts/` in the app data directory.

### 6.6 Output layout

The render follows the selected output mode:

- **Mono:** one channel, the level-preserving average of the stereo pair.
- **Stereo:** the default constant-power left/right pair, unchanged from the
  original render.
- **Surround:** five channels - front left, front right, centre, rear left, rear
  right. A slot's side sets its pan; its depth moves it from the front pair to
  the rear pair with an equal-power crossfade. The centre carries the phantom
  centre, and the rear pair is trimmed slightly.

Effects process every channel independently, so a five-channel render is not
read as two-channel frames. The cpal callback maps the rendered channels onto
the device: mono averages, stereo folds the surround frame to its front pair
plus a little centre and rear, and a multichannel device keeps the channel order
and leaves unused channels silent. The band master and the caller level are
separate gains, so turning the band down never hides a call.

### 6.7 Playback and clock

- Render the count-in plus loop offline into a PCM buffer with the configured
  channel count.
- Play through `cpal`; the audio callback only reads from the buffer and updates
  atomics (no allocation, locks, or logging).
- The UI clock comes from the audio playback position, with latency
  compensation. Target visual accuracy: ±20 ms.
- Pause outputs silence but keeps the stream alive; restart resets the sample
  position.
- Tempo, instrument, band, output, or soundfont changes re-render and resume at
  the same beat instead of restarting the loop. Caller and band levels apply
  immediately without a re-render.
- Show a "Preparing music…" state while rendering.

### 6.8 Slow ending

- The rendered buffer appends a once-only ending after the loop passes, so the
  transport can request it without another render.
- By default the ending is gradual: the transport splices in four dance beats
  before the loop end, the accompaniment thins to a single bass line, the tempo
  eases, and the final melody note rings into a held final chord. Turning the
  gradual option off restores the older shape, where only the last bar is
  replaced and stretched.
- The final chord resolves a perfect fifth below the written final chord by
  default. A separate option keeps the written chord.
- The ending leaves the 64-beat grid. The UI reports the loop end, stops calls,
  and shows a slow-finish state while it plays.
- With looping off, the ending replaces the abrupt stop at the loop boundary.

### 6.9 Tune rotation

- A session may keep an ordered set of saved tunes and change at a 64-beat
  boundary after a configurable number of passes (default 8).
- The next tune is parsed and rendered on a worker thread during the current
  tune's last pass, then installed at the boundary without a count-in.
- The call timeline does not change with the tune; calls keep their positions
  on the 64-beat grid.
- Rotation requires looping. The web build has no render worker and prepares
  the next tune at the boundary instead.
- The current tune gets the same treatment: an idle prefetch fills a RAM cache
  keyed by the tune text and the audio settings, so the first Play and repeat
  plays install the cached buffer. A playback-setting change renders the
  replacement on the worker while the old loop keeps playing, then installs it
  at the current beat. The web build renders that replacement synchronously.

### 6.10 MIDI export

- Export the arranged loop as a Standard MIDI File using `midly`.
- The once-only slow ending is a live transport action and is not exported.
- One track and one General MIDI channel per player in the band list, with that
  slot's program, level, and pan. Two pianos or a clarinet countermelody are
  explicit tracks, not a copy count.
- General MIDI has fifteen non-drum channels, so a band with more than fifteen
  pitched players exports its first fifteen players; the remaining rows are
  omitted from the file (playback still plays them).
- The percussion rows are merged onto the single General MIDI drum channel.
- Include tempo, meter, key, and program changes.
- Write the same variation passes as playback: one plain 64-beat pass when
  variations are off, four varied passes when they are on. The individual
  variation choices are applied too, so an exported file matches the loop the
  user hears.
- Add General MIDI effect sends (reverb 91, chorus 93, delay/echo 94) to every
  channel track. The default is a medium-room reverb with a subtle echo and no
  chorus. A GM2 reverb-type message selects the requested room, and the sends
  are replaced rather than stacked when an existing file is post-processed.
- The notes must match what the synth plays. The reverb, chorus, and echo
  settings are shared with playback, where the same chain is applied to the
  rendered buffer; in the file they become General MIDI sends.

### 6.11 Human timing

Playback can humanize the band slightly; the preference is
`humanize_timing`, on by default. The separate options are a tempo-drift switch,
a chord-spread switch, and an amount from 0 to 1 that scales both. MIDI export
is deliberately not humanized.

- **Per-voice deviation.** Melody, accompaniment, and countermelody each get
  their own slow tempo deviation, and every copy of a part gets its own
  independent curve. The deviation is a seeded sum of two slow sines (periods
  of roughly four and eight bars), bounded to ±0.35 % of the user-set tempo.
- **Per-voice PID.** A PID controller per player (`crates/music/src/humanize.rs`)
  treats the un-deviated beat grid as its setpoint. The accumulated timing
  error is integrated on an eighth-of-a-beat control step with `Kp = 0.5`,
  `Ki = 0.05` per beat, and `Kd = 0.02` beats, so a wandering player is pulled
  back over about sixteen bars instead of leaving a permanent offset.
- **Bounded onsets.** One note's total onset offset — deviation plus chord
  asynchrony — is capped at 12 ms and clamped inside the pass, so a note can
  never cross the loop boundary and the sample count does not change.
- **Chord asynchrony.** Notes that start together within a section are spread
  by a deterministic ±6 ms pitch-level offset. A note that starts alone, such
  as a melody note, is left exactly on the voice's curve.
- **Reproducible seed.** The seed is derived from the arrangement (meter, key,
  melody, chords) plus the voice and copy index, so two identical renders are
  bit-identical and a tune keeps its feel between renders.
- **Excluded.** The count-in stays exactly four beats so the potatoes state the
  tempo; percussion stays grid-exact because it is the audible grid; the
  once-only slow ending keeps its written ritardando. MIDI export writes the
  un-deviated grid.
- **Both renderers.** The built-in synth and the sampled soundfont renderer
  apply the same curves. A note's offset shifts its note-on and note-off
  together, so note durations stay sensible.
