#![forbid(unsafe_code)]

//! ABC sequencing, arrangement, synthesis, and MIDI export.

pub mod abc;
pub mod arrange;
pub mod band;
pub mod bowing;
pub mod chord;
pub mod copies;
pub mod dancer;
pub mod effects;
pub mod humanize;
pub mod layout;
pub mod midi;
pub mod percussion;
pub mod render;
pub mod score;
pub mod soundfont;
pub mod synth;

pub use abc::{AbcError, parse_abc};
pub use arrange::{Arrangement, Ending, EndingNote, LengthMismatch, VARIATION_PASSES};
pub use chord::{Chord, ChordQuality, resolve_chords};
pub use humanize::{
    CHORD_ASYNCHRONY_LIMIT_SECONDS, Humanizer, ONSET_OFFSET_LIMIT_SECONDS,
    PITCH_DEVIATION_LIMIT_CENTS, PITCH_DEVIATION_SIGMA_CENTS, PidGains, PitchHumanizer,
    TEMPO_DEVIATION_LIMIT, VoiceTiming,
};
pub use midi::{
    apply_effects, export_midi, export_midi_with_band, export_midi_with_options,
    export_midi_with_passes,
};
pub use render::{AudioBuffer, RenderError, RenderRequest, render};
pub use score::{ChordEvent, Key, Meter, Mode, NoteEvent, Score};
