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
   -> built-in synth or user SoundFont -> PCM buffer -> cpal
   -> MIDI export
```

Rules:

- `core` has no UI, network, filesystem, or audio dependencies.
- `music` is deterministic and offline.
- `sources` parses bytes; it never touches `egui`.
- `app` is the only crate that performs I/O, audio, or speech.
- `mcp` is a development/QA tool. It drives a development build through the
  `inspection` feature (off by default) and is not part of a release artifact.

See `AGENTS.md` for the full dependency and coding rules, and
`docs/automation.md` for the MCP workflow.
