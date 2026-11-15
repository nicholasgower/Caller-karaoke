#![forbid(unsafe_code)]

//! ABC sequencing, arrangement, synthesis, and MIDI export.

pub mod abc;
pub mod arrange;
pub mod chord;
pub mod midi;
pub mod score;

pub use abc::{AbcError, parse_abc};
pub use arrange::Arrangement;
pub use chord::{Chord, ChordQuality, resolve_chords};
pub use midi::export_midi;
pub use score::{ChordEvent, Key, Meter, Mode, NoteEvent, Score};
