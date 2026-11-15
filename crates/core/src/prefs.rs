//! Persisted application preferences.

use serde::{Deserialize, Serialize};

use crate::alias::AliasTable;
use crate::dance::SourceRef;
use crate::roles::RoleTerms;
use crate::time::Tempo;
use crate::timing::LeadSettings;

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
    /// Continuous chord strums on every half beat.
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundFontSetting {
    /// Display name, usually the file stem.
    pub name: String,
    /// Native/Android path. Web builds may keep the file only in memory.
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub schema_version: u32,
    pub role_terms: RoleTerms,
    pub aliases: AliasTable,
    pub melody: MelodyInstrument,
    pub accompaniment: AccompanimentInstrument,
    /// Pattern used for the accompaniment chords; see [`AccompanimentRhythm`].
    pub accompaniment_rhythm: AccompanimentRhythm,
    pub theme: Theme,
    /// Smooth transitions for tab changes, highlights, and screen entries.
    pub animations_enabled: bool,
    pub lead: LeadSettings,
    pub tempo_bpm: f64,
    pub tts_enabled: bool,
    pub tts_rate: f32,
    #[serde(default = "default_tts_voice")]
    pub tts_voice: String,
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
    pub master_volume: f32,
    pub soundfont: Option<SoundFontSetting>,
    pub web_proxy_url: Option<String>,
    pub web_proxy_token: Option<String>,
    pub last_dance: Option<SourceRef>,
    pub last_tune: Option<SourceRef>,
}

fn default_tts_voice() -> String {
    "af_heart".to_string()
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            schema_version: 6,
            role_terms: RoleTerms::default(),
            aliases: AliasTable::default(),
            melody: MelodyInstrument::default(),
            accompaniment: AccompanimentInstrument::default(),
            accompaniment_rhythm: AccompanimentRhythm::default(),
            theme: Theme::default(),
            animations_enabled: true,
            lead: LeadSettings::default(),
            tempo_bpm: Tempo::DEFAULT_BPM,
            tts_enabled: false,
            tts_rate: 1.0,
            tts_voice: default_tts_voice(),
            shuffle_alternatives: false,
            loop_enabled: true,
            count_in_enabled: true,
            accompaniment_variations: true,
            show_original: false,
            melody_volume: 0.75,
            accompaniment_volume: 0.65,
            master_volume: 0.8,
            soundfont: None,
            web_proxy_url: None,
            web_proxy_token: None,
            last_dance: None,
            last_tune: None,
        }
    }
}

impl Preferences {
    pub fn tempo(&self) -> Tempo {
        Tempo::new(self.tempo_bpm)
    }

    /// Clamp values that came from a preference file or a UI control.
    pub fn validate(&mut self) {
        if self.schema_version == 0 {
            self.schema_version = 6;
        }
        if self.tts_voice.trim().is_empty() {
            self.tts_voice = default_tts_voice();
        }
        self.lead.validate();
        self.tempo_bpm = Tempo::new(self.tempo_bpm).bpm();
        self.tts_rate = self.tts_rate.clamp(0.5, 2.0);
        self.melody_volume = self.melody_volume.clamp(0.0, 2.0);
        self.accompaniment_volume = self.accompaniment_volume.clamp(0.0, 2.0);
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
        if self.schema_version < 2 {
            // v1 calibrated at 6.5 syllables/s, a 2-beat maximum, and small
            // word/pause allowances. The measured delivery is 4.0 syllables/s
            // with no additional gaps, and a 5-beat maximum.
            if (self.lead.syllables_per_second - 6.5).abs() < 1e-6 {
                self.lead.syllables_per_second = 4.0;
            }
            if (self.lead.max_beats - 2.0).abs() < 1e-6 {
                self.lead.max_beats = 5.0;
            }
            if (self.lead.word_gap_seconds - 0.04).abs() < 1e-6 {
                self.lead.word_gap_seconds = 0.0;
            }
            if (self.lead.extra_pause_seconds - 0.10).abs() < 1e-6 {
                self.lead.extra_pause_seconds = 0.0;
            }
            self.schema_version = 2;
        }
        if self.schema_version < 3 {
            // Animations are a new v3 field; `serde` supplies the enabled
            // default for older files, so only the version needs recording.
            self.schema_version = 3;
        }
        if self.schema_version < 4 {
            // Accompaniment variations are a new v4 field. `serde` supplies
            // the enabled default for older files.
            self.schema_version = 4;
        }
        if self.schema_version < 5 {
            // v5 adds the selectable TTS voice and alternative-alias
            // shuffling. Existing files keep the previous default voice and
            // start with shuffling disabled.
            if self.tts_voice.trim().is_empty() {
                self.tts_voice = default_tts_voice();
            }
            self.schema_version = 5;
        }
        if self.schema_version < 6 {
            // v6 adds the selectable accompaniment rhythm. `serde` supplies
            // the instrument-matching default for older files, so only the
            // version needs recording.
            self.schema_version = 6;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_product_spec() {
        let prefs = Preferences::default();
        assert_eq!(prefs.role_terms.left_plural, "larks");
        assert_eq!(prefs.role_terms.right_plural, "robins");
        assert_eq!(prefs.melody, MelodyInstrument::Fiddle);
        assert_eq!(prefs.accompaniment, AccompanimentInstrument::Piano);
        assert_eq!(prefs.accompaniment_rhythm, AccompanimentRhythm::Auto);
        assert_eq!(prefs.theme, Theme::Dark);
        assert!(prefs.animations_enabled);
        assert_eq!(prefs.tempo_bpm, 240.0);
        assert!(prefs.loop_enabled);
        assert!(prefs.count_in_enabled);
        assert!(prefs.accompaniment_variations);
        assert!(!prefs.tts_enabled);
        assert_eq!(prefs.tts_voice, "af_heart");
        assert_eq!(prefs.schema_version, 6);
        assert!(!prefs.shuffle_alternatives);
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
            ..Preferences::default()
        };
        old.lead.syllables_per_second = 6.5;
        old.lead.max_beats = 2.0;
        old.lead.word_gap_seconds = 0.04;
        old.lead.extra_pause_seconds = 0.10;
        let json = old.to_json().expect("serialize");
        let migrated = Preferences::from_json(&json).expect("deserialize");

        assert_eq!(migrated.schema_version, 6);
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

        assert_eq!(migrated.schema_version, 6);
        assert!(migrated.animations_enabled);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v3_preferences_enable_accompaniment_variations_by_default() {
        let json = r#"{"schema_version":3,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, 6);
        assert!(migrated.accompaniment_variations);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v4_preferences_use_default_voice_and_disable_alias_shuffling() {
        let json = r#"{"schema_version":4,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, 6);
        assert_eq!(migrated.tts_voice, "af_heart");
        assert!(!migrated.shuffle_alternatives);
        assert_eq!(migrated.tempo_bpm, 220.0);
    }

    #[test]
    fn v5_preferences_use_the_instrument_matching_rhythm() {
        let json = r#"{"schema_version":5,"tempo_bpm":220.0}"#;
        let migrated = Preferences::from_json(json).expect("deserialize");

        assert_eq!(migrated.schema_version, 6);
        assert_eq!(migrated.accompaniment_rhythm, AccompanimentRhythm::Auto);
        assert_eq!(migrated.tempo_bpm, 220.0);
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
}
