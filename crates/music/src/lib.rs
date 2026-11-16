#![forbid(unsafe_code)]

//! ABC sequencing, arrangement, synthesis, and MIDI export.

pub mod abc;
pub mod arrange;
pub mod chord;
pub mod midi;
pub mod percussion;
pub mod render;
pub mod score;
pub mod soundfont;
pub mod synth;

pub use abc::{AbcError, parse_abc};
pub use arrange::{Arrangement, Ending, EndingNote, LengthMismatch, VARIATION_PASSES};
pub use chord::{Chord, ChordQuality, resolve_chords};
pub use midi::export_midi;
pub use render::{AudioBuffer, RenderError, RenderRequest, render};
pub use score::{ChordEvent, Key, Meter, Mode, NoteEvent, Score};
