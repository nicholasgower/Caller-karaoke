---
title: Text to speech
description: Enable spoken calls, choose the local Kokoro voice, set the speaking rate, and understand pronunciation corrections and fallbacks.
sidebar_position: 8
---

Text to speech is optional. The visual call prompt is always the primary
signal; speech is an additional cue for practice or for callers who want to hear
the wording.

Desktop builds use a local **Kokoro-82M neural voice**. The selected voice is a
small style file downloaded once and cached; the model itself runs locally, with
no account or cloud service. Windows and Linux use the neural voice when it is
ready. Web and Android use the platform voices: the browser's installed voices
and Android's TTS engine. If the neural voice cannot start, desktop falls back to
the platform voice.

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

## Choose a voice

<Steps>
  <Step shot="prefs-tts-rate">
    Open **Preferences → Text to speech**. On desktop, choose **Voice** from
    Heart, Bella, Nicole, Michael, or Fenrir. The description below the menu
    summarises the selected voice. Web and Android show the selector disabled
    because they use the platform voice.
  </Step>
</Steps>

The default is **Heart (US female)**. All five voices use the same Kokoro model
and differ only in the downloaded style file. On first use, the section shows a
progress bar while the voice is prepared; later runs load it from the cache. If
preparation fails, desktop shows **Retry neural voice** and continues with the
platform voice.

## Set the rate

<Steps>
  <Step shot="prefs-tts-rate">
    Adjust **Rate**. The range is 0.5 to 2.0; the default is 1.0.
  </Step>
</Steps>

The TTS rate does **not** change the visual lead. If a slower voice needs more
time, raise the maximum speech lead in
[Timing and lead](/docs/guides/timing-and-lead).

## Pronunciation review

Kokoro reads phonemes, not letters, and its grapheme-to-phoneme model is
occasionally wrong for contra vocabulary. The app corrects the terms that matter
before synthesis:

| Term | Corrected pronunciation |
| --- | --- |
| `poussette` | “poo-SET” |
| `petronella` | “pet-roh-NEL-uh”, with the contra stress |
| `promenade` | “prom-uh-NADE” |
| `do-si-do` | “doe see doe” |
| `gentlespoon(s)` | “gentle spoon(s)” |
| `Rory O'More` | “Rory O More”, without spelling out the apostrophe |

The platform voice receives respellings where its own dictionary needs them.
The full review, phoneme tables, and the ignored real-model smoke test live in
`docs/pronunciation.md` in the repository.

## If no voice is available

<Steps>
  <Step shot="prefs-tts-unavailable">
    If neither the neural voice nor a platform voice is usable, the Preferences
    section shows a muted `TTS unavailable:` message and the session shows an
    explanation when **Speak calls** is enabled.
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
