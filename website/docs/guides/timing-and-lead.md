---
title: Timing and lead
description: Understand why calls start before the move, and adjust automatic or manual lead, minimum and maximum beats, and speech rate.
sidebar_position: 7
---

Contra Karaoke's central rule is simple: **the call must finish before the
dancers move**. The app places the call earlier by a lead computed from the
rendered words.

## Open the setting

<Steps>
  <Step shot="prefs-timing-mode">
    Open **Preferences** and find **Calling lead**. Automatic syllable-based
    lead is on by default.
  </Step>
</Steps>

## Automatic lead

<Steps>
  <Step shot="prefs-timing-bounds">
    Adjust **Minimum speech lead** and **Maximum speech lead** in beats. The
    defaults are 1.0 and 5.0. The maximum can be raised to 6.0 for learning
    sessions or slow voices.
  </Step>
  <Step shot="prefs-timing-speech-rate">
    Adjust **Speech rate** in syllables per second. The default is 4.0. A lower
    rate assumes slower speech, so the same words get more lead.
  </Step>
</Steps>

The automatic lead uses:

```
speech_seconds = syllables / speech_rate
               + (words - 1) × word_gap
               + extra_pause
speech_lead_beats = clamp(
                      speech_seconds / seconds_per_dance_beat × lead_scale,
                      min_beats,
                      max_beats
                    )
lead_beats = speech_lead_beats + 0.5 s (converted to dance beats)
```

Only speech rate, minimum, and maximum are exposed in the UI. The word gap and
extra pause are fixed at 0.0, and the lead scale at 1.0. Every call also gets a
flat **0.5 s safety padding** after the speech-lead clamp.

The default is calibrated from a measured 2.25 s delivery of the nine-syllable
call “larks allemande left once and a half”. At 240 BPM that is 4.5 beats of
speech plus 1 beat of padding, so the call starts 5.5 beats before the move.

## Manual lead

Turn off **Automatic syllable-based lead** to use a fixed **Manual lead** in
beats. This is useful when practising with a specific band or when the
automatic estimate does not match your delivery.

## Rebuild the timeline

<Steps>
  <Step shot="prefs-timing-rebuild">
    Select **Rebuild call timing** after changing lead settings. The app also
    rebuilds automatically when preferences change, but the button makes the
    action explicit.
  </Step>
</Steps>

The music is marked pending because the call timeline changed. Press **Play** to
render and start.

## Watch the countdown

<Steps>
  <Step shot="session-lead-countdown">
    In the session, read the **executes in N beats** line under the current
    call. It counts down to the figure's execute beat.
  </Step>
</Steps>

A short call such as `swing` gets the 1-beat minimum speech lead plus the 1-beat
padding at 240 BPM, so it starts about two beats early. A longer phrase
approaches the 5-beat default maximum plus padding. If you raise the maximum to
6, long calls can start even earlier.

## The first call can be negative

<Steps>
  <Step shot="session-first-call">
    Start a session and watch the count-in. The first call is often listed
    below **Listen for the first chord** before the loop begins.
  </Step>
</Steps>

This is intentional. The first figure executes at beat 0, and its call starts
before beat 0, inside the count-in. The app does not clamp the first call to
zero.

## Dance beat, bar, and loop

- A **dance beat** is half a bar in 4/4, 2/2, and 6/8.
- At 240 BPM, `seconds_per_dance_beat = 120 / bpm = 0.5 s`.
- A standard 32-bar tune is **64 dance beats**, or **32 seconds**.
- The count-in is **3 dance beats**, built from the tune's first chord.
- Reels and jigs use the same dance-beat duration at the same BPM setting.

For the complete formula, worked examples, and edge cases, see
[How calls are timed](/docs/reference/how-calls-are-timed).

Next: [Text to speech](/docs/guides/text-to-speech).
