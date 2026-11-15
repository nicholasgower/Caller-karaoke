---
title: Themes and accessibility
description: Switch between the dark and light grey themes, and understand the current keyboard, touch, and screen-reader behaviour.
sidebar_position: 10
---

Contra Karaoke uses a neutral grey interface with limited colour. The large
prompt card adds a glanceable cue: green while the caller must speak the shown
call now, red while the card looks ahead to the next move, and grey for ready,
count-in, dancing, and completion. The label text still says **CALL NOW** or
**UPCOMING MOVE**, so the fill is never the only signal.

## Choose a theme

<Steps>
  <Step shot="prefs-theme-light">
    Open **Preferences → Theme** and select **Light** or **Dark**. Dark is the
    default.
  </Step>
  <Step shot="main-light">
    Return to **Session** to see the chosen theme on the call screen.
  </Step>
</Steps>

The light theme uses the same structure with its own light card and text palette:
a light grey page, near-black text, and muted secondary text. It is intended for
bright rooms and projectors.

<Shot id="main-light" />

## Interface and motion

<Steps>
  <Step shot="prefs-interface">
    Open **Preferences → Interface** to turn **Smooth animations** on or off.
    The keyboard reference sits below the checkbox.
  </Step>
</Steps>

Smooth animations fade and slide between screens and animate tab and highlight
changes. Turn them off to reduce motion; the app then switches screens
immediately. The docs site also respects `prefers-reduced-motion`.

## Touch and narrow windows

Every control in the phone session layout — tab, slider, transport button, and
toggle — uses a 44-pixel interaction height, and the app window has a minimum
size of 360×560. Below about 640 pixels wide the **Session** tab switches to
that phone layout: the wordmark and long tab labels give way to four short tabs
beside the pinned **About** button, the tempo and volume sliders use the full
width, and the transport buttons split the row evenly. On a short window the
whole session page scrolls so the dance card is never pushed out of reach.

<Shot id="main-mobile" />

The two-column pickers are the tightest layout on a phone. If the columns are
too narrow, rotate the device or use a wider window while searching. The
current UI has no separate mobile sheet; it keeps the same screen structure.

## Keyboard

- **Space** plays or pauses.
- **R** restarts from the count-in.
- **Esc** closes the **About** window or returns to **Session** from a picker,
  Preferences, or the alias editor.
- **1–4** switch tabs.
- **Enter** submits a search or adds a custom alias.

Shortcuts are ignored while a text field has focus. See
[Keyboard shortcuts](/docs/reference/keyboard-shortcuts) for details.

## Screen readers and labels

The app uses `egui`'s accessibility tree and is designed to expose control
labels, but the current UI is still being audited. Several fields (search boxes,
alias editors, and some sliders) are visually labelled without a formal
accessible-name association. Treat screen-reader support as incomplete for
0.1.0.

The documentation site itself is keyboard navigable, uses visible focus
outlines, and provides alt text for every screenshot. If you find a specific
unusable control, report it with the control name and screen so it can be
added to the label pass.

## Motion and flashing

The app does not flash or pulse the call prompt. The progress bar moves
continuously with playback. If screen transitions are distracting, turn off
**Smooth animations** in **Preferences → Interface**. The docs site respects
`prefers-reduced-motion` and uses no animated screenshots.

## Text-to-speech

TTS is optional and never the only way to receive a call. If no voice is
available, visual calls continue. See
[Text to speech](/docs/guides/text-to-speech).

Next: [Export MIDI](/docs/guides/export-midi).
