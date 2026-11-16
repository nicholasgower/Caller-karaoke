---
title: Keyboard shortcuts
description: Play, pause, restart, and leave a picker without using the mouse, plus hardware media keys.
sidebar_position: 1
---

Contra Karaoke has a small keyboard shortcut set. Shortcuts are ignored while a
text field or another control has keyboard focus, so typing a dance title does
not start or stop the music. On desktop, the keyboard's media keys control the
transport from any window; see [Media keys](#media-keys).

| Key | Action | Notes |
| --- | --- | --- |
| **Space** | Play or pause | If the music is playing, Space pauses; otherwise it starts or resumes. |
| **R** | Restart | Returns to the count-in and resets the call state. |
| **Esc** | Close About, go back, or return to **Session** | Closes the **About** window. Otherwise returns **Download dance** and **Download tune** to their saved-content lists, and **Select dance**, **Select tune**, **Preferences**, and the alias editor to **Session**. |
| **1–4** | Switch tabs | 1 Session, 2 Select dance, 3 Select tune, 4 Preferences. |
| **Enter** | Submit | Starts a search from a search field, creates a folder from the folder-name field, or adds the custom alias when the alias fields are filled. |

## Details

- **Space** uses the same path as the Play/Pause buttons. If the music needs a
  rebuild after a settings change, Space starts it and renders the new loop.
- **R** restarts even while paused; it clears the last-spoken call and starts
  from the count-in.
- **Esc** does not replace **Use this dance** or **Use this tune**. Leaving a
  picker does not change the loaded dance or tune. From a **Download** screen,
  Esc returns to the corresponding saved-content list. It also closes the
  **About** window; the other shortcuts are suspended while that window is open.
- **1–4** are ignored while a text field has focus, so typing a number does not
  change tabs.
- **Enter** in a search field submits the current filters; in the folder-name
  field it creates the folder; in the custom alias or alternative-call fields it
  adds the entry when both fields are filled.
- On a browser, click the app once before using Space so the canvas has focus.
  If the page scrolls instead, click the session background and try again.

The shortcuts are implemented in the application, not in the docs site. If a
shortcut does not respond, check that a text field does not have focus.

## Media keys

On Windows and Linux, the keyboard's **Play/Pause** media key controls the
session even when another window is in front. The app registers with the
operating system's media session (System Media Transport Controls on Windows,
MPRIS on Linux), so the desktop routes the key to Contra Karaoke while it is the
active player. The system media overlay shows the loaded dance as the title and
the tune as the artist.

- **Play** starts or resumes the session.
- **Pause** pauses it and stops speech. The beat is kept, so the next Play
  resumes from the same place.
- **Stop** pauses. **Next** and **Previous** are ignored.

Media keys are not app shortcuts: they work without clicking the window, while a
text field has focus, and while the window is minimized. The browser and Android
builds do not handle media keys in 0.1.0. If another app is playing media, the
operating system may send the key there instead; pause that app or use **Space**
in Contra Karaoke.

Next: [How calls are timed](/docs/reference/how-calls-are-timed).
