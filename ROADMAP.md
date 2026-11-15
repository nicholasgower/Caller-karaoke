# Contra Karaoke — Roadmap

**Status:** 0.1.0 development — desktop and web MVP implemented; Android and
release hardening in progress · **Version:** 0.1.0 (unreleased) · **License:** MIT

Contra Karaoke is a caller-training app for contra dancing. It behaves like a
karaoke machine for callers: the dance card and a moving call prompt show the
caller what to say, while ABC-sourced music plays in sync. Calls are timed to
land **before** the dancers execute each figure, with lead time derived from how
long the words take to say.

The same Rust codebase targets Windows, Linux, the web (WASM), and Android.

---

## 1. Goals

1. Let a caller practise a real contra dance against real ABC tune playback.
2. Show calls early enough that dancers hear them before they must move.
3. Make the caller's spoken wording adaptable through aliases and role terms.
4. Work offline after content has been cached.
5. Keep dance/tune parsing replaceable so new sources can be added without
   touching the session engine or UI.
6. Ship as a small, MIT-licensed application with reproducible nightly builds.

### Non-goals for 0.1.0

- Authoring or editing dances and tunes inside the app.
- Multi-user accounts, cloud sync, or telemetry.
- Recording or scoring the caller's voice.
- iOS/macOS releases (the code should remain portable, but these are not
  release targets yet).
- Bundling a general-purpose soundfont or redistributing a third-party tune or
  dance database.

---

## 2. Locked product decisions

| Decision | Choice |
| --- | --- |
| Product name | **Contra Karaoke** |
| Crate/binary prefix | `contra-karaoke`, `contra-core`, `contra-music`, `contra-sources`, `contra-app` |
| Android application ID | `org.contrakaraoke.app` |
| Desktop UI toolkit | `eframe` / `egui` |
| Audio I/O | `cpal` |
| ABC parsing | `abc-parser` plus a project-owned sequencer |
| Default instruments | Fiddle (melody) and piano (accompaniment) |
| Soundfont support | Built-in synth by default; optional **user-supplied SF2** |
| Web data access | Small Cloudflare Worker CORS proxy; bundled defaults as fallback |
| Theme | Dark grey by default; light grey option |
| Version until further notice | **0.1.0** |
| Windows packaging | Inno Setup installer + portable ZIP |
| Nightly releases | Every push to `main`, dated tags plus a rolling `nightly` release |

The default dance is **Another Last Dance** (ContraDB dance 2409).
The default tune is **Sandy Boys** (ABC Session Tunes tune 75815).

---

## 3. User-facing feature set

### 3.1 Main session screen

- Top tab row: **Select dance**, **Select Tune**, **Preferences**.
- Session controls directly below the tabs:
  - Tempo (default 240 BPM), with stepper and slider.
  - Play/pause, restart, loop toggle.
  - TTS on/off.
  - Master volume and current tune/instrument summary.
- The remaining space is the dance area:
  - Title, choreographer, formation, source link.
  - Dance card grouped by A1/A2/B1/B2 with beat counts.
  - Large "current call" prompt, next-call preview, and a lead countdown.
  - Karaoke states: *upcoming*, *call now*, *executing*. Once a figure is
    executing, the prompt becomes an *upcoming move* box naming the move to
    call next.
  - Beat/progress bar and a 3-2-1 count-in display.
- On narrow screens the current call is pinned; the card scrolls below it.

### 3.2 Select dance

A search screen modelled on ContraDB:

- Text filters: title, choreographer, user, hook.
- Verified filter.
- Formation filter: improper, becket, proper, everything else.
- Figure filter: a move selector plus a "figure contains words" search.
- Sorting, page size (10/30/100), and pagination.
- Results show title, choreographer, formation, and hook.
- Selecting a result opens a preview of its parsed figures and a
  **Use this dance** action.
- The default dance is loaded on first run and remains usable offline.

### 3.3 Select tune

A search screen modelled on ABC Session Tunes:

- Free-text search across title and composer.
- Composer, key, meter, and genre filters.
- Pagination and result counts.
- Results show title, composer, key, meter, and rhythm.
- Selecting a result shows metadata and a rendered sheet-music preview, with a
  **Show ABC** / **Show sheet music** toggle for the source and a **Use this
  tune** action. The sheet is the default view; if it cannot be engraved the
  source remains available in the ABC view.
- The default tune is loaded on first run and remains usable offline.

### 3.4 Preferences

Listed in this order:

1. **Role terms** — default **Larks / Robins**.
   Presets: Ladies/Gentlemen, Women/Men, Leads/Follows, Lefts/Rights, plus
   custom left/right terms.
   Replace ContraDB's default "ladles"/"gentlespoons" (and related forms) in
   both the card and the alias text.
2. **Dance move aliases** — opens a sub-screen listing move types with editable
   alias templates, live previews, reset-to-default, and custom aliases for
   unmatched figures.
3. **Instrument preferences**
   - Melody: Fiddle (default), Accordion.
   - Accompaniment: Piano (default), Guitar, Accordion.
   - Accompaniment rhythm: match instrument (default), block chords,
     boom-chuck, offbeat stabs, arpeggio, shuffle, strum, sustained.
   - Melody/accompaniment volume.
   - Soundfont: None (built-in synth) or a user-loaded `.sf2`.
4. **Application theme** — Dark (default) or Light.
5. **Interface animations** — smooth tab and screen transitions, on by
   default; can be turned off to reduce motion.

Advanced settings:

- Calling lead: automatic or manual, minimum/maximum beats, syllables per
  second, word gap, extra pause.
- TTS voice (Kokoro Heart, Bella, Nicole, Michael, or Fenrir on desktop) and rate.
- Alternative call aliases (default off): randomly uses an alternative wording
  for moves that have one, such as “partner swing” instead of “swing your
  partner”. The chosen wording is shown and spoken, and its lead is computed
  from it. Callers can add their own alternatives per move type.
- Count-in on/off (default on), loop (default on).
- Accompaniment variations on each loop (default on): a few added emphasis
  notes and an occasional bar of melody alone, chosen deterministically from
  the tune.
- Cache size, clear cache, refresh cached content.
- Web data proxy URL and token (web build only).

### 3.5 TTS

- Optional setting: speak the alias text at the computed call time.
- The desktop build prefers the local Kokoro-82M neural voice. The model and
  voice style files are downloaded once, verified, and cached; the platform TTS
  backend remains the fallback (and the only backend on web/Android).
- Preferences offers five Kokoro voices: Heart, Bella, and Nicole (US female),
  and Michael and Fenrir (US male).
- Dance terms that the neural G2P mispronounces are corrected by a reviewed
  lexicon and pinned by tests (`docs/pronunciation.md`).
- The visual prompt still appears when TTS is enabled.
- If TTS is unavailable, the toggle is disabled with an explanation.
- Pausing/stopping stops speech immediately.

### 3.6 Aliases and calls

- Dance-card text is parsed into a move type plus parameters (actor, direction,
  count).
- Each move type has a default alias template. Templates support placeholders
  such as `{actor}`, `{actor_singular}`, `{actor_plural}`, `{actor_object}`,
  `{actor_object_plural}`, `{direction}`, and `{count}`.
- The caller can override any template and add aliases for figures the parser
  does not recognise.
- Each move type may also have built-in **alternative** templates. When the
  optional shuffle setting is on, each call picks one of the applicable
  templates (default plus alternatives) at random. The timeline stores the
  chosen wording, so the card, TTS, syllable count, and lead all agree.
- Callers can add their own alternative templates for any move type in the
  alias editor. An alternative that needs a placeholder the call lacks is
  skipped, and alternatives never repeat the active template.
- The rendered alias replaces the card text during a session; an optional
  advanced setting can show the original figure as secondary text.
- Lead timing is computed from the rendered alias, so making a call longer
  automatically moves it earlier.

Default alias style examples:

| Card figure | Default call |
| --- | --- |
| `neighbors do si do once` | `neighbors do-si-do once` |
| `ladles allemande left 1½` | `larks allemande left once and a half` |
| `partners balance & swing` | `partners balance and swing` |
| `circle left 3 places` | `circle left three places` |
| `neighbors swing` | `swing your neighbor` |
| `long lines forward & back` | `long lines forward and back` |

These defaults are meant to be editable; the exact list will be reviewed with
an experienced caller during Milestone 4.

---

## 4. Architecture

### 4.1 Workspace layout

```
contra-karaoke/
  Cargo.toml                 workspace, shared version 0.1.0
  crates/
    core/                    domain, timing, aliases, role terms, preferences
    music/                   ABC sequencing, arrangement, synth, MIDI export
    sources/                 provider traits, ContraDB, ABC Session Tunes,
                             HTTP, cache
    app/                     egui UI, transport, TTS, platform bootstrap
    web/                     WASM entry point and Trunk page
  xtask/                     packaging/changelog helper commands
  scripts/                   Scripts to aid agents in development
  assets/
    defaults/                bundled default dance and tune (attributed)
    icons/                   app icons
  proxy/                     Cloudflare Worker for web CORS
  fixtures/                  small, attributed parser test fixtures
  docs/                      architecture, call timing, sources, packaging
  .github/workflows/         CI, nightly, changelog, optional Pages deploy
  ROADMAP.md
  AGENTS.md
  CREDITS.md
  CHANGELOG.md
  README.md
```

### 4.2 Dependency direction

```
app  ──▶ sources ──▶ core
 │          │          ▲
 └────────▶ music ─────┘
```

Rules:

- `core` has no UI, HTTP, filesystem, audio, or platform dependencies.
- `music` depends only on `core` and pure-Rust libraries.
- `sources` depends on `core`; HTTP is abstracted behind a small client.
- `app` is the only crate that touches `egui`, `cpal`, `tts`, file dialogs, and
  platform storage.
- Parsers are pure functions: bytes in, domain model out. A parser never
  performs a request.

### 4.3 Data flow

```
UI action
  └─▶ SourceService (provider + cache + HTTP)
        └─▶ Dance / Tune domain models
              ├─▶ AliasEngine + RoleTerms
              ├─▶ PracticeTimeline (call events)
              └─▶ ABC parser ─▶ Score ─▶ Arranger (64-beat loop)
                                    └─▶ SynthBackend ─▶ PCM buffer
                                    └─▶ MIDI export (.mid)
        └─▶ Transport (cpal) ─▶ audio clock ─▶ karaoke timeline
```

### 4.4 Provider abstraction

Dance and tune sources implement traits. The UI never knows a provider's URL
shape or JSON/HTML format.

```rust
pub trait DanceProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn filter_specs(&self) -> Vec<FilterSpec>;
    fn search(&self, query: &DanceQuery) -> HttpCall;
    fn parse_search(&self, body: &[u8]) -> Result<DanceSearchPage>;
    fn detail(&self, id: &str) -> HttpCall;
    fn parse_dance(&self, body: &[u8], url: &str) -> Result<Dance>;
    fn parse_url(&self, url: &str) -> Option<String>;
}

pub trait TuneProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn filter_specs(&self) -> Vec<FilterSpec>;
    fn search(&self, query: &TuneQuery) -> HttpCall;
    fn parse_search(&self, body: &[u8]) -> Result<TuneSearchPage>;
    fn detail(&self, id: &str) -> HttpCall;
    fn parse_tune(&self, body: &[u8], url: &str) -> Result<Tune>;
    fn parse_url(&self, url: &str) -> Option<String>;
}
```

`FilterSpec` describes a field generically (text, select, checkbox, keyword
row), so the picker screens can render any provider's search options.

---

## 5. Timing model (critical element)

### 5.1 Definitions

- **Dance beat:** the ContraDB beat unit. It is half a bar in 4/4, 2/2, and
  6/8. A standard 32-bar tune is 64 dance beats.
- **Tempo:** `bpm` is the quarter-note pulse for reels (default 240). The
  dance-beat duration is derived, not the quarter note directly:

```
seconds_per_dance_beat = 120 / bpm
```

At 240 BPM a dance beat is 0.500 s, a 64-beat dance is 32 s, and a jig bar is
1.0 s (120 dotted-quarter beats per minute). This keeps reels and jigs at the
same dance duration for the same BPM value.

- **Count-in:** exactly 3 dance beats of accompaniment, built from the tune's
  first chord. The loop starts at dance beat 0 after the count-in.

### 5.2 Call lead

Calls must start before the dancers execute the move. The lead is based on the
rendered alias text.

```
syllables        = estimated syllables in the alias
words            = word count
speech_seconds   = syllables / syllables_per_second
                 + (words - 1) * word_gap_seconds
                 + extra_pause_seconds
speech_lead_beats = clamp(
                     (speech_seconds / seconds_per_dance_beat) * lead_scale,
                     min_lead_beats,
                     max_lead_beats
                   )
lead_beats       = speech_lead_beats + 0.5 s (as dance beats)
call_start_beat  = execution_beat - lead_beats
```

Defaults are calibrated from measured delivery: saying
“larks allemande left once and a half” (nine syllables) took **2.25 seconds**,
which is 4.5 dance beats at 240 BPM. A flat **0.5 s safety padding** is added
to every call after the speech-lead clamp.

| Setting | Default |
| --- | --- |
| syllables per second | 4.0 |
| word gap | 0.0 s |
| extra pause | 0.0 s |
| lead scale | 1.0 |
| min speech lead | 1.0 beat |
| max speech lead | 5.0 beats |
| safety padding | 0.5 s |

The max speech lead is user-adjustable up to 6 beats for learning sessions or
slower TTS voices; the padding is always added on top.

Example at 240 BPM:

| Alias | Lead | Call starts |
| --- | ---: | ---: |
| `swing` | 2.0 beats | 1.0 s before the move |
| `circle left three` | 3.0 beats | 1.5 s before the move |
| `neighbors do-si-do once` | 4.0 beats | 2.0 s before the move |
| `larks allemande left once and a half` | 5.5 beats | 2.75 s before the move |

The first call of a dance may fall at a negative beat (inside the count-in).
That is intentional and must be displayed/spoken. When the dance loops, the
same call is also scheduled at `music_loop_beats - lead_beats` so the first move
is called near the end of the previous loop. If the lead begins before the
transport can reach it — a lead longer than the count-in, or the count-in turned
off — the first call is spoken as soon as the run starts instead of being
dropped. Resuming a paused dance does not repeat it.

Syllable estimation:

- Lowercase, strip punctuation, count vowel groups per word.
- Subtract a silent trailing `e` where appropriate.
- Minimum one syllable per word.
- The estimator is approximate; aliases give the caller control.
- Unit tests must pin representative phrases.

### 5.3 Call timeline

```rust
pub struct CallEvent {
    pub figure_index: usize,
    pub call_start_beat: f32, // may be negative
    pub execute_beat: f32,
    pub end_beat: f32,
    pub text: String,
    pub syllables: u16,
}
```

Rules:

- Calls are sorted by `call_start_beat`.
- A figure may produce multiple calls when its text has explicit separators
  (` - ` or `;`).
- "balance & swing" remains a single call; it is one rhythmic phrase.
- Sub-move beat allocation is proportional, with small heuristics
  (e.g. a trailing "balance" gets about 2 beats).
- If an earlier call would overlap a later one, keep source order and enforce a
  small minimum gap; never reorder calls.
- The timeline is rebuilt whenever the dance, aliases, role terms, or lead
  settings change.

### 5.4 Portable grid

Dance cards do not always sum to exactly 64 beats. Use cumulative figure beats
for call placement, but keep a 64-beat music grid:

- If the card totals 64, it matches the loop exactly.
- If it totals less than 64, the music continues after the last call.
- If it totals more than 64, loop the music and continue the card.
- If the card totals 32 or less, repeat the card within each 64-beat music
  loop.
- Show a warning in the picker when a card's total is not 64.

### 5.5 Karaoke states

For current time `t` (in dance beats):

- **Upcoming:** `t < call_start_beat`.
- **Call now:** `call_start_beat <= t < execute_beat`; show a lead countdown.
- **Executing:** `execute_beat <= t < end_beat`.

The current call is rendered large. The next call is shown smaller. The active
figure row is highlighted in the card.

---

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

Two backends behind one trait:

1. **Built-in synth (default, no assets):**
   - Fiddle: band-limited bowed saw, vibrato, body filter, bowed envelope.
   - Accordion: detuned reeds, tremolo, fast attack.
   - Piano: additive partials, inharmonicity, exponential decay, hammer noise.
   - Guitar: Karplus-Strong pluck and strum.
   - Simple room reverb and a gentle limiter.
2. **User-supplied SoundFont (`.sf2`):**
   - Use `rustysynth` (MIT).
   - Melody: Violin (GM 40) or Accordion (GM 21).
   - Accompaniment: Acoustic Grand Piano (GM 0), Nylon/Steel Guitar (GM
     24/25), or Accordion (GM 21).
   - Fall back to the built-in voice if a program is missing.
   - Never bundle an SF2 file; the user supplies it and is responsible for its
     license.

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

### 6.7 MIDI export

- Export the arranged 64-beat loop as a Standard MIDI File using `midly`.
- Two tracks: melody and accompaniment.
- Include tempo, meter, key, and program changes.
- The export must match what the synth plays.

---

## 7. Data sources

### 7.1 ContraDB

Base: `https://contradb.com`

Search:

```
POST /api/v1/dances
Content-Type: application/json
{
  "count": 10,
  "offset": 0,
  "sort_by": "titleA",
  "filter": ["and", ["figure", "*"], ["formation", "improper"]]
}
```

Observed response:

```json
{
  "numberSearched": 2474,
  "numberMatching": 1234,
  "dances": [
    {
      "id": 3582,
      "title": "Let's Play Already",
      "choreographer_id": 465,
      "choreographer_name": "Michael Karcher",
      "formation": "improper",
      "hook": "",
      "user_id": 96,
      "user_name": "Sarah Trop",
      "created_at": "...",
      "updated_at": "...",
      "publish": "everywhere",
      "matching_figures_html": "whole dance"
    }
  ]
}
```

Filter expressions observed:

| Purpose | Expression |
| --- | --- |
| All figures | `["figure", "*"]` |
| Specific move | `["figure", "swing"]` |
| Figure keyword search | `["figure", "custom", "ladles chain"]` |
| Title contains | `["title", "last"]` |
| Choreographer contains | `["choreographer", "hoffman"]` |
| Formation | `["formation", "improper"]` |
| Public/shared | `["publish", "all"]` |
| Verified | `["tag", "verified"]` |
| Boolean combination | `["and", ...]`, `["or", ...]` |

Boolean operands are variadic and flat: `["and", clause, clause, ...]`. Nesting
them as `["and", [clause, clause]]` makes the endpoint answer HTTP 500.

Sort values use `<column><A|D>`, e.g. `titleA`, `titleD`.

Formation text is case-sensitive and validated server-side. The accepted values
are `improper`, `proper`, `everything else`, and `Becket *` (the wildcard covers
stored variants such as `Becket (ccw)`). An unrecognised value answers HTTP 500
rather than an empty page. Figure names are validated the same way (for example
`slide along set`, not `slide`).

Detail:

```
GET /dances/{id}
```

Parse the HTML:

- `h1.dance-show-title`
- `p.dance-show-choreographer`
- `p.dance-show-formation`
- `div.dance-show-preamble`
- `table tr` with `td.dance-show-beats` and `div.show-figure`
- `div.dance-show-notes`

Normalize entities, Unicode fractions, HTML emphasis, arrows, and progression
markers. Preserve the source URL for attribution.

Default dance: `https://contradb.com/dances/2409` — *Another Last Dance*.

### 7.2 ABC Session Tunes

Base: `https://backend.abcsessiontunes.com`

Search:

```
GET /api/tunes
  ?search=sandy
  &composer=
  &key_signature=Amix
  &meter=4/4
  &rhythm=reel
  &page=1
  &limit=25
  &sort=title
  &dir=asc
  &fields=summary
  &envelope=1
```

Response:

```json
{
  "tunes": [
    {
      "id": 75815,
      "title": "Sandy Boys",
      "composer": "",
      "key_signature": "A mix",
      "meter": "4/4",
      "rhythm": "",
      "origin": null,
      "createdAt": "2025-06-25T06:29:13.000Z"
    }
  ],
  "pagination": { "page": 1, "limit": 2, "total": 6, "pages": 3 }
}
```

Detail:

```
GET /api/tunes/{id}
```

Returns metadata plus `abc_content` (the ABC text), `source_url`, `notes`, and
transcription credit. Unknown metadata is sent as JSON `null` (for example
`composer`, `key_signature`, `meter`, `rhythm`), so parsers must treat `null`
like an empty field.

Filter option lists:

```
GET /api/filters/keys
GET /api/filters/meters
GET /api/filters/rhythms
```

These lists are large and noisy. The UI should present curated common values
plus a searchable selector; the API accepts arbitrary strings.

Default tune: `https://abcsessiontunes.com/tunes/75815` — *Sandy Boys*.

### 7.3 CORS and the web proxy

Verified in November 2026:

- `contradb.com` sends no `Access-Control-Allow-Origin`.
- `backend.abcsessiontunes.com` allows only `https://abcsessiontunes.com`.

Desktop and Android fetch directly. The web build needs a proxy:

- `proxy/` contains a Cloudflare Worker (MIT) with fixed upstream paths, CORS
  headers, an optional shared token, and short GET caching.
- The worker must not be an open proxy; only allow the two upstream hosts and
  the specific API paths used by the app.
- Preferences: **Web data proxy URL** and **token**.
- The web build may bake in a default proxy URL from an environment variable.
- Without a proxy, the web build still works with bundled defaults and cached
  content.

### 7.4 Cache

Cache raw and parsed responses:

- `dance/{provider}/{id}.json` and `.html`
- `tune/{provider}/{id}.json` and `.abc`
- `search/{provider}/{query-hash}.json`
- `prefs.json`

Rules:

- Read from cache first; revalidate in the background when online.
- Fall back to cached content on network failure.
- Detail TTL: 30 days; search TTL: 7 days.
- Manual "Refresh" bypasses TTL.
- LRU eviction with a size budget (native/Android 50 MB; web limited by
  localStorage/IndexedDB quota).
- Never cache an HTML/JSON error page as a valid result.
- Cache keys include the provider ID so sources cannot collide.
- Bundled defaults are separate from the cache and always available.

### 7.5 Adding another source

1. Implement `DanceProvider` or `TuneProvider` in `sources`.
2. Add pure parsing functions and fixture tests.
3. Describe search fields with `FilterSpec`.
4. Register the provider.
5. Implement URL parsing for direct links.
6. Add attribution and any source-specific rate-limit rules to `CREDITS.md`.
7. Do not change `core`, `music`, or the UI to add a source.

---

## 8. UI and visual design

### 8.1 Theme tokens

Dark (default):

| Token | Value |
| --- | --- |
| Background | `#141414` |
| Panel | `#232323` |
| Button | `#2e2e2e` |
| Button hover | `#3a3a3a` |
| Button active | `#4a4a4a` |
| Primary text | `#e6e6e6` |
| Muted text | `#9a9a9a` |
| Active call | `#f2f2f2` on `#303030` |
| Call-now prompt card | `#1e4a2e` (green fill, white text) |
| Upcoming-move prompt card | `#602424` (red fill, white text) |
| Borders | `#3a3a3a` |

Light theme uses the same neutral structure with inverted values. No accent
colours; the active call is distinguished by contrast, weight, and a border.
All colours live in a theme module; do not hard-code colours in widgets.

The one deliberate exception is the large prompt card: it is green while the
caller must speak the shown call ("CALL NOW") and red while it looks ahead to
the move after the one being danced ("UPCOMING MOVE"), so the state is
readable at a glance across the room. The label text still states the state,
so the fill is never the only cue, and both fills keep white text at 4.5:1 or
better in the dark theme and dark text at 4.5:1 or better in the light theme.

### 8.2 Responsive behaviour

- Breakpoint: approximately 640 px.
- Tabs remain horizontally usable on mobile (scroll if necessary).
- Session controls wrap into two rows on narrow screens.
- Minimum 44 px touch targets.
- Avoid hover-only interactions.
- The dance area scrolls; the current call stays visible.
- The pickers become full-screen sheets on mobile and dialogs on desktop.
- Respect safe-area insets on mobile.

### 8.3 Accessibility

- Keyboard: Space = play/pause, R = restart, Esc = back, 1–4 = switch tabs,
  Enter = submit a search or add an alias. Shortcuts are suspended while a text
  field has focus.
- Every control has a text label or accessibility label.
- Text contrast meets WCAG AA against the chosen background.
- TTS is optional and never the only way to receive a call.
- Do not flash or pulse faster than 3 Hz.

### 8.4 Error and loading states

- Network error: keep cached content, show a dismissible banner with retry.
- Parse error: show the source URL and a short reason; do not replace the
  current dance/tune.
- Audio render: "Preparing music…" with a cancel action.
- No soundfont: built-in synth remains active.
- No TTS: disable the toggle and explain why.

---

## 9. Platforms, packaging, and releases

### 9.1 Targets

| Platform | Build | Artifact |
| --- | --- | --- |
| Windows x64 | `cargo build --release` + Inno Setup | `ContraKaraoke-<version>-x64-setup.exe`, `...-portable.zip` |
| Linux x86_64 | `cargo build --release` | `.deb` via `cargo-deb`, `.AppImage`, `.flatpak` |
| Web | `trunk build --release` + `wasm-opt` | `contra-karaoke-web.zip`, optional GitHub Pages deploy |
| Android | `cargo apk build --release` | universal APK (arm64 + armv7), minSdk 26 |

Notes:

- Windows: Inno Setup for the installer; ZIP is portable (no registry writes).
- Linux: AppImage via `linuxdeploy`/`appimagetool`; Flatpak manifest under
  `packaging/flatpak/`; `.deb` metadata under `packaging/deb/`.
- Web: `eframe` with the `glow` renderer; WebGL2 is the baseline.
- Android: `eframe` with `android-native-activity` and `glow`; `cpal` uses
  AAudio; `tts` requires minSdk 26.
- No macOS/iOS artifacts in 0.1.0, but do not add platform-specific code that
  would prevent them.

### 9.2 CI

**`ci.yml`** on pull requests and pushes:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo check --target wasm32-unknown-unknown` for the web crate
- Android check/build smoke test
- `cargo deny check` with an allowlist of permissive licenses
- `cargo machete`

**`nightly.yml`** on every push to `main`:

- Build all release targets.
- Generate release notes with `git-cliff`.
- Create a prerelease tagged `nightly-YYYYMMDD-<short-sha>`.
- Update the rolling `nightly` release.
- Upload all artifacts with checksums.

**`changelog.yml`** on push to `main`:

- Regenerate `CHANGELOG.md` with `git-cliff`.
- Commit with `[skip ci]` and path filters to avoid loops.
- Nightly builds ignore changelog-only commits.

Optional **`pages.yml`** deploys the web build to GitHub Pages.

### 9.3 Versioning

- Stay at **0.1.0** until the project owner declares a release.
- Nightly artifacts carry the date and short SHA, not a new semantic version.
- The changelog keeps an `[Unreleased]` section.
- Release notes are generated from Conventional Commits.

### 9.4 Release checklist

- [ ] `cargo fmt`, `clippy`, and tests pass on Windows and Linux.
- [ ] Web build loads and plays audio in Chrome and Firefox.
- [ ] Android APK installs and plays audio on a real device.
- [ ] Default dance and tune load offline.
- [ ] Proxy-backed search works in the web build.
- [ ] SF2 loading works on Windows, Linux, and web; Android path is verified or
      documented as a known limitation.
- [ ] `.deb`, AppImage, Flatpak, installer, portable ZIP, web ZIP, and APK are
      attached to the release.
- [ ] `CREDITS.md`, `LICENSE`, and `CHANGELOG.md` are present in the artifacts
      where applicable.
- [ ] No third-party content beyond the attributed defaults is bundled.

---

## 10. Testing strategy

### Unit tests

- Syllable estimation and lead calculation, including clamping and negative
  first-call beats.
- Role-term replacement across all presets and custom terms.
- Alias classification and template rendering.
- Timeline construction for the default dance and synthetic edge cases.
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
- Tempo changes and instrument changes.
- TTS timing and stop behaviour.
- Small-screen layout and soft keyboard.
- Offline start and cached content.
- Soundfont load/remove/fallback.
- Tab and screen animations, including with **Smooth animations** turned off.

---

## 11. Documentation and licensing

Required files:

- `README.md` — what it is, screenshots, controls, build/run instructions, data
  source notes, proxy deployment, known limitations.
- `ROADMAP.md` — this document.
- `AGENTS.md` — contributor and AI-agent guidelines.
- `CREDITS.md` — every library, font, tool, and data source with license and
  URL.
- `CHANGELOG.md` — generated by `git-cliff`.
- `LICENSE` — MIT (already present).
- `docs/architecture.md`, `docs/call-timing.md`, `docs/sources.md`,
  `docs/packaging.md`.

Licensing rules:

- The application code is MIT.
- Dependencies must be MIT, Apache-2.0, BSD, ISC, Zlib, Unlicense, CC0, or an
  equivalent permissive license. No GPL/AGPL/LGPL in the shipped binary without
  an explicit owner decision.
- `abc-parser`, `eframe`/`egui`, `cpal`, `ehttp`, `scraper`, `midly`,
  `rustysynth`, and `tts` are all permissive; record their exact licenses in
  `CREDITS.md`.
- Do not bundle a soundfont. Users supply their own.
- Do not redistribute dance or tune databases. Cache locally only.
- The one default dance and one default tune are bundled with attribution and
  source URLs. Confirm permission/normal attribution practice before release.
- Include a short legal note that dance transcriptions and ABC transcriptions
  belong to their respective contributors.

---

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
- [ ] Web bundle and optional Pages deploy.
- [ ] Android APK with minSdk 26.
- [ ] `ci.yml`, `nightly.yml`, `changelog.yml`.
- [ ] `git-cliff` config and `CHANGELOG.md`.
- [ ] `CREDITS.md`, README, and docs.
- [ ] Artifact checksums and release notes.

Exit: a push to `main` produces a complete nightly release.

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

---

## 13. Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| CORS blocks web search | No online search in WASM | Proxy + bundled defaults; configurable URL; document deployment |
| ABC parser gaps on messy tunes | Tunes fail to play | Normalize input; keep the previous tune; add fixtures; report source URL |
| Soundfont format/size problems | Long render, high memory | Render offline with progress; session-only web fallback; built-in synth fallback |
| Audio latency/clock drift | Calls visually out of sync | Use cpal playback timestamps; latency compensation; ±20 ms target; manual offset setting if needed |
| Timing formula too rigid | Calls feel early/late | User-adjustable min/max lead, syllables/s, scale; alias lengths; test with a caller |
| ContraDB HTML changes | Parser breaks | Provider isolation, fixtures, cached raw HTML, clear errors |
| Android build complexity | APK missing from nightly | Spike early; isolate JNI picker; allow built-in synth fallback |
| Flatpak/AppImage CI complexity | Missing Linux assets | Separate jobs, pinned tool versions, cached dependencies |
| Content licensing | Legal risk | Bundle only attributed defaults; local cache only; no database redistribution |
| egui mobile keyboard quirks | Poor search UX | Test early; use full-screen sheets and focus management |

---

## 14. Future work (post-0.1.0)

- Additional dance sources (e.g. other callers' databases) and ABC sources.
- Optional downloadable soundfont packs with clear licensing.
- Real-time tempo changes without re-rendering.
- Caller recording and playback comparison.
- Voice-activity detection to score call timing.
- MIDI-out to external instruments.
- Dance program/set planning.
- More role-term presets and per-dance overrides.
- TTS timing calibration using speech boundary events.
- macOS and iOS builds.
- Localisation.

---

## 15. Decision log

| Date | Decision | Reason |
| --- | --- | --- |
| 2026-11-14 | Rust + `eframe`/`egui` | One codebase for desktop, web, and Android; mature Rust ecosystem |
| 2026-11-14 | `cpal` for all audio | Supports WASAPI, ALSA, AAudio, and WebAudio |
| 2026-11-14 | `abc-parser` + custom sequencer | MIT parser, but playback/repeat handling is project-specific |
| 2026-11-14 | Built-in synth default, optional user SF2 | Small default download, clean licensing, upgrade path to realistic instruments |
| 2026-11-14 | Cloudflare Worker proxy | Only practical way to give WASM online search without redistributing content |
| 2026-11-14 | App name Contra Karaoke | Matches repository and product intent |
| 2026-11-14 | Version stays 0.1.0 | Owner instruction until declared released |
| 2026-11-14 | Dated nightly tags + rolling nightly | Traceable builds without changing semantic version |

---

## 16. Open items

- Final proxy deployment host/domain and token policy.
- Whether to publish the web build automatically to GitHub Pages.
- Android SF2 picker: JNI intent vs. app-folder import fallback.
- Review of default alias templates with an experienced caller.
- Permission/attribution wording for the bundled default dance and tune.
- Windows code signing (not required for nightly; note SmartScreen warnings).
- Exact app icon and final visual polish.
- Flatpak runtime/SDK version pinning.
