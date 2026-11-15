# Architecture

```
crates/core      pure domain: dances, tunes, roles, aliases, timing, prefs
crates/music     ABC parsing, chords, arrangement, synth, MIDI, render
crates/sources   provider traits, ContraDB, ABC Session Tunes, cache
crates/app       eframe UI, cpal transport, TTS, platform storage
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

See `AGENTS.md` for the full dependency and coding rules.
