//! Persisted application preferences.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::alias::AliasTable;
use crate::dance::SourceRef;
use crate::roles::RoleTerms;
use crate::time::Tempo;
use crate::timing::LeadSettings;

/// Factor from the displayed speech rate to the TTS engine's rate.
///
/// The UI keeps the familiar 1.0 default while the engine speaks at 1.2, which
/// is the delivery the calls were calibrated against.
pub const TTS_RATE_SCALE: f32 = 1.2;

/// Slowest speech rate the preferences expose, in displayed units.
pub const MIN_TTS_RATE: f32 = 0.4;

/// Fastest speech rate the preferences expose, in displayed units.
///
/// The old ceiling was 1.5 engine units; on the displayed scale that is
/// 1.25, rounded to one decimal place.
pub const MAX_TTS_RATE: f32 = 1.3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MelodyInstrument {
    #[default]
    Fiddle,
    Accordion,
}

impl MelodyInstrument {
    pub const ALL: [Self; 2] = [Self::Fiddle, Self::Accordion];

    pub fn label(self) -> &'static str {
        match self {
            Self::Fiddle => "Fiddle",
            Self::Accordion => "Accordion",
        }
    }

    /// General MIDI program number (zero-based) used with a loaded soundfont.
    pub fn gm_program(self) -> u8 {
        match self {
            Self::Fiddle => 40,    // Violin
            Self::Accordion => 21, // Accordion
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccompanimentInstrument {
    #[default]
    Piano,
    Guitar,
    Accordion,
}

impl AccompanimentInstrument {
    pub const ALL: [Self; 3] = [Self::Piano, Self::Guitar, Self::Accordion];

    pub fn label(self) -> &'static str {
        match self {
            Self::Piano => "Piano",
            Self::Guitar => "Guitar",
            Self::Accordion => "Accordion",
        }
    }

    /// General MIDI program number (zero-based) used with a loaded soundfont.
    pub fn gm_program(self) -> u8 {
        match self {
            Self::Piano => 0,      // Acoustic Grand Piano
            Self::Guitar => 25,    // Acoustic Guitar (steel)
            Self::Accordion => 21, // Accordion
        }
    }
}

/// The second accompaniment voice, which turns the duo into a trio.
///
/// The melody and [`AccompanimentInstrument`] provide two voices. The second
/// accompaniment is a countermelody played by a melody instrument underneath
/// the tune, so the default session sounds like a small contra band rather than
/// a solo with a backing track. Set it to [`Self::Off`] for a duo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecondAccompaniment {
    /// An accordion countermelody under the tune (the default trio).
    #[default]
    Accordion,
    /// A second fiddle playing the countermelody.
    Fiddle,
    /// No second voice; play the melody and one accompaniment.
    Off,
}

impl SecondAccompaniment {
    /// Every choice, in display order.
    pub const ALL: [Self; 3] = [Self::Accordion, Self::Fiddle, Self::Off];

    pub fn label(self) -> &'static str {
        match self {
            Self::Accordion => "Accordion",
            Self::Fiddle => "Fiddle",
            Self::Off => "Off",
        }
    }

    /// The melody instrument that plays the countermelody, if it is enabled.
    pub fn instrument(self) -> Option<MelodyInstrument> {
        match self {
            Self::Accordion => Some(MelodyInstrument::Accordion),
            Self::Fiddle => Some(MelodyInstrument::Fiddle),
            Self::Off => None,
        }
    }

    /// General MIDI program for the countermelody voice, if it is enabled.
    pub fn gm_program(self) -> Option<u8> {
        self.instrument().map(MelodyInstrument::gm_program)
    }
}

/// Optional percussion layer under the band.
///
/// Percussion is off by default. When it is enabled, a soft foot tap on every
/// dance beat is the default; a simple drum kit is the alternative. Either way
/// it sits on the same dance-beat grid as the music and adds no timing. The
/// separate [`PercussionRhythm`] decides the pattern inside that grid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Percussion {
    /// No percussion (default).
    #[default]
    Off,
    /// A soft foot tap on each dance beat.
    FootTaps,
    /// A basic kit: kick, snare, and hi-hat.
    Drumset,
}

impl Percussion {
    /// Every choice, in display order.
    pub const ALL: [Self; 3] = [Self::Off, Self::FootTaps, Self::Drumset];

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off",
            Self::FootTaps => "Foot taps",
            Self::Drumset => "Drumset",
        }
    }

    /// Whether the layer plays at all.
    pub fn is_enabled(self) -> bool {
        !matches!(self, Self::Off)
    }
}

/// Rhythm used to place the optional percussion layer.
///
/// Like the accompaniment rhythms, these only decide when the percussion
/// sounds inside the 64-beat grid; they never move the grid, the tempo, or the
/// call timeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PercussionRhythm {
    /// Use the pattern that suits the selected percussion style.
    #[default]
    Auto,
    /// The style's basic pattern: foot taps on each beat, or a kick/snare/hat kit.
    Basic,
    /// One accent on each dance beat.
    Beat,
    /// Accents between the dance beats.
    Offbeat,
    /// One accent per bar, with a sparse kit.
    HalfTime,
    /// Two accents per dance beat.
    DoubleTime,
    /// A swung long-short feel.
    Shuffle,
    /// A one-and-a triplet gallop.
    Gallop,
    /// Alternating accents in steady eighth notes.
    Train,
}

impl PercussionRhythm {
    pub const ALL: [Self; 9] = [
        Self::Auto,
        Self::Basic,
        Self::Beat,
        Self::Offbeat,
        Self::HalfTime,
        Self::DoubleTime,
        Self::Shuffle,
        Self::Gallop,
        Self::Train,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Match style",
            Self::Basic => "Basic",
            Self::Beat => "On the beat",
            Self::Offbeat => "Offbeats",
            Self::HalfTime => "Half time",
            Self::DoubleTime => "Double time",
            Self::Shuffle => "Shuffle",
            Self::Gallop => "Gallop",
            Self::Train => "Train beat",
        }
    }
}

/// Rhythm pattern used to realise the accompaniment chords.
///
/// The pattern only decides when the chord and bass notes sound inside the
/// 64-beat grid; it never changes the grid, the tempo, or the call timeline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccompanimentRhythm {
    /// Use the pattern that suits the selected accompaniment instrument.
    #[default]
    Auto,
    /// One sustained chord and bass note per dance beat.
    Block,
    /// Alternate bass and chord on each half beat (`oom-pah`).
    BoomChuck,
    /// Hold the bass and stab the chord off the beat.
    Offbeat,
    /// Broken-chord pattern in steady eighth notes.
    Arpeggio,
    /// Swung boom-chuck with a long-short feel.
    Shuffle,
    /// Alternating down and up strums on every half beat.
    Strum,
    /// One chord per harmony, held as long as the changes allow.
    Sustained,
}

impl AccompanimentRhythm {
    pub const ALL: [Self; 8] = [
        Self::Auto,
        Self::Block,
        Self::BoomChuck,
        Self::Offbeat,
        Self::Arpeggio,
        Self::Shuffle,
        Self::Strum,
        Self::Sustained,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Match instrument",
            Self::Block => "Block chords",
            Self::BoomChuck => "Boom-chuck",
            Self::Offbeat => "Offbeat stabs",
            Self::Arpeggio => "Arpeggio",
            Self::Shuffle => "Shuffle",
            Self::Strum => "Strum",
            Self::Sustained => "Sustained",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub const ALL: [Self; 2] = [Self::Dark, Self::Light];

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
        }
    }
}

/// How the spoken call is delivered.
///
/// The Kokoro model has no emotion or style input, and punctuation is reduced
/// to pauses before synthesis. Tone therefore comes from two model-level cues:
/// a small pitch shift that keeps its own contribution to duration neutral, and
/// a clear-speech pass that holds stressed vowels and lifts consonant presence.
/// Call start times stay on the syllable estimate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallTone {
    /// A lower, projected register with lengthened stressed vowels and a
    /// presence lift for consonants.
    #[default]
    Commanding,
    /// The voice's recorded pitch and even emphasis.
    Natural,
}

impl CallTone {
    pub const ALL: [Self; 2] = [Self::Commanding, Self::Natural];

    pub fn label(self) -> &'static str {
        match self {
            Self::Commanding => "Commanding caller",
            Self::Natural => "Natural",
        }
    }

    pub fn detail(self) -> &'static str {
        match self {
            Self::Commanding => {
                "A lower, projected register with crisper consonants and fuller \
                 stressed vowels. The lead stays on the syllable estimate; the \
                 safety padding covers the clearer delivery."
            }
            Self::Natural => "The voice's recorded pitch and even emphasis.",
        }
    }

    /// Pitch multiplier for the neural voice. `1.0` leaves the voice alone.
    pub fn pitch(self) -> f32 {
        match self {
            Self::Commanding => 0.94,
            Self::Natural => 1.0,
        }
    }

    /// Whether this tone asks the neural voice to sharpen its diction.
    ///
    /// The enunciation pass lengthens the stressed vowel in each content word
    /// and adds a presence/compression stage after synthesis.
    pub fn enunciation(self) -> bool {
        matches!(self, Self::Commanding)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundFontSetting {
    /// Display name, usually the file stem.
    pub name: String,
    /// Native/Android path. Web builds may keep the file only in memory.
    #[serde(default)]
    pub path: Option<String>,
}

/// A set of saved tunes played in turn during a long session.
///
/// One *pass* is one 64-beat time through the dance. A real band usually keeps
/// a tune for several minutes and changes at a repeat, so the default keeps
/// each tune for eight passes (about four minutes at 240 BPM).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TuneRotation {
    pub enabled: bool,
    /// Passes per tune before moving to the next entry.
    pub passes_per_tune: u32,
    /// Saved tunes in play order, by source. Resolved against the saved-tune
    /// library at runtime; a stale entry is skipped.
    pub tunes: Vec<SourceRef>,
}

impl Default for TuneRotation {
    fn default() -> Self {
        Self {
            enabled: false,
            passes_per_tune: 8,
            tunes: Vec::new(),
        }
    }
}

impl TuneRotation {
    /// Shortest rotation interval offered by the UI.
    pub const MIN_PASSES: u32 = 1;
    /// Longest rotation interval offered by the UI.
    pub const MAX_PASSES: u32 = 16;

    /// Whether the set has enough resolvable tunes to rotate.
    ///
    /// Two or more entries are required. The current tune may be one of them
    /// or not; a single entry can never change what is playing.
    pub fn can_rotate(tune_count: usize) -> bool {
        tune_count >= 2
    }

    /// Repair a hand-edited or older value: clamp the interval and drop
    /// duplicate source references.
    pub fn validate(&mut self) {
        self.passes_per_tune = self
            .passes_per_tune
            .clamp(Self::MIN_PASSES, Self::MAX_PASSES);
        let mut seen = std::collections::BTreeSet::new();
        self.tunes
            .retain(|tune| seen.insert((tune.provider.clone(), tune.id.clone())));
    }

    /// Seconds each tune stays, at a given tempo.
    ///
    /// One pass is the 64-beat dance loop, so this is the interval the session
    /// uses for its "about N minutes" label.
    pub fn seconds_per_tune(&self, bpm: f64) -> f64 {
        let bpm = bpm.max(1.0);
        f64::from(self.passes_per_tune) * 64.0 * (120.0 / bpm)
    }

    /// Whether a source is already in the set.
    pub fn contains(&self, source: &SourceRef) -> bool {
        self.tunes
            .iter()
            .any(|tune| tune.provider == source.provider && tune.id == source.id)
    }

    /// Add a tune if it is not already in the set.
    pub fn add(&mut self, source: SourceRef) {
        if !self.contains(&source) {
            self.tunes.push(source);
        }
    }

    /// Remove a tune by its provider and source id.
    pub fn remove(&mut self, provider: &str, id: &str) {
        self.tunes
            .retain(|tune| tune.provider != provider || tune.id != id);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub schema_version: u32,
    pub role_terms: RoleTerms,
    pub aliases: AliasTable,
    pub melody: MelodyInstrument,
    /// The chordal accompaniment voice.
    pub accompaniment: AccompanimentInstrument,
    /// Pattern used for the accompaniment chords; see [`AccompanimentRhythm`].
    pub accompaniment_rhythm: AccompanimentRhythm,
    /// The countermelody voice that completes the default trio; see
    /// [`SecondAccompaniment`].
    pub second_accompaniment: SecondAccompaniment,
    /// Optional percussion layer; see [`Percussion`].
    pub percussion: Percussion,
    /// Rhythm used to place the percussion layer; see [`PercussionRhythm`].
    pub percussion_rhythm: PercussionRhythm,
    pub theme: Theme,
    /// Smooth transitions for tab changes, highlights, and screen entries.
    pub animations_enabled: bool,
    pub lead: LeadSettings,
    pub tempo_bpm: f64,
    pub tts_enabled: bool,
    /// Speaking rate as displayed in preferences. The engine receives
    /// [`Preferences::speech_rate`], which applies [`TTS_RATE_SCALE`].
    pub tts_rate: f32,
    #[serde(default = "default_tts_voice")]
    pub tts_voice: String,
    /// Delivery tone for spoken calls; see [`CallTone`].
    #[serde(default)]
    pub tts_tone: CallTone,
    /// Speak and show a randomly chosen alternative call alias for moves that
    /// have one. Off by default so calls stay predictable.
    pub shuffle_alternatives: bool,
    pub loop_enabled: bool,
    pub count_in_enabled: bool,
    /// Add small, deterministic variations to the accompaniment on later loop
    /// passes (extra emphasis notes; now and then one bar of melody alone).
    pub accompaniment_variations: bool,
    pub show_original: bool,
    pub melody_volume: f32,
    pub accompaniment_volume: f32,
    /// Level of the optional percussion layer, when it is enabled.
    pub percussion_volume: f32,
    pub master_volume: f32,
    pub soundfont: Option<SoundFontSetting>,
    pub web_proxy_url: Option<String>,
    pub web_proxy_token: Option<String>,
    pub last_dance: Option<SourceRef>,
    pub last_tune: Option<SourceRef>,
    /// Play several saved tunes in turn during a long session.
    pub tune_rotation: TuneRotation,
    /// Migration tags this file has already been through.
    ///
    /// Recorded by name rather than inferred from [`Self::schema_version`] so
    /// two branches that add a migration in the same version range both run
    /// after a merge. Missing in older files, which are seeded from the
    /// version; see [`Preferences::migrate_with`].
    #[serde(default)]
    pub applied_migrations: BTreeSet<String>,
}

/// The version written by this build.
///
/// Bump it when the shape of the file changes; gating a migration on it is not
/// required any more, because [`Preferences::migrate_with`] uses the tags in
/// [`Preferences::applied_migrations`].
pub const CURRENT_SCHEMA_VERSION: u32 = 9;

/// One upgrade step for a preference file written by an older build.
pub struct Migration {
    /// Stable identifier, stored in the file. Never rename or reuse one.
    pub tag: &'static str,
    /// The schema version that introduced the change, used only to seed tags
    /// for files written before tags existed.
    pub version: u32,
    /// Applies the change. It must leave values the user already changed
    /// alone.
    pub apply: fn(&mut Preferences),
}

/// Every migration this build knows, oldest first.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        tag: "lead-calibration-v2",
        version: 2,
        apply: migrate_v1_lead_calibration,
    },
    Migration {
        tag: "animations-v3",
        version: 3,
        apply: migrate_v2_animations,
    },
    Migration {
        tag: "accompaniment-variations-v4",
        version: 4,
        apply: migrate_v3_accompaniment_variations,
    },
    Migration {
        tag: "tts-voice-v5",
        version: 5,
        apply: migrate_v4_tts_voice,
    },
    Migration {
        tag: "accompaniment-rhythm-v6",
        version: 6,
        apply: migrate_v5_accompaniment_rhythm,
    },
    Migration {
        tag: "second-accompaniment-v7",
        version: 7,
        apply: migrate_v6_second_accompaniment,
    },
    Migration {
        tag: "tune-rotation-v7",
        version: 7,
        apply: migrate_v6_tune_rotation,
    },
    Migration {
        tag: "tts-tone-v7",
        version: 7,
        apply: migrate_v6_tts_tone,
    },
    Migration {
        tag: "percussion-v8",
        version: 8,
        apply: migrate_v7_percussion,
    },
    Migration {
        tag: "percussion-rhythm-v9",
        version: 9,
        apply: migrate_v8_percussion_rhythm,
    },
    Migration {
        tag: "tts-voice-nicole-v9",
        version: 9,
        apply: migrate_v8_tts_voice_nicole,
    },
];

fn migrate_v1_lead_calibration(prefs: &mut Preferences) {
    // v1 calibrated at 6.5 syllables/s, a 2-beat maximum, and small word and
    // pause allowances. The measured delivery is 4.0 syllables/s with no
    // additional gaps, and a 5-beat maximum.
    if (prefs.lead.syllables_per_second - 6.5).abs() < 1e-6 {
        prefs.lead.syllables_per_second = 4.0;
    }
    if (prefs.lead.max_beats - 2.0).abs() < 1e-6 {
        prefs.lead.max_beats = 5.0;
    }
    if (prefs.lead.word_gap_seconds - 0.04).abs() < 1e-6 {
        prefs.lead.word_gap_seconds = 0.0;
    }
    if (prefs.lead.extra_pause_seconds - 0.10).abs() < 1e-6 {
        prefs.lead.extra_pause_seconds = 0.0;
    }
}

fn migrate_v2_animations(_prefs: &mut Preferences) {
    // Animations are a new field; `serde` supplies the enabled default for
    // older files, so there is nothing to change.
}

fn migrate_v3_accompaniment_variations(_prefs: &mut Preferences) {
    // Idem for the accompaniment variations.
}

fn migrate_v4_tts_voice(prefs: &mut Preferences) {
    // v5 adds the selectable TTS voice and alternative-alias shuffling.
    // Existing files keep the previous default voice and start with shuffling
    // disabled.
    if prefs.tts_voice.trim().is_empty() {
        prefs.tts_voice = default_tts_voice();
    }
}

fn migrate_v5_accompaniment_rhythm(_prefs: &mut Preferences) {
    // v6 adds the selectable accompaniment rhythm and the adjustable safety
    // padding; `serde` supplies the defaults for older files.
}

fn migrate_v6_second_accompaniment(_prefs: &mut Preferences) {
    // v7 adds the second accompaniment. Older files have no value, so `serde`
    // supplies the default (Accordion), which gives every session the trio
    // sound. A file that stores `Off` keeps it.
}

fn migrate_v6_tune_rotation(_prefs: &mut Preferences) {
    // v7 adds the optional tune rotation; `serde` supplies the disabled
    // default for older files.
}

fn migrate_v6_tts_tone(_prefs: &mut Preferences) {
    // v7 adds the caller tone. `serde` supplies the Commanding default for
    // older files, which is the requested change, so there is nothing else to
    // adjust.
}

fn migrate_v8_tts_voice_nicole(prefs: &mut Preferences) {
    // v9 retires Nicole, whose soft delivery was too quiet to call a dance.
    // Sarah is the replacement US female voice, so a stored Nicole preference
    // moves there instead of falling back to the default. Every other voice is
    // left alone, and the app still treats an unknown ID as the default.
    if prefs.tts_voice == "af_nicole" {
        prefs.tts_voice = "af_sarah".to_string();
    }
}

fn migrate_v7_percussion(_prefs: &mut Preferences) {
    // v8 adds the optional percussion layer. `serde` supplies the disabled
    // default, so existing sessions keep their current sound until the user
    // turns percussion on.
}

fn migrate_v8_percussion_rhythm(_prefs: &mut Preferences) {
    // v9 adds the percussion rhythm. `serde` supplies Auto, which keeps the
    // style's existing pattern.
}

/// Every tag in [`MIGRATIONS`], for a fresh file that needs none of them.
fn applied_tags() -> BTreeSet<String> {
    MIGRATIONS
        .iter()
        .map(|migration| migration.tag.to_owned())
        .collect()
}

/// What subsystems must react when preferences change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChangeSet {
    /// Calls must be rebuilt: role terms, aliases, lead settings, tempo,
    /// count-in, or alias shuffling changed.
    pub timeline: bool,
    /// Music must be re-rendered: arrangement, mix, or soundfont changed.
    pub audio: bool,
    /// Spoken calls must be synthesized again: the delivery tone changed.
    pub tone: bool,
}

impl ChangeSet {
    /// Whether anything has to be redone.
    pub fn any(self) -> bool {
        self.timeline || self.audio || self.tone
    }
}

fn default_tts_voice() -> String {
    "af_heart".to_string()
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            role_terms: RoleTerms::default(),
            aliases: AliasTable::default(),
            melody: MelodyInstrument::default(),
            accompaniment: AccompanimentInstrument::default(),
            accompaniment_rhythm: AccompanimentRhythm::default(),
            second_accompaniment: SecondAccompaniment::default(),
            percussion: Percussion::default(),
            percussion_rhythm: PercussionRhythm::default(),
            theme: Theme::default(),
            animations_enabled: true,
            lead: LeadSettings::default(),
            tempo_bpm: Tempo::DEFAULT_BPM,
            tts_enabled: false,
            tts_rate: 1.0,
            tts_voice: default_tts_voice(),
            tts_tone: CallTone::default(),
            shuffle_alternatives: false,
            loop_enabled: true,
            count_in_enabled: true,
            accompaniment_variations: true,
            show_original: false,
            melody_volume: 0.75,
            accompaniment_volume: 0.65,
            percussion_volume: 0.5,
            master_volume: 0.8,
            soundfont: None,
            web_proxy_url: None,
            web_proxy_token: None,
            last_dance: None,
            last_tune: None,
            tune_rotation: TuneRotation::default(),
            applied_migrations: applied_tags(),
        }
    }
}

impl Preferences {
    pub fn tempo(&self) -> Tempo {
        Tempo::new(self.tempo_bpm)
    }

    /// The rate handed to the speech engine.
    pub fn speech_rate(&self) -> f32 {
        self.tts_rate * TTS_RATE_SCALE
    }

    /// Clamp values that came from a preference file or a UI control.
    pub fn validate(&mut self) {
        if self.schema_version == 0 {
            self.schema_version = CURRENT_SCHEMA_VERSION;
        }
        if self.tts_voice.trim().is_empty() {
            self.tts_voice = default_tts_voice();
        }
        self.lead.validate();
        self.tempo_bpm = Tempo::new(self.tempo_bpm).bpm();
        self.tune_rotation.validate();
        self.tts_rate = self.tts_rate.clamp(MIN_TTS_RATE, MAX_TTS_RATE);
        self.melody_volume = self.melody_volume.clamp(0.0, 2.0);
        self.accompaniment_volume = self.accompaniment_volume.clamp(0.0, 2.0);
        self.percussion_volume = self.percussion_volume.clamp(0.0, 2.0);
        self.master_volume = self.master_volume.clamp(0.0, 2.0);
        if self.web_proxy_url.as_deref().is_some_and(str::is_empty) {
            self.web_proxy_url = None;
        }
        if self.web_proxy_token.as_deref().is_some_and(str::is_empty) {
            self.web_proxy_token = None;
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut prefs: Self = serde_json::from_str(json)?;
        prefs.migrate();
        prefs.validate();
        Ok(prefs)
    }

    /// Upgrade stored preferences without overwriting values the user changed
    /// away from the previous defaults.
    fn migrate(&mut self) {
        self.migrate_with(MIGRATIONS);
    }

    /// Run every migration in `migrations` that this file has not recorded.
    ///
    /// Gating on tags rather than on a shared version number is what makes
    /// this merge-safe: when two branches add a setting, each adds a migration
    /// with its own tag, and after a merge both run. With the old
    /// `schema_version < n` chain the first block advanced the version past
    /// the second, so one migration silently never ran.
    fn migrate_with(&mut self, migrations: &[Migration]) {
        if self.applied_migrations.is_empty() {
            // A file written before tags existed records only a version;
            // treat everything at or below it as already applied.
            self.applied_migrations = migrations
                .iter()
                .filter(|migration| migration.version <= self.schema_version)
                .map(|migration| migration.tag.to_owned())
                .collect();
        }
        for migration in migrations {
            if self.applied_migrations.contains(migration.tag) {
                continue;
            }
            (migration.apply)(self);
            self.applied_migrations.insert(migration.tag.to_owned());
        }
        self.schema_version = self.schema_version.max(CURRENT_SCHEMA_VERSION);
    }

    /// What must be rebuilt after moving from `old` to `self`.
    ///
    /// A setting is wired to its subsystem here, next to the field it affects,
    /// so it cannot be saved without the right rebuild. The UI used to carry
    /// this list inline, where a forgotten entry silently did nothing until
    /// the next restart.
    pub fn changes_since(&self, old: &Self) -> ChangeSet {
        ChangeSet {
            timeline: self.role_terms != old.role_terms
                || self.aliases != old.aliases
                || self.lead != old.lead
                || self.tempo_bpm != old.tempo_bpm
                || self.count_in_enabled != old.count_in_enabled
                || self.shuffle_alternatives != old.shuffle_alternatives,
            audio: self.tempo_bpm != old.tempo_bpm
                || self.count_in_enabled != old.count_in_enabled
                || self.accompaniment_variations != old.accompaniment_variations
                || self.melody != old.melody
                || self.accompaniment != old.accompaniment
                || self.accompaniment_rhythm != old.accompaniment_rhythm
                || self.second_accompaniment != old.second_accompaniment
                || self.percussion != old.percussion
                || self.percussion_rhythm != old.percussion_rhythm
                || self.melody_volume != old.melody_volume
                || self.accompaniment_volume != old.accompaniment_volume
                || self.percussion_volume != old.percussion_volume
                || self.soundfont != old.soundfont,
            tone: self.tts_tone != old.tts_tone,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timing::LEAD_PADDING_SECONDS;

    #[test]
    fn defaults_match_product_spec() {
        let prefs = Preferences::default();
        assert_eq!(prefs.role_terms.left_plural, "larks");
        assert_eq!(prefs.role_terms.right_plural, "robins");
        assert_eq!(prefs.melody, MelodyInstrument::Fiddle);
        assert_eq!(prefs.accompaniment, AccompanimentInstrument::Piano);
        assert_eq!(prefs.accompaniment_rhythm, AccompanimentRhythm::Auto);
        assert_eq!(prefs.second_accompaniment, SecondAccompaniment::Accordion);
        assert!(
            prefs.second_accompaniment.instrument().is_some(),
            "the default session is a trio"
        );
        assert_eq!(prefs.percussion, Percussion::Off);
        assert!(!prefs.percussion.is_enabled());
        assert_eq!(prefs.theme, Theme::Dark);
        assert!(prefs.animations_enabled);
        assert_eq!(prefs.tempo_bpm, 240.0);
        assert!(prefs.loop_enabled);
        assert!(prefs.count_in_enabled);
        assert!(prefs.accompaniment_variations);
        assert!(!prefs.tts_enabled);
        assert_eq!(prefs.tts_rate, 1.0);
        assert!((prefs.speech_rate() - 1.2).abs() < 1e-6);
        assert_eq!(prefs.tts_voice, "af_heart");
        assert_eq!(prefs.tts_tone, CallTone::Commanding);
        assert_eq!(prefs.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(!prefs.shuffle_alternatives);
        assert!(!prefs.tune_rotation.enabled);
        assert_eq!(prefs.tune_rotation.passes_per_tune, 8);
    }

    #[test]
    fn clamps_bad_values() {
        let mut prefs = Preferences {
            tempo_bpm: 10_000.0,
            melody_volume: 99.0,
            ..Preferences::default()
        };
        prefs.lead.syllables_per_second = 0.0;
        prefs.validate();
        assert_eq!(prefs.tempo_bpm, Tempo::MAX_BPM);
        assert_eq!(prefs.melody_volume, 2.0);
        assert!(prefs.lead.syllables_per_second >= 0.5);
    }

    #[test]
    fn clamps_tts_rate_to_the_displayed_range() {
        let mut prefs = Preferences {
            tts_rate: 9.0,
            ..Preferences::default()
        };
        prefs.validate();
        assert_eq!(prefs.tts_rate, MAX_TTS_RATE);
        prefs.tts_rate = 0.0;
        prefs.validate();
        assert_eq!(prefs.tts_rate, MIN_TTS_RATE);
    }

    #[test]
    fn json_round_trip() {
        let mut prefs = Preferences {
            tts_enabled: true,
            tempo_bpm: 220.0,
            ..Preferences::default()
        };
        prefs
            .aliases
            .set_override(crate::alias::MoveKind::Swing, "swing it");
        prefs
            .aliases
            .add_alternative(crate::alias::MoveKind::Swing, "swing it around");
        prefs.shuffle_alternatives = true;
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(prefs, restored);
    }

    #[test]
    fn alternative_alias_shuffling_is_off_by_default_and_round_trips() {
        let prefs = Preferences::default();
        assert!(!prefs.shuffle_alternatives);

        let prefs = Preferences {
            shuffle_alternatives: true,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert!(restored.shuffle_alternatives);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let prefs = Preferences::from_json("{}").expect("deserialize");
        assert_eq!(prefs, Preferences::default());
    }

    #[test]
    fn migrates_v1_lead_defaults_to_measured_calibration() {
        let mut old = Preferences {
            schema_version: 1,
            applied_migrations: BTreeSet::new(),
            ..Preferences::default()
        };
        old.lead.syllables_per_second = 6.5;
        old.lead.max_beats = 2.0;
        old.lead.word_gap_seconds = 0.04;
        old.lead.extra_pause_seconds = 0.10;
        let json = old.to_json().expect("serialize");
        let migrated = Preferences::from_json(&json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.tts_voice, "af_heart");
        assert_eq!(migrated.lead.syllables_per_second, 4.0);
        assert_eq!(migrated.lead.max_beats, 5.0);
        assert_eq!(migrated.lead.word_gap_seconds, 0.0);
        assert_eq!(migrated.lead.extra_pause_seconds, 0.0);
    }

    #[test]
    fn v2_preferences_enable_animations_by_default() {
        let json = r#"{"schema_version":2,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(migrated.animations_enabled);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v3_preferences_enable_accompaniment_variations_by_default() {
        let json = r#"{"schema_version":3,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(migrated.accompaniment_variations);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v4_preferences_use_default_voice_and_disable_alias_shuffling() {
        let json = r#"{"schema_version":4,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.tts_voice, "af_heart");
        assert!(!migrated.shuffle_alternatives);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v5_preferences_use_the_new_defaults() {
        let json = r#"{"schema_version":5,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.accompaniment_rhythm, AccompanimentRhythm::Auto);
        assert_eq!(migrated.lead.padding_seconds, LEAD_PADDING_SECONDS);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v6_preferences_use_the_commanding_tone_by_default() {
        let json = r#"{"schema_version":6,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.tts_tone, CallTone::Commanding);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v7_preferences_move_nicole_to_sarah() {
        let json = r#"{"schema_version":7,"tts_voice":"af_nicole"}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.tts_voice, "af_sarah");
    }

    #[test]
    fn the_nicole_migration_leaves_other_voices_alone() {
        for voice in ["af_heart", "am_michael", "am_onyx"] {
            let json = format!(r#"{{"schema_version":7,"tts_voice":"{voice}"}}"#);
            let migrated = Preferences::from_json(&json).expect("deserialize");
            assert_eq!(migrated.tts_voice, voice);
        }
    }

    #[test]
    fn call_tone_round_trips() {
        let prefs = Preferences {
            tts_tone: CallTone::Natural,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.tts_tone, CallTone::Natural);
    }

    #[test]
    fn commanding_tone_lowers_the_neural_pitch() {
        assert!(CallTone::Commanding.pitch() < 1.0);
        assert_eq!(CallTone::Natural.pitch(), 1.0);
        assert!(CallTone::Commanding.enunciation());
        assert!(!CallTone::Natural.enunciation());
        assert_eq!(CallTone::default(), CallTone::Commanding);
    }

    #[test]
    fn accompaniment_rhythm_round_trips() {
        let prefs = Preferences {
            accompaniment_rhythm: AccompanimentRhythm::Shuffle,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.accompaniment_rhythm, AccompanimentRhythm::Shuffle);
    }

    #[test]
    fn second_accompaniment_can_be_turned_off_and_round_trips() {
        let prefs = Preferences {
            second_accompaniment: SecondAccompaniment::Off,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.second_accompaniment, SecondAccompaniment::Off);
        assert_eq!(restored.second_accompaniment.instrument(), None);
    }

    #[test]
    fn second_accompaniment_uses_a_melody_instrument() {
        assert_eq!(
            SecondAccompaniment::Accordion.instrument(),
            Some(MelodyInstrument::Accordion)
        );
        assert_eq!(
            SecondAccompaniment::Fiddle.instrument(),
            Some(MelodyInstrument::Fiddle)
        );
        assert_eq!(SecondAccompaniment::Off.gm_program(), None);
        assert_eq!(
            SecondAccompaniment::Accordion.gm_program(),
            Some(MelodyInstrument::Accordion.gm_program())
        );
    }

    #[test]
    fn percussion_is_off_by_default_and_round_trips() {
        let prefs = Preferences::default();
        assert_eq!(prefs.percussion, Percussion::Off);
        assert!(!prefs.percussion.is_enabled());
        assert_eq!(prefs.percussion_rhythm, PercussionRhythm::Auto);
        assert_eq!(prefs.percussion_volume, 0.5);

        for style in [Percussion::FootTaps, Percussion::Drumset] {
            for rhythm in PercussionRhythm::ALL {
                let prefs = Preferences {
                    percussion: style,
                    percussion_rhythm: rhythm,
                    percussion_volume: 0.8,
                    ..Preferences::default()
                };
                let json = prefs.to_json().expect("serialize");
                let restored = Preferences::from_json(&json).expect("deserialize");
                assert_eq!(restored.percussion, style);
                assert_eq!(restored.percussion_rhythm, rhythm);
                assert_eq!(restored.percussion_volume, 0.8);
                assert!(restored.percussion.is_enabled());
            }
        }
    }

    #[test]
    fn older_files_get_the_default_trio() {
        let json = r#"{"schema_version": 6}"#;
        let prefs = Preferences::from_json(json).expect("deserialize");
        assert_eq!(prefs.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(prefs.second_accompaniment, SecondAccompaniment::Accordion);
    }

    #[test]
    fn tune_rotation_is_off_by_default_and_round_trips() {
        let prefs = Preferences::default();
        assert!(!prefs.tune_rotation.enabled);
        assert_eq!(prefs.tune_rotation.passes_per_tune, 8);
        assert!(prefs.tune_rotation.tunes.is_empty());

        let mut prefs = Preferences {
            tune_rotation: TuneRotation {
                enabled: true,
                passes_per_tune: 4,
                tunes: vec![
                    SourceRef::new("abcsessiontunes", "1", "https://example.invalid/1"),
                    SourceRef::new("abcsessiontunes", "2", "https://example.invalid/2"),
                ],
            },
            ..Preferences::default()
        };
        prefs.validate();
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.tune_rotation, prefs.tune_rotation);
    }

    #[test]
    fn tune_rotation_adds_without_duplicating_and_clamps_the_interval() {
        let mut rotation = TuneRotation::default();
        let first = SourceRef::new("abcsessiontunes", "1", "https://example.invalid/1");
        rotation.add(first.clone());
        rotation.add(first.clone());
        assert_eq!(rotation.tunes.len(), 1);
        assert!(rotation.contains(&first));

        rotation.remove("abcsessiontunes", "1");
        assert!(rotation.tunes.is_empty());

        rotation.passes_per_tune = 0;
        rotation.validate();
        assert_eq!(rotation.passes_per_tune, TuneRotation::MIN_PASSES);
        rotation.passes_per_tune = 999;
        rotation.validate();
        assert_eq!(rotation.passes_per_tune, TuneRotation::MAX_PASSES);
    }

    #[test]
    fn tune_rotation_duration_follows_the_tempo() {
        let rotation = TuneRotation {
            passes_per_tune: 8,
            ..TuneRotation::default()
        };
        // Eight 32-second passes at 240 BPM.
        assert!((rotation.seconds_per_tune(240.0) - 256.0).abs() < 1e-9);
        assert!((rotation.seconds_per_tune(120.0) - 512.0).abs() < 1e-9);
    }

    #[test]
    fn custom_safety_padding_round_trips() {
        let mut prefs = Preferences::default();
        prefs.lead.padding_seconds = 0.8;
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(restored.lead.padding_seconds, 0.8);
    }

    #[test]
    fn accompaniment_variations_can_be_disabled_and_round_trip() {
        let prefs = Preferences {
            accompaniment_variations: false,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert!(!restored.accompaniment_variations);
    }

    #[test]
    fn animations_can_be_disabled_and_round_trip() {
        let prefs = Preferences {
            animations_enabled: false,
            ..Preferences::default()
        };
        let json = prefs.to_json().expect("serialize");
        let restored = Preferences::from_json(&json).expect("deserialize");
        assert!(!restored.animations_enabled);
    }

    #[test]
    fn migration_preserves_custom_lead_values() {
        let mut old = Preferences {
            schema_version: 1,
            applied_migrations: BTreeSet::new(),
            ..Preferences::default()
        };
        old.lead.min_beats = 2.0;
        old.lead.syllables_per_second = 7.25;
        let json = old.to_json().expect("serialize");
        let migrated = Preferences::from_json(&json).expect("deserialize");
        assert_eq!(migrated.lead.min_beats, 2.0);
        assert_eq!(migrated.lead.syllables_per_second, 7.25);
    }

    #[test]
    fn instrument_gm_programs_are_sensible() {
        assert_eq!(MelodyInstrument::Fiddle.gm_program(), 40);
        assert_eq!(AccompanimentInstrument::Piano.gm_program(), 0);
        assert_eq!(AccompanimentInstrument::Guitar.gm_program(), 25);
    }

    #[test]
    fn a_file_records_every_migration_it_has_seen() {
        let prefs = Preferences::from_json(r#"{"schema_version": 7}"#).expect("deserialize");
        assert_eq!(prefs.schema_version, CURRENT_SCHEMA_VERSION);
        for migration in MIGRATIONS {
            assert!(
                prefs.applied_migrations.contains(migration.tag),
                "{} should be recorded",
                migration.tag
            );
        }
    }

    /// Two branches that add a setting in the same release used to collide on
    /// `schema_version`: the first block advanced the version past the second,
    /// so one migration silently never ran.
    #[test]
    fn migrations_sharing_a_version_all_run() {
        let mut prefs = Preferences::default();
        let migrations = [
            Migration {
                tag: "branch-a",
                version: 7,
                apply: |prefs| prefs.tts_rate = 1.3,
            },
            Migration {
                tag: "branch-b",
                version: 7,
                apply: |prefs| prefs.master_volume = 0.25,
            },
        ];
        prefs.migrate_with(&migrations);
        assert_eq!(prefs.tts_rate, 1.3);
        assert_eq!(prefs.master_volume, 0.25);
        assert!(prefs.applied_migrations.contains("branch-a"));
        assert!(prefs.applied_migrations.contains("branch-b"));
    }

    #[test]
    fn a_recorded_migration_does_not_run_again() {
        let mut prefs = Preferences::default();
        prefs.applied_migrations.insert("branch-a".into());
        let migrations = [Migration {
            tag: "branch-a",
            version: 7,
            apply: |prefs| prefs.tts_rate = 0.5,
        }];
        prefs.migrate_with(&migrations);
        assert_eq!(prefs.tts_rate, 1.0);
    }

    #[test]
    fn a_file_without_tags_does_not_rerun_old_migrations() {
        // A v6 file still holds the v1 lead default only because the user
        // chose it; the version says the v2 migration is already done.
        let json = r#"{"schema_version": 7, "lead": {"syllables_per_second": 6.5}}"#;
        let prefs = Preferences::from_json(json).expect("deserialize");
        assert_eq!(prefs.lead.syllables_per_second, 6.5);
    }

    #[test]
    fn changes_since_routes_settings_to_their_subsystems() {
        let base = Preferences::default();
        assert_eq!(base.changes_since(&base), ChangeSet::default());

        let mut tempo = base.clone();
        tempo.tempo_bpm = 220.0;
        assert_eq!(
            tempo.changes_since(&base),
            ChangeSet {
                timeline: true,
                audio: true,
                tone: false
            }
        );

        let mut aliases = base.clone();
        aliases.aliases.set_custom("custom", "improper");
        assert_eq!(
            aliases.changes_since(&base),
            ChangeSet {
                timeline: true,
                audio: false,
                tone: false
            }
        );

        let mut melody = base.clone();
        melody.melody = MelodyInstrument::Accordion;
        assert_eq!(
            melody.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: true,
                tone: false
            }
        );

        let mut tone = base.clone();
        tone.tts_tone = CallTone::Natural;
        assert_eq!(
            tone.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: false,
                tone: true
            }
        );

        let mut second = base.clone();
        second.second_accompaniment = SecondAccompaniment::Off;
        assert_eq!(
            second.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: true,
                tone: false
            }
        );

        let mut percussion = base.clone();
        percussion.percussion = Percussion::FootTaps;
        assert_eq!(
            percussion.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: true,
                tone: false
            }
        );

        let mut rhythm_changed = base.clone();
        rhythm_changed.percussion_rhythm = PercussionRhythm::Shuffle;
        assert_eq!(
            rhythm_changed.changes_since(&base),
            ChangeSet {
                timeline: false,
                audio: true,
                tone: false
            }
        );

        // Volume and appearance changes are applied without a rebuild.
        let mut volume = base.clone();
        volume.master_volume = 0.1;
        assert!(!volume.changes_since(&base).any());

        let mut theme = base.clone();
        theme.theme = Theme::Light;
        assert!(!theme.changes_since(&base).any());
    }
}
