---
title: Choosing a tune
description: Search ABC Session Tunes by title, composer, key, meter, and genre, preview the sheet music or ABC, and load a 64-beat arrangement.
sidebar_position: 2
---

**Select Tune** searches ABC Session Tunes and loads a melody plus chord track.
The current tune stays loaded until you choose **Use this tune**.

## Search fields

| Field | What it does |
| --- | --- |
| **Search** | Matches the tune title (and the source's search index). |
| **Composer** | Matches the composer name. |
| **Key** | Any key, or a curated list including D/G/A/C/F/B♭ major, A/E/B/D minor, A dorian, and A mixolydian. |
| **Meter** | Any meter, or 4/4, 2/2, 2/4, 6/8, 9/8, 12/8, or 3/4. |
| **Genre** | Any genre, or reel, jig, slip jig, hornpipe, polka, slide, strathspey, or waltz. |

<Shot id="tune-picker-open" />

The key and genre lists are curated rather than exhaustive. ABC Session Tunes
accepts other values, but the UI keeps the common contra choices short.

## Search and page through results

<Steps>
  <Step shot="tune-search">
    Type a title or composer and select **Search tunes**. Results show the
    title, composer, key, and meter.
  </Step>
  <Step shot="tune-filter-key">
    Open **Key** to narrow the result set. The same pattern applies to **Meter**
    and **Genre**.
  </Step>
  <Step shot="tune-filter-meter">
    Choose a meter, then search again.
  </Step>
  <Step shot="tune-filter-genre">
    Choose a genre such as **Reel** or **Jig**, then search again.
  </Step>
</Steps>

Results are ten per page; **Previous** and **Next** sit below the columns.

## Preview the sheet music and ABC

<Steps>
  <Step shot="tune-picker-preview">
    Select **Preview** on a result. The right column shows the title, composer,
    key, meter, credits, the rendered sheet music, and an **Open on ABC Session
    Tunes** link. Use **Show ABC** to switch between the staff and the source.
    Select **Use this tune** to load it.
  </Step>
</Steps>

The sheet music is engraved from the ABC by the app. The **Show ABC** view
displays the source text in a preview field; edits there are local and do not
change the loaded tune or the stored cache. If the engraver cannot render the
tune, the app explains why and the ABC view remains available.

## What happens to the tune

The arranger turns the tune into a **32-bar, 64-dance-beat loop**:

- a shorter tune is repeated at bar boundaries;
- a longer tune is trimmed at the loop boundary;
- pickup notes are preserved around the loop seam;
- short fades avoid clicks at the start and end of the rendered buffer;
- the accompaniment follows the quoted chords, or an inferred progression if
  the ABC has no chord symbols.

The UI does not show an extended/trimmed warning in the tune preview. The
arranger's warning appears on the session screen after the music is rendered.
The arrangement itself keeps the same 64-beat grid as the dance.

## Defaults and attribution

The bundled default is **Sandy Boys** (ABC Session Tunes 75815). Its credits
are shown in the preview: ABC transcription by Josh Larios, chords from Sarah
Comer. Tunes remain the work of their transcribers; the app caches them locally
rather than bundling a tune database.

## Errors and caching

A failed tune search or detail request appears in the red bottom panel. The
current tune is not replaced. Search results are cached for seven days and tune
details for thirty days.

<Shot id="error-tune-load" />

Next: [Session controls](/docs/guides/session-controls).
