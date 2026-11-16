//! Regression tests for the bundled Fluid R3 GM subset.
//!
//! The app ships a trimmed FluidR3_GM (MIT) so the default playback uses
//! sampled instruments. `scripts/rebuild-soundfont.py` produces the file from
//! the keep-list in `scripts/soundfont-keep.txt`; these tests fail if it is
//! regenerated without the programs the preferences offer, or if the keep-list
//! and the instrument enums drift apart.
//!
//! `bundled_font_matches_the_reference` is ignored because it needs the full
//! 141 MB source bank. Run it after regenerating the subset:
//!
//! ```sh
//! CONTRA_REFERENCE_SF2=/path/to/FluidR3_GM.sf2 \
//!     cargo test -p contra-music --test bundled_soundfont -- --ignored
//! ```

use std::collections::BTreeSet;
use std::sync::Arc;

use contra_core::{
    AccompanimentInstrument, AccompanimentRhythm, COUNT_IN_BEATS, Countermelody, InstrumentCopies,
    MelodyInstrument,
};
use contra_music::soundfont::{load_soundfont, render_soundfont};
use contra_music::{Arrangement, RenderRequest, parse_abc};
use rustysynth::SoundFont;

const FONT: &[u8] = include_bytes!("../../../assets/defaults/fluid-r3-gm-subset.sf2");

/// The keep-list that `scripts/rebuild-soundfont.py` regenerates the font from.
const KEEP_LIST: &str = include_str!("../../../scripts/soundfont-keep.txt");

const TUNE: &str = "X:1\nT:Test\nM:4/4\nL:1/8\nK:D\n\"D\"A2 A2 A2 A2 | \"G\"B2 B2 B2 B2|\n";

/// Programs the preferences can select, sorted and deduplicated.
fn required_programs() -> BTreeSet<i32> {
    MelodyInstrument::ALL
        .iter()
        .map(|instrument| i32::from(instrument.gm_program()))
        .chain(
            AccompanimentInstrument::ALL
                .iter()
                .map(|instrument| i32::from(instrument.gm_program())),
        )
        .chain(
            Countermelody::ALL
                .iter()
                .filter_map(|instrument| instrument.gm_program())
                .map(i32::from),
        )
        .collect()
}

/// Numbers in `scripts/soundfont-keep.txt`, ignoring comments and blank lines.
fn keep_list_programs() -> BTreeSet<i32> {
    KEEP_LIST
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.parse::<i32>()
                .unwrap_or_else(|_| panic!("soundfont-keep.txt has a bad entry: {line:?}"))
        })
        .collect()
}

fn request() -> RenderRequest {
    let score = parse_abc(TUNE).expect("test tune parses");
    RenderRequest {
        arrangement: Arrangement::from_score(&score, 64.0),
        bpm: 240.0,
        melody: MelodyInstrument::Fiddle,
        accompaniment: AccompanimentInstrument::Piano,
        accompaniment_rhythm: AccompanimentRhythm::Auto,
        second_accompaniment: Countermelody::Accordion,
        copies: InstrumentCopies::default(),
        percussion: contra_core::Percussion::Off,
        percussion_rhythm: contra_core::PercussionRhythm::Auto,
        melody_volume: 0.75,
        accompaniment_volume: 0.65,
        percussion_volume: 0.5,
        effects: contra_core::AudioEffects::default(),
        count_in_beats: COUNT_IN_BEATS,
        sample_rate: 24_000,
        variation_passes: 1,
        ending: false,
    }
}

fn bundled() -> Arc<SoundFont> {
    load_soundfont(FONT).expect("the bundled subset loads")
}

#[test]
fn bundled_font_has_the_programs_the_preferences_offer() {
    let font = bundled();
    for program in required_programs() {
        let found = font
            .get_presets()
            .iter()
            .any(|preset| preset.get_bank_number() == 0 && preset.get_patch_number() == program);
        assert!(
            found,
            "bundled font is missing bank 0 program {program}; \
             regenerate it with `python scripts/rebuild-soundfont.py`"
        );
    }
}

/// The keep-list is the one place that says which programs the bundled font
/// carries. If the preferences gain an instrument without that list (or the
/// other way round), the font and the picker drift apart.
#[test]
fn keep_list_matches_the_instrument_enums() {
    assert_eq!(
        keep_list_programs(),
        required_programs(),
        "scripts/soundfont-keep.txt no longer matches the instrument enums"
    );
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
fn every_countermelody_choice_is_audible() {
    let font = bundled();
    for second in Countermelody::ALL {
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

/// A full section uses twelve of the fifteen non-drum General MIDI channels.
/// This is the renderer's test that the channel plan fits a real SoundFont.
#[test]
fn bundled_font_renders_a_section_of_every_copy() {
    let font = bundled();
    let mut request = request();
    request.copies = InstrumentCopies {
        melody: 4,
        accompaniment: 4,
        countermelody: 4,
        percussion: 3,
    };
    request.percussion = contra_core::Percussion::Drumset;
    let section = render_soundfont(&request, &font).expect("renders the full section");
    let peak = section
        .samples
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!(peak > 0.005, "the section rendered at peak {peak}");
    assert!(section.samples.iter().all(|sample| sample.is_finite()));
    assert!(section.samples.iter().all(|sample| sample.abs() <= 1.0));
    assert_eq!(
        section.loop_pass_samples,
        request_loop_pass_samples(&request)
    );
    assert_eq!(
        section.loop_end_sample - section.loop_start_sample,
        section.loop_pass_samples
    );

    // The copies are separate players, so the section is not the single-player
    // render at a higher level.
    let mut single = request.clone();
    single.copies = InstrumentCopies::default();
    let one = render_soundfont(&single, &font).expect("renders one player");
    assert_eq!(section.frames(), one.frames());
    let one_peak = one
        .samples
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!(
        peak > one_peak * 0.25,
        "a section of four players should stay audible next to one ({peak} vs {one_peak})"
    );
    let difference = section
        .samples
        .iter()
        .zip(&one.samples)
        .fold(0.0f32, |worst, (a, b)| worst.max((a - b).abs()));
    assert!(
        difference > 1e-3,
        "the copy plan must change the sampled render, not {difference}"
    );
}

fn request_loop_pass_samples(request: &RenderRequest) -> u64 {
    let beat_seconds = 120.0 / request.bpm;
    let samples_per_beat = beat_seconds * f64::from(request.sample_rate.max(16_000));
    (request.arrangement.loop_beats * samples_per_beat).round() as u64
}

#[test]
fn percussion_mixes_with_the_bundled_font() {
    let font = bundled();
    let mut request = request();
    request.percussion = contra_core::Percussion::Drumset;
    let buffer = contra_music::render(&request, Some(&font)).expect("renders trio with drums");
    let peak = buffer
        .samples
        .iter()
        .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    assert!(peak > 0.05, "drumset rendered at peak {peak}");
    assert!(buffer.samples.iter().all(|sample| sample.abs() <= 1.0));
}

#[test]
fn bundled_font_renders_the_loop() {
    let font = bundled();
    let request = request();
    let buffer = render_soundfont(&request, &font).expect("renders with the bundled font");

    let expected_frames = 34.0 * request.sample_rate as f64;
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
fn bundled_font_renders_the_ending() {
    let font = bundled();
    let mut request = request();
    request.ending = true;
    let buffer =
        render_soundfont(&request, &font).expect("renders the ending with the bundled font");

    let ending_start = buffer.ending_start_sample.expect("ending region");
    assert_eq!(ending_start, buffer.loop_end_sample);
    assert!(buffer.frames() > buffer.loop_end_sample as usize);
    assert!(buffer.in_ending(ending_start));
    assert!(
        buffer.samples[ending_start as usize * 2..]
            .iter()
            .any(|sample| sample.abs() > 0.001),
        "the ending region must contain audio"
    );
    assert!(buffer.samples.iter().all(|sample| sample.is_finite()));
    assert!(
        buffer.samples[buffer.samples.len() - 2..]
            .iter()
            .all(|sample| sample.abs() < 1e-6),
        "the final frame must reach silence"
    );
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
