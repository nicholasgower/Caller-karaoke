//! Reversibility tests for the preference controls.
//!
//! Every control that changes a setting can be changed back. After the round
//! trip the app must look exactly as it did before: preferences, the selected
//! dance and tune, the call timeline, the screen and picker state, the saved
//! libraries, banners, render flags, transport state, rotation bookkeeping, and
//! the spoken-call record.
//!
//! Deliberate one-way actions are exempt and are not in this table: Play,
//! Pause, Restart, End tune, downloads, clearing the cache, loading a
//! SoundFont, and anything that edits the saved libraries are user actions, not
//! reversible controls.
//!
//! A widget that is hard to drive directly (a combo or a slider inside a
//! scrolling form) is changed through its preference field and then run through
//! the same [`CallerApp::reconcile_preferences`] pass the Preferences screen
//! uses. One kittest test clicks a real control twice to prove the path works
//! through a drawn frame as well.

use super::*;
use caller_core::{
    AccompanimentInstrument, AccompanimentRhythm, CallTone, CardPadding, Countermelody, EchoPreset,
};
use eframe::egui;

/// A snapshot of every piece of app state a reversed control must leave alone.
#[derive(Clone, Debug, PartialEq)]
struct AppStateFingerprint {
    prefs: Preferences,
    dance: Dance,
    tune: Tune,
    timeline: PracticeTimeline,
    screen: Screen,
    prefs_query: String,
    prefs_reveal: Option<&'static str>,
    picker: PickerState,
    dance_library: Library<Dance>,
    tune_library: Library<Tune>,
    status: String,
    error: Option<String>,
    length_warning: Option<String>,
    needs_render: bool,
    pending_audio: bool,
    started: bool,
    last_spoken: Option<f64>,
    last_beat: f64,
    first_call_pending: bool,
    audio_playing: bool,
    audio_looping: bool,
    audio_volume: f32,
    audio_position: u64,
    rotation_passes: u32,
    rotation_last_beat: f64,
    rotation_prepared: Option<String>,
    rotation_failed: Option<String>,
    rotation_loading: u64,
    history_undo: usize,
    history_redo: usize,
    history_status: Option<String>,
    speech_enabled: bool,
    speech_rate_tone: (f32, CallTone),
    speech_deliveries: u8,
    speech_calls: usize,
    speech_status: String,
    soundfont_error: Option<String>,
    soundfont_loaded: bool,
    bundled_soundfont_loaded: bool,
    arrangement_loaded: bool,
    tune_view: TuneView,
    tune_sheet_loaded: bool,
    alias_filter: String,
    alias_seed: u64,
    custom_key: String,
    custom_alias: String,
    alternative_inputs: std::collections::BTreeMap<String, String>,
    about_reveal: bool,
    cache_clear_confirm: bool,
    library_status: Option<String>,
    role_edit_active: bool,
    role_edit_pending: bool,
}

impl CallerApp {
    #[cfg(test)]
    fn fingerprint(&self) -> AppStateFingerprint {
        AppStateFingerprint {
            prefs: self.prefs.clone(),
            dance: self.dance.clone(),
            tune: self.tune.clone(),
            timeline: self.timeline.clone(),
            screen: self.screen,
            prefs_query: self.prefs_query.clone(),
            prefs_reveal: self.prefs_reveal,
            picker: self.picker.clone(),
            dance_library: self.dance_library.clone(),
            tune_library: self.tune_library.clone(),
            status: self.status.clone(),
            error: self.error.clone(),
            length_warning: self.length_warning.clone(),
            needs_render: self.needs_render,
            pending_audio: self.pending_audio,
            started: self.started,
            last_spoken: self.last_spoken,
            last_beat: self.last_beat,
            first_call_pending: self.first_call_pending,
            audio_playing: self.audio.is_playing(),
            audio_looping: self.audio.looping(),
            audio_volume: self.audio.volume(),
            audio_position: self.audio.position_samples(),
            rotation_passes: self.rotation.passes,
            rotation_last_beat: self.rotation.last_beat,
            rotation_prepared: self
                .rotation
                .prepared
                .as_ref()
                .map(|prepared| prepared.tune.source.id.clone()),
            rotation_failed: self.rotation.failed_source.clone(),
            rotation_loading: self.rotation.loading_generation,
            history_undo: self.history.undo_len(),
            history_redo: self.history.redo_len(),
            history_status: self.history_status.clone(),
            speech_enabled: self.speech.enabled_for_test(),
            speech_rate_tone: self.speech.rate_and_tone(),
            speech_deliveries: self.speech.deliveries_for_test(),
            speech_calls: self.speech.prepared_calls(),
            speech_status: self.speech.status().message.clone(),
            soundfont_error: self.soundfont_error.clone(),
            soundfont_loaded: self.soundfont.is_some(),
            bundled_soundfont_loaded: self.bundled_soundfont.is_some(),
            arrangement_loaded: self.arrangement.is_some(),
            tune_view: self.tune_view,
            tune_sheet_loaded: self.tune_sheet.is_some(),
            alias_filter: self.alias_filter.clone(),
            alias_seed: self.alias_seed,
            custom_key: self.custom_key.clone(),
            custom_alias: self.custom_alias.clone(),
            alternative_inputs: self.alternative_inputs.clone(),
            about_reveal: self.about_reveal,
            cache_clear_confirm: self.cache_clear_confirm,
            library_status: self.library_status.clone(),
            role_edit_active: self.role_edit_active,
            role_edit_pending: self.role_edit_pending,
        }
    }
}

/// One reversible preference control.
struct Control {
    name: &'static str,
    /// Flip, choose another option, or nudge the value.
    change: fn(&mut Preferences),
}

/// Every control named in the request, plus the settings that have no widget
/// yet but are already wired to the renderer.
const CONTROLS: &[Control] = &[
    Control {
        name: "checkbox: loop",
        change: |prefs| prefs.loop_enabled = !prefs.loop_enabled,
    },
    Control {
        name: "checkbox: count-in",
        change: |prefs| prefs.count_in_enabled = !prefs.count_in_enabled,
    },
    Control {
        name: "checkbox: speak calls",
        change: |prefs| prefs.tts_enabled = !prefs.tts_enabled,
    },
    Control {
        name: "checkbox: cycle tunes",
        change: |prefs| prefs.tune_rotation.enabled = !prefs.tune_rotation.enabled,
    },
    Control {
        name: "checkbox: variations",
        change: |prefs| prefs.accompaniment_variations = !prefs.accompaniment_variations,
    },
    Control {
        name: "checkbox: variation articulation",
        change: |prefs| {
            prefs.variation_options.articulation = !prefs.variation_options.articulation
        },
    },
    Control {
        name: "checkbox: variation double stops",
        change: |prefs| {
            prefs.variation_options.double_stops = !prefs.variation_options.double_stops
        },
    },
    Control {
        name: "checkbox: variation grace notes",
        change: |prefs| prefs.variation_options.grace_notes = !prefs.variation_options.grace_notes,
    },
    Control {
        name: "checkbox: variation octave lift",
        change: |prefs| prefs.variation_options.octave_lift = !prefs.variation_options.octave_lift,
    },
    Control {
        name: "checkbox: variation chord stabs",
        change: |prefs| prefs.variation_options.chord_stabs = !prefs.variation_options.chord_stabs,
    },
    Control {
        name: "checkbox: variation melody alone",
        change: |prefs| {
            prefs.variation_options.melody_alone_bar = !prefs.variation_options.melody_alone_bar
        },
    },
    Control {
        name: "checkbox: humanize the band",
        change: |prefs| prefs.humanize_timing = !prefs.humanize_timing,
    },
    Control {
        name: "checkbox: humanize tempo drift",
        change: |prefs| prefs.humanize_options.tempo_drift = !prefs.humanize_options.tempo_drift,
    },
    Control {
        name: "checkbox: humanize chord spread",
        change: |prefs| prefs.humanize_options.chord_spread = !prefs.humanize_options.chord_spread,
    },
    Control {
        name: "checkbox: humanize pitch drift",
        change: |prefs| prefs.humanize_options.pitch_drift = !prefs.humanize_options.pitch_drift,
    },
    Control {
        name: "checkbox: show original",
        change: |prefs| prefs.show_original = !prefs.show_original,
    },
    Control {
        name: "checkbox: animations",
        change: |prefs| prefs.animations_enabled = !prefs.animations_enabled,
    },
    Control {
        name: "checkbox: glossary",
        change: |prefs| prefs.glossary_enabled = !prefs.glossary_enabled,
    },
    Control {
        name: "checkbox: automatic lead",
        change: |prefs| prefs.lead.auto = !prefs.lead.auto,
    },
    Control {
        name: "checkbox: shuffle aliases",
        change: |prefs| prefs.shuffle_alternatives = !prefs.shuffle_alternatives,
    },
    Control {
        name: "checkbox: dancer sounds",
        change: |prefs| prefs.dancer_sounds = !prefs.dancer_sounds,
    },
    Control {
        name: "checkbox: ending gradual",
        change: |prefs| prefs.ending_options.gradual = !prefs.ending_options.gradual,
    },
    Control {
        name: "checkbox: ending down a fifth",
        change: |prefs| {
            prefs.ending_options.final_chord_down_fifth =
                !prefs.ending_options.final_chord_down_fifth
        },
    },
    Control {
        name: "combo: theme",
        change: |prefs| {
            prefs.theme = match prefs.theme {
                Theme::Dark => Theme::Light,
                Theme::Light => Theme::Dark,
            }
        },
    },
    Control {
        name: "choice: dance card padding",
        change: |prefs| {
            prefs.card_padding = match prefs.card_padding {
                CardPadding::Compact => CardPadding::Sparse,
                CardPadding::Sparse => CardPadding::Compact,
            }
        },
    },
    Control {
        name: "combo: melody",
        change: |prefs| {
            prefs.melody = match prefs.melody {
                MelodyInstrument::Fiddle => MelodyInstrument::Accordion,
                _ => MelodyInstrument::Fiddle,
            }
        },
    },
    Control {
        name: "combo: accompaniment",
        change: |prefs| {
            prefs.accompaniment = match prefs.accompaniment {
                AccompanimentInstrument::Piano => AccompanimentInstrument::Guitar,
                _ => AccompanimentInstrument::Piano,
            }
        },
    },
    Control {
        name: "combo: accompaniment rhythm",
        change: |prefs| {
            prefs.accompaniment_rhythm = match prefs.accompaniment_rhythm {
                AccompanimentRhythm::Auto => AccompanimentRhythm::BoomChuck,
                _ => AccompanimentRhythm::Auto,
            }
        },
    },
    Control {
        name: "combo: countermelody",
        change: |prefs| {
            prefs.second_accompaniment = match prefs.second_accompaniment {
                Countermelody::Accordion => Countermelody::Off,
                _ => Countermelody::Accordion,
            }
        },
    },
    Control {
        name: "combo: percussion",
        change: |prefs| {
            prefs.percussion = match prefs.percussion {
                Percussion::Off => Percussion::FootTaps,
                _ => Percussion::Off,
            }
        },
    },
    Control {
        name: "combo: percussion rhythm",
        change: |prefs| {
            prefs.percussion_rhythm = match prefs.percussion_rhythm {
                PercussionRhythm::Auto => PercussionRhythm::Gallop,
                _ => PercussionRhythm::Auto,
            }
        },
    },
    Control {
        name: "combo: output mode",
        change: |prefs| {
            prefs.output_mode = match prefs.output_mode {
                OutputMode::Stereo => OutputMode::Mono,
                _ => OutputMode::Stereo,
            }
        },
    },
    Control {
        name: "combo: reverb",
        change: |prefs| {
            prefs.effects.reverb = match prefs.effects.reverb {
                caller_core::ReverbPreset::MediumRoom => caller_core::ReverbPreset::Off,
                _ => caller_core::ReverbPreset::MediumRoom,
            }
        },
    },
    Control {
        name: "combo: echo",
        change: |prefs| {
            prefs.effects.echo = match prefs.effects.echo {
                EchoPreset::Subtle => EchoPreset::Off,
                _ => EchoPreset::Subtle,
            }
        },
    },
    Control {
        name: "combo: chorus",
        change: |prefs| {
            prefs.effects.chorus = match prefs.effects.chorus {
                caller_core::ChorusPreset::Off => caller_core::ChorusPreset::Subtle,
                _ => caller_core::ChorusPreset::Off,
            }
        },
    },
    Control {
        name: "number: tempo",
        change: |prefs| prefs.tempo_bpm += 5.0,
    },
    Control {
        name: "number: master volume",
        change: |prefs| prefs.master_volume = (prefs.master_volume - 0.1).max(0.0),
    },
    Control {
        name: "number: melody volume",
        change: |prefs| prefs.melody_volume = (prefs.melody_volume - 0.1).max(0.0),
    },
    Control {
        name: "number: accompaniment volume",
        change: |prefs| prefs.accompaniment_volume = (prefs.accompaniment_volume - 0.1).max(0.0),
    },
    Control {
        name: "number: percussion volume",
        change: |prefs| prefs.percussion_volume = (prefs.percussion_volume - 0.1).max(0.0),
    },
    Control {
        name: "number: caller volume",
        change: |prefs| prefs.caller_volume = (prefs.caller_volume - 0.1).max(0.0),
    },
    Control {
        name: "number: dancer sound volume",
        change: |prefs| prefs.dancer_sound_volume = (prefs.dancer_sound_volume - 0.1).max(0.0),
    },
    Control {
        name: "number: lead minimum",
        change: |prefs| prefs.lead.min_beats += 0.5,
    },
    Control {
        name: "number: lead maximum",
        change: |prefs| prefs.lead.max_beats -= 0.5,
    },
    Control {
        name: "number: lead speech rate",
        change: |prefs| prefs.lead.syllables_per_second += 0.5,
    },
    Control {
        name: "number: lead padding",
        change: |prefs| prefs.lead.padding_seconds += 0.1,
    },
    Control {
        name: "number: manual lead",
        change: |prefs| prefs.lead.manual_beats += 0.5,
    },
    Control {
        name: "number: tts rate",
        change: |prefs| prefs.tts_rate = (prefs.tts_rate + 0.1).min(caller_core::MAX_TTS_RATE),
    },
    Control {
        name: "number: tts deliveries",
        change: |prefs| {
            prefs.tts_deliveries = if prefs.tts_deliveries > caller_core::MIN_TTS_DELIVERIES {
                prefs.tts_deliveries - 1
            } else {
                caller_core::MAX_TTS_DELIVERIES
            }
        },
    },
    Control {
        name: "number: tune set interval",
        change: |prefs| prefs.tune_rotation.passes_per_tune += 1,
    },
    Control {
        name: "number: timing amount",
        change: |prefs| {
            prefs.humanize_options.amount = (prefs.humanize_options.amount - 0.1).max(0.0)
        },
    },
    Control {
        name: "number: tuning amount",
        change: |prefs| {
            prefs.humanize_options.pitch_amount =
                (prefs.humanize_options.pitch_amount - 0.1).max(0.0)
        },
    },
    Control {
        name: "number: humanize pid proportional",
        change: |prefs| prefs.humanize_options.pid_kp += 0.1,
    },
    Control {
        name: "number: humanize pid integral",
        change: |prefs| prefs.humanize_options.pid_ki += 0.01,
    },
    Control {
        name: "number: humanize pid derivative",
        change: |prefs| prefs.humanize_options.pid_kd += 0.01,
    },
];

/// A headless app in a stable, idle state.
fn test_app() -> (CallerApp, egui::Context) {
    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx.clone());
    let mut app = CallerApp::new_headless(&cc);
    // The headless clock has a buffer; the test does not need a render for the
    // preference path. A spoken call is set so a rebuild that forgets it is
    // visible.
    app.clear_render_pending();
    app.started = false;
    app.speech.prewarm(call_texts(&app.timeline));
    app.last_spoken = app.timeline.calls.first().map(|call| call.call_start_beat);
    app.status = "Ready".into();
    app.error = Some("Network unavailable".into());
    app.library_status = Some("Saved".into());
    app.history_status = Some("Undid the last change".into());
    (app, ctx)
}

/// Change a preference the way the Preferences screen does, then settle it.
fn change(app: &mut CallerApp, control: &Control) {
    let before = app.prefs.clone();
    (control.change)(&mut app.prefs);
    app.reconcile_preferences(before);
}

#[test]
fn every_reversible_control_restores_the_full_app_state() {
    for control in CONTROLS {
        let (mut app, _ctx) = test_app();
        let original = app.prefs.clone();
        let before = app.fingerprint();

        change(&mut app, control);
        let changed = app.prefs.clone();
        assert_ne!(
            app.fingerprint(),
            before,
            "{} did not change any state",
            control.name
        );

        app.prefs = original;
        app.reconcile_preferences(changed);

        assert_eq!(
            app.fingerprint(),
            before,
            "{} left residual state after being changed back",
            control.name
        );
    }
}

#[test]
fn a_no_op_preference_pass_writes_nothing_and_marks_nothing() {
    let (mut app, _ctx) = test_app();
    app.save_prefs();
    let writes = app.prefs_save_count;
    let before = app.fingerprint();

    let unchanged = app.prefs.clone();
    app.reconcile_preferences(unchanged);

    assert_eq!(app.fingerprint(), before);
    assert_eq!(
        app.prefs_save_count, writes,
        "an unchanged preference pass rewrote the file"
    );
}

#[test]
fn changing_a_preference_back_does_not_leave_the_render_pending() {
    let (mut app, _ctx) = test_app();
    let original = app.prefs.clone();
    let before = app.fingerprint();
    assert!(!before.needs_render);
    assert!(!before.pending_audio);

    // A playback setting, changed and changed back.
    let first = app.prefs.clone();
    app.prefs.melody = MelodyInstrument::Accordion;
    app.reconcile_preferences(first);
    assert!(app.needs_render);
    assert!(app.pending_audio);

    let changed = app.prefs.clone();
    app.prefs = original;
    app.reconcile_preferences(changed);
    assert_eq!(app.fingerprint(), before);
}

#[test]
fn a_timeline_edit_changed_back_keeps_the_call_already_spoken() {
    let (mut app, _ctx) = test_app();
    let spoken = app.last_spoken.expect("the test starts mid-call");
    let original = app.prefs.clone();
    let before = app.fingerprint();

    let first = app.prefs.clone();
    app.prefs.count_in_enabled = false;
    app.reconcile_preferences(first);
    assert_eq!(
        app.last_spoken, None,
        "a changed timeline must forget the old call"
    );

    let changed = app.prefs.clone();
    app.prefs = original;
    app.reconcile_preferences(changed);
    assert_eq!(
        app.last_spoken,
        Some(spoken),
        "changing the setting back restored the timeline and the call"
    );
    assert_eq!(app.fingerprint(), before);
}

#[test]
fn a_preference_edit_before_the_first_render_keeps_the_render_pending() {
    let ctx = egui::Context::default();
    let cc = eframe::CreationContext::_new_kittest(ctx.clone());
    let mut app = CallerApp::new_headless(&cc);
    assert!(
        app.needs_render,
        "a fresh app has not rendered the music yet"
    );

    let before = app.prefs.clone();
    app.prefs.theme = Theme::Light;
    app.reconcile_preferences(before);

    assert!(
        app.needs_render,
        "changing an unrelated setting must not claim the first render is done"
    );
}

#[test]
fn two_preference_edits_that_cancel_leave_no_undo_step() {
    let (mut app, ctx) = test_app();
    // `record_history` skips the headless screenshot renderer; this test drives
    // it explicitly as a visible window would.
    app.headless = false;
    let before = app.snapshot();
    app.prefs.loop_enabled = false;
    app.record_history(before, &ctx);
    assert_eq!(app.history.undo_len(), 1);

    let before = app.snapshot();
    app.prefs.loop_enabled = true;
    app.record_history(before, &ctx);
    assert_eq!(
        app.history.undo_len(),
        0,
        "off and back on is not an undo step through the rejected value"
    );
}

/// The real session control: two clicks on the Loop checkbox must restore the
/// preference, the transport flag, and everything else.
#[test]
fn clicking_loop_twice_restores_the_session() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1100.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = CallerApp::new_headless(cc);
            app.clear_render_pending();
            app.speech.prewarm(call_texts(&app.timeline));
            app.last_spoken = app.timeline.calls.first().map(|call| call.call_start_beat);
            app.status = "Ready".into();
            app
        });
    harness.run_steps(3);

    let before = harness.state().fingerprint();
    assert!(before.audio_looping);

    harness.get_by_label("Loop").click();
    harness.run_steps(3);
    assert!(!harness.state().prefs.loop_enabled);
    assert!(
        !harness.state().audio.looping(),
        "the transport followed the control"
    );

    harness.get_by_label("Loop").click();
    harness.run_steps(3);
    assert_eq!(
        harness.state().fingerprint(),
        before,
        "the second click did not restore the full session"
    );
}

/// A typed number belongs to the field until editing finishes.
///
/// Parsing and clamping every digit used to turn a partial "1" into the
/// minimum and let the tempo window swap its two bounds, rewriting the text
/// under the keyboard. The same churn restarts an Android IME's composition, so
/// the value box now commits on Enter or focus loss.
#[test]
fn typing_into_a_number_box_does_not_rewrite_it_mid_entry() {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .allow_missing_glyphs()
        .with_accessibility_check(false)
        .with_max_steps(10)
        .build_eframe(|cc| {
            let mut app = CallerApp::new_headless(cc);
            app.clear_render_pending();
            app.screen = Screen::Preferences;
            app.prefs_page = crate::app::ui::preferences::PreferencesPage::Category(
                crate::app::ui::preferences::PreferencesCategory::Playback,
            );
            app.status = "Ready".into();
            app
        });
    harness.run_steps(3);

    let minimum = harness.get_by_label("Minimum tempo");
    minimum.scroll_to_me();
    harness.run_steps(3);
    let minimum = harness.get_by_label("Minimum tempo");
    minimum.click();
    harness.run_steps(3);

    for (typed, expected) in [("1", "1"), ("5", "15"), ("0", "150")] {
        harness.event(egui::Event::Text(typed.to_string()));
        harness.run_steps(1);
        assert_eq!(
            harness.state().prefs.tempo_min_bpm,
            200.0,
            "a partial number must not be applied while the field is focused"
        );
        assert_eq!(
            harness.get_by_label("Minimum tempo").value().as_deref(),
            Some(expected),
            "the field must keep what was typed"
        );
    }

    harness.key_press(egui::Key::Enter);
    harness.run_steps(2);
    assert_eq!(
        harness.state().prefs.tempo_min_bpm,
        150.0,
        "Enter applies the typed number once"
    );
}
