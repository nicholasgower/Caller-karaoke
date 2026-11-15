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
| **Esc** | Return to **Session** | Works from **Select dance**, **Select Tune**, **Preferences**, and the alias editor. Does nothing on the session screen. |

## Details

- **Space** uses the same path as the Play/Pause buttons. If the music needs a
  rebuild after a settings change, Space starts it and renders the new loop.
- **R** restarts even while paused; it clears the last-spoken call and starts
  from the count-in.
- **Esc** does not replace **Use this dance** or **Use this tune**. Leaving a
  picker does not change the loaded dance or tune.
- Shortcuts do not switch tabs, focus search fields, or open Preferences.
- On a browser, click the app once before using Space so the canvas has focus.
  If the page scrolls instead, click the session background and try again.

The shortcuts are implemented in the application, not in the docs site. If a
shortcut does not respond, check that a text field does not have focus.

Next: [How calls are timed](/docs/reference/how-calls-are-timed).
