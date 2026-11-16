#![forbid(unsafe_code)]

//! Pure domain logic for Contra Karaoke.
//!
//! This crate contains no UI, network, audio, or platform code.

pub mod alias;
pub mod dance;
pub mod effects;
pub mod library;
pub mod prefs;
pub mod pronounce;
pub mod roles;
pub mod syllables;
pub mod time;
pub mod timing;
pub mod tune;

pub use alias::{Actor, ActorRef, AliasTable, ClassifiedMove, MoveKind, MoveParams, RenderedCall};
pub use dance::{Dance, Figure, SourceRef};
pub use effects::{AudioEffects, ChorusPreset, EchoPreset, ReverbPreset};
pub use library::{
    FolderError, FolderFilter, LIBRARY_SCHEMA_VERSION, Library, LibraryEntry, LibraryItem,
};
pub use prefs::{
    AccompanimentInstrument, AccompanimentRhythm, CallTone, ChangeSet, MAX_TTS_RATE, MIN_TTS_RATE,
    MelodyInstrument, Percussion, PercussionRhythm, Preferences, SecondAccompaniment,
    SoundFontSetting, TTS_RATE_SCALE, Theme, TuneRotation,
};
pub use roles::{RolePreset, RoleSide, RoleTerms};
pub use time::{Beats, Samples, Seconds, Tempo};
pub use timing::{
    COUNT_IN_BEATS, CallEvent, LEAD_PADDING_SECONDS, LeadSettings, MAX_PADDING_SECONDS,
    MIN_PADDING_SECONDS, PhraseSpan, PracticeTimeline, ScheduledCall, TimelineConfig,
    TimelineError,
};
pub use tune::Tune;
