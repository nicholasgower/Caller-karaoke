# Memory use

This page records where Contra Karaoke's memory goes, what is measured in the
app, and which reductions are already available. Numbers vary by platform,
window size, GPU driver, and whether the neural voice has ever run. The values
below were measured on Windows with a 48 kHz stereo output and the bundled
defaults.

Users have reported roughly 540 MB resident (RSS). A prepared session that has
also used the neural voice explains most of that; the rest is runtime and
driver overhead that the app does not own.

## The short version

| Consumer | Approximate resident memory | How it gets there |
| --- | ---: | --- |
| Neural voice (Kokoro + ONNX Runtime) | ~155 MiB | Optional; loaded when **Speak calls** is on. 92 MiB quantized model plus session arenas and the 16.5 MiB runtime |
| Rendered music, four variation passes | ~50 MiB | Count-in + 4 × 64-beat passes + ending, interleaved stereo f32 at 48 kHz |
| RAM render cache | shares the rendered music allocation | The cache and the audio callback hold the same `Arc`; a settings change can briefly hold the old loop and the new one |
| Next tune prepared for **Cycle tunes** | ~50 MiB | A second rendered pass held until the rotation boundary |
| Bundled SoundFont file | ~21 MiB | `fluid-r3-gm-subset.sf2` is inside the executable |
| Decoded SoundFont samples | ~21–22 MiB | rustysynth decodes the bank to `i16` sample data |
| Fonts and glyph atlas | ~1 MiB | egui's bundled fonts plus the rasterized atlas |
| Saved libraries, caches, preferences, undo snapshots | < 2 MiB | Small JSON cards and raw source responses; the desktop source cache is on disk |
| Runtime, GPU driver, allocator, thread stacks | remainder | eframe/wgpu/GL, the OS, and allocator arenas; not controlled by this app |

A cold session with speech off is therefore around 120–150 MiB. Using speech
once adds the model; cycling tunes adds the prepared buffer, and a settings
change can briefly hold the previous loop while the replacement renders. The
reported 540 MB suggests speech on, cycling on, and platform/driver overhead on
top of the table above.

## What the app reports

**Preferences → Data & Storage → Memory use** shows the categories for the running
app and has a **Refresh memory use** button. The same table is printed by:

```sh
cargo test -p contra-app memory_report -- --nocapture
```

Representative output from the test (headless, 1280×800, speech off, four
variation passes):

```
Category                          MiB  Source
bundled_soundfont_file           20.8  measured
soundfont_samples                 0.0  measured
audio_buffer                     49.8  measured
prepared_tune                     0.0  measured
speech_audio                      0.0  measured
arrangement                       0.0  measured
call_timeline                     0.0  measured
saved_libraries                   0.0  estimated
preferences                       0.0  estimated
picker_state                      0.0  estimated
fonts                             0.9  measured
sheet_music_texture               0.0  measured
undo_history                      0.0  estimated
source_cache                      0.0  measured
neural_tts                        0.0  estimated
Total                            71.5  total
```

The test measures the rounded-to-zero categories at byte level; they are below
0.05 MiB in this state. The neural estimate is larger than the 92 MiB model
file because ONNX Runtime keeps per-inference arenas beside the weights; the
app uses 1.5× the file size plus the runtime library as a labelled estimate.

## Where each category comes from

- **Bundled SoundFont file.** `assets/defaults/fluid-r3-gm-subset.sf2` is
  `include_bytes!`-ed into the executable. It is attributed in `CREDITS.md` and
  is the default voice. The file is in the binary even when it has not been
  parsed.
- **Decoded SoundFont samples.** On the first render, `rustysynth` parses the
  bank into 16-bit sample data plus preset and instrument tables. A user-loaded
  `.sf2` replaces the bundled one. Loading a user bank now drops the parsed
  bundled copy; it is parsed again only if the user returns to **Use bundled**.
- **Rendered music.** The arranger produces exactly 64 dance beats. With
  **Vary the music on each loop** on (the default) four passes are rendered back
  to back, plus the four-beat count-in and the once-only ending. At 240 BPM one
  pass is 32 s, so the buffer is about 50 MiB at 48 kHz stereo.
- **Prepared tune.** **Cycle tunes** renders the next tune on a worker while the
  current one plays. The finished buffer waits at the pass boundary, so two
  music buffers can be resident during a rotation. The current tune has the same
  treatment: an idle prefetch fills the RAM cache, and a playback-setting change
  can render the replacement while the old loop keeps playing.
- **Speech.** The platform voice costs the app almost nothing. On desktop the
  Kokoro neural voice is the default when available: a 92 MiB quantized model
  is loaded into ONNX Runtime, with session arenas and the 16.5 MiB runtime.
  Turning **Speak calls** off releases the model and the synthesized-call
  cache; turning it back on reloads the already-cached files.
- **Fonts.** egui's default fonts are a few hundred KiB of font data; the glyph
  atlas is a rasterized texture that grows with the characters actually drawn.
- **Libraries and caches.** Saved dances and tunes are JSON cards, typically a
  few KiB each. Search responses are cached; the desktop cache is on disk and
  reports zero resident bytes, while the web build uses `localStorage`.
- **Sheet-music texture.** The tune preview is rasterized once and capped at
  1200×2400. It is one texture, freed when the preview is cleared or the
  display scale changes.
- **Undo history.** Up to 100 snapshots of preferences and the selected cards.
  Each snapshot is small, but the category is shown so a long session is not a
  surprise.

## Reducing memory today

These are already possible from the UI:

1. **Turn off Speak calls** when the session does not need spoken prompts.
   This releases the neural model (the next enable reloads it from the cache).
2. **Turn off Vary the music on each loop** in **Preferences → Playback**. The
   next render has one pass instead of four, cutting the music buffer to about
   one quarter.
3. **Leave Cycle tunes off**, or keep the tune set small. Cycling keeps a second
   rendered buffer ready.
4. **Use one SoundFont.** A user bank replaces the bundled sample data; the
   bundled file remains in the executable image but is not decoded.
5. **Clear the source cache** if the web build has accumulated responses. The
   desktop cache is already on disk.
6. **Close a picker preview.** The sheet texture is released when the preview is
   cleared or another tune is selected.

## Changes made for this report

- The Preferences maintenance section gained the measured table and a refresh
  button.
- Loading a user SoundFont now drops the parsed bundled bank instead of keeping
  both sample sets resident.
- Disabling the neural voice now releases the ONNX session and the
  synthesized-call cache in the worker; enabling it loads from the cached files.
- Preference writes are skipped when the values on disk are already current,
  so a control changed and changed back does not rewrite the file.

## Possible future reductions

These are not implemented and would need a product decision:

- Render one pass and apply the variation passes at playback time, trading CPU
  for memory.
- Stream the rendered loop from disk instead of holding it in RAM.
- Release the prepared rotation buffer earlier when the rotation is disabled.
- Make the neural voice optional at build time for a small installer, or expose
  a "release voice memory" action with an explicit reload cost.
- Swap the four-pass default for one pass on low-memory devices while keeping
  the 64-beat grid and timing invariants.

Numbers vary by platform. Linux and Android allocators and the web build's
browser-managed memory do not match the Windows figures above; the in-app table
is the source of truth for a given run.
