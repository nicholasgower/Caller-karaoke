#![forbid(unsafe_code)]

//! Pure domain logic for Contra Karaoke.
//!
//! This crate contains no UI, network, audio, or platform code.

pub mod alias;
pub mod dance;
pub mod prefs;
pub mod roles;
pub mod syllables;
pub mod time;
pub mod timing;
pub mod tune;

pub use alias::{Actor, ActorRef, AliasTable, ClassifiedMove, MoveKind, MoveParams, RenderedCall};
pub use dance::{Dance, Figure, SourceRef};
pub use prefs::{AccompanimentInstrument, MelodyInstrument, Preferences, SoundFontSetting, Theme};
pub use roles::{RolePreset, RoleSide, RoleTerms};
pub use time::{Beats, Samples, Seconds, Tempo};
pub use timing::{
    CallEvent, LeadSettings, PhraseSpan, PracticeTimeline, TimelineConfig, TimelineError,
};
pub use tune::Tune;
