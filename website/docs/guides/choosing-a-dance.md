---
title: Choosing a dance
description: Search ContraDB by title, choreographer, user, hook, formation, figure type, or figure words, then preview and load a dance card.
sidebar_position: 1
---

**Select dance** opens a ContraDB search screen. It is a normal form: fill in
any combination of fields, select **Search dances**, and choose a result.

The current dance is not replaced until you press **Use this dance**. Closing
the screen with **Back to session** leaves the session untouched.

## Search fields

| Field | What it does |
| --- | --- |
| **Title** | Matches words in the dance title. |
| **Choreographer** | Matches the choreographer name. |
| **User** | Matches the ContraDB user who contributed the card. |
| **Hook** | Matches the card's short hook/tag line. |
| **Formation** | Improper, Becket, Proper, Everything else, or Any formation. |
| **Figure type** | A move-type selector: Swing, Allemande, Do-si-do, Balance, Chain, Circle, Star, Hey, Gyre, Promenade, Long lines, Right and left through, Pass through, Petronella, Mad robin, Box the gnat, Swat the flea, Roll away, or Slide. |
| **Figure contains** | Searches the card text for a move phrase, for example `chain` or `balance`. |
| **Verified only** | Restricts results to ContraDB's verified tag. |
| **Sort** | Relevance, Title A–Z, Title Z–A, Choreographer A–Z, or Newest. |

<Shot id="dance-picker-open" />

**Figure type** takes precedence over **Figure contains** when both are set: the
provider sends a move-type filter and ignores the free-text figure clause. Use
one or the other when you want a predictable search.

**Figure contains** is a text match against the card, not a move-type
classifier. Use a short distinctive phrase; the alias classifier runs only after
a dance is loaded.

<Shot id="dance-filter-user" />
<Shot id="dance-filter-hook" />
<Shot id="dance-filter-figure-type" />

## Search and page through results

<Steps>
  <Step shot="dance-picker-search">
    Fill in a field and select **Search dances**. Results appear on the left
    with title, choreographer, and formation.
  </Step>
  <Step shot="dance-sort">
    Change **Sort**, then search again. Sorting applies to the next request.
  </Step>
  <Step shot="dance-pagination">
    Use **Previous** and **Next** under the results. The current page and total
    are shown between them; results are ten per page.
  </Step>
</Steps>

If you are offline, the app may return a cached search page or cached detail.
There is no “using cached results” banner in 0.1.0; stale data is used silently
when a request fails and a cache entry exists.

## Preview before loading

<Steps>
  <Step shot="dance-picker-preview">
    Select **Preview** on a result. The right column shows the title,
    choreographer, formation, figure count, beat total, each figure with its
    phrase label and beats, and an **Open on ContraDB** link.
  </Step>
</Steps>


Check the beat total. A standard contra card is **64 beats**, which is exactly
one 64-beat music loop. Cards that total fewer or more beats can be previewed,
but the current transport has only one 64-beat loop:

- calls follow the card's cumulative figure beats up to beat 64;
- a short card leaves music playing after the last call;
- a long card's figures after beat 64 are not reached, because the playhead
  resets at the loop boundary.

The preview shows the card total. After you load it, the session compares the
dance total with the tune body length and shows a **length check** warning when
they differ. It does not rescale either one. A card longer than 64 beats is
still not fully reached because the transport has one 64-beat loop.

<Shot id="session-length-warning" />

<Shot id="dance-preview-beats" />

## Load the dance

Select **Use this dance**. The app returns to **Session**, replaces the card,
rebuilds the call timeline, and marks the music for a rebuild. Press **Play** to
render and start the new arrangement.

<Shot id="main-dance-loaded" />

## Errors and caching

A failed search or detail request appears in a red panel at the bottom of the
window. Select **Dismiss** to close it. The current dance stays loaded.

<Shot id="error-dance-load" />

Search responses are cached for seven days and dance details for thirty days.
Clearing the cache is covered in [Cache and offline](/docs/guides/cache-and-offline).

## Attribution

Dances come from [ContraDB](https://contradb.com) and remain the work of their
choreographers and transcribers. The preview includes an **Open on ContraDB**
link for the loaded card; use it to check the original source and credit. The
bundled default is *Another Last Dance* by Erik Hoffman (ContraDB 2409).

Next: [Choosing a tune](/docs/guides/choosing-a-tune).
