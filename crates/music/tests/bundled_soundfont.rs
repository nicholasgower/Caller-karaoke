//! Regression tests for the bundled Fluid R3 GM subset.
//!
//! The app ships a trimmed FluidR3_GM (MIT) so the default playback uses
//! sampled instruments. `scripts/soundfont-subset.py` produces the file; these
//! tests fail if it is regenerated without the programs the preferences offer.
//!
//! `bundled_font_matches_the_reference` is ignored because it needs the full
//! 141 MB source bank. Run it after regenerating the subset:
//!
//! ```sh
//! CONTRA_REFERENCE_SF2=/path/to/FluidR3_GM.sf2 \
//!     cargo test -p contra-music --test bundled_soundfont -- --ignored
//! ```

use std::sync::Arc;

use contra_core::{
    AccompanimentInstrument, AccompanimentRhythm, MelodyInstrument, SecondAccompaniment,
};
use contra_music::soundfont::{load_soundfont, render_soundfont};
use contra_music::{Arrangement, RenderRequest, parse_abc};
use rustysynth::SoundFont;

const FONT: &[u8] = include_bytes!("../../../assets/defaults/fluid-r3-gm-subset.sf2");

/// Programs the preferences can select, as (bank, program).
const REQUIRED_PROGRAMS: [(i32, i32); 4] = [(0, 0), (0, 21), (0, 25), (0, 40)];

const TUNE: &str = "X:1\nT:Test\nM:4/4\nL:1/8\nK:D\n\"D\"A2 A2 A2 A2 | \"G\"B2 B2 B2 B2|\n";

fn request() -> RenderRequest {
    let score = parse_abc(TUNE).expect("test tune parses");
    RenderRequest {
        arrangement: Arrangement::from_score(&score, 64.0),
        bpm: 240.0,
        melody: MelodyInstrument::Fiddle,
        accompaniment: AccompanimentInstrument::Piano,
        accompaniment_rhythm: AccompanimentRhythm::Auto,
        second_accompaniment: SecondAccompaniment::Accordion,
        melody_volume: 0.75,
        accompaniment_volume: 0.65,
        count_in_beats: 3.0,
        sample_rate: 24_000,
        variation_passes: 1,
    }
}

fn bundled() -> Arc<SoundFont> {
    load_soundfont(FONT).expect("the bundled subset loads")
}

#[test]
fn bundled_font_has_the_programs_the_preferences_offer() {
    let font = bundled();
    for (bank, program) in REQUIRED_PROGRAMS {
        let found = font
            .get_presets()
            .iter()
            .any(|preset| preset.get_bank_number() == bank && preset.get_patch_number() == program);
        assert!(
            found,
            "bundled font is missing bank {bank} program {program}"
        );
    }
}

#[test]
fn bundled_font_is_small() {
    // The point of the subset is a download measured in megabytes, not the
    // 141 MB of the full bank. Raise this only with a deliberate decision.
    assert!(
        FONT.len() < 32 * 1024 * 1024,
        "bundled font grew to {} bytes",
        FONT.len()
    );
}

#[test]
fn every_second_accompaniment_choice_is_audible() {
    let font = bundled();
    for second in SecondAccompaniment::ALL {
        let mut request = request();
        request.second_accompaniment = second;
        let buffer =
            render_soundfont(&request, &font).unwrap_or_else(|error| panic!("{second:?}: {error}"));
        let peak = buffer
            .samples
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        assert!(peak > 0.05, "{second:?} rendered at peak {peak}");
    }
}

#[test]
fn bundled_font_renders_the_loop() {
    let font = bundled();
    let request = request();
    let buffer = render_soundfont(&request, &font).expect("renders with the bundled font");

    let expected_frames = 33.5 * request.sample_rate as f64;
    assert!((buffer.frames() as f64 - expected_frames).abs() < 2.0);
    assert!(buffer.count_in_samples > 0);
    assert!(buffer.samples.iter().all(|sample| sample.is_finite()));
    assert!(buffer.samples.iter().all(|sample| sample.abs() <= 1.0));

    // A silent or near-silent render means the subset lost its samples.
    let peak = buffer
        .samples
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!(peak > 0.05, "bundled font rendered at peak {peak}");
}

#[test]
fn every_melody_and_accompaniment_choice_is_audible() {
    let font = bundled();
    for melody in MelodyInstrument::ALL {
        for accompaniment in AccompanimentInstrument::ALL {
            let mut request = request();
            request.melody = melody;
            request.accompaniment = accompaniment;
            request.variation_passes = 1;
            let buffer = render_soundfont(&request, &font)
                .unwrap_or_else(|error| panic!("{melody:?}/{accompaniment:?}: {error}"));
            let peak = buffer
                .samples
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
            assert!(
                peak > 0.05,
                "{melody:?}/{accompaniment:?} rendered at peak {peak}"
            );
        }
    }
}

#[test]
#[ignore = "needs the 141 MB reference bank in CONTRA_REFERENCE_SF2"]
fn bundled_font_matches_the_reference() {
    let Ok(reference_path) = std::env::var("CONTRA_REFERENCE_SF2") else {
        panic!("set CONTRA_REFERENCE_SF2 to the full FluidR3_GM.sf2");
    };
    let reference_bytes = std::fs::read(reference_path).expect("reference font reads");
    let reference = load_soundfont(&reference_bytes).expect("reference font loads");
    let subset = bundled();

    for melody in MelodyInstrument::ALL {
        for accompaniment in AccompanimentInstrument::ALL {
            let mut request = request();
            request.melody = melody;
            request.accompaniment = accompaniment;
            let expected = render_soundfont(&request, &reference).expect("reference renders");
            let actual = render_soundfont(&request, &subset).expect("subset renders");

            assert_eq!(expected.frames(), actual.frames());
            let difference = expected
                .samples
                .iter()
                .zip(&actual.samples)
                .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
            assert!(
                difference < 1e-4,
                "{melody:?}/{accompaniment:?} differs from the reference by {difference}"
            );
        }
    }
}
