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

- **Count-in:** exactly 4 dance beats of accompaniment (“four potatoes”), built
  from the tune's first chord. The loop starts at dance beat 0 after the
  count-in. Four beats is the contra convention and gives the first call more
  room than the earlier three-beat choice.

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
lead_beats       = ceil(speech_lead_beats + safety_padding (as dance beats))
call_start_beat  = execution_beat - lead_beats
```

The automatic lead — speech estimate plus safety padding — rounds up to the next
whole dance beat; a manual lead is used as entered.

Defaults are calibrated from measured delivery: saying
“larks allemande left once and a half” (nine syllables) took **2.25 seconds**,
which is 4.5 dance beats at 240 BPM. A **safety padding** is added to every call
after the speech-lead clamp; it defaults to 0.5 s and is user-adjustable.

| Setting | Default |
| --- | --- |
| syllables per second | 4.0 |
| word gap | 0.0 s |
| extra pause | 0.0 s |
| lead scale | 1.0 |
| min speech lead | 1.0 beat |
| max speech lead | 5.0 beats |
| safety padding | 0.5 s (adjustable 0–2 s) |

The max speech lead is user-adjustable up to 6 beats for learning sessions or
slower TTS voices, and the safety padding is adjustable separately; it is always
added on top.

Example at 240 BPM:

| Alias | Lead | Call starts |
| --- | ---: | ---: |
| `swing` | 2.0 beats | 1.0 s before the move |
| `circle left three` | 3.0 beats | 1.5 s before the move |
| `neighbors do-si-do` | 4.0 beats | 2.0 s before the move |
| `larks allemande left once and a half` | 6.0 beats | 3.0 s before the move |

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
  small minimum gap; never reorder calls. *Current 0.1.0 sorts by start beat but
  does not yet shorten calls or enforce the gap. Collision-free scheduling is
  planned in §17 (C2).*
- The timeline is rebuilt whenever the dance, aliases, role terms, or lead
  settings change.

### 5.4 Portable grid

Dance cards do not always sum to exactly 64 beats. Use cumulative figure beats
for call placement, but keep a 64-beat music grid:

- If the card totals 64, it matches the loop exactly.
- If it totals less than 64, the music continues after the last call.
- If it totals more than 64, loop the music and continue the card. *Current
  0.1.0 restarts the music at beat 0; figures after beat 64 are not reached.
  Continuing the card is planned in §17 (C6).*
- If the card totals 32 or less, repeat the card within each 64-beat music
  loop. *Current 0.1.0 does not repeat the card; the music simply continues.
  Repetition is planned in §17 (C6).*
- Show a warning in the picker when a card's total is not 64.

### 5.5 Karaoke states

For current time `t` (in dance beats):

- **Upcoming:** `t < call_start_beat`.
- **Call now:** `call_start_beat <= t < execute_beat`; show a lead countdown.
- **Executing:** `execute_beat <= t < end_beat`.

The current call is rendered large. The next call is shown smaller. The active
figure row is highlighted in the card.

### 5.6 Planned call-craft refinements

The timing model above answers *when* a call starts. It does not yet answer
*how much* to say. The research in §17 adds four rules to this chapter's target
model:

- a call is chosen from long, medium, and short wordings so it fits the
  available beats, rather than stretching every wording to its natural speech
  length;
- two calls never overlap; when they would, the later call is shortened or the
  earlier one is moved, never the figure order;
- the first call fits inside the four-beat count-in when possible, and the app
  says so when it cannot;
- the amount of calling thins over successive loops, following the priority
  rules in §17.

These are roadmap items. The 0.1.0 implementation keeps the speech-derived lead
and the fixed alias wording described in §5.2.
