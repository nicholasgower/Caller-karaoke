## 3. User-facing feature set

### 3.1 Main session screen

- Top tab row: **Select dance**, **Select tune**, **Preferences**.
- Session controls directly below the tabs:
  - Tempo (default 240 BPM), with stepper and slider.
  - Play/pause, restart, loop toggle, and **End tune** (a slow, held final
    cadence).
  - **Cycle tunes**: play a set of saved tunes in turn, changing at a repeat.
  - TTS on/off.
  - Master volume and current tune/instrument summary.
- The remaining space is the dance area:
  - Title, choreographer, formation, source link.
  - Dance card grouped by A1/A2/B1/B2 with beat counts.
  - Large "current call" prompt, next-call preview, and a lead countdown.
  - Karaoke states: *upcoming*, *call now*, *executing*. Once a figure is
    executing, the prompt becomes an *upcoming move* box naming the move to
    call next.
  - Beat/progress bar and a 4-3-2-1 count-in display.
- On narrow screens the current call is pinned; the card scrolls below it.

### 3.2 Select dance

The saved-dance library, available offline:

- Local search over title, choreographer, formation, and figure text.
- Flat user folders: create, rename, delete, and move cards; deleting a folder
  keeps its cards as unfiled.
- A **Download dance** submenu opens the ContraDB search screen:
  - Text filters: title, choreographer, user, hook.
  - Verified filter.
  - Formation filter: improper, becket, proper, everything else.
  - Figure filter: a move selector plus a "figure contains words" search.
  - Sorting and pagination.
  - Results show title, choreographer, formation, and hook, plus a downloaded
    marker.
  - Selecting a result opens a preview of its parsed figures, a **Download
    dance** action, and a **Use this dance** action that saves and loads.
- The default dance is downloaded on first run and remains usable offline.
- Downloads are stored separately from the HTTP source cache; clearing the
  cache keeps them.

### 3.3 Select tune

The saved-tune library, available offline:

- Local search over title, composer, key, meter, genre, and credits.
- The same flat folders and folder actions as dances.
- A **Download tune** submenu opens the ABC Session Tunes search screen:
  - Free-text search across title and composer.
  - Composer, key, meter, and genre filters.
  - Pagination and result counts.
  - Results show title, composer, key, meter, and rhythm, plus a downloaded
    marker.
  - Selecting a result shows metadata and a rendered sheet-music preview, with
    a **Show ABC** / **Show sheet music** toggle for the source, a **Download
    tune** action, and a **Use this tune** action that saves and loads. The
    sheet is the default view; if it cannot be engraved the source remains
    available in the ABC view.
- The default tune is downloaded on first run and remains usable offline.
- **Tune set**: saved tunes can be added to an ordered set. With **Cycle tunes**
  on, the session keeps each tune for a configurable number of 64-beat times
  through (default 8, about four minutes at 240 BPM) and changes at the repeat.
  The next tune is rendered ahead of the boundary.

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
   - Second accompaniment: Accordion countermelody (default), Fiddle
     countermelody, or Off for a duo. Melody plus both accompaniments is the
     default trio.
   - Percussion: Off (default), Foot taps on each dance beat, or Drumset.
   - Percussion rhythm: match style (default), basic, on the beat, offbeats,
     half time, double time, shuffle, gallop, or train beat. Match style gives
     foot taps a one-and-a gallop.
   - Melody/accompaniment volume; the second accompaniment follows the
     accompaniment volume. Percussion has its own volume.
   - Soundfont: Bundled Fluid R3 GM (default) or a user-loaded `.sf2`.
4. **Application theme** — Dark (default) or Light.
5. **Interface animations** — smooth tab and screen transitions, on by
   default; can be turned off to reduce motion.

Advanced settings:

- Calling lead: automatic or manual, minimum/maximum beats, syllables per
  second, word gap, extra pause.
- TTS voice (Kokoro Heart, Bella, Sarah, Michael, Fenrir, or Onyx on desktop),
  rate, and tone (Commanding caller or Natural).
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
- Preferences offers six Kokoro voices: Heart, Bella, and Sarah (US female), and
  Michael, Fenrir, and Onyx (US male).
- Call tone defaults to **Commanding caller**: a slightly lower, more projected
  register with a clear-speech pass (stressed vowels held longer and a
  presence/compression stage for consonants); **Natural** restores the recorded
  pitch and even emphasis. Calls are still timed from the syllable estimate and
  the safety padding, so the lead and 64-beat grid are unchanged.
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
- A count of one is omitted from the rendered alias; fractional counts such as
  “once and a half” remain. The speech path connects “and a half” into the
  natural “n' a half”.
- Lead timing is computed from the rendered alias, so making a call longer
  automatically moves it earlier.

Default alias style examples:

| Card figure | Default call |
| --- | --- |
| `neighbors do si do once` | `neighbors do-si-do` |
| `ladles allemande left 1½` | `larks allemande left once and a half` |
| `partners balance & swing` | `partners balance and swing` |
| `circle left 3 places` | `circle left three places` |
| `neighbors swing` | `swing your neighbor` |
| `long lines forward & back` | `long lines forward and back` |

These defaults are meant to be editable; the exact list will be reviewed with
an experienced caller during Milestone 4.

### 3.7 Planned calling and teaching features

Call variants, collision-free scheduling, call support and thinning, walkthrough
mode, loop endings, and practice feedback are post-0.1.0 work. They are
described, prioritised, and sourced in §17 (Calling and teaching). The
autonomous-caller goal is also recorded there; choreography-dependent features
remain gated on a dance model.
