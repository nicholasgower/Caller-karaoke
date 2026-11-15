---
title: Text to speech
description: Enable spoken calls, set the speaking rate, and understand what happens when a system voice is unavailable.
sidebar_position: 8
---

Text to speech is optional. The visual call prompt is always the primary
signal; speech is an additional cue for practice or for callers who want to hear
the wording. TTS uses the platform voices: Windows SAPI, Linux Speech
Dispatcher, Android's TTS engine, and the browser's installed voices.

## Enable spoken calls

<Steps>
  <Step shot="transport-tts-on">
    In the session controls, select **Speak calls**. The same checkbox is
    available in **Preferences → Text to speech**.
  </Step>
</Steps>

When a call becomes active and the music is playing, the app speaks the rendered
alias once. It does not queue stale calls: pausing stops speech, and restarting
or rebuilding the timeline clears the last-spoken state. Pause before restarting
if a long call is still being spoken.

## Set the rate

<Steps>
  <Step shot="prefs-tts-rate">
    Open **Preferences → Text to speech** and adjust **Rate**. The range is 0.5
    to 2.0; the default is 1.0.
  </Step>
</Steps>

The TTS rate does **not** change the visual lead. If a slower voice needs more
time, raise the maximum speech lead in
[Timing and lead](/docs/guides/timing-and-lead).

## If no voice is available

<Steps>
  <Step shot="prefs-tts-unavailable">
    If the system has no usable speech voice, the Preferences section shows a
    muted `TTS unavailable:` message and the session shows an explanation when
    **Speak calls** is enabled.
  </Step>
</Steps>

The app continues with visual calls. The TTS checkbox may remain visible so you
can retry after installing a voice, but the toggle has no effect until a voice
is available.

## Pause and restart

Pause stops speech immediately. Restart clears the last-spoken call so the first
call is spoken again when the new pass begins. Changing an alias or role term
rebuilds the timeline and also clears the spoken state.

## What is spoken

The app speaks the **rendered alias**, not the original card text. That means
role terms, placeholders, and custom aliases are reflected in speech. If
**Show original card text** is on, the original is only visual.

Next: [Cache and offline](/docs/guides/cache-and-offline).
