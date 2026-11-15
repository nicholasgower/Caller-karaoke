//! Headless screenshot renderer for the documentation site.
//!
//! This runs the real `ContraApp` UI through `egui_kittest` with a silent
//! transport, in-memory source cache, and no preference file. No window,
//! audio device, network client, or system TTS is opened.

use std::error::Error;
use std::path::Path;

use contra_core::{Dance, FolderFilter, Library, LibraryEntry, SourceRef, Tune};
use contra_sources::{DanceSearchPage, DanceSummary, TuneSearchPage, TuneSummary};
use eframe::egui::{self, Vec2};
use egui_kittest::Harness;

use super::{ContraApp, Screen};

const DESKTOP: Vec2 = Vec2::new(1280.0, 800.0);
const MOBILE: Vec2 = Vec2::new(390.0, 844.0);

/// Render all headless screenshots into `out`.
pub fn run() -> Result<(), Box<dyn Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "website/static/img/screens".to_owned());
    let out = Path::new(&out);
    std::fs::create_dir_all(out)?;

    let mut harness = Harness::builder()
        .with_size(DESKTOP)
        .with_pixels_per_point(1.0)
        .with_theme(egui::Theme::Dark)
        .with_os(egui::os::OperatingSystem::Windows)
        .with_render_cursor(false)
        .with_accessibility_check(false)
        .allow_missing_glyphs()
        .with_max_steps(10)
        .build_eframe(|cc| ContraApp::new_headless(cc));

    // Launch state: READY, defaults, music not rendered yet.
    {
        let app = harness.state_mut();
        app.screen = Screen::Session;
        app.needs_render = true;
        app.started = false;
        app.error = None;
    }
    capture(&mut harness, out, "main-idle")?;

    {
        let app = harness.state_mut();
        app.needs_render = false;
    }
    capture(&mut harness, out, "main-dance-loaded")?;

    // Session controls and summary are present in different states.
    {
        let app = harness.state_mut();
        app.prefs.tempo_bpm = 220.0;
    }
    capture(&mut harness, out, "transport-tempo")?;
    {
        let app = harness.state_mut();
        app.prefs.tempo_bpm = 240.0;
        app.prefs.melody = contra_core::MelodyInstrument::Accordion;
        app.prefs.accompaniment = contra_core::AccompanimentInstrument::Guitar;
    }
    capture(&mut harness, out, "transport-summary")?;
    {
        let app = harness.state_mut();
        app.prefs.melody = contra_core::MelodyInstrument::Fiddle;
        app.prefs.accompaniment = contra_core::AccompanimentInstrument::Piano;
        app.prefs.master_volume = 0.5;
    }
    capture(&mut harness, out, "transport-volume")?;
    {
        let app = harness.state_mut();
        app.prefs.master_volume = 0.8;
    }

    // Count-in, call-now, paused, and finished states.
    {
        let app = harness.state_mut();
        app.started = true;
        app.needs_render = false;
        app.audio.play();
        app.audio.seek_beat(-2.0);
    }
    capture(&mut harness, out, "session-count-in")?;
    {
        let app = harness.state_mut();
        app.audio.seek_beat(-1.0);
    }
    capture(&mut harness, out, "session-first-call")?;

    {
        let app = harness.state_mut();
        app.audio.seek_beat(4.0);
    }
    capture(&mut harness, out, "session-call-now")?;
    {
        let app = harness.state_mut();
        app.audio.seek_beat(4.5);
    }
    capture(&mut harness, out, "session-lead-countdown")?;

    {
        let app = harness.state_mut();
        app.audio.seek_beat(9.0);
    }
    capture(&mut harness, out, "session-upcoming-move")?;
    {
        let app = harness.state_mut();
        app.audio.pause();
    }
    capture(&mut harness, out, "session-paused")?;

    {
        let app = harness.state_mut();
        app.audio.force_finished();
        // The transport maps a loop-end position back to beat 0, so clear the
        // schedule to render the completed state without an active call.
        app.timeline.calls.clear();
        app.started = true;
    }
    capture(&mut harness, out, "session-finished")?;

    // Pending rebuild state.
    {
        let app = harness.state_mut();
        app.started = true;
        app.needs_render = true;
        app.audio.play();
        app.audio.seek_beat(4.0);
    }
    capture(&mut harness, out, "session-changes-pending")?;

    // Role and alias effects in an active session.
    {
        let app = harness.state_mut();
        app.prefs.role_terms =
            contra_core::RoleTerms::preset(contra_core::RolePreset::LadiesGentlemen);
        app.prefs.aliases.set_override(
            contra_core::MoveKind::BalanceAndSwing,
            "partners balance and swing with your partner",
        );
        app.rebuild_timeline();
        app.started = true;
        app.needs_render = false;
        app.audio.play();
        app.audio.seek_beat(4.0);
    }
    capture(&mut harness, out, "session-roles-applied")?;
    {
        let app = harness.state_mut();
        app.audio.seek_beat(12.0);
    }
    capture(&mut harness, out, "session-alias-call")?;

    {
        let app = harness.state_mut();
        app.prefs.aliases = contra_core::AliasTable::default();
        app.prefs.role_terms = contra_core::RoleTerms::default();
        app.prefs.loop_enabled = false;
        app.prefs.tts_enabled = false;
        app.audio.set_looping(false);
    }
    capture(&mut harness, out, "transport-loop-off")?;
    {
        let app = harness.state_mut();
        app.prefs.loop_enabled = true;
        app.prefs.tts_enabled = true;
        app.audio.set_looping(true);
    }
    capture(&mut harness, out, "transport-tts-on")?;
    {
        let app = harness.state_mut();
        app.prefs.tts_enabled = false;
        app.prefs.loop_enabled = true;
        app.audio.stop();
        app.started = false;
    }

    // Saved-dance library: all saved cards with a preview, then a search
    // narrowed to one folder.
    {
        let app = harness.state_mut();
        app.dance_library = library_dances();
        app.dance_library_query.clear();
        app.dance_library_folder = FolderFilter::All;
        app.dance_preview = app
            .dance_library
            .get("contradb", "2409")
            .map(|entry| entry.item.clone());
        app.dance_selected = None;
        app.screen = Screen::DancePicker;
        app.error = None;
    }
    capture(&mut harness, out, "dance-library")?;
    {
        let app = harness.state_mut();
        app.dance_library_query = "belle".into();
        app.dance_library_folder = FolderFilter::Folder("Warmups".into());
        app.dance_preview = app
            .dance_library
            .get("contradb", "2154")
            .map(|entry| entry.item.clone());
    }
    capture(&mut harness, out, "dance-library-search")?;

    // Download dance keeps the existing ContraDB search and preview UI.
    {
        let app = harness.state_mut();
        app.screen = Screen::DanceDownload;
        app.dance_library_query.clear();
        app.dance_library_folder = FolderFilter::All;
        app.dance_results = None;
        app.dance_preview = None;
        app.dance_query.title.clear();
        app.dance_query.user.clear();
        app.dance_query.hook.clear();
        app.dance_query.figure_move = None;
    }
    capture(&mut harness, out, "dance-picker-open")?;

    {
        let app = harness.state_mut();
        app.dance_results = Some(dance_results());
        app.dance_query.title = "last".into();
        app.dance_preview = None;
        app.dance_selected = None;
    }
    capture(&mut harness, out, "dance-picker-search")?;
    {
        let app = harness.state_mut();
        app.dance_query.sort = contra_sources::DanceSort::TitleAsc;
    }
    capture(&mut harness, out, "dance-sort")?;
    {
        let app = harness.state_mut();
        app.dance_query.sort = contra_sources::DanceSort::Relevancy;
        app.dance_query.page = 2;
        app.dance_results = Some(dance_results_page_two());
    }
    // The pagination row sits below the fold at 800 px.
    harness.set_size(Vec2::new(1280.0, 1050.0));
    capture(&mut harness, out, "dance-pagination")?;
    harness.set_size(DESKTOP);
    {
        let app = harness.state_mut();
        app.dance_query.page = 1;
        app.dance_results = Some(dance_results());
    }

    {
        let app = harness.state_mut();
        let dance = super::defaults::default_dance();
        app.dance_selected = Some(DanceSummary {
            id: dance.source.id.clone(),
            title: dance.title.clone(),
            choreographer: dance.choreographer.clone(),
            formation: dance.formation.clone(),
            hook: String::new(),
            user: "Koren A Wake".into(),
        });
        app.dance_preview = Some(dance);
    }
    capture(&mut harness, out, "dance-picker-preview")?;

    {
        let app = harness.state_mut();
        app.dance_query.user = "Koren".into();
    }
    capture(&mut harness, out, "dance-filter-user")?;
    {
        let app = harness.state_mut();
        app.dance_query.user.clear();
        app.dance_query.hook = "star".into();
    }
    capture(&mut harness, out, "dance-filter-hook")?;
    {
        let app = harness.state_mut();
        app.dance_query.hook.clear();
        app.dance_query.figure_move = Some("swing".into());
    }
    capture(&mut harness, out, "dance-filter-figure-type")?;

    // A short dance preview shows the length warning.
    {
        let app = harness.state_mut();
        let mut dance = super::defaults::default_dance();
        dance.figures[0].beats -= 0.5;
        app.length_warning = super::length_warning_for(&dance, &app.tune);
        app.dance = dance.clone();
        app.dance_preview = Some(dance);
        app.rebuild_timeline();
    }
    capture(&mut harness, out, "dance-preview-beats")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::Session;
        app.started = false;
        app.needs_render = false;
    }
    capture(&mut harness, out, "session-length-warning")?;

    // Saved-tune library: all saved tunes with a sheet-music preview, then a
    // search narrowed to one folder.
    {
        let app = harness.state_mut();
        app.tune_library = library_tunes();
        app.tune_library_query.clear();
        app.tune_library_folder = FolderFilter::All;
        app.tune_preview = app
            .tune_library
            .get("abcsessiontunes", "75815")
            .map(|entry| entry.item.clone());
        app.tune_sheet = None;
        app.tune_selected = None;
        app.screen = Screen::TunePicker;
    }
    capture(&mut harness, out, "tune-library")?;
    {
        let app = harness.state_mut();
        app.tune_library_query = "saint".into();
        app.tune_library_folder = FolderFilter::Folder("Reels".into());
        app.tune_preview = app
            .tune_library
            .get("abcsessiontunes", "75412")
            .map(|entry| entry.item.clone());
        app.tune_sheet = None;
    }
    capture(&mut harness, out, "tune-library-search")?;

    // Download tune keeps the existing search and sheet-music preview UI.
    {
        let app = harness.state_mut();
        app.screen = Screen::TuneDownload;
        app.tune_library_query.clear();
        app.tune_library_folder = FolderFilter::All;
        app.dance = super::defaults::default_dance();
        app.rebuild_timeline();
        app.length_warning = super::length_warning_for(&app.dance, &app.tune);
        app.tune_results = None;
        app.tune_preview = None;
        app.tune_selected = None;
        app.tune_sheet = None;
    }
    capture(&mut harness, out, "tune-picker-open")?;
    {
        let app = harness.state_mut();
        app.tune_results = Some(tune_results());
        app.tune_query.search = "sandy".into();
        app.tune_preview = Some(super::defaults::default_tune());
        app.tune_selected = Some(TuneSummary {
            id: "75815".into(),
            title: "Sandy Boys".into(),
            composer: String::new(),
            key: "A mix".into(),
            meter: "4/4".into(),
            rhythm: "reel".into(),
        });
    }
    capture(&mut harness, out, "tune-picker-preview")?;
    {
        let app = harness.state_mut();
        app.tune_preview = None;
        app.tune_selected = None;
        app.tune_query.search = "sandy".into();
    }
    capture(&mut harness, out, "tune-search")?;

    {
        let app = harness.state_mut();
        app.tune_query.search.clear();
        app.tune_query.key = Some("Amix".into());
    }
    capture(&mut harness, out, "tune-filter-key")?;
    {
        let app = harness.state_mut();
        app.tune_query.key = None;
        app.tune_query.meter = Some("4/4".into());
    }
    capture(&mut harness, out, "tune-filter-meter")?;
    {
        let app = harness.state_mut();
        app.tune_query.meter = None;
        app.tune_query.rhythm = Some("reel".into());
    }
    capture(&mut harness, out, "tune-filter-genre")?;

    // Preferences and alias editor.
    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
        app.tune_query.rhythm = None;
        app.prefs.theme = contra_core::Theme::Dark;
        app.error = None;
    }
    capture(&mut harness, out, "prefs-open")?;
    {
        let app = harness.state_mut();
        app.prefs.role_terms =
            contra_core::RoleTerms::preset(contra_core::RolePreset::LadiesGentlemen);
    }
    capture(&mut harness, out, "prefs-roles-presets")?;
    {
        let app = harness.state_mut();
        // `RoleTerms::preset(Custom)` is identical to Larks/Robins, so use
        // clearly custom words to show the editable custom-role fields.
        app.prefs.role_terms = contra_core::RoleTerms {
            left_singular: "port".into(),
            left_plural: "ports".into(),
            right_singular: "starboard".into(),
            right_plural: "starboards".into(),
        };
    }
    capture(&mut harness, out, "prefs-roles-custom")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
        app.prefs.animations_enabled = true;
    }
    harness.set_size(Vec2::new(1280.0, 1000.0));
    capture(&mut harness, out, "prefs-interface")?;
    harness.set_size(DESKTOP);
    {
        let app = harness.state_mut();
        app.prefs.animations_enabled = false;
    }
    {
        let app = harness.state_mut();
        app.screen = Screen::Aliases;
        app.alias_filter.clear();
    }
    capture(&mut harness, out, "prefs-alias-list")?;
    {
        let app = harness.state_mut();
        app.alias_filter = "balance".into();
    }
    capture(&mut harness, out, "prefs-alias-placeholders")?;
    {
        let app = harness.state_mut();
        app.alias_filter = "swing".into();
    }
    capture(&mut harness, out, "prefs-alias-editor")?;
    {
        let app = harness.state_mut();
        app.alias_filter = "allemande".into();
    }
    capture(&mut harness, out, "prefs-alias-reset")?;
    {
        let app = harness.state_mut();
        app.alias_filter = "star".into();
    }
    capture(&mut harness, out, "prefs-alias-reset-all")?;

    // Preferences sections that sit below the fold at 1280x800.
    harness.set_size(Vec2::new(1280.0, 2250.0));
    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
        app.alias_filter.clear();
        app.prefs.theme = contra_core::Theme::Dark;
        app.prefs.tts_enabled = true;
        app.speech = crate::tts::Speech::stub();
        app.prefs.soundfont = None;
        app.soundfont_error = None;
        app.cache_clear_confirm = false;
        app.status = "Ready".into();
        app.prefs.web_proxy_url = None;
        app.prefs.web_proxy_token = None;
    }
    capture(&mut harness, out, "prefs-instrument-melody")?;
    {
        let app = harness.state_mut();
        app.prefs.accompaniment = contra_core::AccompanimentInstrument::Guitar;
    }
    capture(&mut harness, out, "prefs-instrument-accompaniment")?;
    {
        let app = harness.state_mut();
        app.prefs.melody_volume = 0.5;
        app.prefs.accompaniment_volume = 0.5;
    }
    capture(&mut harness, out, "prefs-instrument-mix")?;
    {
        let app = harness.state_mut();
        app.prefs.accompaniment_variations = true;
    }
    capture(&mut harness, out, "prefs-accompaniment-variations")?;
    {
        let app = harness.state_mut();
        app.prefs.accompaniment_variations = false;
        app.prefs.lead.auto = false;
    }
    capture(&mut harness, out, "prefs-timing-mode")?;
    {
        let app = harness.state_mut();
        app.prefs.lead.auto = true;
        app.prefs.lead.min_beats = 1.5;
        app.prefs.lead.max_beats = 4.0;
    }
    capture(&mut harness, out, "prefs-timing-bounds")?;
    {
        let app = harness.state_mut();
        app.prefs.lead.syllables_per_second = 5.0;
    }
    capture(&mut harness, out, "prefs-timing-speech-rate")?;
    {
        let app = harness.state_mut();
        app.prefs.lead.max_beats = 5.5;
    }
    capture(&mut harness, out, "prefs-timing-rebuild")?;
    {
        let app = harness.state_mut();
        app.prefs.tts_rate = 1.5;
    }
    capture(&mut harness, out, "prefs-tts-rate")?;
    {
        let app = harness.state_mut();
        app.prefs.tts_rate = 1.0;
        app.prefs.soundfont = Some(contra_core::SoundFontSetting {
            name: "demo.sf2".into(),
            path: None,
        });
    }
    capture(&mut harness, out, "prefs-sf2-loaded")?;
    {
        let app = harness.state_mut();
        app.prefs.soundfont = None;
        app.prefs.tts_rate = 0.8;
    }
    capture(&mut harness, out, "prefs-sf2-removed")?;
    {
        let app = harness.state_mut();
        app.prefs.soundfont = Some(contra_core::SoundFontSetting {
            name: "demo.sf2".into(),
            path: None,
        });
        app.soundfont_error = Some("unsupported SoundFont: bad RIFF header".into());
    }
    capture(&mut harness, out, "error-sf2")?;
    {
        let app = harness.state_mut();
        app.soundfont_error = None;
        app.cache_clear_confirm = true;
    }
    capture(&mut harness, out, "prefs-cache-confirm")?;
    {
        let app = harness.state_mut();
        app.cache_clear_confirm = false;
        app.status = "Cache cleared".into();
    }
    capture(&mut harness, out, "prefs-cache-cleared")?;
    {
        let app = harness.state_mut();
        app.prefs.web_proxy_url = Some("https://proxy.example.workers.dev".into());
        app.prefs.web_proxy_token = Some("test-token".into());
    }
    capture(&mut harness, out, "prefs-proxy")?;
    {
        let app = harness.state_mut();
        app.status = "Music ready".into();
    }
    capture(&mut harness, out, "prefs-maintenance")?;
    {
        let app = harness.state_mut();
        app.speech = crate::tts::Speech::stub_unavailable();
    }
    capture(&mut harness, out, "prefs-tts-unavailable")?;

    // Alias custom section with the list filtered away.
    harness.set_size(DESKTOP);
    {
        let app = harness.state_mut();
        app.screen = Screen::Aliases;
        app.alias_filter = "zzzz-no-match".into();
    }
    capture(&mut harness, out, "prefs-alias-custom")?;

    // Light theme session.
    {
        let app = harness.state_mut();
        app.alias_filter.clear();
        app.prefs.theme = contra_core::Theme::Light;
        app.screen = Screen::Session;
        app.started = false;
        app.needs_render = false;
        app.audio.stop();
    }
    capture(&mut harness, out, "main-light")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
    }
    // A taller viewport shows the Theme buttons below the Calling lead heading.
    harness.set_size(Vec2::new(1280.0, 1000.0));
    capture(&mut harness, out, "prefs-theme-light")?;
    harness.set_size(DESKTOP);

    // Errors are visible on the screen that produced them.
    {
        let app = harness.state_mut();
        app.prefs.theme = contra_core::Theme::Dark;
        app.screen = Screen::DanceDownload;
        app.speech = crate::tts::Speech::stub();
        app.prefs.tts_enabled = true;
        app.error = Some("HTTP 500 Internal Server Error".into());
    }
    capture(&mut harness, out, "error-network")?;
    {
        let app = harness.state_mut();
        app.error = None;
        app.screen = Screen::Session;
        app.audio_error = Some("no audio output device found".into());
    }
    capture(&mut harness, out, "error-audio")?;
    {
        let app = harness.state_mut();
        app.audio_error = None;
        app.screen = Screen::TuneDownload;
        app.error = Some("could not parse source response: unexpected token".into());
    }
    capture(&mut harness, out, "error-parse")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::DanceDownload;
        app.error = Some("HTTP 503 Service Unavailable".into());
    }
    capture(&mut harness, out, "error-dance-load")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::TuneDownload;
    }
    capture(&mut harness, out, "error-tune-load")?;

    // Mobile viewport.
    {
        let app = harness.state_mut();
        app.error = None;
        app.screen = Screen::Session;
        app.started = false;
        app.needs_render = false;
        app.speech = crate::tts::Speech::stub();
        app.prefs.tts_enabled = true;
    }
    harness.set_size(MOBILE);
    capture(&mut harness, out, "main-mobile")?;
    {
        let app = harness.state_mut();
        app.started = true;
        app.audio.play();
        app.audio.seek_beat(4.0);
    }
    capture(&mut harness, out, "session-mobile-controls")?;

    Ok(())
}

fn capture(
    harness: &mut Harness<'_, ContraApp>,
    out: &Path,
    id: &str,
) -> Result<(), Box<dyn Error>> {
    harness.run_steps(3);
    let image = harness
        .render()
        .map_err(|error| -> Box<dyn Error> { format!("render {id}: {error}").into() })?;
    image.save(out.join(format!("{id}.png")))?;
    Ok(())
}

fn dance_results() -> DanceSearchPage {
    DanceSearchPage {
        total: 23,
        page: 1,
        page_size: 10,
        dances: vec![
            DanceSummary {
                id: "2409".into(),
                title: "Another Last Dance".into(),
                choreographer: "Erik Hoffman".into(),
                formation: "improper".into(),
                hook: String::new(),
                user: "Koren A Wake".into(),
            },
            DanceSummary {
                id: "2380".into(),
                title: "Allison's Favorite".into(),
                choreographer: "Bob Isaacs".into(),
                formation: "improper".into(),
                hook: "A smooth, flowing dance".into(),
                user: "Koren A Wake".into(),
            },
            DanceSummary {
                id: "3061".into(),
                title: "Arkansas Traveler".into(),
                choreographer: "Traditional".into(),
                formation: "improper".into(),
                hook: String::new(),
                user: "ContraDB".into(),
            },
        ],
    }
}

fn dance_results_page_two() -> DanceSearchPage {
    DanceSearchPage {
        total: 23,
        page: 2,
        page_size: 10,
        dances: vec![
            DanceSummary {
                id: "2154".into(),
                title: "Belle of the Ball".into(),
                choreographer: "Ralph Sweet".into(),
                formation: "proper".into(),
                hook: String::new(),
                user: "ContraDB".into(),
            },
            DanceSummary {
                id: "3022".into(),
                title: "Chorus Jig".into(),
                choreographer: "Traditional".into(),
                formation: "proper".into(),
                hook: "A classic proper dance".into(),
                user: "ContraDB".into(),
            },
            DanceSummary {
                id: "2477".into(),
                title: "The Baby Rose".into(),
                choreographer: "Don Flaherty".into(),
                formation: "improper".into(),
                hook: String::new(),
                user: "Koren A Wake".into(),
            },
        ],
    }
}

fn tune_results() -> TuneSearchPage {
    TuneSearchPage {
        total: 8,
        page: 1,
        page_size: 10,
        tunes: vec![
            TuneSummary {
                id: "75815".into(),
                title: "Sandy Boys".into(),
                composer: String::new(),
                key: "A mix".into(),
                meter: "4/4".into(),
                rhythm: "reel".into(),
            },
            TuneSummary {
                id: "75412".into(),
                title: "Saint Anne's Reel".into(),
                composer: String::new(),
                key: "D".into(),
                meter: "4/4".into(),
                rhythm: "reel".into(),
            },
        ],
    }
}

fn library_dances() -> Library<Dance> {
    let mut library = Library::default();
    let _ = library.create_folder("Warmups");
    let _ = library.create_folder("Encores");
    library.entries = vec![
        saved_dance(
            "2409",
            "Another Last Dance",
            "Erik Hoffman",
            "improper",
            None,
        ),
        saved_dance(
            "2154",
            "Belle of the Ball",
            "Ralph Sweet",
            "proper",
            Some("Warmups"),
        ),
        saved_dance(
            "2380",
            "Allison's Favorite",
            "Bob Isaacs",
            "improper",
            Some("Encores"),
        ),
    ];
    library
}

fn saved_dance(
    id: &str,
    title: &str,
    choreographer: &str,
    formation: &str,
    folder: Option<&str>,
) -> LibraryEntry<Dance> {
    let mut dance = super::defaults::default_dance();
    dance.source = SourceRef::new("contradb", id, format!("https://contradb.com/dances/{id}"));
    dance.title = title.into();
    dance.choreographer = choreographer.into();
    dance.formation = formation.into();
    LibraryEntry {
        item: dance,
        folder: folder.map(str::to_string),
        downloaded_at: 0,
    }
}

fn library_tunes() -> Library<Tune> {
    let mut library = Library::default();
    let _ = library.create_folder("Reels");
    let _ = library.create_folder("Jigs");
    library.entries = vec![
        saved_tune("75815", "Sandy Boys", "", "A mix", "reel", None),
        saved_tune("75412", "Saint Anne's Reel", "", "D", "reel", Some("Reels")),
        saved_tune(
            "1279",
            "Banish Misfortune",
            "Traditional",
            "D mix",
            "jig",
            Some("Jigs"),
        ),
    ];
    library
}

fn saved_tune(
    id: &str,
    title: &str,
    composer: &str,
    key: &str,
    rhythm: &str,
    folder: Option<&str>,
) -> LibraryEntry<Tune> {
    let mut tune = super::defaults::default_tune();
    tune.source = SourceRef::new(
        "abcsessiontunes",
        id,
        format!("https://thesession.org/tunes/{id}"),
    );
    tune.title = title.into();
    tune.composer = composer.into();
    tune.key = key.into();
    tune.rhythm = rhythm.into();
    LibraryEntry {
        item: tune,
        folder: folder.map(str::to_string),
        downloaded_at: 0,
    }
}
