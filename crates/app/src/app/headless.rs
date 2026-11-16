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
use egui_kittest::kittest::Queryable;

use super::ui::preferences::{PreferencesCategory, PreferencesPage};
use super::{ContraApp, Screen};

const DESKTOP: Vec2 = Vec2::new(1280.0, 800.0);
const MOBILE: Vec2 = Vec2::new(390.0, 844.0);

/// Render all headless screenshots into `out`.
///
/// Pass `mobile` as the second argument to render the phone-width inspection
/// set into `out/mobile` instead of the desktop documentation set.
pub fn run() -> Result<(), Box<dyn Error>> {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "website/static/img/screens".to_owned());
    let out = Path::new(&out);
    let mobile = std::env::args().nth(2).as_deref() == Some("mobile");
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

    if mobile {
        let out = out.join("mobile");
        std::fs::create_dir_all(&out)?;
        mobile_shots(&mut harness, &out)?;
        return Ok(());
    }

    session_shots(&mut harness, out)?;
    dance_library_shots(&mut harness, out)?;
    dance_picker_shots(&mut harness, out)?;
    tune_library_shots(&mut harness, out)?;
    tune_picker_shots(&mut harness, out)?;
    preference_shots(&mut harness, out)?;
    remaining_shots(&mut harness, out)?;

    Ok(())
}
/// Launch state, the transport, and the live session states.
fn session_shots(harness: &mut Harness<'_, ContraApp>, out: &Path) -> Result<(), Box<dyn Error>> {
    // Launch state: READY, defaults, music not rendered yet.
    {
        let app = harness.state_mut();
        app.screen = Screen::Session;
        app.needs_render = true;
        app.started = false;
        app.error = None;
    }
    capture(harness, out, "main-idle")?;

    {
        let app = harness.state_mut();
        app.needs_render = false;
    }
    capture(harness, out, "main-dance-loaded")?;

    // Session controls and summary are present in different states.
    {
        let app = harness.state_mut();
        app.prefs.tempo_bpm = 220.0;
    }
    capture(harness, out, "transport-tempo")?;
    {
        let app = harness.state_mut();
        app.prefs.tempo_bpm = 240.0;
        app.prefs.band = vec![
            contra_core::BandSlot {
                part: contra_core::BandPart::Melody,
                voice: contra_core::BandVoice::Accordion,
                players: 2,
                volume: 0.75,
                pan: -0.15,
                depth: 0.0,
            },
            contra_core::BandSlot {
                part: contra_core::BandPart::Accompaniment,
                voice: contra_core::BandVoice::Guitar,
                players: 2,
                volume: 0.65,
                pan: 0.10,
                depth: 0.10,
            },
            contra_core::BandSlot {
                part: contra_core::BandPart::Countermelody,
                voice: contra_core::BandVoice::Fiddle,
                players: 2,
                volume: 0.55,
                pan: 0.30,
                depth: 0.05,
            },
            contra_core::BandSlot {
                part: contra_core::BandPart::Percussion,
                voice: contra_core::BandVoice::Drumset,
                players: 2,
                volume: 0.5,
                pan: 0.0,
                depth: 0.20,
            },
        ];
        app.prefs.percussion_rhythm = contra_core::PercussionRhythm::Shuffle;
    }
    capture(harness, out, "transport-summary")?;
    {
        let app = harness.state_mut();
        app.prefs.band = contra_core::default_band();
        app.prefs.percussion_rhythm = contra_core::PercussionRhythm::Auto;
        app.prefs.master_volume = 0.5;
    }
    capture(harness, out, "transport-volume")?;
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
    capture(harness, out, "session-count-in")?;
    {
        let app = harness.state_mut();
        app.audio.seek_beat(-1.0);
    }
    capture(harness, out, "session-first-call")?;

    {
        let app = harness.state_mut();
        app.audio.seek_beat(4.0);
    }
    capture(harness, out, "session-call-now")?;
    {
        let app = harness.state_mut();
        app.audio.seek_beat(4.5);
    }
    capture(harness, out, "session-lead-countdown")?;

    {
        let app = harness.state_mut();
        app.audio.seek_beat(9.0);
    }
    capture(harness, out, "session-upcoming-move")?;
    {
        let app = harness.state_mut();
        app.audio.pause();
    }
    capture(harness, out, "session-paused")?;

    {
        let app = harness.state_mut();
        app.audio.force_finished();
        // The transport maps a loop-end position back to beat 0, so clear the
        // schedule to render the completed state without an active call.
        app.timeline.calls.clear();
        app.started = true;
    }
    capture(harness, out, "session-finished")?;

    // Pending rebuild state.
    {
        let app = harness.state_mut();
        app.started = true;
        app.needs_render = true;
        app.audio.play();
        app.audio.seek_beat(4.0);
    }
    capture(harness, out, "session-changes-pending")?;

    // Role and alias effects in an active session.
    {
        let app = harness.state_mut();
        app.prefs.role_terms =
            contra_core::RoleTerms::preset(contra_core::RolePreset::LadiesGentlemen);
        app.prefs.role_preset = contra_core::RolePreset::LadiesGentlemen;
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
    capture(harness, out, "session-roles-applied")?;
    {
        let app = harness.state_mut();
        app.audio.seek_beat(12.0);
    }
    capture(harness, out, "session-alias-call")?;

    {
        let app = harness.state_mut();
        app.prefs.aliases = contra_core::AliasTable::default();
        app.prefs.role_terms = contra_core::RoleTerms::default();
        app.prefs.role_preset = contra_core::RolePreset::LarksRobins;
        app.prefs.loop_enabled = false;
        app.prefs.tts_enabled = false;
        app.audio.set_looping(false);
    }
    capture(harness, out, "transport-loop-off")?;
    {
        let app = harness.state_mut();
        app.prefs.loop_enabled = true;
        app.prefs.tts_enabled = true;
        app.audio.set_looping(true);
    }
    capture(harness, out, "transport-tts-on")?;
    {
        let app = harness.state_mut();
        app.prefs.tts_enabled = false;
        app.prefs.loop_enabled = true;
        app.audio.stop();
        app.started = false;
    }

    Ok(())
}

/// The saved-dance library and its folder search.
fn dance_library_shots(
    harness: &mut Harness<'_, ContraApp>,
    out: &Path,
) -> Result<(), Box<dyn Error>> {
    // Saved-dance library: all saved cards with a preview, then a search
    // narrowed to one folder.
    {
        let app = harness.state_mut();
        app.dance_library = library_dances();
        app.dance_library_query.clear();
        app.dance_library_folder = FolderFilter::All;
        app.picker.dance_preview = app
            .dance_library
            .get("contradb", "2409")
            .map(|entry| entry.item.clone());
        app.picker.dance_selected = None;
        app.screen = Screen::DancePicker;
        app.error = None;
    }
    capture(harness, out, "dance-library")?;
    {
        let app = harness.state_mut();
        app.dance_library_query = "belle".into();
        app.dance_library_folder = FolderFilter::Folder("Warmups".into());
        app.picker.dance_preview = app
            .dance_library
            .get("contradb", "2154")
            .map(|entry| entry.item.clone());
    }
    capture(harness, out, "dance-library-search")?;

    Ok(())
}

/// Downloading a dance: search, sort, filters, and preview.
fn dance_picker_shots(
    harness: &mut Harness<'_, ContraApp>,
    out: &Path,
) -> Result<(), Box<dyn Error>> {
    // Download dance keeps the existing ContraDB search and preview UI.
    {
        let app = harness.state_mut();
        app.screen = Screen::DanceDownload;
        app.dance_library_query.clear();
        app.dance_library_folder = FolderFilter::All;
        app.picker.dance_results = None;
        app.picker.dance_preview = None;
        app.picker.dance_query.title.clear();
        app.picker.dance_query.user.clear();
        app.picker.dance_query.hook.clear();
        app.picker.dance_query.figure_move = None;
    }
    capture(harness, out, "dance-picker-open")?;

    {
        let app = harness.state_mut();
        app.picker.dance_results = Some(dance_results());
        app.picker.dance_query.title = "last".into();
        app.picker.dance_preview = None;
        app.picker.dance_selected = None;
    }
    capture(harness, out, "dance-picker-search")?;
    {
        let app = harness.state_mut();
        app.picker.dance_query.sort = contra_sources::DanceSort::TitleAsc;
    }
    capture(harness, out, "dance-sort")?;
    {
        let app = harness.state_mut();
        app.picker.dance_query.sort = contra_sources::DanceSort::Relevancy;
        app.picker.dance_query.page = 2;
        app.picker.dance_results = Some(dance_results_page_two());
    }
    // The pagination row sits below the fold at 800 px.
    harness.set_size(Vec2::new(1280.0, 1050.0));
    capture(harness, out, "dance-pagination")?;
    harness.set_size(DESKTOP);
    {
        let app = harness.state_mut();
        app.picker.dance_query.page = 1;
        app.picker.dance_results = Some(dance_results());
    }

    {
        let app = harness.state_mut();
        let dance = super::defaults::default_dance();
        app.picker.dance_selected = Some(DanceSummary {
            id: dance.source.id.clone(),
            title: dance.title.clone(),
            choreographer: dance.choreographer.clone(),
            formation: dance.formation.clone(),
            hook: String::new(),
            user: "Koren A Wake".into(),
        });
        app.picker.dance_preview = Some(dance);
    }
    capture(harness, out, "dance-picker-preview")?;

    {
        let app = harness.state_mut();
        app.picker.dance_query.user = "Koren".into();
    }
    capture(harness, out, "dance-filter-user")?;
    {
        let app = harness.state_mut();
        app.picker.dance_query.user.clear();
        app.picker.dance_query.hook = "star".into();
    }
    capture(harness, out, "dance-filter-hook")?;
    {
        let app = harness.state_mut();
        app.picker.dance_query.hook.clear();
        app.picker.dance_query.figure_move = Some("swing".into());
    }
    capture(harness, out, "dance-filter-figure-type")?;

    // A short dance preview shows the length warning.
    {
        let app = harness.state_mut();
        let mut dance = super::defaults::default_dance();
        dance.figures[0].beats -= 0.5;
        app.length_warning = super::length_warning_for(&dance, &app.tune);
        app.dance = dance.clone();
        app.picker.dance_preview = Some(dance);
        app.rebuild_timeline();
    }
    capture(harness, out, "dance-preview-beats")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::Session;
        app.started = false;
        app.needs_render = false;
    }
    capture(harness, out, "session-length-warning")?;

    Ok(())
}

/// The saved-tune library and its folder search.
fn tune_library_shots(
    harness: &mut Harness<'_, ContraApp>,
    out: &Path,
) -> Result<(), Box<dyn Error>> {
    // Saved-tune library: all saved tunes with a sheet-music preview, then a
    // search narrowed to one folder.
    {
        let app = harness.state_mut();
        app.tune_library = library_tunes();
        app.tune_library_query.clear();
        app.tune_library_folder = FolderFilter::All;
        app.picker.tune_preview = app
            .tune_library
            .get("abcsessiontunes", "75815")
            .map(|entry| entry.item.clone());
        app.tune_sheet = None;
        app.picker.tune_selected = None;
        app.screen = Screen::TunePicker;
    }
    capture(harness, out, "tune-library")?;
    {
        let app = harness.state_mut();
        app.tune_library_query = "saint".into();
        app.tune_library_folder = FolderFilter::Folder("Reels".into());
        app.picker.tune_preview = app
            .tune_library
            .get("abcsessiontunes", "75412")
            .map(|entry| entry.item.clone());
        app.tune_sheet = None;
    }
    capture(harness, out, "tune-library-search")?;

    Ok(())
}

/// Downloading a tune: search, filters, and sheet preview.
fn tune_picker_shots(
    harness: &mut Harness<'_, ContraApp>,
    out: &Path,
) -> Result<(), Box<dyn Error>> {
    // Download tune keeps the existing search and sheet-music preview UI.
    {
        let app = harness.state_mut();
        app.screen = Screen::TuneDownload;
        app.tune_library_query.clear();
        app.tune_library_folder = FolderFilter::All;
        app.dance = super::defaults::default_dance();
        app.rebuild_timeline();
        app.length_warning = super::length_warning_for(&app.dance, &app.tune);
        app.picker.tune_results = None;
        app.picker.tune_preview = None;
        app.picker.tune_selected = None;
        app.tune_sheet = None;
    }
    capture(harness, out, "tune-picker-open")?;
    {
        let app = harness.state_mut();
        app.picker.tune_results = Some(tune_results());
        app.picker.tune_query.search = "sandy".into();
        app.picker.tune_preview = Some(super::defaults::default_tune());
        app.picker.tune_selected = Some(TuneSummary {
            id: "75815".into(),
            title: "Sandy Boys".into(),
            composer: String::new(),
            key: "A mix".into(),
            meter: "4/4".into(),
            rhythm: "reel".into(),
        });
    }
    capture(harness, out, "tune-picker-preview")?;
    {
        let app = harness.state_mut();
        app.picker.tune_preview = None;
        app.picker.tune_selected = None;
        app.picker.tune_query.search = "sandy".into();
    }
    capture(harness, out, "tune-search")?;

    {
        let app = harness.state_mut();
        app.picker.tune_query.search.clear();
        app.picker.tune_query.key = Some("Amix".into());
    }
    capture(harness, out, "tune-filter-key")?;
    {
        let app = harness.state_mut();
        app.picker.tune_query.key = None;
        app.picker.tune_query.meter = Some("4/4".into());
    }
    capture(harness, out, "tune-filter-meter")?;
    {
        let app = harness.state_mut();
        app.picker.tune_query.meter = None;
        app.picker.tune_query.rhythm = Some("reel".into());
    }
    capture(harness, out, "tune-filter-genre")?;

    Ok(())
}

/// Preferences, the alias editor, and maintenance.
fn preference_shots(
    harness: &mut Harness<'_, ContraApp>,
    out: &Path,
) -> Result<(), Box<dyn Error>> {
    // Preferences and alias editor.
    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::Calling);
        app.picker.tune_query.rhythm = None;
        app.prefs.theme = contra_core::Theme::Dark;
        app.error = None;
    }
    capture(harness, out, "prefs-open")?;
    {
        let app = harness.state_mut();
        app.prefs.role_terms = contra_core::RoleTerms::default();
        app.prefs.role_preset = contra_core::RolePreset::LarksRobins;
    }
    capture(harness, out, "prefs-roles-presets")?;
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
        app.prefs.role_preset = contra_core::RolePreset::Custom;
    }
    capture(harness, out, "prefs-roles-custom")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::Appearance);
        app.prefs.animations_enabled = true;
    }
    harness.set_size(Vec2::new(1280.0, 1000.0));
    capture(harness, out, "prefs-interface")?;
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
    capture(harness, out, "prefs-alias-list")?;
    {
        let app = harness.state_mut();
        app.alias_filter = "balance".into();
    }
    capture(harness, out, "prefs-alias-placeholders")?;
    {
        let app = harness.state_mut();
        app.alias_filter = "swing".into();
    }
    capture(harness, out, "prefs-alias-editor")?;
    {
        let app = harness.state_mut();
        app.alias_filter = "allemande".into();
    }
    capture(harness, out, "prefs-alias-reset")?;
    {
        let app = harness.state_mut();
        app.alias_filter = "star".into();
    }
    capture(harness, out, "prefs-alias-reset-all")?;

    // Preferences sections that sit below the fold at 1280x800. The height
    // leaves room for the whole form; `npm run screenshots` records the actual
    // size for the manifest at release time.
    harness.set_size(Vec2::new(1280.0, 2900.0));
    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::BandAndSound);
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
        // Show a section in the first instrument shot; the shots after it add
        // the other rows, and the mix shot returns to the default list.
        if let Some(melody) = app.prefs.band.first_mut() {
            melody.players = 2;
        }
    }
    capture(harness, out, "prefs-instrument-melody")?;
    {
        let app = harness.state_mut();
        if let Some(accompaniment) = app
            .prefs
            .band
            .iter_mut()
            .find(|slot| slot.part == contra_core::BandPart::Accompaniment)
        {
            accompaniment.voice = contra_core::BandVoice::Guitar;
            accompaniment.players = 2;
        }
        app.prefs.accompaniment_rhythm = contra_core::AccompanimentRhythm::BoomChuck;
    }
    capture(harness, out, "prefs-instrument-accompaniment")?;
    {
        let app = harness.state_mut();
        if let Some(countermelody) = app
            .prefs
            .band
            .iter_mut()
            .find(|slot| slot.part == contra_core::BandPart::Countermelody)
        {
            countermelody.voice = contra_core::BandVoice::Fiddle;
            countermelody.players = 2;
        }
    }
    capture(harness, out, "prefs-instrument-second")?;
    {
        let app = harness.state_mut();
        if let Some(percussion) = app
            .prefs
            .band
            .iter_mut()
            .find(|slot| slot.part == contra_core::BandPart::Percussion)
        {
            percussion.voice = contra_core::BandVoice::FootTaps;
            percussion.players = 2;
        }
        app.prefs.percussion_rhythm = contra_core::PercussionRhythm::Gallop;
    }
    capture(harness, out, "prefs-instrument-percussion")?;
    {
        let app = harness.state_mut();
        app.prefs.band = contra_core::default_band();
        app.prefs.caller_volume = 1.0;
    }
    capture(harness, out, "prefs-instrument-mix")?;
    {
        let app = harness.state_mut();
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::Playback);
        app.prefs.accompaniment_rhythm = contra_core::AccompanimentRhythm::Auto;
        app.prefs.accompaniment_variations = true;
    }
    capture(harness, out, "prefs-accompaniment-variations")?;
    {
        let app = harness.state_mut();
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::Calling);
        app.prefs.accompaniment_variations = false;
        app.prefs.lead.auto = false;
    }
    capture(harness, out, "prefs-timing-mode")?;
    {
        let app = harness.state_mut();
        app.prefs.lead.auto = true;
        app.prefs.lead.min_beats = 1.5;
        app.prefs.lead.max_beats = 4.0;
    }
    capture(harness, out, "prefs-timing-bounds")?;
    {
        let app = harness.state_mut();
        app.prefs.lead.syllables_per_second = 5.0;
    }
    capture(harness, out, "prefs-timing-speech-rate")?;
    {
        let app = harness.state_mut();
        app.prefs.lead.max_beats = 5.5;
    }
    capture(harness, out, "prefs-timing-rebuild")?;
    {
        let app = harness.state_mut();
        app.prefs.tts_rate = contra_core::MAX_TTS_RATE;
    }
    capture(harness, out, "prefs-tts-rate")?;
    {
        let app = harness.state_mut();
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::BandAndSound);
        app.prefs.tts_rate = 1.0;
        app.prefs.soundfont = Some(contra_core::SoundFontSetting {
            name: "demo.sf2".into(),
            path: None,
        });
    }
    capture(harness, out, "prefs-sf2-loaded")?;
    {
        let app = harness.state_mut();
        app.prefs.soundfont = None;
        app.prefs.tts_rate = 0.8;
    }
    capture(harness, out, "prefs-sf2-removed")?;
    {
        let app = harness.state_mut();
        app.prefs.soundfont = Some(contra_core::SoundFontSetting {
            name: "demo.sf2".into(),
            path: None,
        });
        app.soundfont_error = Some("unsupported SoundFont: bad RIFF header".into());
    }
    capture(harness, out, "error-sf2")?;
    {
        let app = harness.state_mut();
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::DataAndStorage);
        app.soundfont_error = None;
        app.cache_clear_confirm = true;
    }
    capture(harness, out, "prefs-cache-confirm")?;
    {
        let app = harness.state_mut();
        app.cache_clear_confirm = false;
        app.status = "Cache cleared".into();
    }
    capture(harness, out, "prefs-cache-cleared")?;
    {
        let app = harness.state_mut();
        app.prefs.web_proxy_url = Some("https://proxy.example.workers.dev".into());
        app.prefs.web_proxy_token = Some("test-token".into());
    }
    capture(harness, out, "prefs-proxy")?;
    {
        let app = harness.state_mut();
        app.status = "Music ready".into();
    }
    capture(harness, out, "prefs-maintenance")?;
    {
        let app = harness.state_mut();
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::Calling);
        app.speech = crate::tts::Speech::stub_unavailable();
    }
    capture(harness, out, "prefs-tts-unavailable")?;

    Ok(())
}

/// The light theme, error banners, and the phone layout.
fn remaining_shots(harness: &mut Harness<'_, ContraApp>, out: &Path) -> Result<(), Box<dyn Error>> {
    // Alias custom section with the list filtered away.
    harness.set_size(DESKTOP);
    {
        let app = harness.state_mut();
        app.screen = Screen::Aliases;
        app.alias_filter = "zzzz-no-match".into();
    }
    capture(harness, out, "prefs-alias-custom")?;

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
    capture(harness, out, "main-light")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::Appearance);
    }
    // A taller viewport shows the Theme buttons below the Calling lead heading.
    harness.set_size(Vec2::new(1280.0, 1000.0));
    capture(harness, out, "prefs-theme-light")?;
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
    capture(harness, out, "error-network")?;
    {
        let app = harness.state_mut();
        app.error = None;
        app.screen = Screen::Session;
        app.audio_error = Some("no audio output device found".into());
    }
    capture(harness, out, "error-audio")?;
    {
        let app = harness.state_mut();
        app.audio_error = None;
        app.screen = Screen::TuneDownload;
        app.error = Some("could not parse source response: unexpected token".into());
    }
    capture(harness, out, "error-parse")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::DanceDownload;
        app.error = Some("HTTP 503 Service Unavailable".into());
    }
    capture(harness, out, "error-dance-load")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::TuneDownload;
    }
    capture(harness, out, "error-tune-load")?;

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
    capture(harness, out, "main-mobile")?;
    {
        let app = harness.state_mut();
        app.started = true;
        app.audio.play();
        app.audio.seek_beat(4.0);
    }
    capture(harness, out, "session-mobile-controls")?;

    // About is the last section of Preferences. This shot scrolls that page,
    // so it comes after every other Preferences shot in the run: the scroll
    // position is remembered and would move a later screenshot.
    harness.set_size(DESKTOP);
    {
        let app = harness.state_mut();
        app.audio.stop();
        app.started = false;
        app.prefs.web_proxy_url = None;
        app.prefs.web_proxy_token = None;
        app.screen = Screen::Preferences;
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::About);
        app.about_reveal = true;
    }
    capture(harness, out, "about-window")?;

    // The glossary is a reference page opened from a term or from About. Open
    // one entry so the emphasis around the followed term is visible.
    {
        let app = harness.state_mut();
        app.open_glossary(Some("Balance and swing"), Screen::Preferences);
    }
    capture(harness, out, "glossary")?;

    // A desktop definition tooltip follows the pointer over a linked term.
    {
        let app = harness.state_mut();
        app.screen = Screen::Session;
        app.started = true;
        app.needs_render = false;
        app.audio.play();
        app.audio.seek_beat(4.0);
    }
    harness.ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.interaction.tooltip_delay = 0.0
    });
    harness.run_steps(2);
    if let Some(term) = harness.query_all_by_label("balance and swing").next() {
        harness.hover_at(term.rect().center());
    }
    capture(harness, out, "glossary-tooltip")?;

    Ok(())
}

fn mobile_shots(harness: &mut Harness<'_, ContraApp>, out: &Path) -> Result<(), Box<dyn Error>> {
    harness.set_size(MOBILE);
    {
        let app = harness.state_mut();
        app.prefs.animations_enabled = false;
        app.prefs.tts_enabled = true;
        app.speech = crate::tts::Speech::stub();
        app.error = None;
        app.audio_error = None;
        app.screen = Screen::Session;
        app.started = false;
        app.needs_render = true;
        app.about_reveal = false;
    }
    capture(harness, out, "main-idle")?;

    {
        let app = harness.state_mut();
        app.started = true;
        app.needs_render = false;
        app.audio.play();
        app.audio.seek_beat(4.0);
    }
    capture(harness, out, "session-call-now")?;

    harness.run_steps(2);
    super::ui::show_tooltip_preview(&harness.ctx, "Balance and swing");
    capture(harness, out, "glossary-tooltip")?;

    {
        let app = harness.state_mut();
        app.audio.stop();
        app.started = false;
        app.dance_library = library_dances();
        app.dance_library_query.clear();
        app.dance_library_folder = FolderFilter::All;
        app.picker.dance_preview = app
            .dance_library
            .get("contradb", "2409")
            .map(|entry| entry.item.clone());
        app.picker.dance_selected = None;
        app.screen = Screen::DancePicker;
    }
    capture(harness, out, "dance-library")?;

    {
        let app = harness.state_mut();
        app.screen = Screen::DanceDownload;
        app.picker.dance_results = Some(dance_results());
        app.picker.dance_query.title = "last".into();
        app.picker.dance_preview = None;
        app.picker.dance_selected = None;
    }
    capture(harness, out, "dance-picker-search")?;
    {
        let app = harness.state_mut();
        app.picker.dance_preview = Some(super::defaults::default_dance());
        app.picker.dance_selected = None;
    }
    // A taller phone-width viewport shows the stacked preview below the
    // results; the 390×844 shot is how a user starts the screen.
    harness.set_size(Vec2::new(390.0, 1500.0));
    capture(harness, out, "dance-picker-preview")?;
    harness.set_size(MOBILE);

    {
        let app = harness.state_mut();
        app.tune_library = library_tunes();
        app.tune_library_query.clear();
        app.tune_library_folder = FolderFilter::All;
        app.picker.tune_preview = app
            .tune_library
            .get("abcsessiontunes", "75815")
            .map(|entry| entry.item.clone());
        app.picker.tune_selected = None;
        app.tune_sheet = None;
        app.screen = Screen::TunePicker;
    }
    capture(harness, out, "tune-library")?;

    {
        let app = harness.state_mut();
        app.screen = Screen::TuneDownload;
        app.picker.tune_results = Some(tune_results());
        app.picker.tune_query.search = "sandy".into();
        app.picker.tune_preview = Some(super::defaults::default_tune());
        app.picker.tune_selected = None;
        app.tune_sheet = None;
    }
    harness.set_size(Vec2::new(390.0, 1700.0));
    capture(harness, out, "tune-picker-preview")?;
    harness.set_size(MOBILE);

    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
        app.prefs_page = PreferencesPage::Categories;
        app.prefs.role_terms = contra_core::RoleTerms::default();
        app.prefs.role_preset = contra_core::RolePreset::LarksRobins;
        app.prefs.theme = contra_core::Theme::Dark;
    }
    capture(harness, out, "preferences")?;
    {
        let app = harness.state_mut();
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::Calling);
        app.prefs.role_terms = contra_core::RoleTerms {
            left_singular: "port".into(),
            left_plural: "ports".into(),
            right_singular: "starboard".into(),
            right_plural: "starboards".into(),
        };
        app.prefs.role_preset = contra_core::RolePreset::Custom;
    }
    capture(harness, out, "preferences-custom-roles")?;
    harness.set_size(Vec2::new(390.0, 3600.0));
    capture(harness, out, "preferences-all")?;
    harness.set_size(MOBILE);

    {
        let app = harness.state_mut();
        app.screen = Screen::Aliases;
        app.alias_filter.clear();
    }
    capture(harness, out, "aliases")?;
    {
        let app = harness.state_mut();
        app.alias_filter = "zzzz-no-match".into();
    }
    capture(harness, out, "aliases-custom")?;

    {
        let app = harness.state_mut();
        app.alias_filter.clear();
        app.open_glossary(None, Screen::Aliases);
    }
    capture(harness, out, "glossary")?;

    {
        let app = harness.state_mut();
        app.alias_filter.clear();
        app.screen = Screen::DanceDownload;
        app.error = Some("HTTP 500 Internal Server Error".into());
    }
    capture(harness, out, "error-network")?;

    // The window's minimum width is 360 points; check the densest screens at
    // that size as well, where a single extra label would clip a control.
    harness.set_size(Vec2::new(360.0, 640.0));
    {
        let app = harness.state_mut();
        app.error = None;
        app.screen = Screen::DanceDownload;
        app.picker.dance_results = Some(dance_results());
        app.picker.dance_preview = None;
    }
    capture(harness, out, "dance-picker-360")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::DancePicker;
        app.dance_library_query.clear();
        app.dance_library_folder = FolderFilter::All;
        app.picker.dance_preview = app
            .dance_library
            .get("contradb", "2409")
            .map(|entry| entry.item.clone());
        app.picker.dance_selected = None;
    }
    capture(harness, out, "dance-library-360")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::TuneDownload;
        app.picker.tune_results = Some(tune_results());
        app.picker.tune_preview = None;
    }
    capture(harness, out, "tune-picker-360")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::TunePicker;
        app.tune_library_query.clear();
        app.tune_library_folder = FolderFilter::All;
        app.picker.tune_preview = app
            .tune_library
            .get("abcsessiontunes", "75815")
            .map(|entry| entry.item.clone());
        app.picker.tune_selected = None;
        app.tune_sheet = None;
    }
    capture(harness, out, "tune-library-360")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
        app.prefs.role_terms = contra_core::RoleTerms {
            left_singular: "port".into(),
            left_plural: "ports".into(),
            right_singular: "starboard".into(),
            right_plural: "starboards".into(),
        };
        app.prefs.role_preset = contra_core::RolePreset::Custom;
    }
    capture(harness, out, "preferences-360")?;
    {
        let app = harness.state_mut();
        app.screen = Screen::Session;
        app.started = false;
        app.needs_render = false;
    }
    capture(harness, out, "main-360")?;

    // About is the last section of Preferences. This shot scrolls that page,
    // so it comes last: the scroll position is remembered per screen and would
    // otherwise move a later Preferences screenshot.
    harness.set_size(MOBILE);
    {
        let app = harness.state_mut();
        app.screen = Screen::Preferences;
        app.prefs_page = PreferencesPage::Category(PreferencesCategory::About);
        app.about_reveal = true;
    }
    capture(harness, out, "about-window")?;

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
