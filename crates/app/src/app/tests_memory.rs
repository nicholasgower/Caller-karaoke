//! Memory-report tests.
//!
//! Build a representative prepared session, print the category table (run with
//! `cargo test -p contra-app memory_report -- --nocapture`), and check that the
//! big optional consumers disappear when they are switched off.

use std::collections::BTreeSet;

use super::*;
use contra_music::{Arrangement, AudioBuffer, parse_abc};
use eframe::egui;

/// A 64-beat pass at 240 BPM is 32 seconds. The ending adds about six seconds.
const SAMPLE_RATE: u32 = 48_000;
const SAMPLES_PER_BEAT: f64 = 24_000.0;
const LOOP_BEATS: f64 = 64.0;

/// An interleaved stereo buffer shaped like a real render: count-in, the
/// requested variation passes, and the once-only ending.
fn shaped_buffer(passes: u32) -> AudioBuffer {
    let count_in = (COUNT_IN_BEATS * SAMPLES_PER_BEAT).round() as u64;
    let pass = (LOOP_BEATS * SAMPLES_PER_BEAT).round() as u64;
    let ending = (6.0 * f64::from(SAMPLE_RATE)).round() as u64;
    let loop_samples = pass * u64::from(passes.max(1));
    let frames = count_in + loop_samples + ending;
    AudioBuffer {
        samples: vec![0.0; frames as usize * 2],
        sample_rate: SAMPLE_RATE,
        channels: 2,
        count_in_samples: count_in,
        loop_start_sample: count_in,
        loop_end_sample: count_in + loop_samples,
        loop_pass_samples: pass,
        samples_per_beat: SAMPLES_PER_BEAT,
        ending_from_beat: 62.0,
        ending_start_sample: Some(count_in + loop_samples),
    }
}

fn measured_app() -> (ContraApp, egui::Context) {
    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx.clone());
    let mut app = ContraApp::new_headless(&cc);
    app.clear_render_pending();
    // The real app prepares the calls as soon as it starts.
    app.speech.prewarm(call_texts(&app.timeline));
    let score = parse_abc(&app.tune.abc).expect("the bundled tune parses");
    app.arrangement = Some(Arrangement::from_score(&score, LOOP_BEATS));
    app.audio.set_buffer(shaped_buffer(4));
    // Run one real frame so the font atlas exists.
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 800.0),
            )),
            ..Default::default()
        },
        |ui| {
            ui.label("measure the fonts");
        },
    );
    output.textures_delta.clear();
    (app, ctx)
}

#[test]
fn memory_report_accounts_for_every_category() {
    let (app, ctx) = measured_app();
    let report = app.memory_report(Some(&ctx));
    println!("\n{}", report.table());

    let expected = [
        "bundled_soundfont_file",
        "soundfont_samples",
        "audio_buffer",
        "prepared_tune",
        "speech_audio",
        "speech_cache",
        "arrangement",
        "call_timeline",
        "saved_libraries",
        "preferences",
        "picker_state",
        "fonts",
        "sheet_music_texture",
        "undo_history",
        "source_cache",
        "neural_tts",
    ];
    let mut names = BTreeSet::new();
    for name in expected {
        let bytes = report
            .bytes_for(name)
            .unwrap_or_else(|| panic!("missing memory category {name}"));
        assert!(names.insert(name), "duplicate memory category {name}");
        assert!(
            bytes < 4 * 1024 * 1024 * 1024,
            "{name} is implausibly large"
        );
    }
    assert_eq!(
        report.categories().len(),
        expected.len(),
        "a category was added without a test"
    );
    let summed: u64 = report
        .categories()
        .iter()
        .map(|category| category.bytes)
        .sum();
    assert_eq!(report.total_bytes(), summed);

    // The bundled file and a four-pass audio buffer dominate a prepared
    // session; the table must show both.
    assert_eq!(
        report.bytes_for("bundled_soundfont_file"),
        Some(crate::defaults::default_soundfont_bytes())
    );
    assert!(
        report.bytes_for("audio_buffer").unwrap_or(0) > 40 * 1024 * 1024,
        "four passes at 48 kHz stereo should be tens of megabytes"
    );
    assert!(
        report.bytes_for("fonts").unwrap_or(0) > 0,
        "the font atlas is measured after a frame"
    );
    assert_eq!(report.bytes_for("neural_tts"), Some(0), "TTS is off here");

    let total = report.total_bytes();
    assert!(
        (60 * 1024 * 1024..1024 * 1024 * 1024).contains(&total),
        "total {total} bytes is outside the plausible range"
    );
}

#[test]
fn memory_report_shrinks_with_one_variation_pass() {
    let (app, ctx) = measured_app();
    let four = app.memory_report(Some(&ctx));
    let mut app = app;
    app.prefs.accompaniment_variations = false;
    app.audio.set_buffer(shaped_buffer(1));
    let one = app.memory_report(Some(&ctx));

    let four_bytes = four.bytes_for("audio_buffer").expect("four passes");
    let one_bytes = one.bytes_for("audio_buffer").expect("one pass");
    assert!(
        four_bytes > one_bytes * 3,
        "four passes {four_bytes} should be about four times one pass {one_bytes}"
    );
    assert!(
        one.total_bytes() < four.total_bytes(),
        "the report must shrink when the variation passes are turned off"
    );
}

#[test]
fn memory_report_shrinks_when_the_neural_voice_is_off() {
    let (mut app, ctx) = measured_app();
    app.speech.set_neural_bytes_for_test(115 * 1024 * 1024);
    app.prefs.tts_enabled = true;
    app.speech.set_enabled(true);
    let on = app.memory_report(Some(&ctx));
    assert!(
        on.bytes_for("neural_tts").unwrap_or(0) > 100 * 1024 * 1024,
        "the neural category should carry the model estimate"
    );

    app.prefs.tts_enabled = false;
    app.speech.set_enabled(false);
    let off = app.memory_report(Some(&ctx));
    assert_eq!(off.bytes_for("neural_tts"), Some(0));
    assert!(
        on.total_bytes().saturating_sub(off.total_bytes()) > 114 * 1024 * 1024,
        "turning speech off must release the model estimate and nothing else"
    );
}

#[test]
fn memory_report_counts_prepared_deliveries() {
    let (mut app, ctx) = measured_app();
    app.speech.set_speech_bytes_for_test(8 * 1024 * 1024);
    app.prefs.tts_enabled = true;
    app.speech.set_enabled(true);
    let on = app.memory_report(Some(&ctx));
    assert_eq!(
        on.bytes_for("speech_cache"),
        Some(8 * 1024 * 1024),
        "the prepared deliveries must be visible in the table"
    );

    app.speech.set_enabled(false);
    let off = app.memory_report(Some(&ctx));
    assert_eq!(off.bytes_for("speech_cache"), Some(0));
    assert_eq!(off.bytes_for("neural_tts"), Some(0));
}

#[test]
fn a_user_soundfont_releases_the_bundled_copy() {
    let (mut app, ctx) = measured_app();
    app.bundled_soundfont = None;
    assert_eq!(
        app.memory_report(Some(&ctx)).bytes_for("soundfont_samples"),
        Some(0)
    );

    app.bundled_soundfont = crate::defaults::default_soundfont();
    let one = app
        .memory_report(Some(&ctx))
        .bytes_for("soundfont_samples")
        .unwrap_or(0);
    assert!(
        one > 20 * 1024 * 1024,
        "the parsed bundled font is tens of megabytes"
    );

    // A user bank and the bundled subset are distinct allocations.
    let user = crate::defaults::default_soundfont().expect("the bundled font loads");
    app.soundfont = Some(user.clone());
    let both = app
        .memory_report(Some(&ctx))
        .bytes_for("soundfont_samples")
        .unwrap_or(0);
    assert!(
        both > one,
        "two parsed fonts should count twice: {both} <= {one}"
    );

    app.install_user_soundfont(user, "demo.sf2".into(), None);
    assert!(
        app.bundled_soundfont.is_none(),
        "a user bank must not retain the bundled samples as well"
    );
    let after = app
        .memory_report(Some(&ctx))
        .bytes_for("soundfont_samples")
        .unwrap_or(0);
    assert!(
        after < both,
        "the report still counts both fonts: {after} >= {both}"
    );
}
