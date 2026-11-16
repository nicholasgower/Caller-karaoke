---
title: Keyboard shortcuts
description: Play, pause, restart, open Preferences, and leave a picker without using the mouse, plus hardware media keys.
sidebar_position: 1
---

Contra Karaoke has a small keyboard shortcut set. Shortcuts are ignored while a
text field or another control has keyboard focus, so typing a dance title does
not start or stop the music. On Windows, Linux, and Android, the operating
system's media controls drive the transport from outside the app; see
[Media keys](#media-keys).

| Key | Action | Notes |
| --- | --- | --- |
| **Space** | Play or pause | If the music is playing, Space pauses; otherwise it starts or resumes. |
| **R** | Restart | Returns to the count-in and resets the call state. |
| **Esc** | Go back or return to **Session** | Returns **Download dance** and **Download tune** to their saved-content lists, the alias editor to **Preferences**, the **Dance glossary** to the screen that opened it, and **Select dance**, **Select tune**, and **Preferences** to **Session**. On a phone, a Preferences category steps back to the category list first. Android's system back button does the same. |
| **1-3** | Switch tabs | 1 Session, 2 Select dance, 3 Select tune. |
| **4** | Open **Preferences** | Preferences is the gear button at the top right rather than a tab, but its number key still works from any screen. On a wide header the button also shows the word **Preferences**. |
| **← / →** | Previous or next tab | The tab row wraps. Ignored while a control has keyboard focus. |
| **Enter** | Submit | Starts a search from a search field, creates a folder from the folder-name field, or adds the custom alias when the alias fields are filled. |
| **Ctrl+Z** | Undo | Restores the state before the last preference or dance/tune change. The last 100 changes are kept. On macOS this would be Cmd+Z. |
| **Ctrl+Y** or **Ctrl+Shift+Z** | Redo | Re-applies a change that was just undone. A new edit discards the redo steps. |

## Details

- **Space** uses the same path as the **Play/Pause** button. If the music needs a
  rebuild after a settings change, Space starts it and renders the new loop.
- **R** restarts even while paused; it clears the last-spoken call and starts
  from the count-in.
- **Esc** does not replace **Use this dance** or **Use this tune**. Leaving a
  picker does not change the loaded dance or tune. From a **Download** screen,
  Esc returns to the corresponding saved-content list, and from **Preferences**
  it returns to **Session**. The **Dance glossary** returns to the session or
  Preferences page that opened it. On a phone it first leaves a Preferences
  category for the category list. The About text is the **About** category of
  **Preferences**, so nothing on that screen is modal and the transport
  shortcuts keep working.
- **1-3** are ignored while a text field has focus, so typing a number does not
  change tabs. **4** does the same and opens **Preferences**.
- **← / →** are ignored whenever any control has keyboard focus. A focused
  slider still moves with the arrow keys, and a focused list still scrolls, so
  the shortcuts never steal an arrow key from the control you are using. On a
  detail screen, such as **Download dance**, the arrows step from the tab that
  contains it.
- **Enter** in a search field submits the current filters; in the folder-name
  field it creates the folder; in the custom alias or alternative-call fields it
  adds the entry when both fields are filled.
- **Ctrl+Z** and **Ctrl+Y** work on every screen, including **Preferences** and
  the alias editor. They step through preferences (role terms, aliases,
  instruments, band rows, caller and band levels, output mode, theme, timing,
  speech, proxy) and the
  selected dance and tune. Typing in a text field is left to the field's own
  undo: the app shortcut only fires when no text field has focus, and an open
  drop-down keeps it from firing. A drag on a slider is one step, not one step
  per pixel. Downloading, removing, or re-filing a saved card is a library
  change rather than a setting, so it is not undone.
- On a browser, click the app once before using Space so the canvas has focus.
  If the page scrolls instead, click the session background and try again.

The shortcuts are implemented in the application, not in the docs site. If a
shortcut does not respond, check that a text field does not have focus.

## Touch screens

On Android and other touch devices, **swipe left or right** across empty screen
content to move to the next or previous tab. The gesture needs about 60 points
of mostly horizontal travel and changes the tab once per swipe. A press that
starts on any control — a button, slider, folder row, resizable splitter, text
field, or open menu — belongs to that control and never changes tabs. A mostly
vertical swipe scrolls instead. The
system back button follows the **Esc** path: a Preferences category returns to
the category list, and the list, a picker, or a download screen returns as
described above.

## Media keys

On Windows, Linux, and Android, the operating system's media controls drive the
session from outside the app. That covers the keyboard's **Play/Pause** key, the
Windows media overlay, the media widget in a Linux panel (including the Cinnamon
sound applet on Linux Mint), and on Android the headset and Bluetooth buttons,
the lock screen, and the media notification.

Contra Karaoke registers a media session with the platform — System Media
Transport Controls on Windows, MPRIS on Linux, and `MediaSession` on Android —
so the operating system routes the button to the app while it is the active
player. The system media controls show the loaded dance as the title, the tune
as the artist, and *Contra Karaoke* as the album.

- **Play** starts or resumes the session.
- **Pause** pauses it and stops speech. The beat is kept, so the next Play
  resumes from the same place.
- **Stop** pauses. **Next** and **Previous** are ignored.

Media keys are not app shortcuts: they work without clicking the window, while a
text field has focus, and while the window is minimized. On Android, denying the
notification permission only hides the notification; the headset, Bluetooth,
lock-screen, and system controls still work.

If another app is playing media, the operating system may send the key there
instead; pause that app or use **Space** in Contra Karaoke. The browser build
does not register a media session in 0.1.0, so use **Space** in the page.

Next: [How calls are timed](/docs/reference/how-calls-are-timed).
