//! Application state and UI.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use contra_core::{
    AccompanimentInstrument, AccompanimentRhythm, AliasTable, Dance, FolderFilter, Library,
    MelodyInstrument, MoveKind, PracticeTimeline, Preferences, RolePreset, RoleTerms, Theme,
    TimelineConfig, Tune,
};
use contra_music::{
    Arrangement, AudioBuffer, LengthMismatch, RenderRequest, export_midi, parse_abc, render,
};
#[cfg(not(target_arch = "wasm32"))]
use contra_sources::FileCache;
use contra_sources::{
    Cache, DanceQuery, DanceSearchPage, DanceSort, DanceSummary, TuneQuery, TuneSearchPage,
    TuneSummary,
};
use eframe::egui;
use rustysynth::SoundFont;

use crate::audio::AudioEngine;
use crate::defaults;
use crate::library;
use crate::sheet::SheetPreview;
use crate::source::{SourceClient, SourceEvent};
use crate::theme;
use crate::tts::Speech;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Session,
    DancePicker,
    DanceDownload,
    TunePicker,
    TuneDownload,
    Preferences,
    Aliases,
}

/// How the tune picker shows the selected tune.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum TuneView {
    #[default]
    Sheet,
    Abc,
}

impl TuneView {
    fn toggled(self) -> Self {
        match self {
            Self::Sheet => Self::Abc,
            Self::Abc => Self::Sheet,
        }
    }

    fn toggle_label(self) -> &'static str {
        match self {
            Self::Sheet => "Show ABC",
            Self::Abc => "Show sheet music",
        }
    }
}

impl Screen {
    /// Where Escape leads from this screen. The session itself ignores Escape.
    fn escape_target(self) -> Option<Self> {
        SCREENS
            .iter()
            .find(|def| def.screen == self)
            .and_then(|def| def.escape)
    }
}

/// One top-row tab: the screens it covers, its labels, and its shortcut key.
///
/// A tab stays lit for every screen it covers, so a detail screen such as the
/// download search does not light the session tab instead.
struct TabDef {
    screens: &'static [Screen],
    label: &'static str,
    /// Shorter label used at phone width.
    short_label: &'static str,
    hover: &'static str,
    shortcut: &'static str,
    key: egui::Key,
}

impl TabDef {
    fn is_active(&self, screen: Screen) -> bool {
        self.screens.contains(&screen)
    }

    /// The screen the tab opens; the first one in its list.
    fn landing(&self) -> Screen {
        self.screens[0]
    }
}

const TABS: [TabDef; 4] = [
    TabDef {
        screens: &[Screen::Session],
        label: "Session",
        short_label: "Session",
        hover: "Session",
        shortcut: "1",
        key: egui::Key::Num1,
    },
    TabDef {
        screens: &[Screen::DancePicker, Screen::DanceDownload],
        label: "Select dance",
        short_label: "Dance",
        hover: "Select dance",
        shortcut: "2",
        key: egui::Key::Num2,
    },
    TabDef {
        screens: &[Screen::TunePicker, Screen::TuneDownload],
        label: "Select Tune",
        short_label: "Tune",
        hover: "Select Tune",
        shortcut: "3",
        key: egui::Key::Num3,
    },
    TabDef {
        screens: &[Screen::Preferences],
        label: "Preferences",
        short_label: "Prefs",
        hover: "Preferences",
        shortcut: "4",
        key: egui::Key::Num4,
    },
];

/// One screen: how to draw it and where Escape leads from it.
struct ScreenDef {
    screen: Screen,
    draw: fn(&mut ContraApp, &mut egui::Ui, f64),
    escape: Option<Screen>,
}

const SCREENS: [ScreenDef; 7] = [
    ScreenDef {
        screen: Screen::Session,
        draw: draw_session,
        escape: None,
    },
    ScreenDef {
        screen: Screen::DancePicker,
        draw: draw_dance_library,
        escape: Some(Screen::Session),
    },
    ScreenDef {
        screen: Screen::DanceDownload,
        draw: draw_dance_download,
        escape: Some(Screen::DancePicker),
    },
    ScreenDef {
        screen: Screen::TunePicker,
        draw: draw_tune_library,
        escape: Some(Screen::Session),
    },
    ScreenDef {
        screen: Screen::TuneDownload,
        draw: draw_tune_download,
        escape: Some(Screen::TunePicker),
    },
    ScreenDef {
        screen: Screen::Preferences,
        draw: draw_preferences,
        escape: Some(Screen::Session),
    },
    ScreenDef {
        screen: Screen::Aliases,
        draw: draw_aliases,
        escape: Some(Screen::Preferences),
    },
];

// The registry needs uniform draw functions; the screens themselves have
// different signatures.
fn draw_session(app: &mut ContraApp, ui: &mut egui::Ui, beat: f64) {
    app.session_ui(ui, beat);
}
fn draw_dance_library(app: &mut ContraApp, ui: &mut egui::Ui, _beat: f64) {
    app.dance_library_ui(ui);
}
fn draw_dance_download(app: &mut ContraApp, ui: &mut egui::Ui, _beat: f64) {
    app.dance_download_ui(ui);
}
fn draw_tune_library(app: &mut ContraApp, ui: &mut egui::Ui, _beat: f64) {
    app.tune_library_ui(ui);
}
fn draw_tune_download(app: &mut ContraApp, ui: &mut egui::Ui, _beat: f64) {
    app.tune_download_ui(ui);
}
fn draw_preferences(app: &mut ContraApp, ui: &mut egui::Ui, _beat: f64) {
    app.preferences_ui(ui);
}
fn draw_aliases(app: &mut ContraApp, ui: &mut egui::Ui, _beat: f64) {
    app.aliases_ui(ui);
}

/// Project links shown in the About window.
///
/// The pages may not be live yet; the addresses are the ones the documentation
/// and packaging use so they start working as soon as the site is published.
const REPOSITORY_URL: &str = "https://github.com/storytold/Contra-karaoke";
const DOCUMENTATION_URL: &str = "https://storytold.github.io/Contra-karaoke/";

/// Map the number-key tab shortcuts to screens.
fn screen_for_tab_key(key: egui::Key) -> Option<Screen> {
    TABS.iter()
        .find(|tab| tab.key == key)
        .map(|tab| tab.landing())
}

/// The screen selected by a tab shortcut pressed this frame, if any.
///
/// The key must actually have been pressed: resolving the first tab's
/// shortcut regardless of input would pin the app to the session screen every
/// frame and undo any click on another tab.
fn tab_shortcut(input: &egui::InputState) -> Option<Screen> {
    let pressed = TABS.iter().find(|tab| input.key_pressed(tab.key))?;
    screen_for_tab_key(pressed.key)
}

pub struct ContraApp {
    prefs: Preferences,
    prefs_path: Option<PathBuf>,
    /// Directory for the saved dance and tune libraries; `None` in the
    /// headless renderer, which must not touch user data.
    library_dir: Option<PathBuf>,
    audio: AudioEngine,
    audio_error: Option<String>,
    speech: Speech,
    source: SourceClient,
    source_rx: Receiver<SourceEvent>,
    screen: Screen,
    /// Progress (0..1) of the fade/slide used when the active screen changes.
    screen_anim: f32,
    /// The screen the animation progress belongs to; a mismatch restarts it.
    screen_anim_screen: Screen,
    /// True for `contra-headless`: ignore global shortcuts so the renderer can
    /// hold an arbitrary screen and state.
    headless: bool,
    /// Keeps the window's logical size when it moves to a monitor with a
    /// different display scale.
    scale_guard: crate::window::ScaleGuard,
    /// The About window is open. It is a modal over every screen.
    about_open: bool,
    dance: Dance,
    tune: Tune,
    timeline: PracticeTimeline,
    arrangement: Option<Arrangement>,
    /// A font the user loaded; `None` means use the bundled one.
    soundfont: Option<Arc<SoundFont>>,
    /// The bundled subset, loaded on the first render.
    bundled_soundfont: Option<Arc<SoundFont>>,
    bundled_soundfont_checked: bool,
    soundfont_error: Option<String>,
    needs_render: bool,
    /// Playback settings changed and need a render once the user stops
    /// adjusting the control.
    pending_audio: bool,
    render_loading: bool,
    started: bool,
    /// Transient state of the dance and tune download screens.
    picker: PickerState,
    dance_library: Library<Dance>,
    tune_library: Library<Tune>,
    dance_library_query: String,
    tune_library_query: String,
    dance_library_folder: FolderFilter,
    tune_library_folder: FolderFilter,
    new_dance_folder: String,
    new_tune_folder: String,
    renaming_dance_folder: Option<String>,
    renaming_tune_folder: Option<String>,
    folder_rename_text: String,
    library_status: Option<String>,
    tune_view: TuneView,
    tune_sheet: Option<Result<SheetPreview, String>>,
    status: String,
    error: Option<String>,
    length_warning: Option<String>,
    last_spoken: Option<f64>,
    last_beat: f64,
    /// A playback run just started at the top of the dance, so the first call
    /// may need to be announced even if its lead already began.
    first_call_pending: bool,
    cache_clear_confirm: bool,
    alias_filter: String,
    custom_key: String,
    custom_alias: String,
    alternative_inputs: std::collections::BTreeMap<String, String>,
    /// Seed for the alternative-alias shuffle. Selecting a dance advances it.
    alias_seed: u64,
}

/// Transient state of the two download screens.
///
/// The dance and tune pickers have the same shape, so they share one struct: a
/// new search or preview field belongs here instead of being threaded through
/// the app struct and both constructors.
#[derive(Default)]
pub(in crate::app) struct PickerState {
    dance_query: DanceQuery,
    tune_query: TuneQuery,
    dance_results: Option<DanceSearchPage>,
    tune_results: Option<TuneSearchPage>,
    dance_preview: Option<Dance>,
    tune_preview: Option<Tune>,
    dance_selected: Option<DanceSummary>,
    tune_selected: Option<TuneSummary>,
    search_loading: bool,
    preview_loading: bool,
}

impl ContraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (source_tx, source_rx): (Sender<SourceEvent>, Receiver<SourceEvent>) = channel();
        let project = directories::ProjectDirs::from("org", "contrakaraoke", "ContraKaraoke");
        let (config_dir, cache_dir) = if let Some(project) = project {
            (
                project.config_dir().to_path_buf(),
                project.cache_dir().to_path_buf(),
            )
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let base = std::env::temp_dir().join("contra-karaoke");
                (base.join("config"), base.join("cache"))
            }
            #[cfg(target_arch = "wasm32")]
            {
                (
                    PathBuf::from("contra-karaoke-config"),
                    PathBuf::from("contra-karaoke-cache"),
                )
            }
        };
        let _ = std::fs::create_dir_all(&config_dir);
        let _ = std::fs::create_dir_all(&cache_dir);
        let prefs_path = config_dir.join("preferences.json");
        let mut prefs = load_prefs(&prefs_path).unwrap_or_default();
        prefs.validate();
        prefs.tts_voice = crate::tts::voice(&prefs.tts_voice).id.to_string();

        let cache = build_cache(&cache_dir);
        let source = SourceClient::new(
            source_tx,
            cache,
            prefs.web_proxy_url.clone(),
            prefs.web_proxy_token.clone(),
        );

        let audio = match AudioEngine::new() {
            Ok(audio) => {
                audio.set_looping(prefs.loop_enabled);
                audio.set_volume(prefs.master_volume);
                (audio, None)
            }
            Err(error) => {
                let audio = AudioEngine::silent(48_000);
                audio.set_looping(prefs.loop_enabled);
                audio.set_volume(prefs.master_volume);
                (audio, Some(error))
            }
        };

        let mut speech = Speech::new(cache_dir.join("tts"), audio.0.sample_rate());
        speech.set_rate(prefs.tts_rate);
        speech.set_voice(&prefs.tts_voice);
        if prefs.tts_enabled {
            speech.set_enabled(true);
        }

        let dance = defaults::default_dance();
        let tune = defaults::default_tune();
        let library_dir = config_dir.join("library");
        let mut dance_library = match library::load_dances(Some(&library_dir)) {
            Some(saved) => saved,
            None => {
                let mut saved = Library::default();
                saved.insert(dance.clone(), None, library::now_seconds());
                library::save_dances(Some(&library_dir), &saved);
                saved
            }
        };
        let mut tune_library = match library::load_tunes(Some(&library_dir)) {
            Some(saved) => saved,
            None => {
                let mut saved = Library::default();
                saved.insert(tune.clone(), None, library::now_seconds());
                library::save_tunes(Some(&library_dir), &saved);
                saved
            }
        };
        dance_library.validate();
        tune_library.validate();
        let timeline = build_timeline(&dance, &prefs, 0).unwrap_or_else(|_| empty_timeline());
        speech.prewarm(call_texts(&timeline));
        let length_warning = length_warning_for(&dance, &tune);

        // A saved font that no longer loads must not be reported as active: the
        // bundled instruments take its place, so say why.
        let mut soundfont_error = None;
        let soundfont = match prefs
            .soundfont
            .as_ref()
            .and_then(|setting| setting.path.as_ref())
        {
            None => None,
            Some(path) => match std::fs::read(path) {
                Ok(bytes) => match contra_music::soundfont::load_soundfont(&bytes) {
                    Ok(soundfont) => Some(soundfont),
                    Err(error) => {
                        soundfont_error = Some(format!(
                            "Could not load the saved SoundFont; using the bundled instruments: {error}"
                        ));
                        None
                    }
                },
                Err(error) => {
                    soundfont_error = Some(format!(
                        "Could not read the saved SoundFont; using the bundled instruments: {error}"
                    ));
                    None
                }
            },
        };

        theme::apply(&cc.egui_ctx, prefs.theme, prefs.animations_enabled);

        Self {
            prefs,
            prefs_path: Some(prefs_path),
            library_dir: Some(library_dir),
            audio: audio.0,
            audio_error: audio.1,
            speech,
            source,
            source_rx,
            screen: Screen::Session,
            screen_anim: 1.0,
            screen_anim_screen: Screen::Session,
            headless: false,
            scale_guard: crate::window::ScaleGuard::default(),
            about_open: false,
            dance,
            tune,
            timeline,
            arrangement: None,
            soundfont,
            bundled_soundfont: None,
            bundled_soundfont_checked: false,
            soundfont_error,
            needs_render: true,
            pending_audio: false,
            render_loading: false,
            started: false,
            picker: PickerState {
                dance_query: DanceQuery {
                    page_size: 10,
                    ..DanceQuery::default()
                },
                tune_query: TuneQuery {
                    page_size: 10,
                    ..TuneQuery::default()
                },
                ..PickerState::default()
            },
            dance_library,
            tune_library,
            dance_library_query: String::new(),
            tune_library_query: String::new(),
            dance_library_folder: FolderFilter::All,
            tune_library_folder: FolderFilter::All,
            new_dance_folder: String::new(),
            new_tune_folder: String::new(),
            renaming_dance_folder: None,
            renaming_tune_folder: None,
            folder_rename_text: String::new(),
            library_status: None,

            tune_view: TuneView::default(),
            tune_sheet: None,

            status: "Ready".into(),
            error: None,
            length_warning,
            last_spoken: None,
            last_beat: 0.0,
            first_call_pending: false,
            cache_clear_confirm: false,
            alias_filter: String::new(),
            custom_key: String::new(),
            custom_alias: String::new(),
            alternative_inputs: std::collections::BTreeMap::new(),
            alias_seed: 0,
        }
    }

    /// Handle app-wide keyboard shortcuts.
    ///
    /// Only called while no widget has keyboard focus, so typing in a text
    /// field never triggers play, restart, or a tab change.
    fn handle_shortcuts(&mut self, ui: &egui::Ui) {
        // The About window is modal: let it handle Escape and ignore the
        // transport and tab shortcuts until it closes.
        if self.about_open {
            return;
        }
        let (space, restart, escape, tab) = ui.input(|input| {
            (
                input.key_pressed(egui::Key::Space),
                input.key_pressed(egui::Key::R),
                input.key_pressed(egui::Key::Escape),
                tab_shortcut(input),
            )
        });

        if let Some(screen) = tab {
            self.screen = screen;
        }
        if space {
            if self.audio.is_playing() {
                self.pause();
            } else {
                self.play();
            }
        }
        if restart {
            self.restart();
        }
        if escape && let Some(target) = self.screen.escape_target() {
            self.screen = target;
        }
    }

    /// Start a dance search from the current filters.
    fn search_dances(&mut self) {
        if self.picker.search_loading {
            return;
        }
        self.picker.search_loading = true;
        self.source.search_dances(self.picker.dance_query.clone());
    }

    /// Start a tune search from the current filters.
    fn search_tunes(&mut self) {
        if self.picker.search_loading {
            return;
        }
        self.picker.search_loading = true;
        self.source.search_tunes(self.picker.tune_query.clone());
    }

    fn save_dance_library(&self) {
        library::save_dances(self.library_dir.as_deref(), &self.dance_library);
    }

    fn save_tune_library(&self) {
        library::save_tunes(self.library_dir.as_deref(), &self.tune_library);
    }

    /// Store a downloaded dance in the local library.
    ///
    /// Re-downloading replaces the stored card but keeps its folder. The
    /// picker calls this from the Download dance screen and from
    /// [`Self::use_dance`], so anything that reaches the session can be found
    /// offline later.
    fn download_dance(&mut self, dance: Dance) {
        let title = dance.title.clone();
        let refreshed = self
            .dance_library
            .contains(&dance.source.provider, &dance.source.id);
        self.dance_library
            .insert(dance, None, library::now_seconds());
        self.save_dance_library();
        self.library_status = Some(if refreshed {
            format!("Updated \"{title}\" in your saved dances.")
        } else {
            format!("Downloaded \"{title}\" to your saved dances.")
        });
    }

    fn download_tune(&mut self, tune: Tune) {
        let title = tune.title.clone();
        let refreshed = self
            .tune_library
            .contains(&tune.source.provider, &tune.source.id);
        self.tune_library.insert(tune, None, library::now_seconds());
        self.save_tune_library();
        self.library_status = Some(if refreshed {
            format!("Updated \"{title}\" in your saved tunes.")
        } else {
            format!("Downloaded \"{title}\" to your saved tunes.")
        });
    }

    fn remove_saved_dance(&mut self, id: &str) {
        let Some(entry) = self.dance_library.remove("contradb", id) else {
            return;
        };
        self.save_dance_library();
        if self
            .picker
            .dance_preview
            .as_ref()
            .is_some_and(|dance| dance.source.id == id)
        {
            self.picker.dance_preview = None;
        }
        self.library_status = Some(format!(
            "Removed \"{}\" from saved dances.",
            entry.item.title
        ));
    }

    fn remove_saved_tune(&mut self, id: &str) {
        let Some(entry) = self.tune_library.remove("abcsessiontunes", id) else {
            return;
        };
        self.save_tune_library();
        if self
            .picker
            .tune_preview
            .as_ref()
            .is_some_and(|tune| tune.source.id == id)
        {
            self.picker.tune_preview = None;
            self.tune_sheet = None;
        }
        self.library_status = Some(format!(
            "Removed \"{}\" from saved tunes.",
            entry.item.title
        ));
    }

    fn move_saved_dance(&mut self, id: &str, folder: Option<String>) {
        let label = folder.clone().unwrap_or_else(|| "Unfiled".into());
        match self.dance_library.set_folder("contradb", id, folder) {
            Ok(()) => {
                self.save_dance_library();
                self.library_status = Some(format!("Moved the dance to {label}."));
            }
            Err(error) => self.library_status = Some(error.to_string()),
        }
    }

    fn move_saved_tune(&mut self, id: &str, folder: Option<String>) {
        let label = folder.clone().unwrap_or_else(|| "Unfiled".into());
        match self.tune_library.set_folder("abcsessiontunes", id, folder) {
            Ok(()) => {
                self.save_tune_library();
                self.library_status = Some(format!("Moved the tune to {label}."));
            }
            Err(error) => self.library_status = Some(error.to_string()),
        }
    }

    /// Add the custom alias currently entered in the alias editor.
    fn add_custom_alias(&mut self) {
        let key = self.custom_key.trim().to_string();
        let alias = self.custom_alias.trim().to_string();
        if key.is_empty() || alias.is_empty() {
            return;
        }
        self.prefs.aliases.set_custom(&key, &alias);
        self.custom_key.clear();
        self.custom_alias.clear();
        self.rebuild_timeline();
        self.needs_render = true;
        self.save_prefs();
    }

    /// Advance the screen-entry animation and return its eased progress.
    fn screen_transition(&mut self, ctx: &egui::Context) -> f32 {
        if self.screen_anim_screen != self.screen {
            self.screen_anim_screen = self.screen;
            self.screen_anim = 0.0;
            self.library_status = None;
        }
        if !self.prefs.animations_enabled {
            self.screen_anim = 1.0;
            return 1.0;
        }
        if self.screen_anim < 1.0 {
            let dt = ctx.input(|input| input.stable_dt).clamp(0.0, 0.05);
            self.screen_anim = (self.screen_anim + dt / SCREEN_ANIM_SECONDS).min(1.0);
            ctx.request_repaint();
        }
        ease_out_cubic(self.screen_anim)
    }

    /// Construct the real app without opening a window, audio device, TTS voice,
    /// network client, or user preference file. Used by `contra-headless` and
    /// screenshot tests.
    #[cfg(feature = "headless")]
    pub fn new_headless(cc: &eframe::CreationContext<'_>) -> Self {
        let (source_tx, source_rx): (Sender<SourceEvent>, Receiver<SourceEvent>) = channel();
        let mut prefs = Preferences::default();
        prefs.validate();
        // Screenshot frames must not be captured mid-transition.
        prefs.animations_enabled = false;

        let audio = AudioEngine::headless_clock(48_000, prefs.tempo_bpm);
        audio.set_looping(prefs.loop_enabled);
        audio.set_volume(prefs.master_volume);

        let mut speech = Speech::stub();
        speech.set_rate(prefs.tts_rate);

        let source = SourceClient::new(
            source_tx,
            Cache::new(contra_sources::MemoryCache::default()),
            None,
            None,
        );

        let dance = defaults::default_dance();
        let tune = defaults::default_tune();
        let mut dance_library = Library::default();
        dance_library.insert(dance.clone(), None, 0);
        let mut tune_library = Library::default();
        tune_library.insert(tune.clone(), None, 0);
        let timeline = build_timeline(&dance, &prefs, 0).unwrap_or_else(|_| empty_timeline());
        let length_warning = length_warning_for(&dance, &tune);

        theme::apply(&cc.egui_ctx, prefs.theme, prefs.animations_enabled);

        Self {
            prefs,
            prefs_path: None,
            library_dir: None,
            audio,
            audio_error: None,
            speech,
            source,
            source_rx,
            screen: Screen::Session,
            screen_anim: 1.0,
            screen_anim_screen: Screen::Session,
            headless: true,
            scale_guard: crate::window::ScaleGuard::default(),
            about_open: false,
            dance,
            tune,
            timeline,
            arrangement: None,
            soundfont: None,
            bundled_soundfont: None,
            bundled_soundfont_checked: false,
            soundfont_error: None,
            needs_render: true,
            pending_audio: false,
            render_loading: false,
            started: false,
            picker: PickerState {
                dance_query: DanceQuery {
                    page_size: 10,
                    ..DanceQuery::default()
                },
                tune_query: TuneQuery {
                    page_size: 10,
                    ..TuneQuery::default()
                },
                ..PickerState::default()
            },
            dance_library,
            tune_library,
            dance_library_query: String::new(),
            tune_library_query: String::new(),
            dance_library_folder: FolderFilter::All,
            tune_library_folder: FolderFilter::All,
            new_dance_folder: String::new(),
            new_tune_folder: String::new(),
            renaming_dance_folder: None,
            renaming_tune_folder: None,
            folder_rename_text: String::new(),
            library_status: None,

            tune_view: TuneView::default(),
            tune_sheet: None,

            status: "Ready".into(),
            error: None,
            length_warning,
            last_spoken: None,
            last_beat: 0.0,
            first_call_pending: false,
            cache_clear_confirm: false,
            alias_filter: String::new(),
            custom_key: String::new(),
            custom_alias: String::new(),
            alternative_inputs: std::collections::BTreeMap::new(),
            alias_seed: 0,
        }
    }

    fn save_prefs(&self) {
        if let Ok(json) = self.prefs.to_json() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some(path) = &self.prefs_path {
                    let _ = std::fs::write(path, json);
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                if self.prefs_path.is_some() {
                    crate::web_store::write_prefs(&json);
                }
            }
        }
    }

    fn rebuild_timeline(&mut self) {
        match build_timeline(&self.dance, &self.prefs, self.alias_seed) {
            Ok(timeline) => {
                self.timeline = timeline;
                self.speech.prewarm(call_texts(&self.timeline));
                self.last_spoken = None;
                self.last_beat = 0.0;
            }
            Err(error) => {
                self.error = Some(error);
            }
        }
    }

    /// The font to render with: the user's file, else the bundled subset.
    ///
    /// The bundled font is parsed on first use, because loading ~15 MB of
    /// sample data is wasted work until the user actually presses Play. A font
    /// that fails to load leaves `None`, so the built-in synth takes over.
    fn active_soundfont(&mut self) -> Option<Arc<SoundFont>> {
        if let Some(soundfont) = &self.soundfont {
            return Some(Arc::clone(soundfont));
        }
        if !self.bundled_soundfont_checked {
            self.bundled_soundfont = defaults::default_soundfont();
            self.bundled_soundfont_checked = true;
            if self.bundled_soundfont.is_none() {
                self.soundfont_error = Some(
                    "The bundled instruments could not be loaded; using the built-in synth.".into(),
                );
            }
        }
        self.bundled_soundfont.clone()
    }

    /// Parse and render the current tune with the current playback settings.
    fn render_music(&mut self) -> Option<(AudioBuffer, Arrangement)> {
        let score = match parse_abc(&self.tune.abc) {
            Ok(score) => score,
            Err(error) => {
                self.length_warning = None;
                self.error = Some(format!("Could not parse tune: {error}"));
                return None;
            }
        };
        let arrangement = Arrangement::from_score(&score, 64.0);
        let dance_beats = self.dance.total_beats();
        self.length_warning = arrangement
            .check_dance_length(dance_beats)
            .err()
            .map(format_length_mismatch);
        let request = RenderRequest {
            arrangement: arrangement.clone(),
            bpm: self.prefs.tempo_bpm,
            melody: self.prefs.melody,
            accompaniment: self.prefs.accompaniment,
            accompaniment_rhythm: self.prefs.accompaniment_rhythm,
            melody_volume: self.prefs.melody_volume,
            accompaniment_volume: self.prefs.accompaniment_volume,
            count_in_beats: if self.prefs.count_in_enabled {
                3.0
            } else {
                0.0
            },
            sample_rate: self.audio.sample_rate(),
            variation_passes: if self.prefs.accompaniment_variations {
                contra_music::VARIATION_PASSES
            } else {
                1
            },
        };
        let soundfont = self.active_soundfont();
        match render(&request, soundfont.as_ref()) {
            Ok(buffer) => Some((buffer, arrangement)),
            Err(error) => {
                self.error = Some(format!("Could not render music: {error}"));
                None
            }
        }
    }

    /// Install a freshly rendered loop without restarting the transport.
    ///
    /// `resume_beat` is the position captured from the old buffer just before
    /// the swap, so a settings change keeps the dance where it was.
    fn install_music(
        &mut self,
        buffer: AudioBuffer,
        arrangement: Arrangement,
        resume_beat: Option<f64>,
    ) {
        // Briefly mute the callback around the swap so the audio thread cannot
        // play the new buffer from sample zero before the seek lands.
        let was_playing = self.audio.is_playing();
        if was_playing {
            self.audio.pause();
        }
        self.audio.set_buffer(buffer);
        self.arrangement = Some(arrangement);
        self.audio.set_looping(self.prefs.loop_enabled);
        self.audio.set_volume(self.prefs.master_volume);
        self.needs_render = false;
        self.pending_audio = false;
        self.status = "Music ready".into();
        if let Some(beat) = resume_beat {
            self.audio.seek_beat(beat);
            self.last_beat = beat;
            // A render that lands mid-call while the music is already playing
            // must not repeat that call. When the transport was not running,
            // nothing has been spoken for this position, so the call must stay
            // eligible ΓÇö otherwise the first call of a freshly selected dance
            // is marked as spoken before it is ever heard.
            if was_playing {
                self.last_spoken = self
                    .timeline
                    .scheduled_call_at(beat, self.prefs.loop_enabled)
                    .filter(|call| call.is_calling(beat))
                    .map(|call| call.start_beat);
            }
        }
        if was_playing {
            self.audio.play();
        }
    }

    /// Re-render after a playback setting changed, keeping the transport where
    /// it is and resuming playback if it was running.
    fn apply_audio_settings(&mut self) {
        self.pending_audio = false;
        self.rebuild_timeline();
        let was_playing = self.audio.is_playing();
        let Some((buffer, arrangement)) = self.render_music() else {
            return;
        };
        // The old buffer kept playing while we rendered, so sample it now to
        // avoid rewinding by the render time.
        let resume_beat = self.audio.has_buffer().then(|| self.audio.position_beat());
        self.install_music(buffer, arrangement, resume_beat);
        if was_playing {
            self.audio.play();
            self.started = true;
            self.status = "Playing".into();
        } else if self.started {
            self.status = "Paused".into();
        }
    }

    fn ensure_ready(&mut self) {
        if self.needs_render || !self.audio.has_buffer() {
            self.apply_audio_settings();
        }
    }

    fn play(&mut self) {
        self.ensure_ready();
        if self.audio.has_buffer() {
            self.audio.play();
            self.started = true;
            self.status = "Playing".into();
            // A resume keeps the call the caller was already speaking; only a
            // run from the top of the dance may need to announce the first
            // call.
            self.first_call_pending = self.last_spoken.is_none() && self.current_beat() <= 0.0;
        }
    }

    fn pause(&mut self) {
        self.audio.pause();
        self.speech.stop();
        self.status = "Paused".into();
    }

    fn restart(&mut self) {
        self.ensure_ready();
        self.last_spoken = None;
        self.last_beat = 0.0;
        self.audio.restart();
        self.started = true;
        self.first_call_pending = true;
        self.status = "Playing".into();
    }

    fn use_dance(&mut self, dance: Dance) {
        self.error = None;
        self.download_dance(dance.clone());
        self.dance = dance;
        self.length_warning = length_warning_for(&self.dance, &self.tune);
        self.prefs.last_dance = Some(self.dance.source.clone());
        self.picker.dance_preview = None;
        self.picker.dance_selected = None;
        self.screen = Screen::Session;
        self.needs_render = true;
        self.arrangement = None;
        self.audio.stop();
        self.speech.stop();
        self.started = false;
        // Give a newly selected dance a fresh shuffle of alternative aliases.
        self.alias_seed = self.alias_seed.wrapping_add(1);
        self.rebuild_timeline();
        self.save_prefs();
        self.status = "Dance selected".into();
    }

    fn use_tune(&mut self, tune: Tune) {
        self.error = None;
        self.download_tune(tune.clone());
        self.tune = tune;
        self.length_warning = length_warning_for(&self.dance, &self.tune);
        self.prefs.last_tune = Some(self.tune.source.clone());
        self.picker.tune_preview = None;
        self.tune_sheet = None;
        self.picker.tune_selected = None;
        self.screen = Screen::Session;
        self.needs_render = true;
        self.arrangement = None;
        self.audio.stop();
        self.speech.stop();
        self.started = false;
        self.save_prefs();
        self.status = "Tune selected".into();
    }

    fn handle_source_events(&mut self) {
        while let Ok(event) = self.source_rx.try_recv() {
            match event {
                SourceEvent::DanceSearch { query, result } => {
                    self.picker.search_loading = false;
                    self.picker.dance_query = query;
                    match result {
                        Ok(page) => {
                            self.picker.dance_results = Some(page);
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                SourceEvent::Dance { result } => {
                    self.picker.preview_loading = false;
                    match result {
                        Ok(dance) => {
                            self.picker.dance_preview = Some(dance);
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                SourceEvent::TuneSearch { query, result } => {
                    self.picker.search_loading = false;
                    self.picker.tune_query = query;
                    match result {
                        Ok(page) => {
                            self.picker.tune_results = Some(page);
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                SourceEvent::Tune { result } => {
                    self.picker.preview_loading = false;
                    match result {
                        Ok(tune) => {
                            self.picker.tune_preview = Some(tune);
                            self.tune_sheet = None;
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
            }
        }
    }

    fn update_speech(&mut self, beat: f64) {
        self.speech.poll(&self.audio);
        if beat + 0.25 < self.last_beat {
            self.last_spoken = None;
        }
        self.last_beat = beat;
        // The first call's lead usually starts before the count-in and can
        // start before the transport does when the count-in is off or shorter
        // than the lead. Capture it before the speech gates so a run that
        // starts with speech disabled never announces a finished first figure.
        let starting = if self.first_call_pending {
            self.timeline.starting_call(beat)
        } else {
            None
        };
        self.first_call_pending = false;
        if !self.prefs.tts_enabled || !self.speech.available() || !self.audio.is_playing() {
            return;
        }
        if let Some(call) = starting
            && self.last_spoken != Some(call.start_beat)
        {
            self.speech.speak(&call.call.text);
            self.last_spoken = Some(call.start_beat);
            return;
        }
        if let Some(call) = self
            .timeline
            .scheduled_call_at(beat, self.prefs.loop_enabled)
            && call.is_calling(beat)
            && self.last_spoken != Some(call.start_beat)
        {
            self.speech.speak(&call.call.text);
            self.last_spoken = Some(call.start_beat);
        }
    }

    fn current_beat(&self) -> f64 {
        if !self.audio.has_buffer() {
            return 0.0;
        }
        let beat = self.audio.position_beat();
        if self.prefs.loop_enabled && beat >= self.timeline.music_loop_beats {
            // The audio callback resets on the next sample, but the UI can
            // briefly observe exactly the loop end. Treat that as beat 0.
            beat - self.timeline.music_loop_beats
        } else {
            beat
        }
    }

    fn export_midi(&mut self) {
        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        {
            let _ = export_midi;
            self.error = Some("MIDI export is not available in this build yet.".into());
        }
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let score = match parse_abc(&self.tune.abc) {
                Ok(score) => score,
                Err(error) => {
                    self.error = Some(format!("Could not parse tune: {error}"));
                    return;
                }
            };
            let arrangement = Arrangement::from_score(&score, 64.0);
            let default_name = format!("{}.mid", safe_filename(&self.tune.title));
            let Some(path) = rfd::FileDialog::new()
                .set_file_name(default_name)
                .add_filter("MIDI", &["mid"])
                .save_file()
            else {
                return;
            };
            let bytes = export_midi(
                &arrangement,
                self.prefs.melody,
                self.prefs.accompaniment,
                self.prefs.accompaniment_rhythm,
                self.prefs.tempo_bpm,
            );
            match std::fs::write(&path, bytes) {
                Ok(()) => self.status = format!("Saved MIDI to {}", path.display()),
                Err(error) => self.error = Some(format!("Could not save MIDI: {error}")),
            }
        }
    }

    fn load_soundfont(&mut self) {
        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        {
            self.soundfont_error =
                Some("Loading a SoundFont from a file is not available in this build yet.".into());
        }
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let Some(path) = rfd::FileDialog::new()
                .add_filter("SoundFont", &["sf2", "sf3"])
                .pick_file()
            else {
                return;
            };
            match std::fs::read(&path) {
                Ok(bytes) => match contra_music::soundfont::load_soundfont(&bytes) {
                    Ok(soundfont) => {
                        let name = path
                            .file_stem()
                            .and_then(|name| name.to_str())
                            .unwrap_or("SoundFont")
                            .to_string();
                        self.soundfont = Some(soundfont);
                        self.prefs.soundfont = Some(contra_core::SoundFontSetting {
                            name,
                            path: Some(path.to_string_lossy().to_string()),
                        });
                        self.needs_render = true;
                        self.soundfont_error = None;
                        self.save_prefs();
                    }
                    Err(error) => self.soundfont_error = Some(error.to_string()),
                },
                Err(error) => self.soundfont_error = Some(error.to_string()),
            }
        }
    }

    fn remove_soundfont(&mut self) {
        self.soundfont = None;
        self.prefs.soundfont = None;
        self.needs_render = true;
        self.save_prefs();
    }
}

impl eframe::App for ContraApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.headless && self.scale_guard.update(&ctx) {
            // The sheet music texture is rasterized at the density it was
            // rendered for and would be blurry on the new monitor.
            self.tune_sheet = None;
        }
        theme::apply(&ctx, self.prefs.theme, self.prefs.animations_enabled);
        self.handle_source_events();

        if !self.headless && !ctx.egui_wants_keyboard_input() {
            self.handle_shortcuts(ui);
        }

        let beat = self.current_beat();
        self.update_speech(beat);

        let compact = ui.available_width() < COMPACT_WIDTH;
        egui::Panel::top("tabs")
            .frame(egui::Frame::default().inner_margin(egui::Margin::symmetric(12, 8)))
            .show(ui, |ui| {
                let animate = self.prefs.animations_enabled;
                // The About button is pinned to the top-right corner; the
                // heading and tabs share the remaining width.
                if compact {
                    // Keep the phone header at the 44 pt touch minimum and
                    // narrow the About button so four short tabs fit beside it.
                    ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
                    ui.spacing_mut().button_padding.x = 8.0;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui
                        .button("About")
                        .on_hover_text("About Contra Karaoke")
                        .clicked()
                    {
                        self.about_open = true;
                    }
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                        ui.horizontal_wrapped(|ui| {
                            // At phone width the four tabs, not the wordmark, are
                            // the useful part of the header. Dropping the heading
                            // and the long tab labels keeps the navigation to one
                            // row and leaves that height for the prompt and card.
                            if !compact {
                                ui.heading("Contra Karaoke");
                                ui.add_space(12.0);
                            }
                            for tab_def in &TABS {
                                let label = if compact {
                                    tab_def.short_label
                                } else {
                                    tab_def.label
                                };
                                if tab(
                                    ui,
                                    tab_def.is_active(self.screen),
                                    label,
                                    tab_def.hover,
                                    tab_def.shortcut,
                                    animate,
                                    compact,
                                ) {
                                    self.screen = tab_def.landing();
                                }
                            }
                        });
                    });
                });
            });

        if let Some(error) = self.error.clone() {
            egui::Panel::bottom("error").show(ui, |ui| {
                if compact {
                    ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
                }
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(theme::error(ui), error);
                    if ui.button("Dismiss").clicked() {
                        self.error = None;
                    }
                });
            });
        }

        let screen_progress = self.screen_transition(&ctx);
        egui::CentralPanel::default().show(ui, |ui| {
            if self.prefs.animations_enabled {
                // Fade and slide the new screen in. The offset shrinks to zero
                // as the progress approaches one.
                ui.add_space((1.0 - screen_progress) * 12.0);
                ui.set_opacity(0.15 + 0.85 * screen_progress);
            }
            if let Some(def) = SCREENS.iter().find(|def| def.screen == self.screen) {
                (def.draw)(self, ui, beat);
            }
        });

        // Playback settings are applied once the mouse button is released, so
        // dragging a slider does not re-render on every frame. The transport
        // keeps its position and play state across the re-render.
        if self.pending_audio && self.started && !ctx.input(|input| input.pointer.primary_down()) {
            self.apply_audio_settings();
        }

        self.about_ui(&ctx);

        if self.audio.is_playing() || self.render_loading || self.speech.is_busy() {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.speech.stop();
        self.save_prefs();
    }
}

/// Duration of the fade/slide when the active screen changes.
const SCREEN_ANIM_SECONDS: f32 = 0.22;
/// Width below which the UI switches to its phone layout (ROADMAP ┬º8.2).
const COMPACT_WIDTH: f32 = 640.0;
/// Height below which the session tab scrolls as one page so the dance card
/// stays reachable on short phones and landscape windows.
const SESSION_PAGE_SCROLL_HEIGHT: f32 = 800.0;

/// Height below which the session tab scrolls as one page, even at a wide
/// width, so a landscape phone can still reach the dance card.
const SESSION_SHORT_HEIGHT: f32 = 560.0;

/// How the session tab adapts to the viewport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SessionLayout {
    /// Use the stacked phone layout for the controls and summary.
    compact: bool,
    /// Scroll the whole session page instead of only the dance card.
    page_scroll: bool,
}

fn session_layout(width: f32, height: f32) -> SessionLayout {
    let compact = width < COMPACT_WIDTH;
    SessionLayout {
        compact,
        page_scroll: (compact && height < SESSION_PAGE_SCROLL_HEIGHT)
            || height < SESSION_SHORT_HEIGHT,
    }
}

/// Duration of a tab's selected/hover background and text fade.
const TAB_ANIM_SECONDS: f32 = 0.16;
/// Duration used for animating card and result highlights.
const HIGHLIGHT_ANIM_SECONDS: f32 = 0.22;

fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Animate toward `target`, or snap instantly when animations are disabled.
fn animate_amount(
    ctx: &egui::Context,
    id: impl std::hash::Hash + std::fmt::Debug,
    target: bool,
    animate: bool,
) -> f32 {
    if !animate {
        return if target { 1.0 } else { 0.0 };
    }
    ctx.animate_bool_with_time(egui::Id::unique(id), target, HIGHLIGHT_ANIM_SECONDS)
}

fn lerp_color(from: egui::Color32, to: egui::Color32, t: f32) -> egui::Color32 {
    from.lerp_to_gamma(to, t.clamp(0.0, 1.0))
}

/// A top-row tab with an animated selected background and text colour.
///
/// `tab()` is used instead of `selectable_label` so the selected state can
/// fade in and out; egui's selectable label switches instantly. `label` is the
/// visible text and `hover` is the full name plus shortcut, which lets the
/// phone layout shorten the label without losing the screen name.
fn tab(
    ui: &mut egui::Ui,
    active: bool,
    label: &str,
    hover: &str,
    shortcut: &str,
    animate: bool,
    compact: bool,
) -> bool {
    let ctx = ui.ctx().clone();
    let id = ui.make_persistent_id(("tab", hover));
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, egui::Color32::PLACEHOLDER);
    // Narrower tabs leave room for the pinned About button on a phone.
    let horizontal_padding = if compact { 16.0 } else { 24.0 };
    let desired = egui::vec2(
        galley.size().x + horizontal_padding,
        (galley.size().y + 16.0).max(ui.spacing().interact_size.y),
    );
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let response = response.on_hover_text(format!("{hover} ({shortcut})"));

    let active_t = if animate {
        ctx.animate_bool_with_time(id, active, TAB_ANIM_SECONDS)
    } else if active {
        1.0
    } else {
        0.0
    };
    let hover_t = if animate {
        ctx.animate_bool_with_time(id.with("hover"), response.hovered(), TAB_ANIM_SECONDS)
    } else if response.hovered() {
        1.0
    } else {
        0.0
    };

    if ui.is_rect_visible(rect) {
        let visuals = ui.visuals();
        let muted = visuals.widgets.noninteractive.fg_stroke.color;
        let strong = visuals.strong_text_color();
        let text_color = lerp_color(lerp_color(muted, strong, hover_t * 0.6), strong, active_t);
        let fill = lerp_color(
            visuals.panel_fill,
            visuals.widgets.active.weak_bg_fill,
            active_t,
        );
        let fill = if hover_t > 0.0 {
            lerp_color(fill, visuals.widgets.hovered.weak_bg_fill, hover_t * 0.6)
        } else {
            fill
        };
        ui.painter().rect_filled(rect, 8, fill);
        if active_t > 0.0 {
            let stroke = visuals
                .widgets
                .active
                .bg_stroke
                .color
                .gamma_multiply(active_t);
            ui.painter().rect_stroke(
                rect,
                8,
                egui::Stroke::new(1.0, stroke),
                egui::StrokeKind::Inside,
            );
        }
        ui.painter().galley_with_override_text_color(
            rect.center() - galley.size() / 2.0,
            galley,
            text_color,
        );
    }

    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, hover));
    if response.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response.clicked()
}

fn load_prefs(path: &Path) -> Option<Preferences> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|json| Preferences::from_json(&json).ok())
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = path;
        crate::web_store::read_prefs().and_then(|json| Preferences::from_json(&json).ok())
    }
}

fn build_cache(cache_dir: &Path) -> Cache {
    #[cfg(not(target_arch = "wasm32"))]
    {
        FileCache::new(cache_dir.join("sources"))
            .map(Cache::new)
            .unwrap_or_else(|_| Cache::new(contra_sources::MemoryCache::default()))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = cache_dir;
        Cache::new(crate::web_store::LocalStorageCache)
    }
}

/// Parse the current tune and compare its body with the dance card.
///
/// This runs at startup and whenever a dance or tune is selected, so a mismatch
/// is visible before the first Play as well as after the music is rebuilt.
fn length_warning_for(dance: &Dance, tune: &Tune) -> Option<String> {
    let score = parse_abc(&tune.abc).ok()?;
    let arrangement = Arrangement::from_score(&score, 64.0);
    arrangement
        .check_dance_length(dance.total_beats())
        .err()
        .map(format_length_mismatch)
}

fn format_length_mismatch(mismatch: LengthMismatch) -> String {
    format!("Length check: {mismatch}. Calls and music can drift out of sync.")
}

fn build_timeline(
    dance: &Dance,
    prefs: &Preferences,
    alias_seed: u64,
) -> Result<PracticeTimeline, String> {
    PracticeTimeline::build(
        dance,
        &prefs.aliases,
        &prefs.role_terms,
        &prefs.lead,
        prefs.tempo(),
        TimelineConfig {
            count_in_beats: if prefs.count_in_enabled { 3.0 } else { 0.0 },
            music_loop_beats: 64.0,
            shuffle_alternatives: prefs.shuffle_alternatives,
            alias_seed,
        },
    )
    .map_err(|error| format!("Could not build the call timeline: {error}"))
}

fn empty_timeline() -> PracticeTimeline {
    PracticeTimeline {
        calls: Vec::new(),
        phrases: Vec::new(),
        total_beats: 64.0,
        music_loop_beats: 64.0,
        count_in_beats: 3.0,
    }
}

/// The rendered call texts, in timeline order, for the speech pre-synthesizer.
fn call_texts(timeline: &PracticeTimeline) -> Vec<String> {
    timeline
        .calls
        .iter()
        .map(|call| call.text.clone())
        .collect()
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn safe_filename(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ui::{CallPhase, CardMode, CardTone, card_state, card_tone};

    #[test]
    fn bundled_defaults_pass_the_length_check() {
        let dance = crate::defaults::default_dance();
        let tune = crate::defaults::default_tune();
        assert!(length_warning_for(&dance, &tune).is_none());
    }

    #[test]
    fn a_short_dance_produces_a_visible_warning() {
        let mut dance = crate::defaults::default_dance();
        dance.figures[0].beats -= 0.5;
        let tune = crate::defaults::default_tune();
        let warning = length_warning_for(&dance, &tune).expect("length mismatch");
        assert!(warning.contains("63.5"), "{warning}");
        assert!(warning.contains("64.0"), "{warning}");
    }

    /// Every screen must be registered exactly once, or it becomes unreachable
    /// from the tab row or the dispatcher.
    #[test]
    fn every_screen_is_registered_once() {
        const ALL: [Screen; 7] = [
            Screen::Session,
            Screen::DancePicker,
            Screen::DanceDownload,
            Screen::TunePicker,
            Screen::TuneDownload,
            Screen::Preferences,
            Screen::Aliases,
        ];
        assert_eq!(SCREENS.len(), ALL.len());
        for screen in ALL {
            let registered = SCREENS.iter().filter(|def| def.screen == screen).count();
            assert_eq!(registered, 1, "{screen:?} must be registered exactly once");
        }
        for tab in &TABS {
            for screen in tab.screens {
                assert!(ALL.contains(screen), "{screen:?} is not a screen");
            }
            assert_eq!(screen_for_tab_key(tab.key), Some(tab.landing()));
        }
    }

    #[test]
    fn number_keys_select_top_tabs() {
        assert_eq!(screen_for_tab_key(egui::Key::Num1), Some(Screen::Session));
        assert_eq!(
            screen_for_tab_key(egui::Key::Num2),
            Some(Screen::DancePicker)
        );
        assert_eq!(
            screen_for_tab_key(egui::Key::Num3),
            Some(Screen::TunePicker)
        );
        assert_eq!(
            screen_for_tab_key(egui::Key::Num4),
            Some(Screen::Preferences)
        );
        assert_eq!(screen_for_tab_key(egui::Key::Num5), None);
        assert_eq!(screen_for_tab_key(egui::Key::Space), None);
    }

    /// Run one egui frame with `raw_input` and read the tab shortcut it saw.
    fn tab_shortcut_for(raw_input: egui::RawInput) -> Option<Screen> {
        let ctx = egui::Context::default();
        let mut screen = None;
        let mut output = ctx.run_ui(raw_input, |ui| {
            screen = ui.input(tab_shortcut);
        });
        output.textures_delta.clear();
        screen
    }

    fn key_press(key: egui::Key) -> egui::RawInput {
        egui::RawInput {
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn tab_shortcuts_only_fire_for_a_pressed_key() {
        // No input must not select a tab; otherwise the screen snaps back to
        // the session on the next frame and tab clicks appear to do nothing.
        assert_eq!(tab_shortcut_for(egui::RawInput::default()), None);
        assert_eq!(
            tab_shortcut_for(key_press(egui::Key::Num2)),
            Some(Screen::DancePicker)
        );
        assert_eq!(
            tab_shortcut_for(key_press(egui::Key::Num4)),
            Some(Screen::Preferences)
        );
        assert_eq!(tab_shortcut_for(key_press(egui::Key::Num5)), None);
    }

    #[test]
    fn escape_returns_to_the_previous_screen() {
        assert_eq!(Screen::Session.escape_target(), None);
        assert_eq!(Screen::DancePicker.escape_target(), Some(Screen::Session));
        assert_eq!(
            Screen::DanceDownload.escape_target(),
            Some(Screen::DancePicker)
        );
        assert_eq!(Screen::TunePicker.escape_target(), Some(Screen::Session));
        assert_eq!(
            Screen::TuneDownload.escape_target(),
            Some(Screen::TunePicker)
        );
        assert_eq!(Screen::Preferences.escape_target(), Some(Screen::Session));
        assert_eq!(Screen::Aliases.escape_target(), Some(Screen::Preferences));
    }

    #[test]
    fn ease_out_cubic_hits_the_endpoints_and_clamps() {
        assert_eq!(ease_out_cubic(0.0), 0.0);
        assert!((ease_out_cubic(1.0) - 1.0).abs() < 1e-6);
        assert_eq!(ease_out_cubic(-1.0), 0.0);
        assert!((ease_out_cubic(2.0) - 1.0).abs() < 1e-6);
        assert!(ease_out_cubic(0.5) > 0.5);
    }

    #[test]
    fn executing_figure_prompts_the_upcoming_move() {
        assert_eq!(
            card_state(Some(CallPhase::Executing), true),
            CardMode::UpcomingMove
        );
    }

    #[test]
    fn executing_figure_without_a_following_call_stays_on_the_figure() {
        assert_eq!(
            card_state(Some(CallPhase::Executing), false),
            CardMode::FinalMove
        );
    }

    #[test]
    fn calls_that_are_upcoming_or_being_spoken_stay_on_the_call_card() {
        for phase in [Some(CallPhase::Upcoming), Some(CallPhase::Calling), None] {
            assert_eq!(card_state(phase, true), CardMode::CurrentCall);
            assert_eq!(card_state(phase, false), CardMode::CurrentCall);
        }
    }

    #[test]
    fn speaking_now_turns_the_card_green() {
        for has_next in [true, false] {
            let mode = card_state(Some(CallPhase::Calling), has_next);
            assert_eq!(
                card_tone(true, Some(CallPhase::Calling), mode),
                CardTone::CallNow
            );
        }
    }

    #[test]
    fn looking_ahead_turns_the_card_red() {
        let mode = card_state(Some(CallPhase::Executing), true);
        assert_eq!(
            card_tone(true, Some(CallPhase::Executing), mode),
            CardTone::UpcomingMove
        );
    }

    #[test]
    fn other_card_states_stay_grey() {
        for (started, phase) in [
            (false, None),
            (false, Some(CallPhase::Upcoming)),
            (true, None),
            (true, Some(CallPhase::Upcoming)),
            (true, Some(CallPhase::Executing)),
        ] {
            let mode = card_state(phase, false);
            assert_eq!(
                card_tone(started, phase, mode),
                CardTone::Neutral,
                "started={started} phase={phase:?}"
            );
        }
        // The count-in card without a pending call is not a "speak now" cue.
        assert_eq!(
            card_tone(true, None, CardMode::CurrentCall),
            CardTone::Neutral
        );
    }

    #[test]
    fn session_layout_uses_the_phone_stack_below_the_breakpoint() {
        assert_eq!(
            session_layout(390.0, 900.0),
            SessionLayout {
                compact: true,
                page_scroll: false
            }
        );
        assert_eq!(
            session_layout(639.0, 900.0),
            SessionLayout {
                compact: true,
                page_scroll: false
            }
        );
        assert_eq!(
            session_layout(390.0, 768.0),
            SessionLayout {
                compact: true,
                page_scroll: true
            }
        );
        assert_eq!(
            session_layout(640.0, 800.0),
            SessionLayout {
                compact: false,
                page_scroll: false
            }
        );
        assert_eq!(
            session_layout(1280.0, 800.0),
            SessionLayout {
                compact: false,
                page_scroll: false
            }
        );
    }

    #[test]
    fn short_phone_sessions_scroll_as_one_page() {
        let layout = session_layout(390.0, 560.0);
        assert!(layout.compact);
        assert!(layout.page_scroll);
        // A landscape phone keeps the wide control row, but its short height
        // still makes the whole page scroll so the dance card is reachable.
        let landscape = session_layout(844.0, 390.0);
        assert!(!landscape.compact);
        assert!(landscape.page_scroll);
    }

    #[test]
    fn color_lerp_moves_between_endpoints_and_clamps() {
        use egui::Color32;
        assert_eq!(
            lerp_color(Color32::BLACK, Color32::WHITE, 0.0),
            Color32::BLACK
        );
        assert_eq!(
            lerp_color(Color32::BLACK, Color32::WHITE, 1.0),
            Color32::WHITE
        );
        assert_eq!(
            lerp_color(Color32::BLACK, Color32::WHITE, 2.0),
            Color32::WHITE
        );
        let middle = lerp_color(Color32::BLACK, Color32::WHITE, 0.5);
        assert!(middle.r() > 120 && middle.r() < 136);
    }
}

// Each screen lives in its own file under `app/ui/`.
mod ui;

#[cfg(feature = "headless")]
pub mod headless;
