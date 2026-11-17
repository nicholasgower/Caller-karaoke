# caller-core

Pure domain logic for [Caller Karaoke][repo], a contra dance caller trainer: the
parts that turn a dance card into a timed sequence of calls, with no UI, network,
audio, or platform code.

The crate is published so other tools can reuse the timing and wording model. The
API is 0.x and follows the application: expect breaking changes between minor
versions.

[repo]: https://github.com/nicholasgower/Caller-karaoke

## What is here

- `Dance`, `Figure`, `SourceRef` — a dance card as fetched from a source or
  saved by the user.
- `LeadSettings`, `PracticeTimeline`, `CallEvent`, `ScheduledCall` — when each
  call is spoken, when the figure executes, and which call is current at a beat.
- `AliasTable`, `MoveKind`, `RenderedCall` — caller wording for a figure, with
  the configured role terms substituted.
- `RoleTerms`, `RolePreset` — larks/robins and the other role vocabularies.
- `Beats`, `Seconds`, `Samples`, `Tempo` — explicit time units, so beat,
  wall-clock, and sample conversions live in one place.
- `Preferences`, `Library` — persisted settings with a schema version and
  migration.

## Timing in one paragraph

One dance beat is half a bar. At the default 240 BPM a dance beat is 0.5 s, so a
standard 64-beat dance lasts 32 s and a jig bar sounds the same length as a reel
bar. A call's lead is computed from its rendered text — syllable count, word
count, and the speech rate — clamped to the user's lead range and extended by a
safety padding, so the caller starts speaking before the dancers move. The first
call of a dance may start before beat 0, during the count-in; it is not clamped.

```rust
use caller_core::{Beats, Seconds, Tempo};

let tempo = Tempo::default(); // 240 BPM: half a second per dance beat
assert_eq!(Seconds::from_beats(Beats(64.0), tempo).0, 32.0);
```

## Building

Part of the Caller Karaoke workspace; build and test it from the repository root:

```sh
cargo test -p caller-core
```

## License

MIT. See [LICENSE](https://github.com/nicholasgower/Caller-karaoke/blob/main/LICENSE).
