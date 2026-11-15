---
title: Instruments and soundfonts
description: Choose the melody and accompaniment voices, balance the mix, and load an optional user-supplied SF2 SoundFont.
sidebar_position: 6
---

Contra Karaoke imitates a small contra band: a melody instrument and a chordal
accompaniment. The built-in synthesizer needs no downloaded assets.

## Choose instruments

<Steps>
  <Step shot="prefs-instrument-melody">
    Open **Preferences** and find **Instruments**. Set **Melody** to **Fiddle**
    (default) or **Accordion**.
  </Step>
  <Step shot="prefs-instrument-accompaniment">
    Set **Accompaniment** to **Piano** (default), **Guitar**, or **Accordion**.
  </Step>
</Steps>

The session's tune summary shows the selected pair above the call prompt.

## Balance the mix

<Steps>
  <Step shot="prefs-instrument-mix">
    Adjust **Melody volume** and **Accompaniment volume**. Both range from 0 to
    1.5, with defaults of 0.75 and 0.65.
  </Step>
</Steps>

The session **Volume** control is a master level applied on top of this mix.

## Vary the accompaniment

<Steps>
  <Step shot="prefs-accompaniment-variations">
    In **Preferences → Text to speech**, enable **Vary the accompaniment on
    each loop**. The arranger adds small emphasis notes and the occasional bar
    with melody alone.
  </Step>
</Steps>

The variation is deterministic for a given tune and arrangement; it does not
change the dance timing or the 64-beat loop length.

## Apply the change

Instrument, volume, and accompaniment-variation changes apply without restarting
the music. Release the control and the app re-renders the loop, keeping the
current beat. If the session is not playing, press **Play** to render and start.
Session **Volume** changes apply immediately to the existing buffer.

## Optional SoundFont

The built-in synth is the default and always works. If you have a `.sf2` or
`.sf3` file on Windows or Linux, you can use it for more realistic instruments.
The browser and Android builds cannot load a SoundFont from a file in 0.1.0;
the Soundfont section reports that limitation. The project does not bundle a
SoundFont, and you are responsible for the file's license.

<Steps>
  <Step shot="prefs-sf2-loaded">
    On Windows or Linux, open **Preferences → Soundfont**, select **Load
    .sf2…**, and choose a file. The section shows the loaded name and a
    **Remove** button.
  </Step>
  <Step shot="prefs-sf2-removed">
    Select **Remove** to return to **None (built-in synth)**.
  </Step>
</Steps>

With a SoundFont active, the app maps the chosen instruments to General MIDI
programs:

| Instrument | GM program |
| --- | --- |
| Fiddle (melody) | 40 — Violin |
| Accordion (melody) | 21 — Accordion |
| Piano (accompaniment) | 0 — Acoustic Grand Piano |
| Guitar (accompaniment) | 25 — Acoustic Guitar (steel) |
| Accordion (accompaniment) | 21 — Accordion |

If the SoundFont does not contain a program, the result depends on the file and
may be silent for that part. Remove the SoundFont to return to the built-in
voices.

<Shot id="error-sf2" />

The file picker is a native OS dialog, so it is not captured in the docs
screenshots. The screenshots show the resulting Preferences state.

## What the arranger plays

The melody follows the ABC note stream. The accompaniment follows quoted chord
symbols when the ABC has them; otherwise a simple progression is inferred from
the key and strong-beat melody notes. Short fades and peak normalization are
applied to the rendered buffer.

Next: [Timing and lead](/docs/guides/timing-and-lead).
