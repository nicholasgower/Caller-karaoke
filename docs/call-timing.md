# Call timing

This note documents the critical timing rule implemented in
`crates/core/src/timing.rs`.

## Units

- One **dance beat** is half a bar in 4/4, 2/2, and 6/8.
- A standard 32-bar tune is 64 dance beats.
- At `bpm = 240`, `seconds_per_dance_beat = 120 / 240 = 0.5 s`.
- A standard 64-beat dance therefore lasts 32 s.
- The count-in is four dance beats (2.0 s at 240 BPM).

The music sequencer converts ABC note lengths to dance beats using:

```
quarters_per_bar        = numerator * 4 / denominator
dance_beats_per_quarter = 2 / quarters_per_bar
```

so a bar is always two dance beats.

## Lead

```
speech_seconds = syllables / syllables_per_second
               + (words - 1) * word_gap_seconds
               + extra_pause_seconds
speech_lead_beats = clamp(
  (speech_seconds / seconds_per_dance_beat) * lead_scale,
  min_lead_beats,
  max_lead_beats
)
lead_beats = ceil(speech_lead_beats + safety_padding (converted to dance beats))
call_start_beat = execute_beat - lead_beats
```

The automatic lead — the speech estimate plus the safety padding — is rounded
**up** to the next whole dance beat, so the caller always gets at least the time
the words need and the call starts on a beat boundary. A manual lead is used
exactly as entered.

Defaults are calibrated from a measured 2.25 s delivery of the nine-syllable
call “larks allemande left once and a half”. A safety padding is added to every
call after the speech-lead clamp; it defaults to 0.5 s and is adjustable from 0
to 2 s in Preferences.

| Setting | Value |
| --- | --- |
| syllables/second | 4.0 |
| word gap | 0.0 s |
| extra pause | 0.0 s |
| lead scale | 1.0 |
| minimum speech lead | 1.0 beat |
| maximum speech lead | 5.0 beats |
| automatic-lead rounding | up to the next whole beat |
| safety padding | 0.5 s (adjustable 0–2 s) |

At 240 BPM that phrase gets 4.5 beats of speech plus 1 beat of padding = 5.5,
which rounds up to 6 beats, or 3.0 seconds. The maximum speech lead can be raised
to 6 beats in Preferences, and the safety padding is adjustable; it is always
added. Calls are computed from the **rendered alias**, not the original card
text, so a longer alias starts earlier.

Clear enunciation can lengthen the acoustic delivery by a few percent. The
safety padding is part of the automatic lead before it is rounded up and covers
that variation, so the visual lead does not change.

A count of one is omitted from the rendered alias, so it adds no syllable and no
lead. Fractional counts keep their words; “and a half” is spoken as the
connected “n' a half”.

When alternative-alias shuffling is enabled, the timeline first chooses one of
the applicable templates for each call (the active template or one of its
alternatives), then computes syllables and the lead from that wording. This
keeps the card, TTS, and timing on the same words. An alternative whose
placeholder has no value for the call (for example a `{direction}` alternative
on a card that names no direction) is skipped.

The first call can be negative (inside the count-in). Do not clamp it to zero.
When the dance loops, the same call is scheduled again at
`music_loop_beats - lead_beats`; the UI and TTS treat that as a wrapped call at
the end of the loop so the first move of the next repetition is still called.

The transport cannot always reach the start of that first lead. A lead can be
longer than the four-beat count-in, and the count-in can be turned off, so
playback may begin at or after the first call's execute beat. The TTS still
speaks the first call as soon as a run starts (at the count-in, or at beat 0
when there is no count-in); it is never dropped just because its lead window
had already begun. `PracticeTimeline::starting_call` reports that call, and the
app consumes it once per run from the top. Resuming a paused dance does not
repeat it.

## Sub-calls and grid

- A figure is split on ` - ` or `;`, but `&` is kept as "and". "balance & swing"
  is one call.
- A trailing balance sub-call gets about two beats; the rest is shared.
- Calls use cumulative figure beats. The music always uses a 64-beat grid, so a
  card that is shorter leaves music at the end and a longer card loops the
  music.
- The app compares the dance total (cumulative figure beats) with the tune body
  length, excluding the pickup, and shows a warning when they differ. It does
  not rescale either one.
- The UI has three states: upcoming, call now, executing.
- While the dancers execute a figure, the large prompt switches to an
  **upcoming move** box that names the move the caller must say next and how
  many beats remain until the call starts. Both live countdowns (**executes in**
  and **call in**) show the nearest half beat with one decimal (`4.0`, `3.5`).
  The dance card still highlights the figure in progress and prints the call lead
  in beats next to each call.

## Tests

`crates/core/src/timing.rs` covers the default dance, the negative first call,
the 1–5 beat speech-lead clamp, the round-up of the automatic lead, the 0.5 s
padding, manual leads, loop wrapping, sub-call splitting, and tempo sensitivity.
`crates/app/src/app/ui/mod.rs` covers the half-beat countdown formatting.
`crates/music/src/arrange.rs` verifies the bundled Sandy Boys tune has a
64-beat AABB body plus a 0.5-beat pickup, and checks the dance/tune length
comparison. `crates/app/src/defaults.rs` asserts that the bundled dance and
tune are both 64 beats.
