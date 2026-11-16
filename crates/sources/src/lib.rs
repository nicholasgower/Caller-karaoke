#![forbid(unsafe_code)]

//! Providers and parsers for external dance and tune sources.
//!
//! The rest of the application knows only the traits in [`provider`]; a new
//! website can be supported by adding another module and registering it.

pub mod abcsessiontunes;
pub mod cache;
pub mod callersbox;
pub mod contradb;
pub mod provider;

pub use abcsessiontunes::ABC_SESSION_TUNES;
pub use cache::{Cache, CacheStore, FileCache, MemoryCache};
pub use callersbox::CALLERS_BOX;
pub use contradb::CONTRADB;
pub use provider::{
    DanceProvider, DanceQuery, DanceSearchPage, DanceSort, DanceSummary, FilterKind, FilterSpec,
    HttpMethod, HttpRequest, SourceError, TuneProvider, TuneQuery, TuneSearchPage, TuneSummary,
    dance_provider, dance_providers, tune_provider, tune_providers,
};
