# Call timing

This note documents the critical timing rule implemented in
`crates/core/src/timing.rs`.

## Units

- One **dance beat** is half a bar in 4/4, 2/2, and 6/8.
- A standard 32-bar tune is 64 dance beats.
- At `bpm = 240`, `seconds_per_dance_beat = 120 / 240 = 0.5 s`.
- A standard 64-beat dance therefore lasts 32 s.
- The count-in is three dance beats (1.5 s at 240 BPM).

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
lead_beats = clamp(
  (speech_seconds / seconds_per_dance_beat) * lead_scale,
  min_lead_beats,
  max_lead_beats
)
call_start_beat = execute_beat - lead_beats
```

Defaults:

| Setting | Value |
| --- | --- |
| syllables/second | 6.5 |
| word gap | 0.04 s |
| extra pause | 0.10 s |
| lead scale | 1.0 |
| minimum | 1.0 beat |
| maximum | 2.0 beats |

The maximum can be raised to 4 beats in Preferences. Calls are computed from
the **rendered alias**, not the original card text, so a longer alias starts
earlier.

The first call can be negative (inside the count-in). Do not clamp it to zero.

## Sub-calls and grid

- A figure is split on ` - ` or `;`, but `&` is kept as "and". "balance & swing"
  is one call.
- A trailing balance sub-call gets about two beats; the rest is shared.
- Calls use cumulative figure beats. The music always uses a 64-beat grid, so a
  card that is shorter leaves music at the end and a longer card loops the
  music.
- The UI has three states: upcoming, call now, executing.

## Tests

`crates/core/src/timing.rs` covers the default dance, the negative first call,
the 1–2 beat clamp, manual leads, sub-call splitting, and tempo sensitivity.
`crates/music/src/arrange.rs` verifies the bundled Sandy Boys tune forms a
64-beat loop with a pickup and an A-major count-in.
