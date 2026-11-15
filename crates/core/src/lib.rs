#![forbid(unsafe_code)]

//! Pure domain logic for Contra Karaoke.
//!
//! This crate contains no UI, network, audio, or platform code.

pub mod alias;
pub mod dance;
pub mod prefs;
pub mod pronounce;
pub mod roles;
pub mod syllables;
pub mod time;
pub mod timing;
pub mod tune;

pub use alias::{Actor, ActorRef, AliasTable, ClassifiedMove, MoveKind, MoveParams, RenderedCall};
pub use dance::{Dance, Figure, SourceRef};
pub use prefs::{
    AccompanimentInstrument, AccompanimentRhythm, MelodyInstrument, Preferences, SoundFontSetting,
    Theme,
};
pub use roles::{RolePreset, RoleSide, RoleTerms};
pub use time::{Beats, Samples, Seconds, Tempo};
pub use timing::{
    CallEvent, LEAD_PADDING_SECONDS, LeadSettings, MAX_PADDING_SECONDS, MIN_PADDING_SECONDS,
    PhraseSpan, PracticeTimeline, ScheduledCall, TimelineConfig, TimelineError,
};
pub use tune::Tune;
