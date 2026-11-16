---
title: Editing a saved tune's ABC
description: Change the ABC text of a saved tune, see the sheet music update, and restore the downloaded version.
sidebar_position: 2.5
---

**Edit ABC** in a saved tune's preview opens a plain text editor. The draft is
shown as monospace ABC above the same engraved sheet music the tune preview
uses, so a typo in the notation is visible before it reaches the library.

The editor changes only the tune's ABC text. The title, composer, key, meter,
genre, source, and folder stay as they were downloaded or imported; re-reading
the headers is future work. If the edited tune is the one loaded in the session,
the session uses the new text the next time it renders the music.

## Make an edit

1. Open **Select tune** and choose a saved tune.
2. Select **Edit ABC** in the preview.
3. Change the text in the **ABC source** field.
4. Select **Save**.

While you type, the editor parses the draft. A parse error appears in the error
colour directly below the field, and **Save** is disabled, so the saved copy
cannot be replaced with malformed ABC. Once the draft parses, the sheet music
re-engraves after a short pause instead of on every keystroke.

| Control | What it does |
| --- | --- |
| **Save** | Writes the draft to the saved tune and returns to the saved tunes. |
| **Cancel** | Discards the draft and returns to the saved tunes. |
| **Restore original** | Puts the ABC from the download or import back. It appears only when the tune has an original text and the draft differs from it. |
| **< Back to saved tunes** | The same as **Cancel**. |

A tune imported from a local file also has an original text, so **Restore
original** returns to the text in the file you imported. The bundled *Sandy
Boys* has no original text, so it has no Restore button.

## Errors and what is kept

- If the draft does not parse, nothing is written and the editor stays open.
- If the tune was removed from the library in another window or frame, Save
  reports that it is no longer saved instead of recreating it.
- The sheet-music engraver can fail on ABC that the player's parser accepts (for
  example an unsupported notation symbol). The editor shows that message and
  still lets you save text that plays, because playback and engraving use
  separate parsers.

## Keyboard and accessibility

The text area is a normal multiline field: the arrow keys, Home/End, selection,
and the platform's cut/copy/paste all work. The field's accessible name is
**ABC source**, and every button has a label, so the editor can be driven by
screen readers and the development MCP tools.

Escape is handled by the text field while it has focus, so use **Cancel** or
**< Back to saved tunes** to leave without saving.

## Where the text goes

Saved tunes live with your app data, so an edit is available offline and is
included in the next **Export as ABC**. See
[Choosing a tune](/docs/guides/choosing-a-tune#export-a-saved-tune) for export
and import.

## Credit

The idea of editing ABC beside a rendered score comes from Michael Eskin's
[abctools](https://github.com/seisiuneer/abctools). Contra Karaoke's editor is an
independent, minimal implementation and does not include code from that project.
The possible next steps are listed in the
[ABC editor roadmap](https://github.com/storytold/Contra-karaoke/blob/dev/docs/roadmap/abc-editor.md).

Next: [Session controls](/docs/guides/session-controls).
