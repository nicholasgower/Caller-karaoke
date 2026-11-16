---
title: Keyboard shortcuts
description: Play, pause, restart, and leave a picker without using the mouse, plus hardware media keys.
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
| **Esc** | Close About, go back, or return to **Session** | Closes the **About** window. Otherwise returns **Download dance** and **Download tune** to their saved-content lists, the alias editor to **Preferences**, and **Select dance**, **Select tune**, and **Preferences** to **Session**. |
| **1–4** | Switch tabs | 1 Session, 2 Select dance, 3 Select tune, 4 Preferences. |
| **← / →** | Previous or next tab | The tab row wraps: Left from Session reaches Preferences, Right from Preferences returns to Session. Ignored while a control has keyboard focus. |
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
- **← / →** are ignored whenever any control has keyboard focus. A focused
  slider still moves with the arrow keys, and a focused list still scrolls, so
  the shortcuts never steal an arrow key from the control you are using. On a
  detail screen, such as **Download dance**, the arrows step from the tab that
  contains it.
- **Enter** in a search field submits the current filters; in the folder-name
  field it creates the folder; in the custom alias or alternative-call fields it
  adds the entry when both fields are filled.
- On a browser, click the app once before using Space so the canvas has focus.
  If the page scrolls instead, click the session background and try again.

The shortcuts are implemented in the application, not in the docs site. If a
shortcut does not respond, check that a text field does not have focus.

## Touch screens

On Android and other touch devices, **swipe left or right** across the screen
content to move to the next or previous tab. The gesture needs about 60 points
of mostly horizontal travel and changes the tab once per swipe; it works over
labels and cards but not while a text field has focus or a pop-up is open. A
swipe that starts on a slider is treated as an adjustment, and a mostly
vertical swipe on a slider scrolls the page instead of changing the value.

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
