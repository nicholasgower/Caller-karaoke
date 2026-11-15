---
title: Troubleshooting
description: Recover from audio, source, TTS, SoundFont, timing, and rendering problems.
sidebar_position: 4
---

Most recoverable errors appear in a red panel at the bottom of the window.
Select **Dismiss** to close it. Some problems are reported inline where they
belong, such as the audio device message above the dance card or the SoundFont
message in Preferences.

## Quick checks

| Symptom | What to try |
| --- | --- |
| No music | Check **Volume**, the OS output device, and the `Audio device:` message. Pause and Play again. |
| Music does not start | The first Play renders the loop synchronously. Wait for it; the window may be busy for a moment. In a browser, press **Play** once to allow audio. |
| `Press Play to prepare the music…` will not go away | Press **Play**. Tempo, instrument, melody/accompaniment volume, SoundFont, and count-in changes require a rebuild. |
| A call seems late or early | Check **Calling lead** and the alias length. A longer call is moved earlier automatically. |
| The first call appears during the count-in | Expected. The first call starts before beat 0. |
| Keyboard shortcuts do nothing | Click a non-text area first. Space, R, and Esc are ignored while a text field has focus. |
| Search returns an error | Dismiss it and retry. Cached content may still be available. In a browser, check the web proxy URL/token. |
| Search results look old | The cache is 7 days for searches and 30 days for details. Clear it from Maintenance. |
| No spoken calls | Check the OS/browser voice, the **Speak calls** toggle, **Rate**, and that the music is playing. |
| SoundFont sounds wrong or silent | On desktop, remove it and return to the built-in synth; check the GM programs required by your file. |
| SoundFont loading is unavailable | The browser and Android builds cannot load `.sf2` in 0.1.0; use the built-in synth. |
| MIDI export is unavailable | Export is desktop-only in 0.1.0; the browser and Android builds report the limitation. |
| A card does not total 64 beats | The card is allowed; the music keeps its 64-beat grid. Check the preview total. |

## Source errors

A failed network request or parse is shown in the bottom panel with the source's
message. The currently loaded dance and tune are not replaced.

<Shot id="error-network" />

<Shot id="error-parse" />

If the app can use stale cached content, it does so silently. If it cannot, it
shows the error. There is no separate offline banner in 0.1.0.

## Audio errors

If no output device is available, the app falls back to a silent engine and
shows an `Audio device:` message. The session still runs so you can read the
calls, but there is no sound. Restore the device and restart the app.

<Shot id="error-audio" />

In the browser, audio requires a user gesture. If the page has just loaded,
press **Play**. If the browser still blocks audio, check the site's sound
permission and any muted tab.

The audio callback is real-time and does not allocate or block. If the audio
stream reports an error, the app logs it and keeps the UI running.

## Browser-specific problems

- **No online search:** deploy and configure the `proxy/` worker, then set the
  URL and token under **Preferences → Web data proxy**. Defaults and cached
  content still work without it.
- **Preferences did not persist:** the browser build uses `localStorage`. Check
  that site data is not being cleared on exit.
- **Page does not load the app:** serve the bundle over HTTP; `file://` will not
  start the WASM app.

## Android-specific problems

Android audio and TTS are still being device-tested. If the app opens but has no
sound, check the media volume and the device's audio route. Loading a SoundFont
and exporting MIDI are not available in the Android build yet.

## TTS errors

If the system has no speech voice, **Speak calls** does not produce sound. The
Preferences section shows the reason in muted text. Visual calls continue; see
[Text to speech](/docs/guides/text-to-speech).

## SoundFont errors

A failed `.sf2`/`.sf3` load appears below the current SoundFont name. The app
keeps the previous SoundFont or the built-in synth. Remove the file and try a
different one.

<Shot id="error-sf2" />

## Rendering and timing

The app renders the count-in and loop offline before playback. A long tune, a
large SoundFont, or a slow machine can make the first Play take a moment. There
is no cancel button in 0.1.0; the window may appear busy while rendering.

If the music and the call prompt appear to drift, confirm that the tempo matches
the band and that the machine is not overloaded. The UI follows the audio
playback position, not wall-clock time.

## Still stuck?

Open an issue with:

- the screen and action;
- the exact error text;
- whether it reproduces with the bundled defaults;
- the platform and whether audio/TTS/SF2 were involved.

Do not include proxy tokens, private dance URLs, or full file paths.
