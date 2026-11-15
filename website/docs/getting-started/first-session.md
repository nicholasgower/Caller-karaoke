---
title: Your first session
description: Choose a dance and a tune, set the tempo, and practise the calls in about five minutes.
sidebar_position: 6
---

This is the shortest useful path through the app. It uses the bundled defaults
first, then shows how to replace them.

## Before you start

- Launch the installed app, a source build, or the
  [hosted browser copy](/play). See the installation pages or
  [Build and run](/docs/getting-started/build-and-run).
- Have an audio output device available. In a browser, press Play once to allow
  audio.
- The first time you download a dance or tune, the app needs network access to
  ContraDB and ABC Session Tunes. The bundled defaults do not.

## 1. Look at the starting screen

The app opens on **Session** with **Another Last Dance** and **Sandy Boys**
loaded. The dance card and tune summary are already populated. The prompt says
**READY**, and the line above it says
`Press Play to prepare the music with these settings.`

<Steps>
  <Step shot="main-idle">
    Start the app and confirm that the dance, tune, and tempo controls are
    visible. Do not worry that the music has not been rendered yet.
  </Step>
</Steps>

The first **Play** renders the arrangement and starts the count-in.

## 2. Choose a dance

**Select dance** lists the dances saved on this device. The bundled default is
already there, and the current dance is not replaced until you choose **Use**.

<Steps>
  <Step shot="dance-library">
    Select **Select dance** in the top tab row. Search saved content, select a
    card to preview it, and press **Use** to load it.
  </Step>
  <Step shot="dance-picker-open">
    To add another dance, select **Download dance**. This opens the ContraDB
    search screen; previewing a result does not save it.
  </Step>
  <Step shot="dance-picker-search">
    Type part of a title in **Title** and select **Search dances**. The default
    results are ten per page; use **Next** and **Previous** to page.
  </Step>
  <Step shot="dance-picker-preview">
    Select **Preview** on a result, read the figure list and beat total, then
    select **Download dance** to keep it or **Use this dance** to download and
    load it immediately.
  </Step>
</Steps>

You can also filter by **Choreographer**, **User**, **Hook**, **Formation**,
**Figure type**, or **Figure contains**, show only **Verified only** dances, and
change **Sort**. See [Choosing a dance](/docs/guides/choosing-a-dance) for the
full search and folder reference.

## 3. Choose a tune

**Select tune** lists the tunes saved on this device. Its search and folder
panel work like the dance library.

<Steps>
  <Step shot="tune-library">
    Select **Select tune** in the top tab row, then select a saved tune to see
    its sheet music.
  </Step>
  <Step shot="tune-picker-open">
    Select **Download tune** to search ABC Session Tunes for another melody.
  </Step>
  <Step shot="tune-picker-preview">
    Search by title or composer, optionally filter by **Key**, **Meter**, or
    **Genre**, then select **Preview**. **Download tune** saves the tune and
    **Use this tune** saves and loads it.
  </Step>
</Steps>

The preview engraves the ABC as sheet music. **Show ABC** switches to the source
text; edits there are not saved and do not change the tune. The preview also
links to the tune on ABC Session Tunes when a source URL is available.

<Shot id="main-dance-loaded" />

## 4. Set the tempo

The default is **240 BPM**, which gives a 0.5-second dance beat and a 32-second
64-beat dance. The slider ranges from 120 to 320 BPM.

<Steps>
  <Step shot="transport-tempo">
    Move the **Tempo** slider. The app shows the prepare-music message when
    stopped; while playing it re-renders the loop without restarting and keeps
    the current beat.
  </Step>
</Steps>

## 5. Start the session

<Steps>
  <Step shot="session-count-in">
    Press **Play**. The music starts with a four-beat count-in built from the
    tune's first chord. If the first call has already started, the count-in
    panel shows **CALL NOW** and the call text.
  </Step>
  <Step shot="session-call-now">
    Follow the large call prompt. It changes from **CALL NOW** to **DANCING**
    as the figure executes; the sub-line reads **figure in progress** and the
    next call is previewed below it.
  </Step>
  <Step shot="session-changes-pending">
    Change a setting such as tempo or an instrument. The music stops and the
    prepare-music message returns; press **Play** to rebuild.
  </Step>
  <Step shot="session-paused">
    Press **Pause** to stop the music and any speech. Press **Play** to resume
    from the same position.
  </Step>
  <Step shot="session-finished">
    With **Loop** off, the dance ends with **Dance complete**. Press **Restart**
    to run it again from the count-in.
  </Step>
</Steps>

With **Loop** on (the default), the 64-beat music loop repeats from beat 0
and the call sequence restarts with it. A card shorter than 64 beats leaves
music playing after its last call. A card longer than 64 beats is not fully
reached in 0.1.0: the playhead resets at the loop boundary. Prefer cards whose
preview total is 64 beats.

Pausing is visible without sound: the progress bar names the transport state, so
after **Pause** it reads `Paused · beat 12 / 64` and stays there until **Play**
resumes from the same beat.

## What the call prompt means

| Label | Meaning |
| --- | --- |
| `READY` | Nothing has started yet. Press **Play**. |
| `COUNT-IN` | The four accompaniment beats before beat 0. If a call is already due, the panel shows **CALL NOW** and its text. |
| `CALL NOW` | Say the displayed words now; the dancers execute shortly. |
| `DANCING` | The figure is executing; the prompt is muted and the sub-line reads **figure in progress**. |
| `Dance complete` | The music reached the end with looping off. |

The **executes in N beats** line counts down to the figure's execute beat. It is
not a metronome; the music is the clock.

## If something does not work

- **No sound:** check the audio device. The session shows an `Audio device:`
  message when the app had to fall back to silence.
- **The music takes a moment to start:** the first **Play** renders the loop
  synchronously. There is no separate progress screen in 0.1.0.
- **A search fails:** dismiss the red bottom panel with **Dismiss** and retry.
  The current dance or tune is not replaced by a failed search.
- **A call starts during the count-in:** that is expected. The first call is
  placed before beat 0 so the words finish before the first figure.

Next: [Choosing a dance](/docs/guides/choosing-a-dance).
