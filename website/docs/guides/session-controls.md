---
title: Session controls
description: Use tempo, play/pause, restart, looping, count-in, spoken calls, and volume, and understand the call prompt and dance card.
sidebar_position: 3
---

The **Session** tab is the practice screen. The top row holds the tabs; the
session controls below hold the tempo, transport, loop, count-in, speech, and
volume settings.

## Transport and tempo

| Control | Range / default | Notes |
| --- | --- | --- |
| **Tempo** | 120–320 BPM, default 240 | A dance beat is half a bar. At 240 BPM one beat is 0.5 s and a 64-beat dance is 32 s. |
| **Play** | — | Renders pending music, then starts or resumes. |
| **Pause** | — | Stops the music and any speech; the position is retained. |
| **Restart** | — | Returns to the count-in and clears the call state. |
| **Loop** | on by default | Repeats the 64-beat music loop. |
| **Count-in** | on by default | Three accompaniment beats built from the tune's first chord. |
| **Speak calls** | off by default | Speaks the active call if a system voice is available. |
| **Volume** | 0–1, default 0.8 | Master playback volume. |

<Shot id="transport-tempo" />

Changing tempo, count-in, an instrument, a melody/accompaniment volume, or a
SoundFont marks the music for a rebuild. Playback settings apply without
restarting the music: release the control and the app re-renders the loop and
keeps the current beat. The session master **Volume** applies immediately. If
the session is not playing, it shows:

> Press Play to prepare the music with these settings.

Press **Play** to render and start. The app renders synchronously on that press;
there is no separate “preparing music” screen in 0.1.0. Role, alias, and lead
changes rebuild the call timeline without re-rendering the music.

## Keyboard

- **Space** plays or pauses.
- **R** restarts from the count-in.
- **Esc** returns to the session from a picker, Preferences, or the alias
  editor.
- **1–4** switch between Session, Select dance, Select Tune, and Preferences.
- **Enter** starts a search from a search field or adds a custom alias.

Shortcuts are ignored while a text field has focus. See
[Keyboard shortcuts](/docs/reference/keyboard-shortcuts).

## The call prompt

The large panel above the card is the karaoke prompt. It moves through these
states:

| State | Display | What to do |
| --- | --- | --- |
| Ready | `READY`, `Press Play to begin` | Press **Play** to build the music. |
| Count-in | `COUNT-IN`, a large number, and either **CALL NOW** or `Listen for the first chord` | Wait for the loop, or say the first call if it is already due. |
| Calling | `CALL NOW`, green card | Say the words now. The line below counts down to the execute beat. |
| Executing | `UPCOMING MOVE`, red card, the next call, and `call in N beats` | The figure is happening; prepare to say the next call. The dance card still highlights the figure in progress. |
| Finished | `Dance complete` | With looping off, press **Restart** to run again. |

While a figure executes, the prompt switches to the move you must say next. The
**Next:** line remains below the prompt for the following call.

<Shot id="session-upcoming-move" />

<Shot id="session-call-now" />

The **executes in N beats** line is the remaining lead. A short call may show
about two beats; a long call up to six by default once the 0.5 s safety padding
is included. See [Timing and lead](/docs/guides/timing-and-lead).

If **Show original card text** is enabled in Preferences, the original card
wording appears in small muted text next to the rendered call.

## Count-in and the first call

<Shot id="session-count-in" />

The count-in is three dance beats of accompaniment only. At 240 BPM that is
1.5 seconds. The first call often starts during the count-in because its lead is
subtracted from beat 0. That is intentional: the caller needs to say the first
figure before the dancers move on beat 0. When a call is already active during
the count-in, the panel shows **CALL NOW** and the call text instead of
`Listen for the first chord`.

<Shot id="session-first-call" />

## The tune summary and dance card

Above the prompt, the tune line shows the tune title, key, rhythm, melody, and
accompaniment. The dance card below the prompt shows every figure with its
phrase label and beat count. The active figure row is highlighted as the music
moves.

<Shot id="transport-summary" />

The progress bar shows the current beat out of the 64-beat loop. During the
count-in it stays at zero.

## Looping, pausing, and finishing

- With **Loop** on, the 64-beat music repeats from beat 0 and the call sequence
  restarts with it.
- With **Loop** off, the audio stops at the end of the loop and the prompt shows
  **Dance complete**.
- **Pause** stops both audio and TTS. **Play** resumes from the current position.
- **Restart** resets to the count-in, not to the beginning of the card.

<Shot id="transport-loop-off" />

<Shot id="session-finished" />

## Spoken calls

Enable **Speak calls** in the session controls or in Preferences. The visual
prompt is always shown; speech is optional. The app speaks each active call once,
stops speaking when you pause or disable speech, and clears the spoken-call
state when you restart or rebuild the timeline.

<Shot id="transport-tts-on" />

See [Text to speech](/docs/guides/text-to-speech) for availability and rate.

## Volume

The session **Volume** slider is the master level. The melody and accompaniment
balances live in **Preferences → Instruments**. They are separate controls; a
quiet master with a loud melody is still quiet overall.

<Shot id="transport-volume" />

## Narrow windows

The session controls use wrapping rows. At phone width they take several lines,
and the dance card scrolls below the prompt. The pickers use two columns; on a
narrow screen this can be tight, so rotate the device or use a wider window when
searching.

<Shot id="session-mobile-controls" />

<Shot id="main-mobile" />

Next: [Role terms](/docs/guides/role-terms).
