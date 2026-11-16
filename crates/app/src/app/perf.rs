//! CPU profiler for every user-facing action.
//!
//! This is a development-only tool behind the `headless` feature. It drives
//! the real [`ContraApp`] through `egui_kittest`, just like the documentation
//! screenshot renderer, and times:
//!
//! * one frame on every screen at desktop and phone widths,
//! * the synchronous work behind every control a user can operate (play,
//!   select a dance, rebuild the call timeline, import, export, undo, ...),
//! * the pure functions those actions call (ABC parse, arrange, synth render,
//!   sheet engraving, SoundFont load, MIDI export).
//!
//! Wall-clock time is used as the CPU proxy: every measured path runs on the
//! caller's thread, and the UI has no long blocking I/O in these paths. The
//! report turns each median into a percentage of the 60 fps frame budget, so a
//! measurement can be read as CPU load rather than only as elapsed time.
//!
//! The profiler does not change behaviour and does not fix anything. It exists
//! to point at the functions that need attention; see `scripts/profile.ps1`
//! and `scripts/profile.sh` for the wrapper that builds release mode and runs
//! it.

use std::error::Error;
use std::time::Instant;

use contra_core::{Dance, FolderFilter, Library, SourceRef, Tune};
use contra_music::{Arrangement, export_midi_with_band, parse_abc, render};
use eframe::egui::{self, Vec2};
use egui_kittest::Harness;

use super::ui::AbcEditorState;
use super::ui::preferences::{PreferencesCategory, PreferencesPage};
use super::{ContraApp, Screen};

const DESKTOP: Vec2 = Vec2::new(1280.0, 800.0);
const PHONE: Vec2 = Vec2::new(390.0, 844.0);

/// One 60 fps frame. A screen frame above this cannot sustain 60 fps.
const FRAME_BUDGET_MS: f64 = 1000.0 / 60.0;
/// A discrete control press should finish well inside one or two frames.
/// Above this the app visibly freezes while the user waits.
const ACTION_BUDGET_MS: f64 = 100.0;
/// Startup is deliberately allowed a little more room than a control press:
/// it includes building the egui context and the default libraries.
const STARTUP_BUDGET_MS: f64 = 250.0;

/// Everything the report needs about one measurement.
struct Measurement {
    group: &'static str,
    action: &'static str,
    function: &'static str,
    budget_ms: f64,
    samples_ms: Vec<f64>,
    note: &'static str,
}

impl Measurement {
    fn median_ms(&self) -> f64 {
        median(&self.samples_ms)
    }

    fn max_ms(&self) -> f64 {
        self.samples_ms.iter().copied().fold(0.0, f64::max)
    }

    fn min_ms(&self) -> f64 {
        self.samples_ms
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min)
    }

    /// How much of one 60 fps frame this work consumes.
    fn frame_load_pct(&self) -> f64 {
        self.median_ms() / FRAME_BUDGET_MS * 100.0
    }

    fn over_budget(&self) -> bool {
        self.median_ms() > self.budget_ms
    }

    fn heavy(&self) -> bool {
        !self.over_budget() && self.median_ms() > self.budget_ms * 0.5
    }
}

struct Report {
    measurements: Vec<Measurement>,
    started: Instant,
}

impl Report {
    fn new() -> Self {
        Self {
            measurements: Vec::new(),
            started: Instant::now(),
        }
    }

    fn record(
        &mut self,
        group: &'static str,
        action: &'static str,
        function: &'static str,
        budget_ms: f64,
        samples_ms: Vec<f64>,
        note: &'static str,
    ) {
        debug_assert!(!samples_ms.is_empty(), "{action} produced no samples");
        self.measurements.push(Measurement {
            group,
            action,
            function,
            budget_ms,
            samples_ms,
            note,
        });
    }

    fn print(&self) {
        let elapsed = self.started.elapsed().as_secs_f64();
        println!("Contra Karaoke CPU profile");
        println!("==========================");
        println!(
            "Build: {} · {} actions measured in {:.1} s",
            build_label(),
            self.measurements.len(),
            elapsed
        );
        println!(
            "Frame budget: {FRAME_BUDGET_MS:.2} ms (60 fps) · action budget: {ACTION_BUDGET_MS:.0} ms · startup budget: {STARTUP_BUDGET_MS:.0} ms"
        );
        println!(
            "Time is wall-clock CPU time on the UI thread; load is the share of one 60 fps frame."
        );
        println!();

        let mut flagged: Vec<&Measurement> = self
            .measurements
            .iter()
            .filter(|m| m.over_budget())
            .collect();
        flagged.sort_by(|a, b| {
            b.median_ms()
                .partial_cmp(&a.median_ms())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        println!("FUNCTIONS OVER BUDGET");
        println!("---------------------");
        if flagged.is_empty() {
            println!("None. Every measured action finished inside its budget.");
        } else {
            for (index, measure) in flagged.iter().enumerate() {
                println!("{:2}. {}  [{}]", index + 1, measure.action, measure.group);
                println!("    function: {}", measure.function);
                println!(
                    "    median {:.2} ms · min {:.2} ms · max {:.2} ms · budget {:.2} ms · {:>6.1}% of a 60 fps frame",
                    measure.median_ms(),
                    measure.min_ms(),
                    measure.max_ms(),
                    measure.budget_ms,
                    measure.frame_load_pct()
                );
                if !measure.note.is_empty() {
                    println!("    note: {}", measure.note);
                }
                println!();
            }
        }

        let mut heavy: Vec<&Measurement> = self.measurements.iter().filter(|m| m.heavy()).collect();
        heavy.sort_by(|a, b| {
            b.median_ms()
                .partial_cmp(&a.median_ms())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if !heavy.is_empty() {
            println!("HEAVY BUT INSIDE BUDGET (over half the budget)");
            println!("-----------------------------------------------");
            for measure in heavy {
                println!(
                    "  {:>8.2} ms  {:>5.1}%  {} [{}] — {}",
                    measure.median_ms(),
                    measure.frame_load_pct(),
                    measure.action,
                    measure.group,
                    measure.function
                );
            }
            println!();
        }

        println!("ALL MEASUREMENTS (sorted by median)");
        println!("-----------------------------------");
        println!(
            "{:<18} {:<46} {:>10} {:>10} {:>8}  VERDICT",
            "GROUP", "ACTION", "MEDIAN", "MAX", "LOAD"
        );
        let mut all: Vec<&Measurement> = self.measurements.iter().collect();
        all.sort_by(|a, b| {
            b.median_ms()
                .partial_cmp(&a.median_ms())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for measure in all {
            let verdict = if measure.over_budget() {
                "OVER"
            } else if measure.heavy() {
                "HEAVY"
            } else {
                "ok"
            };
            println!(
                "{:<18} {:<46} {:>8.2}ms {:>8.2}ms {:>7.1}%  {}",
                truncate(measure.group, 18),
                truncate(measure.action, 46),
                measure.median_ms(),
                measure.max_ms(),
                measure.frame_load_pct(),
                verdict
            );
        }
    }
}

fn build_label() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

fn median(samples: &[f64]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn truncate(value: &str, width: usize) -> String {
    if value.chars().count() <= width {
        value.to_string()
    } else {
        let mut out: String = value.chars().take(width.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// Run `f` once per sample and collect milliseconds.
fn timed<F: FnMut()>(mut f: F, samples: usize) -> Vec<f64> {
    let mut times = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        f();
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times
}

/// Build the in-process app used by every measurement.
fn harness() -> Harness<'static, ContraApp> {
    Harness::builder()
        .with_size(DESKTOP)
        .with_pixels_per_point(1.0)
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .with_render_cursor(false)
        .with_accessibility_check(false)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| ContraApp::new_headless(cc))
}

/// Put the app in a quiet idle state before a frame benchmark.
fn idle(app: &mut ContraApp) {
    app.needs_render = false;
    app.pending_audio = false;
    app.render_loading = false;
    app.started = false;
    app.audio.stop();
    app.error = None;
    app.audio_error = None;
    app.length_warning = None;
}

/// Time `samples` frames after two warm-up frames.
fn frame_times(harness: &mut Harness<'_, ContraApp>, samples: usize) -> Vec<f64> {
    harness.run_steps(1);
    harness.run_steps(1);
    // Guard against a harness update that only runs `logic` and skips the UI:
    // a screen frame must actually build drawable shapes.
    assert!(
        !harness.output().shapes.is_empty(),
        "the profiled screen frame drew no shapes"
    );
    timed(|| harness.run_steps(1), samples)
}

/// A default saved-dance library with a few folders, for picker frames.
fn dance_library() -> Library<Dance> {
    let mut library = Library::default();
    let _ = library.create_folder("Warmups");
    let _ = library.create_folder("Encores");
    for (index, (id, title, folder)) in [
        ("2409", "Another Last Dance", None),
        ("2154", "Belle of the Ball", Some("Warmups")),
        ("2380", "Allison's Favorite", Some("Encores")),
    ]
    .into_iter()
    .enumerate()
    {
        let mut dance = crate::defaults::default_dance();
        dance.source = SourceRef::new("contradb", id, format!("https://contradb.com/dances/{id}"));
        dance.title = title.into();
        library.insert(dance, folder.map(str::to_string), index as u64);
    }
    library
}

/// A default saved-tune library with a few folders, for picker frames.
fn tune_library() -> Library<Tune> {
    let mut library = Library::default();
    let _ = library.create_folder("Reels");
    let _ = library.create_folder("Jigs");
    for (index, (id, title, folder)) in [
        ("75815", "Sandy Boys", None),
        ("75412", "Saint Anne's Reel", Some("Reels")),
        ("1279", "Banish Misfortune", Some("Jigs")),
    ]
    .into_iter()
    .enumerate()
    {
        let mut tune = crate::defaults::default_tune();
        tune.source = SourceRef::new(
            "abcsessiontunes",
            id,
            format!("https://thesession.org/tunes/{id}"),
        );
        tune.title = title.into();
        library.insert(tune, folder.map(str::to_string), index as u64);
    }
    library
}

/// Synthetic saved dances for scaling tests. Real libraries are usually small,
/// but a long-lived user can accumulate hundreds; a linear scan that is hidden
/// in a three-item test becomes visible here.
fn stress_dance_library(count: usize) -> Library<Dance> {
    let mut library = Library::default();
    for index in 0..count {
        let mut dance = crate::defaults::default_dance();
        dance.source = SourceRef::new(
            "contradb",
            index.to_string(),
            format!("https://contradb.com/dances/{index}"),
        );
        dance.title = format!("Stress Dance {index}");
        dance.choreographer = format!("Choreographer {}", index % 17);
        library.insert(dance, None, index as u64);
    }
    for folder in 0..20 {
        let _ = library.create_folder(&format!("Folder {folder}"));
    }
    library
}

fn stress_tune_library(count: usize) -> Library<Tune> {
    let mut library = Library::default();
    for index in 0..count {
        let mut tune = crate::defaults::default_tune();
        tune.source = SourceRef::new(
            "abcsessiontunes",
            index.to_string(),
            format!("https://thesession.org/tunes/{index}"),
        );
        tune.title = format!("Stress Tune {index}");
        library.insert(tune, None, index as u64);
    }
    for folder in 0..20 {
        let _ = library.create_folder(&format!("Folder {folder}"));
    }
    library
}

fn dance_search_results() -> contra_sources::DanceSearchPage {
    use contra_sources::{DanceSearchPage, DanceSummary};
    DanceSearchPage {
        total: 23,
        page: 1,
        page_size: 10,
        dances: (0..10)
            .map(|index| DanceSummary {
                id: (2400 + index).to_string(),
                title: format!("Profile Dance {index}"),
                choreographer: "Profile Choreographer".into(),
                formation: if index % 2 == 0 { "improper" } else { "proper" }.into(),
                hook: if index % 3 == 0 {
                    "A profile search result".into()
                } else {
                    String::new()
                },
                user: "Profile".into(),
            })
            .collect(),
    }
}

fn tune_search_results() -> contra_sources::TuneSearchPage {
    use contra_sources::{TuneSearchPage, TuneSummary};
    TuneSearchPage {
        total: 8,
        page: 1,
        page_size: 10,
        tunes: (0..10)
            .map(|index| TuneSummary {
                id: (75800 + index).to_string(),
                title: format!("Profile Tune {index}"),
                composer: String::new(),
                key: "A mix".into(),
                meter: "4/4".into(),
                rhythm: if index % 2 == 0 { "reel" } else { "jig" }.into(),
            })
            .collect(),
    }
}

/// Profile every screen frame at a desktop size.
fn profile_screen_frames(
    harness: &mut Harness<'_, ContraApp>,
    report: &mut Report,
    samples: usize,
) {
    // Static data shared by the picker screens.
    {
        let app = harness.state_mut();
        app.dance_library = dance_library();
        app.tune_library = tune_library();
        app.picker.dance_results = Some(dance_search_results());
        app.picker.tune_results = Some(tune_search_results());
        app.picker.dance_preview = Some(crate::defaults::default_dance());
        app.picker.tune_preview = Some(crate::defaults::default_tune());
        app.picker.dance_query.title = "last".into();
        app.picker.tune_query.search = "sandy".into();
        app.tune_sheet = None;
    }

    let screens: &[(&str, &str, Screen)] = &[
        (
            "Session · ready",
            "ContraApp::ui → draw_session → ContraApp::session_ui",
            Screen::Session,
        ),
        (
            "Select dance",
            "ContraApp::ui → draw_dance_library → ContraApp::dance_library_ui",
            Screen::DancePicker,
        ),
        (
            "Download dance",
            "ContraApp::ui → draw_dance_download → ContraApp::dance_download_ui",
            Screen::DanceDownload,
        ),
        (
            "Select tune",
            "ContraApp::ui → draw_tune_library → ContraApp::tune_library_ui",
            Screen::TunePicker,
        ),
        (
            "Download tune · sheet",
            "ContraApp::ui → draw_tune_download → ContraApp::tune_download_ui → SheetPreview::ui",
            Screen::TuneDownload,
        ),
        (
            "ABC editor",
            "ContraApp::ui → draw_abc_editor → ContraApp::abc_editor_ui",
            Screen::AbcEditor,
        ),
        (
            "Preferences · Calling",
            "ContraApp::ui → draw_preferences → ContraApp::preferences_ui → role_terms/speech",
            Screen::Preferences,
        ),
        (
            "Preferences · Band & Sound",
            "ContraApp::ui → draw_preferences → ContraApp::preferences_ui → instruments/soundfont/effects",
            Screen::Preferences,
        ),
        (
            "Preferences · Playback",
            "ContraApp::ui → draw_preferences → ContraApp::preferences_ui → playback",
            Screen::Preferences,
        ),
        (
            "Preferences · Appearance",
            "ContraApp::ui → draw_preferences → ContraApp::preferences_ui → appearance",
            Screen::Preferences,
        ),
        (
            "Preferences · Data & Storage",
            "ContraApp::ui → draw_preferences → ContraApp::preferences_ui → data_sources/web_proxy/maintenance",
            Screen::Preferences,
        ),
        (
            "Preferences · About",
            "ContraApp::ui → draw_preferences → ContraApp::preferences_ui → about",
            Screen::Preferences,
        ),
        (
            "Aliases",
            "ContraApp::ui → draw_aliases → ContraApp::aliases_ui",
            Screen::Aliases,
        ),
        (
            "Glossary",
            "ContraApp::ui → draw_glossary → ContraApp::glossary_ui",
            Screen::Glossary,
        ),
    ];

    harness.set_size(DESKTOP);
    for (action, function, screen) in screens {
        let prefs_page = match *action {
            "Preferences · Calling" => PreferencesPage::Category(PreferencesCategory::Calling),
            "Preferences · Band & Sound" => {
                PreferencesPage::Category(PreferencesCategory::BandAndSound)
            }
            "Preferences · Playback" => PreferencesPage::Category(PreferencesCategory::Playback),
            "Preferences · Appearance" => {
                PreferencesPage::Category(PreferencesCategory::Appearance)
            }
            "Preferences · Data & Storage" => {
                PreferencesPage::Category(PreferencesCategory::DataAndStorage)
            }
            "Preferences · About" => PreferencesPage::Category(PreferencesCategory::About),
            _ => PreferencesPage::Categories,
        };
        {
            let app = harness.state_mut();
            idle(app);
            app.screen = *screen;
            app.prefs_page = prefs_page;
            app.tune_sheet = None;
            app.abc_editor = None;
            if *screen == Screen::AbcEditor {
                app.abc_editor = Some(AbcEditorState::new(&app.tune));
            }
            if *screen == Screen::Aliases {
                app.alias_filter = "swing".into();
            }
            if *screen == Screen::Glossary {
                app.open_glossary(Some("Balance and swing"), Screen::Session);
            }
        }
        harness.run_steps(1);
        let times = frame_times(harness, samples);
        report.record(
            "frame",
            action,
            function,
            FRAME_BUDGET_MS,
            times,
            "one frame on an idle session; texture/atlas caches warm",
        );
    }

    // A playing session is the frame that matters most: the caller draws the
    // moving card, the meter, and the live transport state.
    {
        let app = harness.state_mut();
        idle(app);
        app.screen = Screen::Session;
        app.started = true;
        app.needs_render = false;
        app.audio.play();
        app.audio.seek_beat(4.0);
    }
    let times = frame_times(harness, samples);
    report.record(
        "frame",
        "Session · playing",
        "ContraApp::ui → ContraApp::tick → update_speech + draw_session",
        FRAME_BUDGET_MS,
        times,
        "playing at beat 4 with the prompt card and dance card live",
    );
}

/// Phone screens are a separate layout with scrolling and stacked panes.
fn profile_phone_frames(harness: &mut Harness<'_, ContraApp>, report: &mut Report, samples: usize) {
    harness.set_size(PHONE);
    let screens: &[(&str, &str, Screen)] = &[
        (
            "Session",
            "ContraApp::ui → draw_session → ContraApp::session_ui (compact)",
            Screen::Session,
        ),
        (
            "Select dance",
            "ContraApp::ui → draw_dance_library → ContraApp::dance_library_ui (compact)",
            Screen::DancePicker,
        ),
        (
            "Download dance",
            "ContraApp::ui → draw_dance_download → ContraApp::dance_download_ui (compact)",
            Screen::DanceDownload,
        ),
        (
            "Select tune",
            "ContraApp::ui → draw_tune_library → ContraApp::tune_library_ui (compact)",
            Screen::TunePicker,
        ),
        (
            "Download tune",
            "ContraApp::ui → draw_tune_download → ContraApp::tune_download_ui (compact)",
            Screen::TuneDownload,
        ),
        (
            "Preferences",
            "ContraApp::ui → draw_preferences → ContraApp::preferences_ui (compact)",
            Screen::Preferences,
        ),
        (
            "Aliases",
            "ContraApp::ui → draw_aliases → ContraApp::aliases_ui (compact)",
            Screen::Aliases,
        ),
        (
            "Glossary",
            "ContraApp::ui → draw_glossary → ContraApp::glossary_ui (compact)",
            Screen::Glossary,
        ),
    ];
    for (action, function, screen) in screens {
        {
            let app = harness.state_mut();
            idle(app);
            app.screen = *screen;
            app.prefs_page = PreferencesPage::Categories;
            app.tune_sheet = None;
            app.abc_editor = None;
            if *screen == Screen::Glossary {
                app.open_glossary(Some("Balance and swing"), Screen::Session);
            }
        }
        harness.run_steps(1);
        let times = frame_times(harness, samples);
        report.record(
            "frame · phone",
            action,
            function,
            FRAME_BUDGET_MS,
            times,
            "390×844 phone layout",
        );
    }
}

/// Synchronous actions behind the controls.
fn profile_actions(harness: &mut Harness<'_, ContraApp>, report: &mut Report, samples: usize) {
    // First Play: parse the ABC, arrange it, load the bundled SoundFont, and
    // synthesize the loop. This is the one action that does everything at once.
    {
        let app = harness.state_mut();
        idle(app);
        app.needs_render = true;
        app.bundled_soundfont = None;
        app.bundled_soundfont_checked = false;
        let start = Instant::now();
        app.play();
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        report.record(
            "session",
            "Play · first run",
            "ContraApp::play → ensure_ready → apply_audio_settings → render_music",
            ACTION_BUDGET_MS,
            vec![elapsed],
            "headless/no-prefetch fallback: ABC parse + SoundFont load + full synth render; the desktop prefetch path is measured below",
        );
    }

    {
        let app = harness.state_mut();
        app.pause();
        let times = timed(|| app.play(), samples);
        report.record(
            "session",
            "Play · resume",
            "ContraApp::play",
            ACTION_BUDGET_MS,
            times,
            "buffer already rendered",
        );
    }
    {
        // The first Play above filled the RAM cache. Asking for the same
        // render again is the path a second Play or an undo uses; it should be
        // an Arc clone, not a synthesis pass.
        let app = harness.state_mut();
        let times = timed(
            || {
                let _ = app.render_music();
            },
            samples,
        );
        report.record(
            "session",
            "Reuse cached render",
            "ContraApp::render_music (RAM cache hit)",
            ACTION_BUDGET_MS,
            times,
            "same tune, preferences, sample rate, and count-in as the last render",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(|| app.pause(), samples);
        report.record(
            "session",
            "Pause",
            "ContraApp::pause",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(|| app.restart(), samples);
        report.record(
            "session",
            "Restart",
            "ContraApp::restart",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(|| app.toggle_playback(), samples);
        report.record(
            "session",
            "Play/Pause toggle",
            "ContraApp::toggle_playback",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(
            || {
                app.end_tune();
            },
            samples,
        );
        report.record(
            "session",
            "End tune",
            "ContraApp::end_tune",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }

    // A tempo change is the most common live control: the UI rebuilds the
    // call timeline and queues a music re-render.
    {
        let app = harness.state_mut();
        app.audio.play();
        // Isolate the two halves of `apply_audio_settings`: the timeline
        // rebuild is cheap, the synth render is not. Both are measured below
        // with the same preferences the tempo change uses.
        let times = timed(
            || {
                app.rebuild_timeline();
            },
            samples,
        );
        report.record(
            "session",
            "Tempo change · timeline",
            "ContraApp::rebuild_timeline → build_timeline_with_count_in",
            ACTION_BUDGET_MS,
            times,
            "the timeline half of a live tempo change",
        );
        let times = timed(
            || {
                let _ = app.render_music();
            },
            samples,
        );
        report.record(
            "session",
            "Tempo change · render",
            "ContraApp::render_music → parse_abc + Arrangement::from_score + render",
            ACTION_BUDGET_MS,
            times,
            "the music half of a live tempo change; the old buffer keeps playing",
        );
        let mut high = false;
        let times = timed(
            || {
                high = !high;
                app.prefs.tempo_bpm = if high { 250.0 } else { 230.0 };
                app.apply_audio_settings();
            },
            samples,
        );
        report.record(
            "session",
            "Tempo change · apply",
            "ContraApp::apply_audio_settings → rebuild_timeline + render_music",
            ACTION_BUDGET_MS,
            times,
            "synchronous fallback (headless/web or a failed prefetch); the desktop UI defers below",
        );
    }

    // The desktop path: start the render on a worker, keep the old loop
    // playing, then install the cached buffer. Only the dispatch and the
    // install run on the UI thread.
    {
        let app = harness.state_mut();
        app.prefs.tempo_bpm = 235.0;
        app.pending_audio = true;
        let dispatch = timed(|| app.start_prefetch(), 1);
        report.record(
            "session",
            "Tempo change · start background render",
            "ContraApp::start_prefetch → prepare_tune (worker thread)",
            ACTION_BUDGET_MS,
            dispatch,
            "the UI only hands the key and settings to the worker",
        );
        loop {
            {
                let app = harness.state_mut();
                if app.prefetch.is_none() {
                    break;
                }
                app.poll_prefetch();
                if app.prefetch.is_none() {
                    break;
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let app = harness.state_mut();
        let times = timed(|| app.apply_audio_settings(), samples);
        report.record(
            "session",
            "Tempo change · install from cache",
            "ContraApp::apply_audio_settings → rebuild_timeline + render_music (cache hit)",
            ACTION_BUDGET_MS,
            times,
            "the new loop was synthesized while the old one kept playing",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(
            || {
                app.prefs.lead.min_beats = if app.prefs.lead.min_beats > 1.0 {
                    1.0
                } else {
                    1.5
                };
                app.rebuild_timeline();
            },
            samples,
        );
        report.record(
            "session",
            "Calling lead change",
            "ContraApp::rebuild_timeline → build_timeline_with_count_in",
            ACTION_BUDGET_MS,
            times,
            "every lead/alias/role edit rebuilds the call schedule",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(
            || {
                app.prefs.caller_volume = if app.prefs.caller_volume > 1.0 {
                    0.8
                } else {
                    1.2
                };
                app.audio.set_caller_volume(app.prefs.caller_volume);
                app.save_prefs();
            },
            samples,
        );
        report.record(
            "session",
            "Caller volume",
            "ContraApp::save_prefs + AudioEngine::set_caller_volume",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(
            || {
                app.prefs.master_volume = if app.prefs.master_volume > 1.0 {
                    0.8
                } else {
                    1.2
                };
                app.audio.set_volume(app.prefs.master_volume);
                app.save_prefs();
            },
            samples,
        );
        report.record(
            "session",
            "Band volume",
            "ContraApp::save_prefs + AudioEngine::set_volume",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let before = app.prefs.clone();
        app.prefs.count_in_enabled = !app.prefs.count_in_enabled;
        let times = timed(
            || {
                app.prefs.count_in_enabled = !app.prefs.count_in_enabled;
                app.reconcile_preferences(before.clone());
            },
            samples,
        );
        report.record(
            "session",
            "Count-in toggle",
            "ContraApp::reconcile_preferences → apply_count_in_change",
            ACTION_BUDGET_MS,
            times,
            "not started: timeline rebuild and render flag only",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(
            || {
                app.prefs.loop_enabled = !app.prefs.loop_enabled;
                app.audio.set_looping(app.prefs.loop_enabled);
                app.save_prefs();
            },
            samples,
        );
        report.record(
            "session",
            "Loop toggle",
            "ContraApp::save_prefs + AudioEngine::set_looping",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(
            || {
                app.prefs.tts_enabled = !app.prefs.tts_enabled;
                app.speech.set_enabled(app.prefs.tts_enabled);
                if !app.prefs.tts_enabled {
                    app.speech.stop();
                }
                app.save_prefs();
            },
            samples,
        );
        report.record(
            "session",
            "Speak calls toggle",
            "ContraApp::save_prefs + Speech::set_enabled",
            ACTION_BUDGET_MS,
            times,
            "stub speech; a real neural voice preparation is asynchronous",
        );
    }
    {
        let app = harness.state_mut();
        let before = app.prefs.clone();
        let times = timed(
            || {
                app.prefs.tts_deliveries = if app.prefs.tts_deliveries > 1 { 1 } else { 4 };
                app.reconcile_preferences(before.clone());
            },
            samples,
        );
        report.record(
            "session",
            "Deliveries per call",
            "ContraApp::reconcile_preferences → Speech::set_deliveries",
            ACTION_BUDGET_MS,
            times,
            "stub speech; extra deliveries prepare asynchronously",
        );
    }

    // Selection and library actions.
    {
        let app = harness.state_mut();
        let dance = crate::defaults::default_dance();
        let times = timed(|| app.use_dance(dance.clone()), samples);
        report.record(
            "library",
            "Use this dance",
            "ContraApp::use_dance → download_dance + rebuild_timeline + save_prefs",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let tune = crate::defaults::default_tune();
        let times = timed(|| app.use_tune(tune.clone()), samples);
        report.record(
            "library",
            "Use this tune",
            "ContraApp::use_tune → download_tune + rotation_settings_changed + save_prefs",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let tune = crate::defaults::default_tune();
        let times = timed(|| app.toggle_rotation_tune(&tune), samples);
        report.record(
            "library",
            "Add/remove tune from set",
            "ContraApp::toggle_rotation_tune",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(
            || {
                let source = if matches!(
                    app.prefs.tune_rotation.source,
                    contra_core::TuneSetSource::List
                ) {
                    contra_core::TuneSetSource::Folder("Reels".into())
                } else {
                    contra_core::TuneSetSource::List
                };
                app.set_tune_set_source(source);
            },
            samples,
        );
        report.record(
            "library",
            "Switch tune set source",
            "ContraApp::set_tune_set_source",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let mut key = 0;
        let times = timed(
            || {
                key += 1;
                app.custom_key = format!("profile phrase {key}");
                app.custom_alias = "profile alias".into();
                app.add_custom_alias();
            },
            samples,
        );
        report.record(
            "library",
            "Add custom alias",
            "ContraApp::add_custom_alias → AliasTable::set_custom + rebuild_timeline",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        app.role_edit_pending = true;
        let times = timed(
            || {
                app.role_edit_pending = true;
                app.commit_role_terms();
            },
            samples,
        );
        report.record(
            "library",
            "Commit custom role terms",
            "ContraApp::commit_role_terms → rebuild_timeline",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(|| app.save_prefs(), samples);
        report.record(
            "library",
            "Save preferences",
            "ContraApp::save_prefs → Preferences::to_json",
            ACTION_BUDGET_MS,
            times,
            "headless has no file path, so this times serialization and the clone",
        );
    }

    // Undo/redo: making an edit then restoring it rebuilds the timeline.
    {
        let app = harness.state_mut();
        let target = app.snapshot();
        // Force the state away from the snapshot so restore does real work.
        app.prefs.lead.min_beats = if app.prefs.lead.min_beats > 1.0 {
            1.0
        } else {
            1.5
        };
        app.rebuild_timeline();
        let times = timed(|| app.restore_snapshot(target.clone()), samples);
        report.record(
            "history",
            "Undo one edit",
            "ContraApp::restore_snapshot → rebuild_timeline + save_prefs",
            ACTION_BUDGET_MS,
            times,
            "the work behind an undo or redo that changed the call timeline",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(
            || {
                let _ = app.snapshot();
            },
            samples,
        );
        report.record(
            "history",
            "Snapshot for history",
            "ContraApp::snapshot",
            ACTION_BUDGET_MS,
            times,
            "runs every frame the UI is visible",
        );
    }

    // Preferences screen work that is not already covered by a screen frame.
    {
        let app = harness.state_mut();
        let before = app.prefs.clone();
        app.prefs.role_terms =
            contra_core::RoleTerms::preset(contra_core::RolePreset::LadiesGentlemen);
        app.prefs.role_preset = contra_core::RolePreset::LadiesGentlemen;
        let times = timed(|| app.reconcile_preferences(before.clone()), samples);
        report.record(
            "preferences",
            "Role preset change",
            "ContraApp::reconcile_preferences → rebuild_timeline",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let app = harness.state_mut();
        let before = app.prefs.clone();
        app.prefs.accompaniment_variations = !app.prefs.accompaniment_variations;
        let times = timed(|| app.reconcile_preferences(before.clone()), samples);
        report.record(
            "preferences",
            "Accompaniment variation toggle",
            "ContraApp::reconcile_preferences → refresh_render_flags",
            ACTION_BUDGET_MS,
            times,
            "queues a render; the render itself is timed under music",
        );
    }
    {
        let app = harness.state_mut();
        let before = app.prefs.clone();
        app.prefs.theme = if app.prefs.theme == contra_core::Theme::Dark {
            contra_core::Theme::Light
        } else {
            contra_core::Theme::Dark
        };
        let times = timed(|| app.reconcile_preferences(before.clone()), samples);
        report.record(
            "preferences",
            "Theme toggle",
            "ContraApp::reconcile_preferences → save_prefs",
            ACTION_BUDGET_MS,
            times,
            "theme::apply runs at the top of the next frame",
        );
    }
    {
        let app = harness.state_mut();
        let times = timed(|| app.save_prefs(), samples);
        report.record(
            "preferences",
            "Save after a settings edit",
            "ContraApp::save_prefs",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }

    // Maintenance: the memory table is rebuilt on demand.
    {
        let ctx = harness.ctx.clone();
        let app = harness.state_mut();
        let times = timed(
            || {
                let _ = app.memory_report(Some(&ctx));
            },
            samples,
        );
        report.record(
            "preferences",
            "Memory table",
            "ContraApp::memory_report",
            ACTION_BUDGET_MS,
            times,
            "scans the loaded SoundFont sample data and font atlas",
        );
    }
}

/// Pure helpers that back import, export, sheet music, and playback.
fn profile_pure_functions(
    harness: &mut Harness<'_, ContraApp>,
    report: &mut Report,
    samples: usize,
) {
    let dance = crate::defaults::default_dance();
    let tune = crate::defaults::default_tune();
    let abc = tune.abc.clone();

    // Import and export formats.
    {
        let times = timed(
            || {
                let _ = crate::files::tune_from_abc("profile.abc", &abc);
            },
            samples,
        );
        report.record(
            "import/export",
            "Import an ABC file",
            "files::tune_from_abc → contra_music::parse_abc",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let json = crate::files::dance_json(&dance).expect("the default dance serializes");
        let times = timed(
            || {
                let _ = crate::files::dance_from_json(&json);
            },
            samples,
        );
        report.record(
            "import/export",
            "Import a dance JSON file",
            "files::dance_from_json",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let times = timed(
            || {
                let _ = crate::files::dance_json(&dance);
            },
            samples,
        );
        report.record(
            "import/export",
            "Export a dance as JSON",
            "files::dance_json",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let times = timed(
            || {
                let _ = crate::files::tune_export(&tune);
            },
            samples,
        );
        report.record(
            "import/export",
            "Export a tune as ABC",
            "files::tune_export",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }

    // Sheet music: engrave once, then rasterize.
    {
        let times = timed(
            || {
                let _ = crate::sheet::abc_to_svg(&abc);
            },
            samples,
        );
        report.record(
            "sheet",
            "Engrave sheet music",
            "sheet::abc_to_svg → acorde::io::parse_abc + acorde_render_svg::render_svg",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let times = timed(
            || {
                let _ = crate::sheet::SheetPreview::render(&harness.ctx, &abc);
            },
            samples,
        );
        report.record(
            "sheet",
            "Render sheet preview",
            "SheetPreview::render → abc_to_svg + resvg rasterize + egui texture upload",
            ACTION_BUDGET_MS,
            times,
            "runs the first time a tune preview is opened",
        );
    }

    // Music pipeline, measured as separate stages so the report can tell
    // parsing from synthesis.
    {
        let times = timed(
            || {
                let _ = parse_abc(&abc);
            },
            samples,
        );
        report.record(
            "music",
            "Parse ABC",
            "contra_music::parse_abc",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    let score = parse_abc(&abc).expect("the bundled tune parses");
    {
        let times = timed(
            || {
                let _ = Arrangement::from_score(&score, 64.0);
            },
            samples,
        );
        report.record(
            "music",
            "Arrange tune",
            "Arrangement::from_score",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    let app = harness.state_mut();
    let soundfont = crate::defaults::default_soundfont();
    {
        let times = timed(
            || {
                let _ = crate::defaults::default_soundfont();
            },
            samples,
        );
        report.record(
            "music",
            "Load bundled SoundFont",
            "defaults::default_soundfont → soundfont::load_soundfont",
            ACTION_BUDGET_MS,
            times,
            "deferred until the first Play",
        );
    }
    {
        let arrangement = Arrangement::from_score(&score, 64.0);
        let original = app.prefs.accompaniment_variations;
        // The default settings play a variation pass set; measure that, and
        // then the single-pass path so the report shows how much of the cost
        // is the arrangement variation rather than synthesis itself.
        app.prefs.accompaniment_variations = true;
        let times = timed(
            || {
                let settings = app.render_settings(app.timeline.count_in_beats);
                let _ = render(&settings.request(arrangement.clone()), soundfont.as_ref());
            },
            samples,
        );
        report.record(
            "music",
            "Synthesize the loop · variations",
            "contra_music::render (VARIATION_PASSES)",
            ACTION_BUDGET_MS,
            times,
            "the default: one 64-beat loop with accompaniment variations",
        );
        app.prefs.accompaniment_variations = false;
        let times = timed(
            || {
                let settings = app.render_settings(app.timeline.count_in_beats);
                let _ = render(&settings.request(arrangement.clone()), soundfont.as_ref());
            },
            samples,
        );
        report.record(
            "music",
            "Synthesize the loop · single pass",
            "contra_music::render (variation_passes = 1)",
            ACTION_BUDGET_MS,
            times,
            "same loop with accompaniment variations off",
        );
        app.prefs.accompaniment_variations = original;
    }
    {
        let arrangement = Arrangement::from_score(&score, 64.0);
        let times = timed(
            || {
                let _ = export_midi_with_band(
                    &arrangement,
                    &app.prefs.band_slots(),
                    app.prefs.accompaniment_rhythm,
                    app.prefs.percussion_rhythm,
                    app.prefs.tempo_bpm,
                    contra_music::VARIATION_PASSES,
                    &app.prefs.variation_options,
                    app.prefs.effects,
                );
            },
            samples,
        );
        report.record(
            "music",
            "Export MIDI",
            "contra_music::export_midi_with_band",
            ACTION_BUDGET_MS,
            times,
            "the file dialog and disk write are outside this measurement",
        );
    }

    // Break the synth render into the stages `contra_music::render` runs in
    // order, so the report says whether the voices or the effects dominate.
    if let Some(soundfont) = soundfont.as_ref() {
        let arrangement = Arrangement::from_score(&score, 64.0);
        let request = app
            .render_settings(app.timeline.count_in_beats)
            .request(arrangement);
        let times = timed(
            || {
                let _ = contra_music::soundfont::render_soundfont(&request, soundfont);
            },
            samples,
        );
        report.record(
            "music · stage",
            "Synthesize sampled voices",
            "contra_music::soundfont::render_soundfont",
            ACTION_BUDGET_MS,
            times,
            "before percussion and effects; one sampled pass per band slot",
        );
        match contra_music::soundfont::render_soundfont(&request, soundfont) {
            Ok(mut buffer) => {
                let start = Instant::now();
                contra_music::effects::apply(
                    &mut buffer.samples,
                    buffer.sample_rate,
                    buffer.channels,
                    request.effects,
                );
                report.record(
                    "music · stage",
                    "Apply effects",
                    "contra_music::effects::apply",
                    ACTION_BUDGET_MS,
                    vec![start.elapsed().as_secs_f64() * 1000.0],
                    "chorus, echo, reverb, and limiter",
                );
            }
            Err(error) => {
                eprintln!("could not profile the synth stages: {error}");
            }
        }
    }

    // Library scaling. These are pure in-memory scans; the synthetic size is
    // deliberately larger than a typical library so a linear or quadratic
    // implementation shows up in the report.
    {
        let library = stress_dance_library(1_000);
        let times = timed(
            || {
                let _ = library.search("stress 9", &FolderFilter::All);
            },
            samples,
        );
        report.record(
            "library",
            "Search 1,000 saved dances",
            "Library::<Dance>::search",
            ACTION_BUDGET_MS,
            times,
            "synthetic 1,000-entry library; the field query is the same scan",
        );
    }
    {
        let library = stress_tune_library(1_000);
        let times = timed(
            || {
                let _ = library.search("stress 9", &FolderFilter::All);
            },
            samples,
        );
        report.record(
            "library",
            "Search 1,000 saved tunes",
            "Library::<Tune>::search",
            ACTION_BUDGET_MS,
            times,
            "synthetic 1,000-entry library",
        );
    }
    {
        let library = stress_dance_library(1_000);
        let times = timed(
            || {
                let _ = library.count(&FolderFilter::All);
            },
            samples,
        );
        report.record(
            "library",
            "Count 1,000 saved dances",
            "Library::<Dance>::count",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let mut library = stress_dance_library(1_000);
        let times = timed(
            || {
                let _ = library.insert(crate::defaults::default_dance(), None, 1);
            },
            samples,
        );
        report.record(
            "library",
            "Insert into 1,000 saved dances",
            "Library::<Dance>::insert",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
    {
        let mut library = stress_dance_library(1_000);
        let times = timed(
            || {
                let _ = library.create_folder("Profile Folder");
                let _ = library.delete_folder("Profile Folder");
            },
            samples,
        );
        report.record(
            "library",
            "Create and delete a folder",
            "Library::<Dance>::create_folder + delete_folder",
            ACTION_BUDGET_MS,
            times,
            "",
        );
    }
}

/// Time the real desktop path: prefetch starts on the first idle frame, and
/// the first Play installs the finished cache instead of synthesizing on the
/// UI thread.
fn profile_prefetched_play(report: &mut Report) {
    let mut harness = harness();
    {
        let app = harness.state_mut();
        app.start_prefetch();
    }
    // The prefetch is background work, not a user action. Wait for it the way
    // an idle event loop would, then time the press.
    loop {
        {
            let app = harness.state_mut();
            if app.prefetch.is_none() {
                break;
            }
            app.poll_prefetch();
            if app.prefetch.is_none() {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let app = harness.state_mut();
    let start = Instant::now();
    app.play();
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    report.record(
        "session",
        "Play · after startup prefetch",
        "ContraApp::play → ensure_ready → apply_audio_settings → cached render_music",
        ACTION_BUDGET_MS,
        vec![elapsed],
        "the normal desktop first Play: the idle prefetch already filled the RAM cache",
    );
}

/// Entry point used by the `contra-perf` binary.
pub fn run() -> Result<(), Box<dyn Error>> {
    let samples = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(5)
        .max(1);

    let mut report = Report::new();
    let start = Instant::now();
    let mut harness = harness();
    let startup_ms = start.elapsed().as_secs_f64() * 1000.0;
    report.record(
        "startup",
        "Launch the app",
        "ContraApp::new_headless + egui context + harness",
        STARTUP_BUDGET_MS,
        vec![startup_ms],
        "headless: defaults, in-memory cache, stub speech, no audio device or network",
    );

    // Warm the font atlas and first-frame layout before timing screens; the
    // first egui frame allocates textures that later frames reuse.
    harness.run_steps(3);

    profile_screen_frames(&mut harness, &mut report, samples);
    profile_phone_frames(&mut harness, &mut report, samples);
    profile_actions(&mut harness, &mut report, samples);
    profile_pure_functions(&mut harness, &mut report, samples);
    profile_prefetched_play(&mut report);

    report.print();
    Ok(())
}
