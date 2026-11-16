//! Bundled defaults so the app works on first launch without the network.

use std::sync::Arc;

use contra_core::{Dance, SourceRef, Tune};
use rustysynth::SoundFont;

const DEFAULT_DANCE_JSON: &str = include_str!("../../../assets/defaults/another_last_dance.json");
const DEFAULT_ABC: &str = include_str!("../../../assets/defaults/sandy_boys.abc");

/// A subset of Fluid R3 GM (MIT, Frank Wen) covering the instruments the
/// preferences offer. Regenerate with `scripts/soundfont-subset.py`.
const DEFAULT_SOUNDFONT: &[u8] = include_bytes!("../../../assets/defaults/fluid-r3-gm-subset.sf2");

pub fn default_dance() -> Dance {
    serde_json::from_str(DEFAULT_DANCE_JSON).expect("bundled dance JSON is valid")
}

/// The sampled instruments used when the user has not loaded their own font.
///
/// Loading is deferred: the font costs ~15 MB of sample data and only the
/// rendering path needs it. `None` means the built-in procedural synth takes
/// over, which keeps the app playable if the asset is ever missing.
pub fn default_soundfont() -> Option<Arc<SoundFont>> {
    contra_music::soundfont::load_soundfont(DEFAULT_SOUNDFONT).ok()
}

pub fn default_tune() -> Tune {
    Tune {
        source: SourceRef::new(
            "abcsessiontunes",
            "75815",
            "https://abcsessiontunes.com/tunes/75815",
        ),
        title: "Sandy Boys".into(),
        composer: String::new(),
        key: "A mix".into(),
        meter: "4/4".into(),
        rhythm: "reel".into(),
        abc: DEFAULT_ABC.to_string(),
        credits: "ABC transcription: Josh Larios · chords from Sarah Comer".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_soundfont_covers_the_instrument_choices() {
        let font = default_soundfont().expect("the bundled soundfont loads");
        let mut programs: Vec<u8> = contra_core::MelodyInstrument::ALL
            .iter()
            .map(|instrument| instrument.gm_program())
            .chain(
                contra_core::AccompanimentInstrument::ALL
                    .iter()
                    .map(|instrument| instrument.gm_program()),
            )
            .chain(
                contra_core::Countermelody::ALL
                    .iter()
                    .filter_map(|instrument| instrument.gm_program()),
            )
            .collect();
        programs.sort_unstable();
        programs.dedup();
        for program in programs {
            assert!(
                font.get_presets().iter().any(|preset| {
                    preset.get_bank_number() == 0 && preset.get_patch_number() == program as i32
                }),
                "the bundled soundfont has no program {program}"
            );
        }
    }

    #[test]
    fn bundled_defaults_have_the_same_length() {
        let dance = default_dance();
        let tune = default_tune();
        let score = contra_music::parse_abc(&tune.abc).expect("bundled tune parses");
        let arrangement = contra_music::Arrangement::from_score(&score, 64.0);

        assert_eq!(dance.total_beats(), 64.0);
        assert_eq!(arrangement.body_beats, 64.0);
        assert!(
            arrangement.check_dance_length(dance.total_beats()).is_ok(),
            "the bundled dance and tune must share a 64-beat grid"
        );
    }
}
