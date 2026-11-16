---
title: Choosing a tune
description: Search your saved tunes, organise them into folders, build a tune set for cycling, and download another ABC Session Tunes melody.
sidebar_position: 2
---

**Select tune** lists the tunes saved on this device. Saved tunes are available
offline; they are not the same as the temporary source cache. The bundled
default, *Sandy Boys*, is in the list from first launch.

Unless **Cycle tunes** is on, the current tune stays loaded until you press
**Use** or **Use this tune**.

## Build a tune set for a long session

For a real dance the music usually changes every few minutes. Add tunes to the
**tune set** with **Add to tune set** on a saved tune (or **Add to set** on its
list row), then turn on **Cycle tunes** in the session controls. Each saved tune
can be in the set once; **In set** marks the ones that are already included, and
selecting the button again removes a tune.

The session line shows the current position and the next tune, and **Change
tune every** sets how many 64-beat times through each tune stays. The default is
eight, about four minutes at 240 BPM. Cyclic playback needs **Loop** on.

<Shot id="tune-library-tune-set" />

## Find a saved tune

<Shot id="tune-library" />

Type into **Search saved** to filter the list. The search is local and matches
the tune title, composer, key, meter, genre, and credits. Every word you type
must match. Select a saved tune to read its metadata and engraved sheet music on
the right, or **Use** to load the tune.

The saved list and the sheet-music preview fill the height below the search field
and scroll inside their own panes, so a long tune or a long folder list never
pushes the search field off screen. The **Use**, **Add to set** / **In set**, and
**Folder:** actions on a row, and the preview actions, span their row; on a phone
they stack as full-width buttons.

<Shot id="tune-library-search" />

The folder panel works exactly as it does for dances:

| Choice | Shows |
| --- | --- |
| **All tunes** | Every saved tune. |
| **Unfiled** | Tunes that are not in a folder. |
| **A folder name** | Only the tunes in that folder. |

## Organise saved tunes into folders

Create a folder with **New folder** and **Create folder**, then open the
**Folder:** menu on a saved tune to move it. The counts beside **All tunes**,
**Unfiled**, and each folder update immediately.

Folder names are case-insensitive. Select a folder to reveal **Rename folder**
and **Delete folder**. Renaming keeps the folder's tunes with it. Deleting a
folder keeps its tunes and moves them to **Unfiled**. **Remove from saved
tunes** deletes that saved copy only; a tune already loaded in the session keeps
playing.

## Download another tune

Select **Download tune** at the top of **Select tune** to open the ABC Session
Tunes search screen. Previewing a tune does not save it; use **Download tune**,
or **Use this tune** to download and load it at once.

The search form stays at the top, the **Previous** and **Next** row stays at the
bottom, and the result list and sheet-music preview share the height in between.
On a phone the form, results, and preview stack into one scrolling page.

<Shot id="tune-picker-open" />

| Field | What it does |
| --- | --- |
| **Search** | Matches the tune title (and the source's search index). |
| **Composer** | Matches the composer name. |
| **Key** | Any key, or a curated list including D/G/A/C/F/B♭ major, A/E/B/D minor, A dorian, and A mixolydian. |
| **Meter** | Any meter, or 4/4, 2/2, 2/4, 6/8, 9/8, 12/8, or 3/4. |
| **Genre** | Any genre, or reel, jig, slip jig, hornpipe, polka, slide, strathspey, or waltz. |

The key and genre lists are curated rather than exhaustive. ABC Session Tunes
accepts other values, but the UI keeps the common contra choices short.

## Search and page through results

<Steps>
  <Step shot="tune-search">
    Type a title or composer and select **Search tunes**. Results show the
    title, composer, key, and meter. A **downloaded** line marks tunes already
    in your saved tunes.
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

## Preview, download, and load

<Steps>
  <Step shot="tune-picker-preview">
    Select **Preview** on a result. The right column shows the title, composer,
    key, meter, credits, the rendered sheet music, and an **Open on ABC Session
    Tunes** link. **Download tune** saves the tune for offline use; **Use this
    tune** saves and loads it. **Show ABC** switches between the staff and the
    source text.
  </Step>
</Steps>

The sheet music is engraved from the ABC by the app. The **Show ABC** view
displays the source text in a preview field; edits there are local and do not
change the loaded tune or the saved copy. If the engraver cannot render the
tune, the app explains why and the ABC view remains available.

## What happens to the tune

The arranger turns the tune into a **32-bar, 64-dance-beat loop**:

- a shorter tune is repeated at bar boundaries;
- a longer tune is trimmed at the loop boundary;
- pickup notes are preserved around the loop seam;
- short fades avoid clicks at the start and end of the rendered buffer;
- the accompaniment follows the quoted chords, or an inferred progression if
  the ABC has no chord symbols, using the selected rhythm;
- the accompaniment stays out of the melody's register: a chord tone closer
  than a minor third to the tune is moved down an octave;
- the countermelody adds a countermelody derived from the melody and that
  chord track.

The UI does not show an extended/trimmed warning in the tune preview. The
arranger's warning appears on the session screen after the music is rendered.
The arrangement itself keeps the same 64-beat grid as the dance.

## Defaults, storage, and attribution

The bundled default is **Sandy Boys** (ABC Session Tunes 75815). Its credits
are shown in the preview: ABC transcription by Josh Larios, chords from Sarah
Comer. Tunes remain the work of their transcribers.

Saved tunes are stored with your app data: JSON files on desktop and Android,
`localStorage` in the browser. They survive **Clear source cache**, which only
clears temporary search and detail responses. See
[Cache and offline](/docs/guides/cache-and-offline).

## Errors and storage

A failed tune search or detail request appears in the red bottom panel. The
current tune is not replaced. Search results are cached for seven days and tune
details for thirty days.

<Shot id="error-tune-load" />

Next: [Session controls](/docs/guides/session-controls).
