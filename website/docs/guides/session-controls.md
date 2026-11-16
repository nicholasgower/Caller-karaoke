---
title: Session controls
description: Use tempo, play/pause, restart, ending, looping, count-in, spoken calls, and volume, and understand the call prompt and dance card.
sidebar_position: 3
---

The **Session** tab is the practice screen. The top row holds the tabs; the
session controls below hold the tempo, transport, loop, count-in, speech, and
volume settings.

## Transport and tempo

| Control | Range / default | Notes |
| --- | --- | --- |
| **Tempo** | 200–300 BPM by default; the session range is configurable in **Preferences → Playback**, and the full range is 120–320 | A dance beat is half a bar. At 240 BPM one beat is 0.5 s and a 64-beat dance is 32 s. A tempo outside the configured window stays selectable. |
| **Play** | — | Renders pending music, then starts or resumes. |
| **Pause** | — | Stops the music and any speech; the position is retained. |
| **Restart** | — | Returns to the count-in and clears the call state. |
| **End tune** | — | Slows the tune's last bar, holds the final chord, then stops. |
| **Loop** | on by default | Repeats the 64-beat music loop. |
| **Cycle tunes** | off by default | Plays through a set of saved tunes, changing at a repeat. |
| **Count-in** | on by default | Four accompaniment beats built from the tune's first chord. |
| **Speak calls** | off by default | Speaks the active call if a system voice is available. |
| **Volume** | 0–1, default 0.8 | Master playback volume. |

<Shot id="transport-tempo" />

The tempo slider uses a 200–300 BPM window by default. **Preferences →
Playback → Tempo range** sets a different minimum and maximum inside the
120–320 BPM limits, with **Reset to 200–300** to restore the default. A tempo
outside the window is not snapped: the slider widens just enough to keep the
current value selectable.

Changing tempo, count-in, an instrument, countermelody, or percussion
choice, a percussion rhythm, an accompaniment rhythm, a copy count,
melody/accompaniment/percussion volume, a SoundFont, or **Humanize timing**
marks the music for a rebuild. Playback settings apply without
restarting the music: release the control and the app re-renders the loop and
keeps the current beat. The session master **Volume** applies immediately. If
the session is not playing, it shows:

> Press Play to prepare the music with these settings.

Press **Play** to render and start. The app renders synchronously on that press;
there is no separate “preparing music” screen in 0.1.0. Role, alias, and lead
changes rebuild the call timeline without re-rendering the music.

**Undo** and **Redo** (Ctrl+Z and Ctrl+Y) put a setting or a dance/tune choice
back. A running session keeps its beat when only playback settings changed: the
loop is re-rendered and playback resumes where it was. Undoing a dance or tune
selection stops the old music and speech and leaves the restored tune ready for
the next **Play**, exactly as choosing it from a picker would.

## Background playback

Minimizing the window, covering it, or switching to another app leaves the audio
alone: the music keeps playing and the spoken calls keep coming on the same
beats. Only the drawing stops.

The transport follows the audio position, not the frame rate, so the caller
stays in time whether or not a frame is painted. **Pause** is the only thing
that stops it. On desktop, the keyboard's play/pause media key still works while
the window is in the background. On Android, the headset, lock-screen, and
notification controls keep working while the activity is in front or paused;
the app does not run a foreground service, so a session that Android stops in
the background stops with it.

## Keyboard

- **Space** plays or pauses.
- **R** restarts from the count-in.
- **Esc** returns from a download screen to its saved-content list, from the
  alias editor to **Preferences**, and from a picker or **Preferences** to the
  session. On a phone it first steps out of a Preferences category to the
  category list. The Android system back button does the same.
- **1-3** switch between Session, Select dance, and Select tune; **4** opens
  Preferences from any screen, like the gear button in the header.
- **← / →** switch to the previous or next tab, wrapping at the ends.
- **Enter** starts a search from a search field, creates a folder, or adds a
  custom alias.
- **Ctrl+Z** undoes the last preference or dance/tune change; **Ctrl+Y** or
  **Ctrl+Shift+Z** redoes it. The same actions are buttons in **Preferences →
  Appearance**.

Shortcuts are ignored while a text field has focus, and the arrow keys are left
to a focused slider or list. On desktop, the keyboard's play/pause media key
controls the transport even when the window is in the background; on Android,
the headset, lock-screen, and notification controls do the same while the app is
running. See
[Keyboard shortcuts](/docs/reference/keyboard-shortcuts).

## The call prompt

The large panel at the top of the session, directly under the tab row, is the
karaoke prompt. It moves through these states:

| State | Display | What to do |
| --- | --- | --- |
| Ready | `READY`, `Press Play to begin` | Press **Play** to build the music. |
| Count-in | `COUNT-IN`, a large number, and either **CALL NOW** or `Listen for the first chord` | Wait for the loop, or say the first call if it is already due. |
| Calling | `CALL NOW`, green card | Say the words now. The detail line counts down to the execute beat, and the **NEXT:** footer previews the following call. |
| Executing | `UPCOMING MOVE`, red card, the next call, and `call in N beats` | The figure is happening; prepare to say the next call. The dance card still highlights the figure in progress. |
| Finished | `Dance complete` | Press **Restart** to run again. End tune also finishes here. |

While a figure executes, the prompt switches to the move you must say next. The
call that follows it stays inside the same card as a **NEXT:** footer — it is
part of the prompt, not a separate line under it. The card therefore keeps one
height while the session runs, so switching between **CALL NOW**, **UPCOMING
MOVE**, and the final figure never moves the controls below. The card reserves
room for two lines of call text and the footer; a longer call wraps and makes
the card taller instead of clipping the words. The idle `READY` card and the
completed card stay content-sized.

<Shot id="session-upcoming-move" />

<Shot id="session-call-now" />

The **executes in N beats** line is the remaining lead. A short call may show
about two beats; a long call up to six by default once the safety padding
(0.5 s by default) is included. See
[Timing and lead](/docs/guides/timing-and-lead).

If **Show original card text** is enabled in Preferences, the original card
wording appears in small muted text next to the rendered call.

## Count-in and the first call

<Shot id="session-count-in" />

The count-in is four dance beats of accompaniment only. At 240 BPM that is
2.0 seconds. Turn **Count-in** off to start directly on the first figure. The
setting applies to the next run when the session is idle, and turning it on
while a dance is playing gives the next run a count-in. If you turn it off
during a count-in that is already playing, the app lets that count-in finish and
changes the timeline and the music together at beat 0, so the dance never
restarts in the middle of the count. The first call often starts during the
count-in because its lead is subtracted from beat 0. That is intentional: the
caller needs to say the first figure before the dancers move on beat 0. When a
call is already active during the count-in, the panel shows **CALL NOW** and
the call text instead of `Listen for the first chord`. With the count-in off,
the first call is announced as the run starts, and it is never spoken twice or
dropped when the setting changes.

<Shot id="session-first-call" />

## The tune summary and dance card

Below the prompt, the tune line shows the tune title, key, rhythm, melody,
accompaniment, countermelody, and any enabled percussion, with a copy count such
as `×2` when a part has more than one player. The dance card
at the bottom of the session shows every figure with its phrase label and beat
count. The active figure row is highlighted as the music moves.

On a wide window the space below the controls is split into two boxes: the dance
information and beat meter on the left, and the dance card on the right. The
card scrolls inside its box, so a long card never pushes the prompt or the
controls off screen. Phone and tablet widths keep the stacked layout, with the
card below the meter.

<Shot id="transport-summary" />

The progress bar shows the current beat out of the 64-beat loop, and its text
names the transport state: `Ready`, `Count-in`, `Playing`, `Paused`, `Ending`,
or `Finished`. The state is there because a paused session sounds the same as a
muted one. During the count-in the bar stays at zero and the text counts the
beats left.

## Looping, pausing, and finishing

- With **Loop** on, the 64-beat music repeats from beat 0 and the call sequence
  restarts with it.
- With **Loop** off, the audio stops at the end of the loop and the prompt shows
  **Dance complete**.
- **Pause** stops both audio and TTS. **Play** resumes from the current position.
- **Restart** resets to the count-in, not to the beginning of the card.

<Shot id="transport-loop-off" />

<Shot id="session-finished" />

## Ending the tune

Select **End tune** while music is playing (or paused) to finish the way a band
would. At the start of the tune's last bar the arranger splices into a rendered
ending: the closing bar slows down, the final melody note rings, and the last
chord is held for a few seconds before the audio fades out. The prompt shows
**SLOW FINISH** while the ending plays, and **Dance complete** when it is over.

The ending is prepared with the music, so it starts without a delay. Press
**Cancel ending** before the last bar begins to keep playing normally. With
**Loop** off, **End tune** still plays the slow ending instead of stopping
abruptly.

<Shot id="session-ending" />

## Cycling tunes

Contra bands usually stay on one tune for several minutes and change at a
repeat. **Cycle tunes** does the same with your saved tunes:

1. Open **Select tune** and add each tune with **Add to set** (or **Add to
   tune set** in the preview).
2. Turn on **Cycle tunes** in the session controls.
3. Set **Change tune every** to the number of 64-beat times through. The default
   is eight, about four minutes at 240 BPM; the session line shows the actual
   time at the current tempo.

The next tune is rendered during the current tune's last time through, so the
change is ready at the boundary. The call timeline continues unchanged, and the
new tune starts at beat 0 without a second count-in. **Cycle tunes** needs
**Loop** on, because a non-looping run stops after one pass. The web build has
no background renderer, so it prepares the next tune at the change instead.

<Shot id="session-tune-set" />

## Spoken calls

Enable **Speak calls** in the session controls or in Preferences. The visual
prompt is always shown; speech is optional. The app speaks each active call once,
stops speaking when you pause or disable speech, and clears the spoken-call
state when you restart or rebuild the timeline.

<Shot id="transport-tts-on" />

See [Text to speech](/docs/guides/text-to-speech) for availability and rate.

## Volume

The session **Volume** slider is the master level. The melody, accompaniment,
and percussion balances live in **Preferences → Band & Sound**; the
countermelody follows the accompaniment balance, and the **Copies** steppers in
that section set how many players each part has. They are separate controls; a
quiet master with a loud melody is still quiet overall.

<Shot id="transport-volume" />

## Preferences

**Preferences** has six categories: **Calling** (role terms, lead timing, spoken
calls), **Band & Sound** (instruments, copies, SoundFont, effects), **Playback**
(loop, count-in, tempo range, human timing), **Appearance** (theme, motion,
shortcuts, undo), **Data & Storage** (dance-source pointer, web proxy, cache,
MIDI export), and **About** (version, credits, project links). A desktop shows
the category list in a left frame and the selected category's settings beside
it. A phone shows the list first and opens one category at a time; the back
control, **Esc**, and the Android system back button return to the category
list, and a second Back returns to the session.

## About and project links

The About text is the **About** category of **Preferences**. It shows the
program name, its version, a short description, the author (Nicholas Gower), and
links to the GitHub repository and the documentation site. The project pages may
not be live yet.

<Shot id="about-window" />

It is part of the Preferences screen, so it never covers the session, traps
focus, or suspends the transport and tab shortcuts. Press **Esc** or select
**Session** to leave.

## Narrow windows

At phone width the session tab uses its compact layout. The top row keeps three
short tabs — **Session**, **Dance**, and **Tune** — beside the pinned gear button
that opens **Preferences**, without the wordmark. On a wide window the gear also
carries the word **Preferences** when the header has room; the text is dropped
before anything wraps. The tempo and volume sliders
span the screen with their value boxes, and **Play**, **Pause**,
**Restart**, and **End** split one row evenly. Loop, cycle tunes, count-in, and
spoken calls stay on one wrapping row. Every control keeps a 44-pixel touch
target.

When the window is too short to show the prompt, controls, progress, and dance
card together, the whole session page scrolls; the prompt stays at the top of
that page. On a taller phone the prompt stays put and only the dance card
scrolls. The saved-content and download screens use a single scrolling column
at phone width, with full-width fields and the preview below the results.

On a touch screen, swipe left or right across the content to change tabs. The
gesture ignores a swipe that starts on a slider, and a mostly vertical swipe on
a slider scrolls the page instead of changing the value, so passing a thumb
over the tempo or volume control while scrolling does not disturb it. Drag a
slider sideways to change it on purpose.

<Shot id="session-mobile-controls" />

<Shot id="main-mobile" />

Next: [Role terms](/docs/guides/role-terms).
