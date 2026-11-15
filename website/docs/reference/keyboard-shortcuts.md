---
title: Keyboard shortcuts
description: Play, pause, restart, and leave a picker without using the mouse.
sidebar_position: 1
---

Contra Karaoke has a small keyboard shortcut set. Shortcuts are ignored while a
text field or another control has keyboard focus, so typing a dance title does
not start or stop the music.

| Key | Action | Notes |
| --- | --- | --- |
| **Space** | Play or pause | If the music is playing, Space pauses; otherwise it starts or resumes. |
| **R** | Restart | Returns to the count-in and resets the call state. |
| **Esc** | Go back one screen | Returns **Download dance** and **Download tune** to their saved-content lists, and **Select dance**, **Select tune**, **Preferences**, and the alias editor to **Session**. Does nothing on the session screen. |
| **1–4** | Switch tabs | 1 Session, 2 Select dance, 3 Select tune, 4 Preferences. |
| **Enter** | Submit | Starts a search from a search field, creates a folder from the folder-name field, or adds the custom alias when the alias fields are filled. |

## Details

- **Space** uses the same path as the Play/Pause buttons. If the music needs a
  rebuild after a settings change, Space starts it and renders the new loop.
- **R** restarts even while paused; it clears the last-spoken call and starts
  from the count-in.
- **Esc** does not replace **Use this dance** or **Use this tune**. Leaving a
  picker does not change the loaded dance or tune. From a **Download** screen,
  Esc returns to the corresponding saved-content list.
- **1–4** are ignored while a text field has focus, so typing a number does not
  change tabs.
- **Enter** in a search field submits the current filters; in the folder-name
  field it creates the folder; in the custom alias or alternative-call fields it
  adds the entry when both fields are filled.
- On a browser, click the app once before using Space so the canvas has focus.
  If the page scrolls instead, click the session background and try again.

The shortcuts are implemented in the application, not in the docs site. If a
shortcut does not respond, check that a text field does not have focus.

Next: [How calls are timed](/docs/reference/how-calls-are-timed).
