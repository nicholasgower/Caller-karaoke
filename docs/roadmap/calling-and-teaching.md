## 17. Calling and teaching

This chapter records what the project wants from its calling engine, why those
choices follow from how contra callers actually work, and where the work sits in
the roadmap. It is both a design note and a backlog. The short version is:

> A good caller does not read the card at a constant rate. They choose words
> that fit the music, teach before they prompt, say less as the dancers learn,
> and stay ready to rescue the floor.

Caller Karaoke already computes *when* a call should start. This chapter is
about making the call itself worth hearing.

### 17.1 Two goals, one engine

The project has two related audiences:

1. **A human caller in training.** The app shows and speaks a model call, teaches
   the dance, thins the calls over successive loops, and gives the learner
   something to practise against. This is the 0.1.0 goal.
2. **An autonomous caller.** The app can call a dance on its own, with the
   timing, wording, and restraint of a competent caller. This is a post-0.1.0
   goal, but it is not a separate product: it is the same timeline with the
   human replaced at the microphone.

The two goals reinforce each other:

- every improvement to the autonomous caller (shorter variants, thinning,
  phrase-aware placement) gives the human learner a better model to imitate;
- every teaching feature (walkthrough, call rhythm, cover mode) gives the
  autonomous caller a better script and a better notion of what the room needs;
- both must agree on one source of truth: the rendered call, its beats, and its
  lead.

The autonomous caller is allowed to be ambitious, but it must not turn the
trainer into a machine that does everything for the learner. The human remains
in charge of the session, and every automatic choice must be visible and
overridable.

### 17.2 What good callers do

The findings below come from caller workshops, CDSS resources, and working
callers' notes. They are consistent across the sources even when the style
differs.

**They teach before they call.** A walkthrough gives *who, what, how much, and
where*, plus the finishing orientation: “Larks allemande left once and a half
until you face your partner.” They teach one new figure per dance, check that
dancers are in position before adding the next instruction, and pace the
walkthrough so nobody falls behind. ([Ravitz](#sources); [CDSS
Basics](#sources))

**They prompt in two to four beats, on the downbeats, and finish just before the
move.** Amidon's rule is “end each prompt one beat before the figure starts.”
Kaufman's workshop writes the same call as a four-beat, two-beat, or one-beat
prompt, with the stressed syllables landing on beats. A call that ends too early
makes beginners start early; a call that ends late leaves dancers knowing a move
is coming but not what it is. ([Amidon](#sources); [Kaufman](#sources);
[Ravitz](#sources))

**They say the essential information first and can trail the rest.** “Ladies
allemand right” — the figure starts — “go once and a half” — and later “look for
your partner, balance and swing.” Distance and destination may arrive after the
movement has begun. The lead does not have to contain the whole sentence.
([Ravitz](#sources))

**They phrase to the music.** One cycle is 64 dance beats: four 16-beat phrases
(A1, A2, B1, B2), mostly 8- and 16-count figures. Calls land on phrase
boundaries, and the caller does not drop or add beats to make a card fit. The
music and the dance share one grid. ([CDSS Basics](#sources))

**They call less over time.** “Thick” calling first, then shorter calls, then
only the first move, the move after each swing, and the unusual parts, then
silence. Calls have four-, two-, one-, and zero-beat variants. The goal is to
give the dancers just enough help and no more. ([Kaufman, “Lessons From Dropping
Out”](#sources); [Amidon](#sources); [CDSS Contra
Connection](#sources))

**They read the room and rescue.** If the floor gets ragged, they come back in
with full calls for a phrase, then back off again. They do not overcall, and
they do not drop out so early that beginners panic. “Call softer” and
“intentionally late” are advanced techniques, not defaults. ([CDSS Contra
Connection](#sources))

**They run the evening, not just the dance.** They choose a program with
variety, set a working tempo (about 112–120 counts per minute, i.e. 224–240
dance beats per minute), cue the band for the last times through, and decide
when the dance ends. ([Ravitz](#sources); [ContraMontreal](#sources))

**They practise deliberately.** Call along to recordings, shadow another caller,
record yourself, and get honest feedback. The skill is built away from the
microphone as much as at it. ([Parkes](#sources))

The implications for the app are direct:

- call length is a design choice, not a consequence of the alias text;
- call support should vary over the dance, not stay constant;
- the app needs a teaching mode, not only a prompting mode;
- phrase position and the previous figure matter as much as the words;
- the app should show its reasoning so a learner can internalise it.

### 17.3 Current state (0.1.0)

**Already strong:**

- The lead is computed from the rendered alias, including the safety padding.
- The first call may start before beat 0; it is not clamped.
- Loop wrapping calls the first figure again at the end of the previous loop.
- Alias templates and role terms keep the card, the prompt, the lead, and TTS
  on the same words.
- The UI distinguishes *upcoming*, *call now*, and *executing*, and shows the
  next move while the current figure is being danced.
- The count-in is now the conventional four dance beats (“four potatoes”),
  correcting the earlier three-beat choice, and is tested.

**Still missing or incomplete:**

| Area | Current state | Consequence |
| --- | --- | --- |
| Call length | `LeadSettings::lead_beats` always uses the full alias. Alternatives are synonyms, chosen for variety, not for fit. | Long calls start very early; short figures can be crowded; the learner never practises “chain” or “swing” as a cue. |
| Collisions | `PracticeTimeline::build` sorts by start beat but does not enforce a gap (contrary to §5.3). | Two calls can overlap in the prompt and in TTS. |
| Distance | `find_count` does not recognise `3/4`, `1/4`, `1/2`, `1 1/4`, or `three quarters`. | “Circle left 3/4” renders as “circle left”; the distance disappears. |
| Destination | No `destination` field; text after “to face”, “until”, or “ending” is dropped by most aliases. | The caller loses the “where”, which is often the part beginners need. |
| Teaching | `Dance.preamble` and `Dance.notes` are parsed but never shown. No walkthrough mode. | The app can prompt a dance but cannot teach one. |
| Phrase validation | `Dance::validate` checks only title, non-empty figures, and positive beats. | A card with a 12-beat A1 or a figure starting mid-phrase looks fine until the calls drift. |
| Short cards | A 32-beat card is not repeated inside the 64-beat loop (contrary to §5.4). | Half the music has no calls. |
| Context | No “new neighbor” at the top of a loop, no priority for the call after a swing, no support level. | The prompt cannot distinguish an obvious repeat from a tricky transition. |
| Rhythm | The prompt shows the text but not which syllable lands on which beat. TTS speaks at its own rate. | The learner sees *what* to say but not *how* it sits on the music. |
| Tempo label | “BPM” (120–320) is ambiguous; the research range is 224–240 in the app's units. | Callers may practise far outside a danceable tempo without noticing. |
| Feedback | Recording and timing score are future work. | The app can model a call but cannot tell the learner whether they matched it. |

Two of these are documentation/implementation mismatches rather than new
features: §5.3 already promises a minimum gap, and §5.4 already says to repeat a
32-beat card. They should be fixed as part of the work below, not treated as
scope expansion.

### 17.4 Design principles

1. **Fit the words to the beat.** The beat is fixed; the wording is flexible.
   Choose the longest variant that fits the available window after the safety
   padding. Do not stretch a sentence to fill time.
2. **Never two calls at once.** A caller speaks one instruction at a time. The
   scheduler must guarantee a gap, even if that means shortening a call or
   accepting a shorter lead.
3. **Say the essential first; trail the rest.** The who and what belong in the
   lead. The distance and destination may trail into the figure.
4. **Support fades, it does not vanish.** Move from full calls to shorter calls
   to selected calls to silence, and be ready to come back in.
5. **Teach before you call.** A walkthrough is part of calling, not an optional
   extra. The app should make the preamble, notes, and teaching script visible.
6. **Keep the grid sacred.** 64 beats, 16-beat phrases, 8- and 16-count figures.
   Warn when a card breaks the grid; do not silently rescale it.
7. **The human stays in charge.** Every automatic choice (variant, thinning,
   omission) must be visible and overridable. A “Rescue” control restores full
   calling immediately.
8. **Do not fake choreography.** Anything that needs to know who is where, who
   is the new neighbor, or which hand is free depends on a dance model. Until
   that model exists, those features stay on the roadmap and out of the
   implementation.
9. **Measure with a caller, not only with tests.** Timing and wording should be
   reviewed against recordings and, where possible, an experienced caller.

### 17.5 Planned capabilities

Priorities are about order, not importance. P0 makes the existing engine
trustworthy; P1 makes it teach; P2 makes it a complete practice tool; P3 is
gated on the dance model.

| ID | Capability | Priority | Why | Depends on |
| --- | --- | --- | --- | --- |
| C1 | Time-aware call variants (long / medium / short) | P0 | Calls should fit the beat, not the reverse | — |
| C2 | Collision-free scheduling and first-call fit | P0 | One call at a time; the first call must fit the count-in | C1 |
| C3 | Distance, fraction, and destination correctness | P0 | Losing “3/4” or “face your partner” changes the dance | — |
| C4 | Call-rhythm display and “say by” marker | P0 | Teaches the downbeat placement and the finish line | C1 |
| C5 | Call support and thinning schedule | P1 | Models thick → thin → drop, and trains restraint | C1, C2 |
| C6 | Dance/phrase validation and short-card repetition | P1 | Calls must line up with the 64-beat grid | — |
| C7 | Walkthrough and teaching mode | P1 | A caller teaches before they call | C3 |
| C8 | Loop count, ending, and band cues | P1 | A caller runs a dance, not an endless loop | C5 |
| C9 | Tempo and count-in polish | P2 | The tempo label and range should match dance practice | — |
| C10 | Practice feedback (record, score, cover mode) | P2 | Deliberate practice needs a feedback loop | C4 |
| C11 | Autonomous-caller voice and rhythm | P2 | An autonomous caller must sound like a caller, not a reader | C1, C4 |
| C12 | Callability check for downloaded cards | P2 | Unknown or incomplete figures need to be visible before the dance | C3, C6 |
| C13 | Dance model (progression, neighbor/shadow, phrase geometry) | P3 | Enables who/where, new-neighbor, and positional calling | — |
| C14 | Positional / gender-free prompt style | P3 | Serves communities that call without role terms | C13 |
| C15 | Program and set planning | P3 | Good callers plan an evening, not one dance | C12, C13 |
| C16 | Singing and patter calls | Not planned | A different tradition; not needed for contra prompting | — |

All of C1–C15 are planned; none is implemented in 0.1.0 except the four-beat
count-in correction noted in §17.3. C13–C15 are gated on the dance model.

#### C1 — Time-aware call variants

Add a length dimension to the alias system:

- `CallLength::{Long, Medium, Short}` and a `CallVariant` per `MoveKind`.
- Long is the current full alias. Medium drops the obvious actor or object
  (“partners balance and swing” → “balance and swing”). Short keeps the
  identifying noun or verb (“long lines forward and back” → “lines”).
- `AliasTable` owns the templates; custom alternatives still work. The caller
  can edit each length independently.
- `PracticeTimeline::build` computes the available window for each call, then
  chooses the richest variant whose estimated speech duration fits after the
  safety padding. If none fits, use the shortest and record a warning.
- `CallEvent` stores the chosen length and text, so the card, TTS, syllable
  count, and lead describe the same words.

This replaces “random alternative aliases” as the primary use of the
alternative table. Shuffling remains available for variety, but fit comes first.

#### C2 — Collision-free scheduling and first-call fit

- Walk calls in execute order. For each call, compute the earliest legal start
  (`previous_call_end + minimum_gap`) and the ideal start (`execute - lead`).
- If the ideal start is too early, try a shorter variant. If it still does not
  fit, move the start later and accept a shorter lead, but never later than
  `execute - minimum_speech_lead`.
- Never reorder figures. If a call cannot fit even at the minimum, emit a
  warning that the card or alias is too wordy.
- Fit the first call inside the four-beat count-in when possible. When it cannot
  fit, keep the existing “speak it at the start of the run” behaviour and show a
  warning: “first call needs 5.5 beats; the count-in is 4.”

#### C3 — Distance, fraction, and destination correctness

- Extend `find_count` for `3/4`, `1/4`, `1/2`, `1 1/4`, `three quarters`,
  `once around`, `half sashay`, and similar forms.
- Expand fractions for speech in `normalize_figure`: “three quarters”, “a
  quarter”, “half”, “one and a quarter”.
- Extract a `destination` (`to face …`, `until …`, `ending …`, `finish …`) and
  store it on `CallEvent` as a trailing clause with its own timing.
- Use word-boundary matching in `detect_kind` for short patterns (`hey`, `star`,
  `gate`) so “they” or “start” cannot be classified as a dance figure.
- Add a `Callability` report for downloaded cards: unknown move kind, missing
  actor/direction/distance, figure text that did not render, and figures whose
  beats are not a multiple of 4.

#### C4 — Call-rhythm display and “say by” marker

- Add `CallRhythm` to `CallEvent`: a small grid of beat slots, each holding the
  syllables spoken on that beat. The first syllable of each slot is the stressed
  one.
- Show the grid above the prompt, with the current beat highlighted. Render it
  as `with your | neigh-bor | bal-ance and | swing`.
- Show a “say by” line at `execute_beat - padding`, then `MOVE` when the figure
  starts. The prompt should teach the finish, not only the start.
- Keep the text version accessible: the grid is a supplement, never the only
  cue.

#### C5 — Call support and thinning

- `CallSupport::{Full, Standard, Sparse, Silent, Auto}`.
- `CallPriority` scores each call: first call of the loop, call after a swing,
  unusual move (hey, contra corners, mad robin, poussette, roll away, square
  through, give and take), phrase start, simple/repeated move.
- `Auto` uses a deterministic schedule over loop passes: full for the first pass
  or two, then drop low-priority calls, then shorten the rest, then silence.
- A **Rescue** control restores full calls for the current loop and then returns
  to the selected schedule.
- Show what was dropped and why in a small “this pass” summary, so the learner
  can see the judgment, not just the result.

#### C6 — Dance and phrase validation

- Validate the total (64), the phrase labels (A1/A2/B1/B2, 16 beats each), and
  the figure starts (8-beat boundaries when possible).
- Snap sub-call execute beats to 2-beat subdivisions when within tolerance;
  otherwise warn.
- Repeat a 32-beat card inside the 64-beat loop, as §5.4 already specifies.
- Show the warnings in the picker and in the session, not only in logs.

#### C7 — Walkthrough and teaching mode

- Display `Dance.preamble` and `Dance.notes`.
- Generate a teaching script from the card and aliases: for each figure, the
  actor, the move, the distance, and the destination. Mark the figures that are
  new or unusual.
- Let the caller add per-dance teaching notes; persist them with the library
  entry and a schema migration.
- Add a half-tempo “Teach” mode that steps through figures before the count-in,
  then transitions to the dance. This is the app's equivalent of a walkthrough.

#### C8 — Loop count, ending, and band cues

- A “times through” setting (for example 12) and a visible countdown.
- At B2, show “3 more / 2 more / last time” and, optionally, a band-cue hint.
- End the run instead of looping forever, with a short “thank the band” state.
- Use the loop count to drive the thinning schedule.

#### C9 — Tempo and count-in polish

- Label the tempo as dance beats per minute and show the music equivalent.
- Mark 224–240 as the typical band; allow slower practice tempos with a
  “practice” label.
- Keep the four-beat count-in. If a first call cannot fit, warn rather than
  silently truncating it.
- Do not add a count-in-length control unless the owner asks; four is the
  convention and the simpler product.

#### C10 — Practice feedback

- Record the caller's voice and compare it with the model call. Keep recordings
  local; no upload, no telemetry.
- Score timing (did the call finish before the move?), clarity, and drop-out.
- Add “cover” mode (hide the next call for one phrase), “spot check” mode (mute
  a random call), and a short self-review after each run.
- This aligns with the recording and timing-score items already in §14.

#### C11 — Autonomous-caller voice and rhythm

- Speak calls with beat-aware emphasis and pauses, not as a flat sentence.
- Optionally click or pulse the beat under the call during practice.
- Keep one voice and one call at a time; never let TTS overlap.
- Calibrate the speech rate from the actual voice, not only from the syllable
  estimate. This is the TTS calibration item already in §14.

#### C12 — Callability check

- A pre-dance report: unknown move kinds, missing distance/direction, figures
  that did not render, phrase/beat problems, and repeated segments.
- Offer “use the card text” as the fallback for an unknown figure, with the
  original text visible.
- This is the data-quality counterpart to C3 and C6.

#### C13–C15 — Gated on the dance model

The following are **roadmap only**. They must not be implemented, or depended
on by other tasks, until a dance model is designed and accepted:

- **C13 Dance model.** Represent progression, partner/neighbor/shadow, role
  sides, minor sets, and the geometry needed to know who is where. This is a
  substantial domain model, not a parser tweak.
- **C14 Positional / gender-free prompt style.** “Right-hand dancer”, “those
  with a free right hand”, and “the person on the left” require the model to
  know positions and hand availability. Role-term presets remain the supported
  path in the meantime.
- **C15 Program and set planning.** Difficulty, formation, figure variety,
  tempo, and key. Some of this can start from metadata, but any “this dance will
  flow from that one” claim needs the model.

Until then, the app should not pretend to know who is where. It can show the
card, the notes, and the aliases, and it can warn about missing information.

### 17.6 Data and algorithm sketch

Keep the domain logic in `caller-core`; the UI only renders decisions.

```rust
pub enum CallLength { Long, Medium, Short }

pub struct CallVariant {
    pub length: CallLength,
    pub text: String,
    pub syllables: usize,
    pub words: usize,
}

pub struct CallEvent {
    // existing fields ...
    pub length: CallLength,
    pub rhythm: CallRhythm,
    pub destination: Option<String>,
    pub priority: CallPriority,
}

pub enum CallSupport { Full, Standard, Sparse, Silent, Auto }
```

`PracticeTimeline::build` becomes a small scheduler:

1. Render the long, medium, and short variants for every figure.
2. Compute each call's ideal start from its long variant.
3. Walk in execute order; enforce the gap; shorten when necessary.
4. Fit the first call to the count-in, or record a warning.
5. Assign rhythm slots and priorities.
6. Apply the thinning plan for the requested support level and loop count.

The scheduler must be deterministic for a given dance, aliases, settings, and
seed. It must never mutate the figure order.

Preferences gain `call_support`, `thinning_schedule`, and maybe
`call_length_preference`. Bump the schema and add a migration; older files get
the current full-call behaviour.

### 17.7 Testing

Every capability needs tests in `core`, plus a manual QA note:

- C1: a 4-beat figure chooses a short variant; a 16-beat figure chooses long;
  custom templates and alternatives still work; the chosen text is what TTS
  speaks.
- C2: adjacent 4-beat figures do not overlap; a long first call fits the
  four-beat count-in or produces a warning; figure order is unchanged.
- C3: `3/4`, `1/4`, `1/2`, `1 1/4`, `once around`, and destination phrases
  render correctly; word-boundary classification rejects “they” and “start”.
- C4: syllable slots cover the text exactly once; the “say by” beat is
  `execute - padding`; 1-, 2-, and 4-beat variants all render.
- C5: thinning is deterministic; high-priority calls are dropped last; Rescue
  restores full calls; silent support produces no calls.
- C6: 32-beat card repeats; 48-beat card warns; 64-beat card matches; a 12-beat
  phrase warns; non-4/4 meters still use the dance-beat grid.
- C7: teaching script uses the rendered aliases and role terms; preamble and
  notes display; per-dance notes survive a round trip.
- C8: loop countdown, last-time marker, and end state.
- C9: tempo label conversion; count-in is four beats; first-call warning.
- C10: recording and scoring are platform-gated; no network in tests.

Add a golden snapshot for the default dance's full timeline, including variants,
rhythm slots, and thinning for each support level. Update it deliberately when
the call model changes.

### 17.8 Documentation

This chapter is the design source. When a capability ships:

- update `docs/call-timing.md` and `docs/roadmap/timing-model.md` for the timing
  rules that changed;
- add or extend the user guide under `website/docs/` for call variants, call
  support, walkthrough, and practice feedback;
- update `website/screenshots/manifest.json` for new or changed shots, and
  refresh the PNGs only at release prep;
- keep `README.md` and the About/credits text in step with user-visible
  behaviour;
- run `npm test` and `npm run check` in `website/` after docs changes.

### 17.9 Open decisions

- **Autonomous-caller default.** Should the autonomous caller start in
  `Standard` support (calls every figure, then thins) or `Full`? The trainer
  default should probably be `Standard`; a performance mode may want `Auto`.
- **Dance-model gate.** What is the smallest useful model? Progression and
  partner/neighbor are probably enough for “new neighbor” and positional
  prompts; hand availability and minor-set geometry are much larger.
- **Evaluation.** How do we decide that a call “feels right” without a room?
  Proposals: compare against recordings, have an experienced caller review a
  script, and measure whether the call finishes before the figure at 224, 232,
  and 240 BPM.
- **TTS rhythm.** Is beat-aware TTS worth the complexity, or is a visual rhythm
  strip plus a click track enough? Start with the strip.
- **Positional calling.** Is it a supported style, or a documentation note for
  communities that use it? Do not decide before C13.

### 17.10 Sources

<a id="sources"></a>

- **CDSS, “Calling Contras: The Basics” (2019).**
  <https://cdss.org/wp-content/uploads/resources/CDSS-The-Basics-of-Calling-Contras.pdf>
  — the 64-beat form, AABB phrases, the caller's teaching/prompting job, and
  “prompt a figure before the dancers start”.
- **Cary Ravitz, “Notes on Calling Contra Dances” (2024).**
  <https://www.dance.ravitz.us/calling.pdf> — who/what/where teaching, finishing
  the call as the figure starts, trailing distance/destination, shortening and
  dropping calls, tempo, band cues.
- **Peter Amidon, “A Callers Workshop” (2007).**
  <http://www.quiteapair.us/calling/Amidon_Callers_Workshop.pdf> — end each
  prompt one beat before the figure; thick then thin; call rhythmically.
- **Jeff Kaufman, “Notes From A Contra Calling Workshop” (2011).**
  <http://www.jefftk.com/p/notes-from-a-contra-calling-workshop> — four-, two-,
  and one-beat calls; stressed syllables on downbeats; gradual drop-out.
- **Jeff Kaufman, “Lessons From Dropping Out” (2019).**
  <https://www.jefftk.com/p/lessons-from-dropping-out> — 4/2/1/0-beat variants;
  give just enough help; drop calls as the room learns.
- **CDSS Contra Connection, “How and When a Caller Stops Calling” (1994).**
  <https://cdss.org/wp-content/uploads/publications/online-library/contra-connection/Contra-Connection-20_How-and-When-a-Caller-Stops-Calling_May-Jun94.pdf>
  — keep the first move, the move after a swing, and the unusual parts; call
  softer; late calls only for experienced callers.
- **“Calling your first dance.”**
  <http://www.quiteapair.us/calling/firstdance.html> — walkthrough who/what/how
  much/where; shorten calls; keep the first move, post-swing move, and unusual
  parts.
- **Tony Parkes, *Contra Dance Calling* (excerpt).**
  <https://hands4.com/wp-content/uploads/2017/07/Contra-Dance-Calling.pdf> —
  practising with recordings, shadowing, recording yourself, and getting
  feedback.
- **ContraMontreal, “How To Play For Your First Contradances.”**
  <https://www.contramontreal.org/assets/documents/How-To-Play-For-Contra-Dance.pdf>
  — four potatoes; the caller cues the band and decides the ending.
- **CDSS, “Positional Calling for Contra Callers” (Louise Siddons).**
  <https://cdss.org/programs/online-workshops/positional-calling-for-contra-callers>
  — the case for positional/global calling; deferred here until the dance model
  exists.
- **CDSS, “Global Terminology & Positional Calling” (Brooke Friendly).**
  <https://cdss.org/programs/online-workshops/global-terminology-positional-calling>
  — teaching and prompting without role terms; context for C14.
