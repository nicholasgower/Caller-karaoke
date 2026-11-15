# Architecture

```
crates/core      pure domain: dances, tunes, roles, aliases, timing, prefs
crates/music     ABC parsing, chords, arrangement, synth, MIDI, render
crates/sources   provider traits, ContraDB, ABC Session Tunes, cache
crates/app       eframe UI, cpal transport, TTS, platform storage
crates/web       WASM entry point and Trunk page
crates/mcp       development-only MCP server for agent-driven UI testing
```

The data flow is:

```
UI -> SourceClient -> provider request/parser -> Dance/Tune
   -> AliasTable + PracticeTimeline -> CallEvent list
   -> ABC parser -> Score -> Arrangement (64 beats)
   -> bundled SoundFont, user SoundFont, or built-in synth -> PCM buffer -> cpal
   -> MIDI export
```

The tune picker engraves the ABC separately from playback: `acorde` builds the
display score, `acorde-render-svg` emits SVG, and `resvg` rasterizes it onto a
paper-white texture for `egui`. This preview path never feeds the call timeline
or the synthesiser.

Rules:

- `core` has no UI, network, filesystem, or audio dependencies.
- `music` is deterministic and offline.
- `sources` parses bytes; it never touches `egui`.
- `app` is the only crate that performs I/O, audio, or speech. The desktop
  speech path is a Kokoro worker thread; it downloads and caches the model,
  phonemizes on the worker, applies the caller-tone clear-speech pass
  (lengthening stressed vowels in the phoneme stream), synthesizes with a
  duration-neutral pitch shift, then high-passes, lifts presence, compresses,
  and mixes the result into the cpal output stream. Web and Android use the
  platform TTS backend.
- `mcp` is a development/QA tool. It drives a development build through the
  `inspection` feature (off by default) and is not part of a release artifact.

See `AGENTS.md` for the full dependency and coding rules, and
`docs/automation.md` for the MCP workflow.
