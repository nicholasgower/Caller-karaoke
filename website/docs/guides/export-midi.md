---
title: Export MIDI
description: Export the arranged 64-beat loop as a Standard MIDI File from the desktop app.
sidebar_position: 11
---

The desktop app can export the arranged loop as a Standard MIDI File. The export
re-parses the current tune and builds the same 64-beat arrangement used for
playback; you do **not** need to press Play first.

## Availability

MIDI export is available in the Windows and Linux desktop builds. The browser
and Android builds report that export is not available in this build yet.

## Export

<Steps>
  <Step shot="prefs-maintenance">
    Open **Preferences → Maintenance** and select **Export MIDI…**.
  </Step>
</Steps>

Choose a location in the native save dialog. The default file name is based on
the tune title with unsafe characters replaced.

The export contains:

- three tracks when the second accompaniment is on — melody, accompaniment, and
  second accompaniment (countermelody) — or two tracks when it is **Off**;
- the selected accompaniment rhythm;
- the tempo, meter, key, and program changes;
- the arranged 64-beat loop.

The MIDI does **not** include the four-beat count-in, the pickup notes, or the
live slow ending. It is the arranged loop, not the full playback buffer. Because
the export re-parses the tune's stored ABC, edits made in the preview field are
ignored.

## File format and editing

The file is a `.mid` Standard MIDI File. You can open it in a DAW or notation
program to inspect or edit the arrangement, but edits made outside the app do
not affect the app's playback.

## Native dialog

The save dialog is provided by the operating system and is not part of the
Docusaurus screenshots. The screenshot shows the Maintenance section where the
export starts.

Next: [How calls are timed](/docs/reference/how-calls-are-timed).
