#![forbid(unsafe_code)]

//! Pure domain logic for Contra Karaoke.
//!
//! This crate contains no UI, network, audio, or platform code.

pub mod alias;
pub mod dance;
pub mod dancer;
pub mod effects;
pub mod glossary;
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
pub use dancer::dancer_sounds;
pub use effects::{AudioEffects, ChorusPreset, EchoPreset, ReverbPreset};
pub use library::{
    FolderError, FolderFilter, LIBRARY_SCHEMA_VERSION, Library, LibraryEntry, LibraryItem,
};
pub use prefs::{
    AccompanimentInstrument, AccompanimentRhythm, BandPart, BandSlot, BandVoice, CallTone,
    CardPadding, ChangeSet, Countermelody, DEFAULT_DANCE_SOURCE, DEFAULT_TTS_DELIVERIES,
    DancerSound, DancerSoundEvent, EndingOptions, HumanizeOptions, InstrumentCopies,
    MAX_HUMANIZE_AMOUNT, MAX_PID_KD, MAX_PID_KI, MAX_PID_KP, MAX_TTS_DELIVERIES, MAX_TTS_RATE,
    MIN_TTS_DELIVERIES, MIN_TTS_RATE, MelodyInstrument, OutputMode, Percussion, PercussionRhythm,
    Preferences, SoundFontSetting, TTS_RATE_SCALE, Theme, TuneRotation, TuneSetSource,
    VariationOptions, default_band,
};
pub use roles::{RolePreset, RoleSide, RoleTerms};
pub use time::{Beats, Samples, Seconds, Tempo};
pub use timing::{
    COUNT_IN_BEATS, CallEvent, LEAD_PADDING_SECONDS, LeadSettings, MAX_PADDING_SECONDS,
    MIN_PADDING_SECONDS, PhraseSpan, PracticeTimeline, ScheduledCall, TimelineConfig,
    TimelineError,
};
pub use tune::Tune;
