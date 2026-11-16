---
title: Dance glossary
description: Look up contra dance terms in the app, follow a link from a call or dance card, and see the source of the definitions.
sidebar_position: 7
---

Contra Karaoke includes a short glossary of the dance terms that appear in a
dance card and in the rendered calls. It is a reading aid for a caller who meets
an unfamiliar figure during a session; it does not change the card, the timing,
or the music.

## Opening the glossary

There are two ways in:

- Select the **Glossary** button beside the **Dance card** heading on the
  **Session** tab.
- Select an underlined term in the call prompt or on the dance card. The
  glossary opens at that entry, so the definition is already on screen.

The **About** category of **Preferences** also has an **Open the dance glossary**
link. **Back** and **Esc** return to the screen that opened the glossary.

<Shot id="glossary" />

## Linked terms

Terms are linked wherever the call text is shown: the large prompt, the upcoming
move preview, the dance card, the original card wording when **Show original
call text** is on, and the figure list in a dance preview. The link keeps the
same size and colour as the surrounding text and is marked with an underline, so
it does not rely on colour alone.

Hovering a linked term shows its definition in a tooltip beside the pointer. On a
touch screen, rest a finger on the term instead; the definition appears in a card
at the top of the screen and stays briefly after the finger lifts. Selecting a
term still opens the glossary, and the entry for the selected term keeps an
emphasized box around it so it is easy to find in the list.

<Shot id="glossary-tooltip" />

Only recognised terms become links. Longest matches win, so "balance and swing"
is one link rather than a link for "balance" and another for "swing". A word such
as "swinging" is not treated as a link.

## Turning the glossary off

The glossary is on by default. Turn off **Show dance glossary** in
**Preferences → Appearance** to remove every part of it: terms are no longer
underlined or clickable, definition tooltips and the touch card do not appear,
the **Glossary** button leaves the Dance card header, and the **Open the dance
glossary** link leaves About. The saved glossary text is still on disk; turning
the option back on restores it.

Use **Filter terms** to search the whole list by name, alternative spelling, or
wording in the definition. The count below the filter shows how many entries
match.

## Source and wording

The reference is based on [The Caller's Box glossary of contra dance
figures](https://www.ibiblio.org/contradance/thecallersbox/Glossary.htm). The
definitions in Contra Karaoke are short original summaries written for this
app, not copied entries. Follow the source link in the app for the full
definitions, notation, and regional variants.

The glossary is stored with the app, so it works offline. It contains no dance
cards or tune data and is not fetched from The Caller's Box at runtime. The
source is credited in `CREDITS.md` and in the app's glossary page.

See also [Data sources](/docs/reference/data-sources) for how the app attributes
the dances and tunes it fetches.
