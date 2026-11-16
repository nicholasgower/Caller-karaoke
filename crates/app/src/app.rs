//! Application state and UI.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use contra_core::{
    AccompanimentInstrument, AccompanimentRhythm, AliasTable, AudioEffects, BandSlot,
    COUNT_IN_BEATS, Countermelody, DEFAULT_DANCE_SOURCE, Dance, DancerSoundEvent, EndingOptions,
    FolderFilter, HumanizeOptions, InstrumentCopies, Library, MelodyInstrument, MoveKind,
    OutputMode, Percussion, PercussionRhythm, PracticeTimeline, Preferences, RolePreset,
    ScheduledCall, Theme, TimelineConfig, Tune, VariationOptions,
};
use contra_music::{
    Arrangement, AudioBuffer, LengthMismatch, RenderRequest, export_midi_with_passes, parse_abc,
    render,
};
#[cfg(not(target_arch = "wasm32"))]
use contra_sources::FileCache;
use contra_sources::{
    Cache, DanceProvider, DanceQuery, DanceSearchPage, DanceSort, DanceSummary, FilterKind,
    TuneQuery, TuneSearchPage, TuneSummary, dance_provider,
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
use history::{History, HistoryAction, MergeKey, Snapshot};

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

const TABS: [TabDef; 3] = [
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
];

/// The number key that opens Preferences.
///
/// Preferences is a header button rather than a tab, but the key it had as the
/// fourth tab keeps working so an established shortcut does not disappear.
const PREFERENCES_KEY: egui::Key = egui::Key::Num4;

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

/// Project links shown in the About section of Preferences.
///
/// The pages may not be live yet; the addresses are the ones the documentation
/// and packaging use so they start working as soon as the site is published.
const REPOSITORY_URL: &str = "https://github.com/storytold/Contra-karaoke";
const DOCUMENTATION_URL: &str = "https://storytold.github.io/Contra-karaoke/";

/// Map the number-key shortcuts to screens.
fn screen_for_number_key(key: egui::Key) -> Option<Screen> {
    TABS.iter()
        .find(|tab| tab.key == key)
        .map(|tab| tab.landing())
        .or_else(|| (key == PREFERENCES_KEY).then_some(Screen::Preferences))
}

/// The top-row tab that contains `screen`.
fn tab_index(screen: Screen) -> usize {
    TABS.iter()
        .position(|tab| tab.is_active(screen))
        .unwrap_or(0)
}

/// The landing screen of the tab `step` places away from `current`.
///
/// The tab row wraps in both directions, so Left from Session reaches
/// Preferences and Right from Preferences returns to Session. Detail screens
/// (a download search, the alias editor) step from the tab that contains them.
fn tab_step(current: Screen, step: isize) -> Screen {
    let count = TABS.len() as isize;
    let index = tab_index(current) as isize;
    let next = (index + step).rem_euclid(count) as usize;
    TABS[next].landing()
}

/// How far a horizontal swipe must travel before it changes tabs.
const SWIPE_MIN_DISTANCE: f32 = 60.0;
/// How much more horizontal than vertical a tab swipe must be.
const SWIPE_AXIS_RATIO: f32 = 1.5;

/// The screen a horizontal swipe selects, or `None` when it is not a swipe.
///
/// A swipe left (negative x) moves to the next tab and a swipe right moves to
/// the previous one. A mostly vertical drag is a scroll, not a tab change, and
/// a short drag is a tap or a slider adjustment.
fn swipe_target(current: Screen, dx: f32, dy: f32) -> Option<Screen> {
    if dx.abs() <= SWIPE_MIN_DISTANCE || dx.abs() < dy.abs() * SWIPE_AXIS_RATIO {
        return None;
    }
    Some(tab_step(current, if dx < 0.0 { 1 } else { -1 }))
}

/// The screen selected by a number-key shortcut pressed this frame, if any.
///
/// The key must actually have been pressed: resolving the first tab's
/// shortcut regardless of input would pin the app to the session screen every
/// frame and undo any click on another tab.
fn number_key_shortcut(input: &egui::InputState) -> Option<Screen> {
    let key = TABS
        .iter()
        .map(|tab| tab.key)
        .chain(std::iter::once(PREFERENCES_KEY))
        .find(|key| input.key_pressed(*key))?;
    screen_for_number_key(key)
}

/// The undo or redo shortcut pressed this frame, if any.
///
/// `Ctrl+Z` undoes and `Ctrl+Y` or `Ctrl+Shift+Z` redo; `Modifiers::COMMAND`
/// makes the first chord `Cmd+Z` on macOS. [`egui::InputState::consume_key`]
/// ignores an extra Shift, so the shifted redo chord is checked first: read the
/// other way round, `Ctrl+Shift+Z` would undo.
fn history_shortcut(input: &mut egui::InputState) -> Option<HistoryAction> {
    if !input.modifiers.command {
        return None;
    }
    if input.consume_key(
        egui::Modifiers::COMMAND.plus(egui::Modifiers::SHIFT),
        egui::Key::Z,
    ) {
        return Some(HistoryAction::Redo);
    }
    if input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y) {
        return Some(HistoryAction::Redo);
    }
    input
        .consume_key(egui::Modifiers::COMMAND, egui::Key::Z)
        .then_some(HistoryAction::Undo)
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
    /// Takes the focus when the window first appears, so the app opens in front
    /// of the windows that launched it.
    launch_focus: crate::window::LaunchFocus,
    /// The OS media session, so hardware play/pause keys work while the
    /// window is in the background.
    media_keys: Option<crate::media_keys::MediaKeys>,
    /// Scroll the About section at the bottom of Preferences into view on the
    /// next frame. Set by the headless renderer so a documentation shot can
    /// show the section without rendering the whole scrolling page.
    about_reveal: bool,
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
    /// A custom role-term field has focus, so the timeline rebuild is deferred
    /// until the edit settles.
    role_edit_active: bool,
    /// Custom role terms changed while the field was focused and still need
    /// the deferred rebuild.
    role_edit_pending: bool,
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
    rotation: RotationState,
    /// Bumped whenever a setting changes; prepared buffers from an older
    /// generation are discarded.
    render_generation: u64,
    /// Undo/redo steps over preferences and the selected dance and tune.
    history: History,
    /// An undo or redo asked for during this frame; it is applied after the
    /// frame's own edits have been recorded, so the edit just made is on the
    /// stack when the request arrives.
    pending_history: Option<HistoryAction>,
    /// The result of the last undo or redo, shown under the Interface buttons.
    history_status: Option<String>,
}

/// Transient state of the two download screens.
///
/// The dance and tune pickers have the same shape, so they share one struct: a
/// new search or preview field belongs here instead of being threaded through
/// the app struct and both constructors.
pub(in crate::app) struct PickerState {
    dance_query: DanceQuery,
    tune_query: TuneQuery,
    dance_results: Option<DanceSearchPage>,
    tune_results: Option<TuneSearchPage>,
    dance_preview: Option<Dance>,
    tune_preview: Option<Tune>,
    dance_selected: Option<DanceSummary>,
    tune_selected: Option<TuneSummary>,
    /// Provider that produced [`Self::dance_results`]. Previews load from this
    /// provider, not from whatever is selected now.
    dance_provider: String,
    search_loading: bool,
    preview_loading: bool,
}

impl Default for PickerState {
    fn default() -> Self {
        Self {
            dance_query: DanceQuery::default(),
            tune_query: TuneQuery::default(),
            dance_results: None,
            tune_results: None,
            dance_preview: None,
            tune_preview: None,
            dance_selected: None,
            tune_selected: None,
            dance_provider: DEFAULT_DANCE_SOURCE.to_string(),
            search_loading: false,
            preview_loading: false,
        }
    }
}

/// One tune rendered ahead of a rotation boundary.
struct PreparedTune {
    /// Render generation that produced this buffer; a newer generation makes
    /// it stale.
    generation: u64,
    tune: Tune,
    buffer: AudioBuffer,
    arrangement: Arrangement,
}

/// Bookkeeping for cycling through a set of tunes.
#[derive(Default)]
struct RotationState {
    /// Completed 64-beat passes of the tune now playing.
    passes: u32,
    /// Beat seen on the previous frame, used to detect a pass boundary.
    last_beat: f64,
    /// The next tune, rendered and waiting for the boundary.
    prepared: Option<PreparedTune>,
    /// Render generation of the job in flight, or zero.
    loading_generation: u64,
    /// Native worker receiver. A dropped receiver cancels delivery, not the
    /// render itself.
    rx: Option<Receiver<Result<PreparedTune, String>>>,
    /// A tune whose render failed; skip it until the set or settings change.
    failed_source: Option<String>,
}

/// The values a render needs, captured so a worker can render without the app.
#[derive(Clone)]
struct RenderSettings {
    bpm: f64,
    melody: MelodyInstrument,
    accompaniment: AccompanimentInstrument,
    accompaniment_rhythm: AccompanimentRhythm,
    second_accompaniment: Countermelody,
    copies: InstrumentCopies,
    band: Vec<BandSlot>,
    output: OutputMode,
    variations: VariationOptions,
    humanize_options: HumanizeOptions,
    ending_options: EndingOptions,
    dancer_sounds: bool,
    dancer_sound_volume: f32,
    dancer_sound_events: Vec<DancerSoundEvent>,
    percussion: Percussion,
    percussion_rhythm: PercussionRhythm,
    melody_volume: f32,
    accompaniment_volume: f32,
    percussion_volume: f32,
    effects: AudioEffects,
    count_in_beats: f64,
    sample_rate: u32,
    variation_passes: u32,
    humanize_timing: bool,
}

impl RenderSettings {
    fn request(&self, arrangement: Arrangement) -> RenderRequest {
        RenderRequest {
            arrangement,
            bpm: self.bpm,
            melody: self.melody,
            accompaniment: self.accompaniment,
            accompaniment_rhythm: self.accompaniment_rhythm,
            second_accompaniment: self.second_accompaniment,
            copies: self.copies,
            band: self.band.clone(),
            output: self.output,
            variations: self.variations.clone(),
            humanize_options: self.humanize_options.clone(),
            ending_options: self.ending_options.clone(),
            dancer_sounds: self.dancer_sounds,
            dancer_sound_volume: self.dancer_sound_volume,
            dancer_sound_events: self.dancer_sound_events.clone(),
            percussion: self.percussion,
            percussion_rhythm: self.percussion_rhythm,
            melody_volume: self.melody_volume,
            accompaniment_volume: self.accompaniment_volume,
            percussion_volume: self.percussion_volume,
            effects: self.effects,
            count_in_beats: self.count_in_beats,
            sample_rate: self.sample_rate,
            variation_passes: self.variation_passes,
            ending: true,
            humanize_timing: self.humanize_timing,
        }
    }
}

/// Parse, arrange, and render one tune with captured settings.
fn prepare_tune(
    tune: &Tune,
    settings: &RenderSettings,
    soundfont: Option<Arc<SoundFont>>,
    generation: u64,
) -> Result<PreparedTune, String> {
    let score = parse_abc(&tune.abc).map_err(|error| format!("Could not parse tune: {error}"))?;
    let arrangement = Arrangement::from_score(&score, 64.0);
    let buffer = render(&settings.request(arrangement.clone()), soundfont.as_ref())
        .map_err(|error| format!("Could not render music: {error}"))?;
    Ok(PreparedTune {
        generation,
        tune: tune.clone(),
        buffer,
        arrangement,
    })
}

impl ContraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_with_data_dir(cc, None)
    }

    /// Android entry point: use the app's private files directory for
    /// preferences, libraries, and the source cache. `directories` has no
    /// Android-aware location, and `std::env::temp_dir()` is not writable in
    /// the app sandbox.
    #[cfg(target_os = "android")]
    pub fn new_android(cc: &eframe::CreationContext<'_>, data_dir: Option<PathBuf>) -> Self {
        Self::new_with_data_dir(cc, data_dir)
    }

    fn new_with_data_dir(cc: &eframe::CreationContext<'_>, data_dir: Option<PathBuf>) -> Self {
        let (source_tx, source_rx): (Sender<SourceEvent>, Receiver<SourceEvent>) = channel();
        let (config_dir, cache_dir) = if let Some(base) = data_dir {
            (base.join("config"), base.join("cache"))
        } else {
            let project = directories::ProjectDirs::from("org", "contrakaraoke", "ContraKaraoke");
            if let Some(project) = project {
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
        speech.set_rate(prefs.speech_rate());
        speech.set_voice(&prefs.tts_voice);
        speech.set_tone(prefs.tts_tone);
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
            launch_focus: crate::window::LaunchFocus::default(),
            media_keys: crate::media_keys::MediaKeys::new(cc),
            about_reveal: false,
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
            role_edit_active: false,
            role_edit_pending: false,
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
            rotation: RotationState::default(),
            render_generation: 0,
            history: History::default(),
            pending_history: None,
            history_status: None,
        }
    }

    /// Apply hardware media keys from the OS media session.
    ///
    /// This runs even while a text field has focus and while another window is
    /// in front: a hardware play/pause key is a global transport control, not
    /// an app shortcut.
    fn handle_media_keys(&mut self) {
        let Some(media) = self.media_keys.as_mut() else {
            return;
        };
        let commands = media.poll();
        for command in commands {
            let playing = self.audio.is_playing();
            match (command.desired(playing), playing) {
                (true, false) => self.play(),
                (false, true) => self.pause(),
                _ => {}
            }
        }
    }

    /// Publish the transport state and loaded item to the OS media controls.
    fn sync_media_session(&mut self) {
        let playing = self.audio.is_playing();
        let Some(media) = self.media_keys.as_mut() else {
            return;
        };
        media.set_playback(playing);
        media.set_track(&self.dance.title, &self.tune.title);
    }

    /// Handle app-wide keyboard shortcuts.
    ///
    /// Only called while no widget has keyboard focus, so typing in a text
    /// field never triggers play, restart, or a tab change, and the Left/Right
    /// arrows still adjust a focused slider or list instead of switching tabs.
    fn handle_shortcuts(&mut self, ui: &egui::Ui) {
        // Undo and redo are app shortcuts only while no text field has focus
        // (this method is not called then), so egui's own in-field undo keeps
        // working. An open pop-up owns the keyboard too.
        if !ui.ctx().any_popup_open()
            && let Some(action) = ui.input_mut(history_shortcut)
        {
            self.pending_history = Some(action);
        }

        let (space, restart, escape, tab, previous_tab, next_tab) = ui.input(|input| {
            (
                input.key_pressed(egui::Key::Space),
                input.key_pressed(egui::Key::R),
                input.key_pressed(egui::Key::Escape),
                number_key_shortcut(input),
                input.key_pressed(egui::Key::ArrowLeft),
                input.key_pressed(egui::Key::ArrowRight),
            )
        });

        if let Some(screen) = tab {
            self.screen = screen;
        } else if next_tab {
            self.screen = tab_step(self.screen, 1);
        } else if previous_tab {
            self.screen = tab_step(self.screen, -1);
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

    /// Switch tabs on a horizontal swipe in the screen content.
    ///
    /// The gesture is read from the raw pointer rather than from a widget, so
    /// it works over labels, cards, and buttons without stealing their clicks.
    /// A swipe that starts on a slider is left to the slider, a drag while a
    /// text field has focus is a text selection, and the About window or an
    /// open popup (a combo box, a menu) blocks the gesture. One press can
    /// change tabs once; releasing the pointer arms the next swipe.
    fn tab_swipe(&mut self, ui: &egui::Ui, content: egui::Rect) {
        let fired_id = egui::Id::unique("tab-swipe-fired");
        let gesture = ui.input(|input| {
            let origin = input.pointer.press_origin()?;
            let pointer = input.pointer.interact_pos()?;
            input
                .pointer
                .primary_down()
                .then_some((origin, pointer - origin))
        });
        let Some((origin, delta)) = gesture else {
            ui.data_mut(|data| data.insert_temp(fired_id, false));
            return;
        };
        if self.headless
            || ui.ctx().any_popup_open()
            || ui.ctx().text_edit_focused()
            || !content.contains(origin)
            || ui::press_started_on_slider(ui, origin)
            || ui.data(|data| data.get_temp::<bool>(fired_id).unwrap_or(false))
        {
            return;
        }
        if let Some(screen) = swipe_target(self.screen, delta.x, delta.y) {
            self.screen = screen;
            ui.data_mut(|data| data.insert_temp(fired_id, true));
        }
    }

    /// The dance provider selected in preferences.
    ///
    /// An id this build does not know (a hand-edited file or a preference from
    /// a newer build) falls back to ContraDB instead of breaking the screen.
    fn dance_source(&self) -> &'static dyn DanceProvider {
        dance_provider(&self.prefs.dance_source).unwrap_or(&contra_sources::CONTRADB)
    }

    /// Switch the **Download dance** source and drop results from the old one.
    fn set_dance_source(&mut self, id: &str) {
        if self.prefs.dance_source == id {
            return;
        }
        self.prefs.dance_source = id.to_string();
        self.picker.dance_provider = self.dance_source().id().to_string();
        self.picker.dance_results = None;
        self.picker.dance_preview = None;
        self.picker.dance_selected = None;
        // A reply still in flight belongs to the old source and will be
        // dropped, so it must not leave the new screen spinning.
        self.picker.search_loading = false;
        self.picker.preview_loading = false;
        self.picker.dance_query.page = 1;
        // Formation values and the other source-specific filters are not
        // shared between sites; a stale value would silently match nothing.
        self.picker.dance_query.formation = None;
        self.picker.dance_query.figure_move = None;
        self.picker.dance_query.verified = false;
        self.save_prefs();
    }

    /// Start a dance search from the current filters.
    fn search_dances(&mut self) {
        if self.picker.search_loading {
            return;
        }
        let provider = self.dance_source();
        self.picker.dance_provider = provider.id().to_string();
        if !provider.supports_pagination() {
            // A single-page source cannot honour a page left over from another
            // source.
            self.picker.dance_query.page = 1;
        }
        self.picker.search_loading = true;
        self.source
            .search_dances(provider.id(), self.picker.dance_query.clone());
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

    fn remove_saved_dance(&mut self, provider: &str, id: &str) {
        let Some(entry) = self.dance_library.remove(provider, id) else {
            return;
        };
        self.save_dance_library();
        if self
            .picker
            .dance_preview
            .as_ref()
            .is_some_and(|dance| dance.source.provider == provider && dance.source.id == id)
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
        // A tune rendered ahead for rotation must not be installed after it was
        // removed from the library.
        self.rotation_settings_changed();
    }

    fn move_saved_dance(&mut self, provider: &str, id: &str, folder: Option<String>) {
        let label = folder.clone().unwrap_or_else(|| "Unfiled".into());
        match self.dance_library.set_folder(provider, id, folder) {
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
        speech.set_rate(prefs.speech_rate());
        speech.set_tone(prefs.tts_tone);

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
            launch_focus: crate::window::LaunchFocus::default(),
            media_keys: None,
            about_reveal: false,
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
            role_edit_active: false,
            role_edit_pending: false,
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
            rotation: RotationState::default(),
            render_generation: 0,
            history: History::default(),
            pending_history: None,
            history_status: None,
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

    /// The state an undo step restores.
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            prefs: self.prefs.clone(),
            dance: self.dance.clone(),
            tune: self.tune.clone(),
            alias_seed: self.alias_seed,
        }
    }

    /// The gesture a change made this frame belongs to, if any.
    ///
    /// A focused widget is usually a text field being typed in; a held pointer
    /// is usually a slider being dragged. Either way the whole gesture is one
    /// history step, so undo returns to the value before the field or the
    /// handle was touched instead of walking back a keystroke at a time.
    fn merge_key(ctx: &egui::Context) -> Option<MergeKey> {
        if let Some(id) = ctx.memory(|memory| memory.focused()) {
            return Some(MergeKey::Focus(id));
        }
        ctx.input(|input| input.pointer.any_down())
            .then_some(MergeKey::Pointer)
    }

    /// Record the state from before this frame's edits, if it changed.
    ///
    /// The headless renderer sets state between frames to stage a screenshot;
    /// that is not user input and must not fill the history.
    fn record_history(&mut self, before: Snapshot, ctx: &egui::Context) {
        if self.headless {
            return;
        }
        let now = self.snapshot();
        if self.history.record(before, &now, Self::merge_key(ctx)) {
            // The last undo/redo message is stale once a new edit lands.
            self.history_status = None;
        }
    }

    /// Apply an undo or redo requested by a shortcut or the Interface buttons.
    ///
    /// The request waits until the frame's own changes are recorded, so the
    /// edit the user just made is already on the stack when this runs.
    fn apply_history_request(&mut self, ctx: &egui::Context) {
        let Some(action) = self.pending_history.take() else {
            return;
        };
        let current = self.snapshot();
        let restored = match action {
            HistoryAction::Undo => self.history.undo(current),
            HistoryAction::Redo => self.history.redo(current),
        };
        let message = match (action, restored) {
            (_, None) => match action {
                HistoryAction::Undo => "Nothing to undo".to_string(),
                HistoryAction::Redo => "Nothing to redo".to_string(),
            },
            (HistoryAction::Undo, Some(snapshot)) => {
                self.restore_snapshot(snapshot);
                "Undid the last change".to_string()
            }
            (HistoryAction::Redo, Some(snapshot)) => {
                self.restore_snapshot(snapshot);
                "Redid the change".to_string()
            }
        };
        self.status.clone_from(&message);
        self.history_status = Some(message);
        ctx.request_repaint();
    }

    /// Put a previous state back and rebuild what depends on it.
    ///
    /// A preference change re-applies the subsystems the Preferences screen
    /// drives, keeping the transport where it is. A restored dance or tune takes
    /// the same reset a picker choice does: the old audio and speech stop and
    /// the restored tune is prepared on the next Play.
    fn restore_snapshot(&mut self, snapshot: Snapshot) {
        let changes = snapshot.prefs.changes_since(&self.prefs);
        let dance_changed = snapshot.dance.source != self.dance.source;
        let tune_changed = snapshot.tune.source != self.tune.source;
        let soundfont_changed = snapshot.prefs.soundfont != self.prefs.soundfont;
        let rotation_changed = snapshot.prefs.tune_rotation != self.prefs.tune_rotation;

        self.prefs = snapshot.prefs;
        self.prefs.validate();
        self.dance = snapshot.dance;
        self.tune = snapshot.tune;
        self.alias_seed = snapshot.alias_seed;
        self.error = None;

        // Settings that apply immediately rather than through a rebuild. These
        // mirror the Preferences screen, so undo sounds like choosing the old
        // value again.
        self.audio.set_looping(self.prefs.loop_enabled);
        self.audio.set_volume(self.prefs.master_volume);
        self.source.set_proxy(
            self.prefs.web_proxy_url.clone(),
            self.prefs.web_proxy_token.clone(),
        );
        self.speech.set_rate(self.prefs.speech_rate());
        self.speech.set_voice(&self.prefs.tts_voice);
        self.speech.set_tone(self.prefs.tts_tone);
        self.speech.set_enabled(self.prefs.tts_enabled);
        if !self.prefs.tts_enabled {
            self.speech.stop();
        }
        if soundfont_changed {
            self.reload_soundfont();
        }

        if dance_changed || tune_changed {
            self.picker.dance_preview = None;
            self.picker.dance_selected = None;
            self.picker.tune_preview = None;
            self.picker.tune_selected = None;
            self.tune_sheet = None;
            self.needs_render = true;
            self.pending_audio = false;
            self.arrangement = None;
            self.audio.stop();
            self.speech.stop();
            self.started = false;
            self.rotation.passes = 0;
            self.rotation.last_beat = 0.0;
            self.rotation_settings_changed();
            self.rebuild_timeline();
            self.last_spoken = None;
            self.last_beat = 0.0;
            self.length_warning = length_warning_for(&self.dance, &self.tune);
        } else if changes.any() {
            if changes.timeline {
                self.rebuild_timeline();
                self.last_spoken = None;
            }
            if changes.audio {
                self.needs_render = true;
                self.pending_audio = true;
                // A started session applies the restored playback settings at
                // once and keeps the beat. An unstarted one renders on the next
                // Play, the way a preferences edit behaves.
                if self.started {
                    self.apply_audio_settings();
                }
            }
        }
        if rotation_changed {
            // A prepared next tune was rendered from the old set or interval.
            self.rotation_settings_changed();
        }
        self.save_prefs();
    }

    /// Re-read the SoundFont named by the preferences.
    ///
    /// The app keeps a parsed font rather than a path, so undoing "Use bundled"
    /// or a font change has to load the file again.
    fn reload_soundfont(&mut self) {
        self.soundfont = None;
        self.soundfont_error = None;
        // Web and Android cannot load a font file, so they keep the bundled
        // instruments. A stored path there can only come from a copied
        // preference file.
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let Some(path) = self
                .prefs
                .soundfont
                .as_ref()
                .and_then(|setting| setting.path.clone())
            else {
                return;
            };
            match std::fs::read(&path) {
                Ok(bytes) => match contra_music::soundfont::load_soundfont(&bytes) {
                    Ok(soundfont) => self.soundfont = Some(soundfont),
                    Err(error) => {
                        self.soundfont_error = Some(format!(
                            "Could not load the saved SoundFont; using the bundled instruments: {error}"
                        ));
                    }
                },
                Err(error) => {
                    self.soundfont_error = Some(format!(
                        "Could not read the saved SoundFont; using the bundled instruments: {error}"
                    ));
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

    /// Commit a custom role-term edit that was deferred while typing.
    ///
    /// The text fields save every keystroke, but the timeline (and with it the
    /// speech pre-synthesis) waits until the field loses focus or the user
    /// leaves Preferences, so a long word does not rebuild on each letter. The
    /// session's music is marked pending like every other role and alias
    /// change.
    fn commit_role_terms(&mut self) {
        if !self.role_edit_pending {
            return;
        }
        self.role_edit_pending = false;
        self.role_edit_active = false;
        self.rebuild_timeline();
        self.needs_render = true;
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

    /// The render values for the current preferences.
    fn render_settings(&self, count_in_beats: f64) -> RenderSettings {
        RenderSettings {
            bpm: self.prefs.tempo_bpm,
            melody: self.prefs.melody,
            accompaniment: self.prefs.accompaniment,
            accompaniment_rhythm: self.prefs.accompaniment_rhythm,
            second_accompaniment: self.prefs.second_accompaniment,
            copies: self.prefs.instrument_copies,
            band: self.prefs.band_slots(),
            output: self.prefs.output_mode,
            variations: self.prefs.variation_options.clone(),
            humanize_options: self.prefs.humanize_options.clone(),
            ending_options: self.prefs.ending_options.clone(),
            dancer_sounds: self.prefs.dancer_sounds,
            dancer_sound_volume: self.prefs.dancer_sound_volume,
            dancer_sound_events: Vec::new(),
            percussion: self.prefs.percussion,
            percussion_rhythm: self.prefs.percussion_rhythm,
            melody_volume: self.prefs.melody_volume,
            accompaniment_volume: self.prefs.accompaniment_volume,
            percussion_volume: self.prefs.percussion_volume,
            effects: self.prefs.effects,
            count_in_beats,
            sample_rate: self.audio.sample_rate(),
            variation_passes: if self.prefs.accompaniment_variations {
                contra_music::VARIATION_PASSES
            } else {
                1
            },
            humanize_timing: self.prefs.humanize_timing,
        }
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
        let settings = self.render_settings(if self.prefs.count_in_enabled {
            COUNT_IN_BEATS
        } else {
            0.0
        });
        let soundfont = self.active_soundfont();
        match render(&settings.request(arrangement.clone()), soundfont.as_ref()) {
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
    /// `keep_speech` leaves a call that straddles a tune change audible; a
    /// settings change stops stale speech instead.
    fn install_music(
        &mut self,
        buffer: AudioBuffer,
        arrangement: Arrangement,
        resume_beat: Option<f64>,
        keep_speech: bool,
    ) {
        // Briefly mute the callback around the swap so the audio thread cannot
        // play the new buffer from sample zero before the seek lands.
        let was_playing = self.audio.is_playing();
        if was_playing {
            if keep_speech {
                self.audio.pause_music();
            } else {
                self.audio.pause();
            }
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
        // Any pre-rendered next tune used the old settings.
        self.render_generation = self.render_generation.wrapping_add(1);
        self.rotation.prepared = None;
        self.rotation.rx = None;
        self.rotation.loading_generation = 0;
        self.rotation.failed_source = None;
        let was_playing = self.audio.is_playing();
        let Some((buffer, arrangement)) = self.render_music() else {
            return;
        };
        // The old buffer kept playing while we rendered, so sample it now to
        // avoid rewinding by the render time.
        let resume_beat = self.audio.has_buffer().then(|| self.audio.position_beat());
        self.install_music(buffer, arrangement, resume_beat, false);
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

    /// The saved tunes in the rotation, in play order.
    ///
    /// Entries that are no longer in the saved-tune library are skipped, so a
    /// deleted tune cannot stall the rotation.
    fn rotation_tunes(&self) -> Vec<Tune> {
        self.prefs
            .tune_rotation
            .tunes
            .iter()
            .filter_map(|source| {
                self.tune_library
                    .get(&source.provider, &source.id)
                    .map(|entry| entry.item.clone())
            })
            .collect()
    }

    /// The tune after the one now playing, or `None` when cycling cannot run.
    fn next_rotation_tune(&self) -> Option<Tune> {
        let tunes = self.rotation_tunes();
        if !contra_core::TuneRotation::can_rotate(tunes.len()) {
            return None;
        }
        let current = tunes.iter().position(|tune| {
            tune.source.provider == self.tune.source.provider
                && tune.source.id == self.tune.source.id
        });
        let next = next_rotation_index(current, tunes.len())?;
        Some(tunes[next].clone())
    }

    /// The index of the playing tune in the rotation, if it is a member.
    fn rotation_position(&self) -> Option<usize> {
        self.rotation_tunes().iter().position(|tune| {
            tune.source.provider == self.tune.source.provider
                && tune.source.id == self.tune.source.id
        })
    }

    /// Start rendering the next rotation tune once the current tune reaches its
    /// last pass, leaving a whole pass for the worker to finish.
    fn maybe_prepare_next_tune(&mut self) {
        if !self.prefs.tune_rotation.enabled || !self.prefs.loop_enabled || !self.started {
            return;
        }
        if self.audio.ending_requested() || self.audio.in_ending() {
            return;
        }
        let interval = self.prefs.tune_rotation.passes_per_tune.max(1);
        if self.rotation.passes.saturating_add(1) < interval {
            return;
        }
        if self.rotation.prepared.is_some() {
            return;
        }
        let generation = self.render_generation;
        if self.rotation.loading_generation == generation {
            return;
        }
        let Some(next) = self.next_rotation_tune() else {
            return;
        };
        if self.rotation.failed_source.as_deref() == Some(next.source.id.as_str()) {
            return;
        }
        let settings = self.render_settings(0.0);
        let soundfont = self.active_soundfont();
        self.rotation.loading_generation = generation;

        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, rx) = channel();
            let tune = next.clone();
            std::thread::spawn(move || {
                let _ = tx.send(prepare_tune(&tune, &settings, soundfont, generation));
            });
            self.rotation.rx = Some(rx);
            self.status = format!("Preparing {}", next.title);
        }

        #[cfg(target_arch = "wasm32")]
        {
            // The web build has no render worker. Do the work at the boundary
            // instead of blocking the UI in the middle of a pass.
            let _ = (settings, soundfont);
            self.status = format!("Will prepare {} at the tune change", next.title);
        }
    }

    /// Collect a finished background render.
    fn poll_prepared_tune(&mut self) {
        let Some(rx) = self.rotation.rx.take() else {
            return;
        };
        match rx.try_recv() {
            Ok(result) => {
                self.rotation.loading_generation = 0;
                match result {
                    Ok(prepared) if prepared.generation == self.render_generation => {
                        self.rotation.failed_source = None;
                        self.rotation.prepared = Some(prepared);
                    }
                    Ok(_) => {
                        // Settings changed while the worker rendered.
                    }
                    Err(error) => {
                        self.rotation.failed_source =
                            self.next_rotation_tune().map(|tune| tune.source.id);
                        self.error = Some(format!("Could not prepare the next tune: {error}"));
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.rotation.rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                self.rotation.loading_generation = 0;
            }
        }
    }

    /// Count passes and change tunes at the boundary.
    fn tick_rotation(&mut self, beat: f64) {
        if !self.prefs.tune_rotation.enabled || !self.started {
            self.rotation.last_beat = beat;
            return;
        }
        if self.audio.ending_requested() || self.audio.in_ending() {
            self.rotation.last_beat = beat;
            return;
        }
        let previous = self.rotation.last_beat;
        self.rotation.last_beat = beat;
        if pass_wrapped(previous, beat) {
            self.rotation.passes = self.rotation.passes.saturating_add(1);
            let interval = self.prefs.tune_rotation.passes_per_tune.max(1);
            if self.rotation.passes >= interval {
                self.change_tune_at_boundary();
            }
        }
        self.maybe_prepare_next_tune();
    }

    fn change_tune_at_boundary(&mut self) {
        if !self.prefs.loop_enabled {
            // Without looping the session ends after one pass; cycling needs
            // the repeated 64-beat grid.
            self.rotation.passes = 0;
            return;
        }
        if let Some(prepared) = self.rotation.prepared.take() {
            if prepared.generation == self.render_generation {
                self.switch_to_prepared_tune(prepared);
            } else {
                self.rotation.passes = 0;
            }
            return;
        }

        #[cfg(target_arch = "wasm32")]
        {
            self.switch_to_next_tune_now();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.rotation.loading_generation == 0 {
                // Start a render now; the next boundary is one pass away.
                self.maybe_prepare_next_tune();
            }
            self.status = "Waiting for the next tune…".into();
        }
    }

    /// Render and install the next tune immediately. The web build uses this at
    /// the boundary because it has no render worker.
    #[cfg(target_arch = "wasm32")]
    fn switch_to_next_tune_now(&mut self) {
        if let Some(next) = self.next_rotation_tune() {
            self.rotation.loading_generation = 0;
            let settings = self.render_settings(0.0);
            let generation = self.render_generation;
            let soundfont = self.active_soundfont();
            self.status = format!("Preparing {}", next.title);
            match prepare_tune(&next, &settings, soundfont, generation) {
                Ok(prepared) => self.switch_to_prepared_tune(prepared),
                Err(error) => {
                    self.rotation.failed_source = Some(next.source.id);
                    self.error = Some(format!("Could not prepare the next tune: {error}"));
                    self.rotation.passes = 0;
                }
            }
        } else {
            self.rotation.passes = 0;
        }
    }

    /// Install a pre-rendered tune without interrupting the call timeline.
    fn switch_to_prepared_tune(&mut self, prepared: PreparedTune) {
        let PreparedTune {
            tune,
            buffer,
            arrangement,
            ..
        } = prepared;
        self.length_warning = arrangement
            .check_dance_length(self.dance.total_beats())
            .err()
            .map(format_length_mismatch);
        self.tune = tune;
        self.tune_sheet = None;
        self.prefs.last_tune = Some(self.tune.source.clone());
        self.rotation.passes = 0;
        self.rotation.last_beat = 0.0;
        self.install_music(buffer, arrangement, Some(0.0), true);
        self.last_spoken = None;
        self.last_beat = 0.0;
        self.status = format!("Tune change: {}", self.tune.title);
    }

    /// The tune set or its interval changed: discard work rendered from the
    /// old setup. The current tune keeps playing.
    fn rotation_settings_changed(&mut self) {
        self.render_generation = self.render_generation.wrapping_add(1);
        self.rotation.prepared = None;
        self.rotation.rx = None;
        self.rotation.loading_generation = 0;
        self.rotation.failed_source = None;
        // A set that only just became long enough may carry a large pass
        // count from the time it had nothing to change to. Start its first
        // tune from zero rather than switching immediately.
        if self.rotation.passes >= self.prefs.tune_rotation.passes_per_tune.max(1) {
            self.rotation.passes = 0;
        }
    }

    fn tune_in_rotation(&self, tune: &Tune) -> bool {
        self.prefs.tune_rotation.contains(&tune.source)
    }

    /// Add or remove a tune from the rotation set.
    fn toggle_rotation_tune(&mut self, tune: &Tune) {
        if self.tune_in_rotation(tune) {
            self.prefs
                .tune_rotation
                .remove(&tune.source.provider, &tune.source.id);
            self.status = format!("\"{}\" removed from the tune set.", tune.title);
        } else {
            if !self
                .tune_library
                .contains(&tune.source.provider, &tune.source.id)
            {
                // A searched tune can join the set directly; save it first so
                // the reference resolves on the next run.
                self.download_tune(tune.clone());
            }
            self.prefs.tune_rotation.add(tune.source.clone());
            self.status = format!("\"{}\" added to the tune set.", tune.title);
        }
        self.rotation_settings_changed();
        self.save_prefs();
    }

    /// Ask the transport to take the slow ending, or cancel a pending one.
    fn end_tune(&mut self) {
        if self.audio.in_ending() || self.audio.is_finished() {
            return;
        }
        if self.audio.ending_requested() {
            self.audio.cancel_ending();
            self.status = "Ending cancelled".into();
        } else if self.audio.has_buffer() {
            self.audio.request_ending();
            self.status = if self.prefs.loop_enabled {
                "Ending after this time through".into()
            } else {
                "Ending at the end of the tune".into()
            };
        }
    }

    fn play(&mut self) {
        let from_finished = self.audio.is_finished();
        self.ensure_ready();
        if self.audio.has_buffer() {
            self.audio.play();
            self.started = true;
            self.status = "Playing".into();
            if from_finished {
                // A finished run may have spoken the wrapped first call just
                // before the ending; that call belongs to a pass that never
                // played, so let the new run announce it again.
                self.last_spoken = None;
                self.rotation.passes = 0;
                self.rotation.last_beat = 0.0;
            }
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
        self.rotation.passes = 0;
        self.rotation.last_beat = 0.0;
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
        self.rotation.passes = 0;
        self.rotation.last_beat = 0.0;
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
        self.rotation.passes = 0;
        self.rotation.last_beat = 0.0;
        self.rotation_settings_changed();
        self.save_prefs();
        self.status = "Tune selected".into();
    }

    fn handle_source_events(&mut self) {
        while let Ok(event) = self.source_rx.try_recv() {
            match event {
                SourceEvent::DanceSearch {
                    provider,
                    query,
                    result,
                } => {
                    // A reply for a source the user has switched away from
                    // must not repopulate the screen with another site's
                    // results.
                    if provider != self.picker.dance_provider {
                        continue;
                    }
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
                SourceEvent::Dance { provider, result } => {
                    if provider != self.picker.dance_provider {
                        continue;
                    }
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
        let starting = self.first_call_pending;
        self.first_call_pending = false;
        if !self.prefs.tts_enabled || !self.speech.available() || !self.audio.is_playing() {
            return;
        }
        if let Some(call) = call_to_speak(
            &self.timeline,
            beat,
            self.prefs.loop_enabled,
            self.last_spoken,
            starting,
        ) {
            self.speech.speak(&call.call.text);
            self.last_spoken = Some(call.start_beat);
        }
    }

    fn current_beat(&self) -> f64 {
        if !self.audio.has_buffer() {
            return 0.0;
        }
        if self.audio.in_ending() {
            // The ending leaves the beat grid. Report the loop end so the card
            // stops prompting calls and the progress bar stays full.
            return self.timeline.music_loop_beats;
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
            let _ = export_midi_with_passes;
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
            let variation_passes = if self.prefs.accompaniment_variations {
                contra_music::VARIATION_PASSES
            } else {
                1
            };
            let bytes = export_midi_with_passes(
                &arrangement,
                self.prefs.melody,
                self.prefs.accompaniment,
                self.prefs.accompaniment_rhythm,
                self.prefs.second_accompaniment,
                self.prefs.percussion,
                self.prefs.percussion_rhythm,
                self.prefs.tempo_bpm,
                variation_passes,
                self.prefs.effects,
                self.prefs.instrument_copies,
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

impl ContraApp {
    /// One frame of session work: everything playback needs that draws nothing.
    ///
    /// `eframe` calls [`eframe::App::logic`] but not [`eframe::App::ui`] while
    /// the window is minimized, hidden, or fully occluded, and the audio thread
    /// keeps playing either way. The caller, the tune rotation, the media keys,
    /// the background render, and the queued playback settings therefore live
    /// here, so that minimizing the window cannot change what is heard.
    ///
    /// `logic` runs immediately before every visible frame, so this is the only
    /// place the session advances: the UI path must not call it again, or a
    /// visible frame would speak the same call twice.
    fn tick(&mut self, ctx: &egui::Context) {
        self.handle_source_events();
        self.handle_media_keys();

        // The audio position is the only clock: speech is triggered from the
        // rendered timeline, never from wall-clock time.
        let beat = self.current_beat();
        self.update_speech(beat);
        self.poll_prepared_tune();
        self.tick_rotation(beat);

        // Playback settings apply once the control is released, so dragging a
        // slider does not re-render on every frame. A hidden window has no
        // pointer input, so anything pending applies there even if the last
        // drawn frame still reported a held button.
        let dragging = ctx.input(|input| {
            input.pointer.primary_down() && input.viewport().visible() != Some(false)
        });
        if self.pending_audio && self.started && !dragging {
            self.apply_audio_settings();
        }

        self.sync_media_session();

        // Keep the event loop waking while there is something to advance. A
        // hidden window is throttled by eframe to one pass per 100 ms, but each
        // pass still reads the audio clock, so the caller stays on the beat.
        if self.audio.is_playing() || self.render_loading || self.speech.is_busy() {
            ctx.request_repaint_after(PLAYBACK_TICK);
        }
    }

    fn ui_contents(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.headless {
            self.launch_focus.update(&ctx);
        }
        if !self.headless && self.scale_guard.update(&ctx) {
            // The sheet music texture is rasterized at the density it was
            // rendered for and would be blurry on the new monitor.
            self.tune_sheet = None;
        }
        theme::apply(&ctx, self.prefs.theme, self.prefs.animations_enabled);

        // The sliders redraw themselves every frame; this list lets a swipe
        // tell content from a slider that already owns its drag.
        ui::begin_slider_frame(ui);

        // Captured before any shortcut or widget runs, so the edit a frame makes
        // is compared against the state that came before it.
        let history_before = self.snapshot();

        if !self.headless && !ctx.egui_wants_keyboard_input() {
            self.handle_shortcuts(ui);
        }

        let beat = self.current_beat();

        let compact = compact_layout(ui.available_width());
        let header = header_ui(ui, self.screen, self.prefs.animations_enabled, compact);
        if let Some(screen) = header.screen {
            self.screen = screen;
        }
        if header.preferences {
            self.screen = Screen::Preferences;
        }

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
        let mut content_rect = egui::Rect::NOTHING;
        egui::CentralPanel::default().show(ui, |ui| {
            content_rect = ui.max_rect();
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
        // Swipe navigation is read after the screen draws so the slider list
        // for this frame is complete.
        self.tab_swipe(ui, content_rect);

        // Playback settings are applied by `tick`, which runs before this ui on
        // every visible frame and also while the window is hidden.

        // A tab click can leave Preferences while a custom role field still
        // has focus; commit the deferred rebuild here so the session never
        // keeps the previous calls.
        if self.screen != Screen::Preferences {
            self.commit_role_terms();
        }

        // One frame of edits is one history step (or one merged gesture), and
        // an undo asked for by a shortcut or a button applies after the edits
        // are on the stack.
        self.record_history(history_before, &ctx);
        self.apply_history_request(&ctx);
    }

    fn shutdown(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.speech.stop();
        self.save_prefs();
    }
}

impl eframe::App for ContraApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        #[cfg(target_os = "android")]
        {
            let [left, top, right, bottom] = crate::android_insets::system_bar_insets();
            let pixels_per_point = ctx.pixels_per_point().max(1.0);
            let viewport = raw_input
                .screen_rect
                .or(raw_input.viewport().inner_rect)
                .unwrap_or_else(|| ctx.viewport_rect());
            let screen = egui::vec2(
                viewport.width() * pixels_per_point,
                viewport.height() * pixels_per_point,
            );
            raw_input.safe_area_insets = Some(android_safe_area(
                left,
                top,
                right,
                bottom,
                screen,
                pixels_per_point,
            ));
        }

        #[cfg(not(target_os = "android"))]
        {
            let _ = (ctx, raw_input);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // Android draws edge-to-edge. Keep the whole UI inside the window's
        // content rectangle instead of letting the status and navigation bars
        // cover the tab row and the bottom of the card. egui gets the same
        // insets through `raw_input_hook`, so popups and hit-testing agree.
        #[cfg(target_os = "android")]
        {
            let content = ui.ctx().content_rect();
            ui.scope_builder(egui::UiBuilder::new().max_rect(content), |ui| {
                self.ui_contents(ui, frame);
            });
        }
        #[cfg(not(target_os = "android"))]
        {
            self.ui_contents(ui, frame);
        }
    }

    /// Session work that must also run while the window is minimized or hidden.
    ///
    /// eframe skips [`Self::ui`] for a hidden window, so the transport, the
    /// caller, and the tune rotation are advanced here. Nothing is drawn, and
    /// `ui` does not repeat any of it.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick(ctx);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        self.shutdown(gl);
    }
}

/// Convert Android's system-bar insets from physical pixels to egui points.
#[cfg(any(target_os = "android", test))]
fn android_safe_area(
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    screen: egui::Vec2,
    pixels_per_point: f32,
) -> egui::SafeAreaInsets {
    use egui::epaint::MarginF32;

    if screen.x <= 0.0 || screen.y <= 0.0 {
        return egui::SafeAreaInsets::default();
    }

    let pixels_per_point = pixels_per_point.max(1.0);
    let width = screen.x / pixels_per_point;
    let height = screen.y / pixels_per_point;
    egui::SafeAreaInsets(MarginF32 {
        left: (left.max(0) as f32 / pixels_per_point).min(width),
        top: (top.max(0) as f32 / pixels_per_point).min(height),
        right: (right.max(0) as f32 / pixels_per_point).min(width),
        bottom: (bottom.max(0) as f32 / pixels_per_point).min(height),
    })
}

/// Duration of the fade/slide when the active screen changes.
const SCREEN_ANIM_SECONDS: f32 = 0.22;
/// How often the session asks for another frame while the transport runs.
///
/// eframe clamps a hidden window to one pass per 100 ms, so a minimized app
/// advances at that rate; each pass reads the audio clock, so speech stays in
/// time with the music rather than with the frame rate.
const PLAYBACK_TICK: Duration = Duration::from_millis(16);
/// Width below which the UI switches to its phone layout (ROADMAP §8.2).
const COMPACT_WIDTH: f32 = 640.0;
/// Height below which the session tab scrolls as one page so the dance card
/// stays reachable on short phones and landscape windows.
const SESSION_PAGE_SCROLL_HEIGHT: f32 = 800.0;

/// The one width check shared by the tab row, the session layout, and every
/// screen's form and column layout.
fn compact_layout(width: f32) -> bool {
    width < COMPACT_WIDTH
}

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
    let compact = compact_layout(width);
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

/// What the header asked for this frame.
#[derive(Clone, Debug)]
struct HeaderResponse {
    /// The screen a tab click selected.
    screen: Option<Screen>,
    /// The Preferences gear was pressed.
    preferences: bool,
    /// The rect the header panel occupies, including its margins.
    rect: egui::Rect,
    /// The rect the wordmark occupies, if it was drawn.
    ///
    /// Kept for the resize regression test, which compares the header against a
    /// fresh launch at the same size; production reads nothing but `rect`.
    #[cfg_attr(not(test), allow(dead_code))]
    heading: Option<egui::Rect>,
    /// One rect per tab, in `TABS` order. Test-only, because collecting them
    /// would allocate every frame.
    #[cfg(test)]
    tabs: Vec<egui::Rect>,
}

impl Default for HeaderResponse {
    fn default() -> Self {
        Self {
            screen: None,
            preferences: false,
            rect: egui::Rect::NOTHING,
            heading: None,
            #[cfg(test)]
            tabs: Vec::new(),
        }
    }
}

/// Draw the tab row: the pinned About button, the wordmark, and the tabs.
///
/// The header is a top panel sized by its contents, and egui starts that
/// content-driven layout from the panel's rect on the *previous* frame.
/// Content-driven sizing therefore only works while the contents report their
/// natural height, so every row here is aligned to its **top** edge: a row
/// aligned to `Align::Center` stretches to the height it is offered (egui
/// expands a ui to the frame a centered widget was laid out in), the panel then
/// stores that stretched height, and the next frame offers it again. One narrow
/// window - where the tabs wrap onto a second row - was enough to leave the
/// header permanently taller, even after the window was widened again.
///
/// Wrapping itself is unchanged: at intermediate widths the wordmark and tabs
/// still flow onto a second row, and the panel grows to fit both rows.
fn header_ui(ui: &mut egui::Ui, active: Screen, animate: bool, compact: bool) -> HeaderResponse {
    let mut header = HeaderResponse::default();
    let panel = egui::Panel::top("tabs")
        .frame(
            egui::Frame::default()
                // The panel paints nothing by default, which would show
                // the window's near-black clear colour behind the wordmark.
                .fill(theme::header_bg(ui))
                .inner_margin(egui::Margin::symmetric(12, 8)),
        )
        .show(ui, |ui| {
            if compact {
                // Keep the phone header at the 44 pt touch minimum and narrow
                // the About button so four short tabs fit beside it. This is
                // the panel's own ui, so the tweak ends with the header.
                ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
                ui.spacing_mut().button_padding.x = 8.0;
            }
            // The Preferences gear is pinned to the top-right corner and the
            // wordmark and tabs share the remaining width.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                if preferences_gear(ui, active).clicked() {
                    header.preferences = true;
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        // At phone width the four tabs, not the wordmark, are
                        // the useful part of the header. Dropping the heading
                        // and the long tab labels keeps the navigation to one
                        // row and leaves that height for the prompt and card.
                        if !compact {
                            let heading = ui.heading("Contra Karaoke");
                            header.heading = Some(heading.rect);
                            ui.add_space(12.0);
                        }
                        for tab_def in &TABS {
                            let label = if compact {
                                tab_def.short_label
                            } else {
                                tab_def.label
                            };
                            let response = tab(
                                ui,
                                tab_def.is_active(active),
                                label,
                                tab_def.hover,
                                tab_def.shortcut,
                                animate,
                                compact,
                            );
                            if response.clicked {
                                header.screen = Some(tab_def.landing());
                            }
                            #[cfg(test)]
                            header.tabs.push(response.rect);
                        }
                    });
                });
            });
        });
    header.rect = panel.response.rect;
    header
}

/// A tab's click state and the rect it was drawn in.
#[derive(Clone, Copy, Debug)]
struct TabResponse {
    clicked: bool,
    /// Only the resize regression test reads the rect; it checks that a round
    /// trip through a narrow window leaves every tab where a fresh launch at the
    /// same size puts it.
    #[cfg(test)]
    rect: egui::Rect,
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
) -> TabResponse {
    let ctx = ui.ctx().clone();
    let id = ui.make_persistent_id(("tab", hover));
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, egui::Color32::PLACEHOLDER);
    // Narrower tabs leave room for the pinned Preferences gear on a phone.
    let horizontal_padding = if compact { 16.0 } else { 24.0 };
    let desired = egui::vec2(
        galley.size().x + horizontal_padding,
        (galley.size().y + 16.0).max(ui.spacing().interact_size.y),
    );
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let response = response.on_hover_text(format!(
        "{hover} ({shortcut}) · swipe or use ← → to change tabs"
    ));

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
    TabResponse {
        clicked: response.clicked(),
        #[cfg(test)]
        rect,
    }
}

/// The Preferences gear pinned to the top-right corner of the header.
///
/// The accessibility node is named "Preferences" rather than carrying the
/// icon: a screen reader would otherwise announce only a shape, and the MCP
/// tools find the button by its accessible name. A click opens Preferences
/// from any screen, which is why this also owns the screen change rather than
/// returning a bare flag.
///
/// The button is one row high and square: 40 points on the desktop, and the
/// full 44-point touch target because the phone header raises
/// `interact_size.y`.
fn preferences_gear(ui: &mut egui::Ui, active: Screen) -> egui::Response {
    // Square and one row high: 40 points on the desktop, and the full
    // 44-point touch target in the phone header, which raises
    // `interact_size.y`.
    let side = ui.spacing().interact_size.y;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::click());
    let response = response.on_hover_text("Preferences (4)");
    // The gear has no tab that stays lit, so it shows the selected style for the
    // screen it owns: otherwise the header looks like nothing is current while
    // Preferences (or the alias editor) is in front. The same flag reaches the
    // accessibility node, where the MCP session tools look for the active tab.
    let showing = matches!(active, Screen::Preferences | Screen::Aliases);
    if ui.is_rect_visible(rect) {
        // Painted by hand rather than by `egui::Button` so the selected fill
        // matches the tab row and the accessibility node can carry the
        // "Preferences" name instead of an empty text label.
        let mut visuals = *ui.style().interact(&response);
        if showing {
            visuals.weak_bg_fill = ui.visuals().selection.bg_fill;
            visuals.bg_stroke = ui.visuals().selection.stroke;
        }
        ui.painter().rect_filled(rect, 8, visuals.weak_bg_fill);
        ui.painter()
            .rect_stroke(rect, 8, visuals.bg_stroke, egui::StrokeKind::Inside);
        // A glyph would not do: the bundled monochrome emoji font has no
        // U+2699 GEAR, so a "⚙" label renders as an empty box everywhere.
        paint_gear(ui, rect, visuals.fg_stroke.color);
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::Role::Button, ui.is_enabled(), showing, "Preferences")
    });
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// Number of teeth on the painted gear.
const GEAR_TEETH: usize = 8;

/// The shapes of a gear icon centered in `rect`.
///
/// A stroked ring plus short radial teeth reads as a cog at any size and needs
/// no glyph, which keeps the header identical on desktop, web, and Android.
/// Every point stays inside `rect`, so the icon never paints over the header.
fn gear_shapes(rect: egui::Rect, color: egui::Color32) -> Vec<egui::Shape> {
    let center = rect.center();
    // The teeth reach about 70% of the button, leaving the padding a text
    // label would have had.
    let icon = rect.width().min(rect.height()) * 0.36;
    let mut shapes = vec![egui::Shape::circle_stroke(
        center,
        icon * 0.56,
        egui::Stroke::new(icon * 0.30, color),
    )];
    for tooth in 0..GEAR_TEETH {
        let angle = std::f32::consts::TAU * tooth as f32 / GEAR_TEETH as f32;
        let (sin, cos) = angle.sin_cos();
        let inner = icon * 0.60;
        let outer = icon * 0.98;
        shapes.push(egui::Shape::line_segment(
            [
                center + egui::vec2(cos, sin) * inner,
                center + egui::vec2(cos, sin) * outer,
            ],
            egui::Stroke::new(icon * 0.24, color),
        ));
    }
    shapes
}

fn paint_gear(ui: &egui::Ui, rect: egui::Rect, color: egui::Color32) {
    ui.painter().extend(gear_shapes(rect, color));
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

/// Index of the next tune in a rotation of `len` entries.
///
/// A tune outside the set leads to its first entry. Fewer than two tunes cannot
/// rotate, so there is no next tune.
fn next_rotation_index(position: Option<usize>, len: usize) -> Option<usize> {
    (len >= 2).then(|| position.map_or(0, |index| (index + 1) % len))
}

/// Whether the transport crossed from the end of one 64-beat pass into the
/// next between two UI frames.
///
/// The quarter-beat margin absorbs frame jitter and the rounded sample-to-beat
/// conversion; a deliberate restart resets the stored beat instead of looking
/// like a wrap.
fn pass_wrapped(previous: f64, beat: f64) -> bool {
    beat + 0.25 < previous && previous > 0.5
}

/// The call the caller must speak at `beat`, or `None` while it must stay quiet.
///
/// The decision is a pure function of the audio position and of what has already
/// been spoken, so it behaves the same whether the frame that runs it is drawn
/// or not: a hidden window and a visible one speak the same calls at the same
/// beats. `first_call_pending` is set when a run starts at the top of the dance,
/// where the first call's lead may already have begun.
fn call_to_speak(
    timeline: &PracticeTimeline,
    beat: f64,
    looping: bool,
    last_spoken: Option<f64>,
    first_call_pending: bool,
) -> Option<ScheduledCall> {
    if first_call_pending
        && let Some(call) = timeline.starting_call(beat)
        && last_spoken != Some(call.start_beat)
    {
        return Some(call);
    }
    let call = timeline.scheduled_call_at(beat, looping)?;
    (call.is_calling(beat) && last_spoken != Some(call.start_beat)).then_some(call)
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
            count_in_beats: if prefs.count_in_enabled {
                COUNT_IN_BEATS
            } else {
                0.0
            },
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
        count_in_beats: COUNT_IN_BEATS,
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
    fn android_safe_area_removes_system_bars_in_points() {
        // Portrait on the test device: 1080x2400 at 2.5x, with the status bar
        // occupying the top 110 px and the navigation bar the bottom 120 px.
        let insets = android_safe_area(0, 110, 0, 120, egui::vec2(1080.0, 2400.0), 2.5);
        assert_eq!(insets.0.left, 0.0);
        assert_eq!(insets.0.top, 44.0);
        assert_eq!(insets.0.right, 0.0);
        assert_eq!(insets.0.bottom, 48.0);

        // Landscape moves the bars to the sides.
        let insets = android_safe_area(120, 0, 110, 0, egui::vec2(2400.0, 1080.0), 2.5);
        assert_eq!(insets.0.left, 48.0);
        assert_eq!(insets.0.top, 0.0);
        assert_eq!(insets.0.right, 44.0);
        assert_eq!(insets.0.bottom, 0.0);
    }

    #[test]
    fn android_safe_area_ignores_zero_insets() {
        assert_eq!(
            android_safe_area(0, 0, 0, 0, egui::vec2(1080.0, 2400.0), 2.5),
            egui::SafeAreaInsets::default()
        );
    }

    #[test]
    fn bundled_defaults_pass_the_length_check() {
        let dance = crate::defaults::default_dance();
        let tune = crate::defaults::default_tune();
        assert!(length_warning_for(&dance, &tune).is_none());
    }

    #[test]
    fn custom_role_terms_reach_the_rendered_calls() {
        let dance = crate::defaults::default_dance();
        let mut prefs = Preferences::default();
        prefs.select_role_preset(RolePreset::Custom);
        prefs.role_terms = contra_core::RoleTerms {
            left_singular: "port".into(),
            left_plural: "ports".into(),
            right_singular: "starboard".into(),
            right_plural: "starboards".into(),
        };
        let timeline = build_timeline(&dance, &prefs, 0).expect("timeline");
        assert!(
            timeline
                .calls
                .iter()
                .any(|call| call.text.contains("ports allemande")),
            "{:?}",
            call_texts(&timeline)
        );
        assert!(
            !timeline
                .calls
                .iter()
                .any(|call| call.text.contains("larks")),
            "{:?}",
            call_texts(&timeline)
        );
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
    /// from the tab row, the gear button, or the dispatcher.
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
            assert_eq!(screen_for_number_key(tab.key), Some(tab.landing()));
        }
    }

    /// Preferences moved to a header button, so the top row must stay at three
    /// tabs: a fourth one would put it back where it does not belong and leave
    /// the gear pointing at a screen that is already on screen.
    #[test]
    fn preferences_is_a_header_button_not_a_top_tab() {
        assert_eq!(TABS.len(), 3, "the top row is Session, dance, and tune");
        for tab in &TABS {
            assert!(
                !tab.is_active(Screen::Preferences),
                "{} must not cover Preferences",
                tab.label
            );
        }
        assert_eq!(
            screen_for_number_key(PREFERENCES_KEY),
            Some(Screen::Preferences)
        );
    }

    #[test]
    fn number_keys_select_top_tabs() {
        assert_eq!(
            screen_for_number_key(egui::Key::Num1),
            Some(Screen::Session)
        );
        assert_eq!(
            screen_for_number_key(egui::Key::Num2),
            Some(Screen::DancePicker)
        );
        assert_eq!(
            screen_for_number_key(egui::Key::Num3),
            Some(Screen::TunePicker)
        );
        assert_eq!(screen_for_number_key(egui::Key::Num5), None);
        assert_eq!(screen_for_number_key(egui::Key::Space), None);
    }

    #[test]
    fn arrow_keys_step_through_tabs_and_wrap() {
        assert_eq!(tab_step(Screen::Session, 1), Screen::DancePicker);
        assert_eq!(tab_step(Screen::DancePicker, 1), Screen::TunePicker);
        assert_eq!(tab_step(Screen::TunePicker, 1), Screen::Session);
        assert_eq!(tab_step(Screen::Session, -1), Screen::TunePicker);
        assert_eq!(tab_step(Screen::TunePicker, -1), Screen::DancePicker);
        // A detail screen steps from the tab that contains it.
        assert_eq!(tab_step(Screen::DanceDownload, 1), Screen::TunePicker);
        assert_eq!(tab_step(Screen::DanceDownload, -1), Screen::Session);
        // The alias editor is reached from the Preferences gear, not a tab;
        // stepping from it starts at the first tab.
        assert_eq!(tab_step(Screen::Aliases, 1), Screen::DancePicker);
    }

    #[test]
    fn a_horizontal_swipe_selects_an_adjacent_tab() {
        // Swipe left goes forward, swipe right goes back, and both wrap.
        assert_eq!(
            swipe_target(Screen::Session, -100.0, 10.0),
            Some(Screen::DancePicker)
        );
        assert_eq!(
            swipe_target(Screen::Session, 100.0, 10.0),
            Some(Screen::TunePicker)
        );
        assert_eq!(
            swipe_target(Screen::TunePicker, -100.0, 10.0),
            Some(Screen::Session)
        );
        assert_eq!(
            swipe_target(Screen::TunePicker, 100.0, -10.0),
            Some(Screen::DancePicker)
        );
        // A short drag is a tap or slider adjustment, not a swipe.
        assert_eq!(swipe_target(Screen::Session, 60.0, 0.0), None);
        assert_eq!(swipe_target(Screen::Session, 59.0, 0.0), None);
        // A mostly vertical drag is a scroll.
        assert_eq!(swipe_target(Screen::Session, 80.0, 60.0), None);
    }

    /// Run one egui frame with `raw_input` and read the number-key shortcut it
    /// saw.
    fn number_shortcut_for(raw_input: egui::RawInput) -> Option<Screen> {
        let ctx = egui::Context::default();
        let mut screen = None;
        let mut output = ctx.run_ui(raw_input, |ui| {
            screen = ui.input(number_key_shortcut);
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
    fn number_shortcuts_only_fire_for_a_pressed_key() {
        // No input must not select a tab; otherwise the screen snaps back to
        // the session on the next frame and tab clicks appear to do nothing.
        assert_eq!(number_shortcut_for(egui::RawInput::default()), None);
        assert_eq!(
            number_shortcut_for(key_press(egui::Key::Num2)),
            Some(Screen::DancePicker)
        );
        assert_eq!(
            number_shortcut_for(key_press(egui::Key::Num4)),
            Some(Screen::Preferences)
        );
        assert_eq!(number_shortcut_for(key_press(egui::Key::Num5)), None);
    }

    /// Raw input with a fixed viewport, so a simulated click lands on the
    /// widget the layout put there.
    fn ui_input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(800.0, 600.0),
            )),
            events,
            ..Default::default()
        }
    }

    fn run_header_frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        screen: &mut Screen,
        seen: &mut Option<egui::Response>,
    ) {
        let mut output = ctx.run_ui(ui_input(events), |ui| {
            let response = preferences_gear(ui, *screen);
            if response.clicked() {
                *screen = Screen::Preferences;
            }
            *seen = Some(response);
        });
        output.textures_delta.clear();
    }

    /// The gear is the only way to Preferences from another screen, so a real
    /// pointer click on it has to select that screen.
    #[test]
    fn the_gear_button_opens_preferences() {
        let ctx = egui::Context::default();
        let mut screen = Screen::Session;
        let mut seen = None;
        run_header_frame(&ctx, Vec::new(), &mut screen, &mut seen);
        let rect = seen.as_ref().expect("the header gear").rect;
        assert_eq!(
            screen,
            Screen::Session,
            "laying the button out must not open Preferences"
        );

        // Move onto the button, press, release: one click, exactly as a person
        // using the top-right corner of the header.
        let pos = rect.center();
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        for events in [
            vec![egui::Event::PointerMoved(pos)],
            vec![button(true)],
            vec![button(false)],
        ] {
            run_header_frame(&ctx, events, &mut screen, &mut seen);
        }
        assert_eq!(screen, Screen::Preferences);
    }

    /// On a phone the gear is the only header control that is not a tab, so it
    /// has to carry the 44-point touch target by itself.
    #[test]
    fn the_gear_is_a_touch_target_at_phone_size() {
        let ctx = egui::Context::default();
        let screen = Screen::Session;
        let mut size = egui::Vec2::ZERO;
        let mut output = ctx.run_ui(ui_input(Vec::new()), |ui| {
            // The compact header raises the row height before the gear draws.
            ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
            size = preferences_gear(ui, screen).rect.size();
        });
        output.textures_delta.clear();
        assert!(
            size.x >= theme::TOUCH_TARGET && size.y >= theme::TOUCH_TARGET,
            "gear is {size:?}, want at least {} points",
            theme::TOUCH_TARGET
        );
    }

    /// The gear is painted because the bundled fonts have no gear glyph; the
    /// painted icon must still fit inside the button and look like a cog.
    #[test]
    fn the_painted_gear_fits_inside_its_button() {
        for side in [theme::TOUCH_TARGET, 40.0] {
            let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(side, side));
            let shapes = gear_shapes(rect, egui::Color32::WHITE);
            assert_eq!(
                shapes.len(),
                GEAR_TEETH + 1,
                "a ring and one shape per tooth"
            );
            for shape in &shapes {
                let bounds = shape.visual_bounding_rect();
                assert!(
                    rect.contains_rect(bounds),
                    "{bounds:?} escapes a {side}-point button"
                );
            }
        }
    }

    /// About is a section at the bottom of Preferences, not a window over every
    /// screen. It is drawn only from this list, so a regression that removes it
    /// makes the version, author, and project links unreachable; a regression
    /// that moves it buries a setting below the credits.
    #[test]
    fn about_is_the_last_preferences_section() {
        use crate::app::ui::preferences::{ABOUT_SECTION, SECTIONS};
        let last = SECTIONS.last().copied().expect("preferences sections");
        assert!(
            std::ptr::fn_addr_eq(last, ABOUT_SECTION),
            "About must be the last Preferences section"
        );
        let occurrences = SECTIONS
            .iter()
            .filter(|section| std::ptr::fn_addr_eq(**section, ABOUT_SECTION))
            .count();
        assert_eq!(occurrences, 1, "About must be drawn exactly once");
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
    fn every_screen_uses_the_same_compact_breakpoint() {
        // The tab row, session layout, and screen forms must agree on the
        // breakpoint, or a screen would stack while the tabs still show their
        // long labels (or the reverse).
        assert!(compact_layout(360.0));
        assert!(compact_layout(390.0));
        assert!(compact_layout(639.0));
        assert!(!compact_layout(640.0));
        assert!(!compact_layout(1280.0));
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

    #[test]
    fn tune_rotation_advances_in_order_and_wraps() {
        assert_eq!(next_rotation_index(None, 0), None);
        assert_eq!(next_rotation_index(None, 1), None);
        assert_eq!(next_rotation_index(Some(0), 1), None);
        assert_eq!(next_rotation_index(None, 3), Some(0));
        assert_eq!(next_rotation_index(Some(0), 3), Some(1));
        assert_eq!(next_rotation_index(Some(1), 3), Some(2));
        assert_eq!(next_rotation_index(Some(2), 3), Some(0));
    }

    #[test]
    fn only_a_real_loop_wrap_starts_a_new_pass() {
        // A normal wrap from the end of a pass into the count of the next.
        assert!(pass_wrapped(63.99, 0.02));
        // Ordinary forward motion inside a pass.
        assert!(!pass_wrapped(10.0, 10.05));
        // Starting a run at the count-in is not a wrap.
        assert!(!pass_wrapped(0.0, -2.9));
        // Pausing and resuming keeps the same beat.
        assert!(!pass_wrapped(32.0, 32.0));
        // A deliberate restart resets the stored beat, so it cannot look like
        // a wrap either.
        assert!(!pass_wrapped(0.0, 0.0));
    }

    /// The default timeline, with the bundle's dance and preferences.
    fn default_timeline() -> PracticeTimeline {
        build_timeline(
            &crate::defaults::default_dance(),
            &Preferences::default(),
            0,
        )
        .expect("the bundled dance builds a timeline")
    }

    /// Drive the caller from `from` for `beats` beats the way a hidden window
    /// does: a steady audio position, advancing `step` beats per pass, and no
    /// UI at all. Returns the `(call index, wrapped)` triggers in order.
    ///
    /// This mirrors `update_speech`: the audio position is wrapped at the loop
    /// boundary, the trigger is remembered, and the wrap forgets it so the next
    /// pass can announce its first call.
    fn drive(timeline: &PracticeTimeline, from: f64, beats: f64, step: f64) -> Vec<(usize, bool)> {
        let loop_beats = timeline.music_loop_beats;
        let mut triggers = Vec::new();
        let mut last_spoken = None;
        let mut last_beat: Option<f64> = None;
        let mut beat = from;
        while beat < from + beats {
            let loop_beat = if beat < 0.0 { beat } else { beat % loop_beats };
            if last_beat.is_some_and(|last| pass_wrapped(last, loop_beat)) {
                last_spoken = None;
            }
            last_beat = Some(loop_beat);
            if let Some(call) = call_to_speak(timeline, loop_beat, true, last_spoken, false) {
                last_spoken = Some(call.start_beat);
                triggers.push((call.index, call.wrapped));
            }
            beat += step;
        }
        triggers
    }

    /// Assert that each call index appears exactly once in `triggers`.
    fn assert_each_call_once(triggers: &[(usize, bool)], calls: usize) {
        let mut indices: Vec<usize> = triggers.iter().map(|(index, _)| *index).collect();
        indices.sort_unstable();
        assert_eq!(indices, (0..calls).collect::<Vec<usize>>(), "{triggers:?}");
    }

    #[test]
    fn a_steady_beat_speaks_every_call_once_per_pass() {
        let timeline = default_timeline();
        let calls = timeline.calls.len();

        // A run starts at the beginning of the count-in, where the first
        // call's lead opens. Every call is spoken once, in order, and the
        // first call is announced once more for the coming loop.
        let first = drive(
            &timeline,
            -timeline.count_in_beats,
            timeline.music_loop_beats + timeline.count_in_beats,
            0.05,
        );
        assert_eq!(first.len(), calls + 1, "{first:?}");
        assert_eq!(first.first().copied(), Some((0, false)), "{first:?}");
        assert_eq!(first.last().copied(), Some((0, true)), "{first:?}");
        let run: Vec<usize> = first[..first.len() - 1]
            .iter()
            .map(|(index, _)| *index)
            .collect();
        assert!(run.windows(2).all(|pair| pair[0] < pair[1]), "{first:?}");

        // Every pass after that announces each call exactly once: the wrapped
        // first call at the end of the pass is the one the next pass needs, so
        // the top of the pass does not repeat it.
        let steady = drive(&timeline, 0.0, timeline.music_loop_beats, 0.05);
        assert_each_call_once(&steady, calls);
        assert_eq!(steady.len(), calls, "{steady:?}");
        assert_eq!(steady.last().copied(), Some((0, true)), "{steady:?}");
    }

    #[test]
    fn a_slow_hidden_frame_rate_still_speaks_every_call() {
        let timeline = default_timeline();
        // eframe throttles a hidden window to one pass per 100 ms, which is
        // 0.2 beats at 240 BPM. Even a coarser cadence must not drop a call:
        // the trigger is the audio position, not the frame rate.
        let steady = drive(&timeline, 0.0, timeline.music_loop_beats, 0.2);
        assert_each_call_once(&steady, timeline.calls.len());
        assert_eq!(steady.len(), timeline.calls.len(), "{steady:?}");
    }

    #[test]
    fn a_call_that_was_already_spoken_is_not_repeated() {
        let timeline = default_timeline();
        let first = &timeline.calls[0];
        let inside = first.call_start_beat + 0.5;
        let call = timeline
            .starting_call(inside)
            .expect("the first call opens during the count-in");

        // The first frame of a run announces the first call ...
        assert_eq!(
            call_to_speak(&timeline, inside, true, None, true).map(|call| call.start_beat),
            Some(call.start_beat)
        );
        // ... and every later frame of the same call stays quiet.
        assert!(
            call_to_speak(&timeline, inside, true, Some(call.start_beat), false).is_none(),
            "a resumed or re-rendered frame must not repeat the call"
        );
    }

    #[test]
    fn the_caller_stays_quiet_outside_a_call_lead() {
        let timeline = default_timeline();
        let first = &timeline.calls[0];
        // Before the first lead opens there is nothing to say yet.
        assert!(
            call_to_speak(&timeline, first.call_start_beat - 1.0, false, None, false).is_none()
        );

        // Past the last figure the dance is over.
        let last = timeline.calls.last().expect("the dance has calls");
        assert!(call_to_speak(&timeline, last.end_beat, true, None, false).is_none());
    }

    /// A headless app whose transport is a silent 64-beat clock.
    ///
    /// The audio position is moved with `seek_beat`, so `tick` can be driven
    /// exactly as a hidden window drives it: no UI, no device, no tune render.
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn hidden_app() -> (ContraApp, egui::Context) {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = ContraApp::new_headless(&cc);
        // `new_headless` installs a silent loop, so nothing needs rendering.
        app.needs_render = false;
        app.prefs.tts_enabled = true;
        app.speech.set_enabled(true);
        app.started = true;
        app.audio.play();
        (app, ctx)
    }

    #[test]
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn a_hidden_window_keeps_calling_and_pause_does_not_repeat_a_call() {
        let (mut app, ctx) = hidden_app();
        let stop_at = 30.0;

        // Run the count-in and the first calls through `tick` only: this is
        // all a minimized window runs, and every call must still be announced.
        let mut beat = -app.timeline.count_in_beats;
        while beat < stop_at {
            app.audio.seek_beat(beat);
            app.tick(&ctx);
            beat += 0.05;
        }
        let expected = app
            .timeline
            .calls
            .iter()
            .rfind(|call| call.call_start_beat < stop_at)
            .map(|call| call.call_start_beat)
            .expect("a call opens before the pause point");
        assert_eq!(
            app.last_spoken,
            Some(expected),
            "the newest call was announced without a drawn frame"
        );

        // Pause stops the transport and says nothing new.
        app.pause();
        assert!(!app.audio.is_playing());
        app.tick(&ctx);
        assert_eq!(
            app.last_spoken,
            Some(expected),
            "pause must not announce a call"
        );

        // Resuming at the same position does not repeat the call that was
        // already spoken there ...
        app.audio.play();
        app.started = true;
        app.tick(&ctx);
        assert_eq!(
            app.last_spoken,
            Some(expected),
            "resume must not re-announce a call already spoken"
        );

        // ... but the next call still comes at its own beat.
        let next = app
            .timeline
            .calls
            .iter()
            .find(|call| call.call_start_beat > stop_at)
            .map(|call| call.call_start_beat)
            .expect("another call follows");
        app.audio.seek_beat(next + 0.5);
        app.tick(&ctx);
        assert_eq!(app.last_spoken, Some(next), "the caller carries on");
    }

    /// Run `frames` header frames at `size` and collect what each frame
    /// reported.
    ///
    /// This drives the real header through a plain [`egui::Context`], so the
    /// test sees egui's own panel sizing rather than a stand-in for it.
    fn run_header(ctx: &egui::Context, size: egui::Vec2, frames: usize) -> Vec<HeaderResponse> {
        let mut headers = Vec::with_capacity(frames);
        for _ in 0..frames {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                headers.push(header_ui(
                    ui,
                    Screen::Session,
                    false,
                    compact_layout(size.x),
                ));
            });
            // Nothing renders in this test, but epaint insists its texture
            // updates are acknowledged before they are dropped.
            output.textures_delta.clear();
        }
        headers
    }

    #[test]
    fn a_window_resize_does_not_grow_the_header() {
        let ctx = egui::Context::default();
        theme::apply(&ctx, Theme::Dark, false);

        let launch = egui::vec2(1280.0, 800.0);
        let fresh = run_header(&ctx, launch, 2);
        let fresh = &fresh[1];

        // The phone layout, then an intermediate width. With three tabs and
        // the gear, the wordmark and tabs may fit one row at 700 points; the
        // round-trip below is what matters, not that this width wraps.
        run_header(&ctx, egui::vec2(390.0, 844.0), 2);
        run_header(&ctx, egui::vec2(700.0, 800.0), 2);

        // Back at the launch size, the header and every control inside it must
        // sit exactly where a fresh launch puts them. A content-sized panel
        // starts from the size it stored on the previous frame, so before the
        // header rows were top-aligned, the wrapped height fed itself back and
        // the header stayed tall until the app restarted.
        let again = run_header(&ctx, launch, 2);
        assert_eq!(again[1].rect, fresh.rect, "the header kept its old height");
        assert_eq!(again[1].tabs, fresh.tabs, "the tabs moved");
        assert_eq!(again[1].heading, fresh.heading, "the wordmark moved");
    }

    #[test]
    fn the_header_shows_every_tab_at_every_supported_width() {
        let ctx = egui::Context::default();
        theme::apply(&ctx, Theme::Dark, false);

        // The minimum window is 360 points; 390 is the phone reference size and
        // 640-800 the widths where the desktop row wraps.
        for width in [360.0, 390.0, 640.0, 700.0, 800.0, 1280.0] {
            let frames = run_header(&ctx, egui::vec2(width, 800.0), 3);
            let header = &frames[2];
            assert_eq!(header.tabs.len(), TABS.len(), "{width}px: a tab is missing");
            for (rect, def) in header.tabs.iter().zip(TABS.iter()) {
                assert!(
                    header.rect.contains_rect(*rect),
                    "{width}px: the {} tab is clipped: {rect:?} in {:?}",
                    def.hover,
                    header.rect
                );
                assert!(
                    rect.left() >= 0.0 && rect.right() <= width,
                    "{width}px: the {} tab overflows the window: {rect:?}",
                    def.hover
                );
            }
            if let Some(heading) = header.heading {
                assert!(
                    header.rect.contains_rect(heading),
                    "{width}px: the wordmark is clipped: {heading:?} in {:?}",
                    header.rect
                );
            }
        }
    }

    #[test]
    fn the_phone_header_keeps_its_touch_sizing_to_itself() {
        let ctx = egui::Context::default();
        theme::apply(&ctx, Theme::Dark, false);

        let frames = run_header(&ctx, egui::vec2(390.0, 844.0), 2);
        assert!(
            frames[1]
                .tabs
                .iter()
                .all(|tab| tab.height() >= theme::TOUCH_TARGET),
            "the phone header keeps 44-point tabs: {:?}",
            frames[1].tabs
        );
        assert_eq!(
            ctx.global_style().spacing.interact_size.y,
            40.0,
            "the phone header's row height leaked into the rest of the UI"
        );
    }

    /// A headless app with an empty history and nothing rendered yet.
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn history_app() -> (ContraApp, egui::Context) {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = ContraApp::new_headless(&cc);
        app.needs_render = false;
        (app, ctx)
    }

    /// Run one frame with `events` and read the undo/redo shortcut it saw.
    fn history_shortcut_for(events: Vec<egui::Event>) -> Option<HistoryAction> {
        let ctx = egui::Context::default();
        let mut action = None;
        let mut output = ctx.run_ui(
            egui::RawInput {
                events,
                ..Default::default()
            },
            |ui| action = ui.input_mut(history_shortcut),
        );
        output.textures_delta.clear();
        action
    }

    fn command_key(key: egui::Key, modifiers: egui::Modifiers) -> Vec<egui::Event> {
        vec![
            egui::Event::ModifiersChanged(modifiers),
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            },
        ]
    }

    #[test]
    fn undo_and_redo_shortcuts_use_the_platform_command_key() {
        let command = egui::Modifiers::COMMAND;
        assert_eq!(
            history_shortcut_for(command_key(egui::Key::Z, command)),
            Some(HistoryAction::Undo)
        );
        assert_eq!(
            history_shortcut_for(command_key(egui::Key::Y, command)),
            Some(HistoryAction::Redo)
        );
        assert_eq!(
            history_shortcut_for(command_key(
                egui::Key::Z,
                command.plus(egui::Modifiers::SHIFT)
            )),
            Some(HistoryAction::Redo),
            "Ctrl+Shift+Z redoes rather than undoing"
        );

        // A bare Z or Y is text, not an app shortcut.
        assert_eq!(
            history_shortcut_for(command_key(egui::Key::Z, egui::Modifiers::NONE)),
            None
        );
        assert_eq!(
            history_shortcut_for(command_key(egui::Key::Y, egui::Modifiers::NONE)),
            None
        );
        assert_eq!(history_shortcut_for(Vec::new()), None);
    }

    #[test]
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn undo_restores_role_instrument_and_volume_values() {
        let (mut app, ctx) = history_app();
        let before = app.snapshot();
        assert!(
            app.timeline
                .calls
                .iter()
                .any(|call| call.text.contains("larks"))
        );

        app.prefs.role_preset = RolePreset::Custom;
        app.prefs.role_terms.left_plural = "ports".into();
        app.prefs.melody = MelodyInstrument::Accordion;
        app.prefs.melody_volume = 0.3;
        app.rebuild_timeline();
        app.history.record(before, &app.snapshot(), None);
        assert!(
            app.timeline
                .calls
                .iter()
                .any(|call| call.text.contains("ports")),
            "the changed role terms reach the calls first"
        );

        app.pending_history = Some(HistoryAction::Undo);
        app.apply_history_request(&ctx);

        assert_eq!(app.prefs.role_preset, RolePreset::LarksRobins);
        assert_eq!(app.prefs.role_terms.left_plural, "larks");
        assert_eq!(app.prefs.melody, MelodyInstrument::Fiddle);
        assert!((app.prefs.melody_volume - 0.75).abs() < 1e-6);
        assert!(
            app.timeline
                .calls
                .iter()
                .any(|call| call.text.contains("larks")),
            "undo rebuilds the call timeline"
        );
        assert!(app.history.can_redo());
        assert_eq!(app.history_status.as_deref(), Some("Undid the last change"));
    }

    #[test]
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn a_new_edit_after_undo_discards_the_redo_step() {
        let (mut app, ctx) = history_app();
        let first = app.snapshot();
        app.prefs.theme = Theme::Light;
        app.history.record(first, &app.snapshot(), None);

        app.pending_history = Some(HistoryAction::Undo);
        app.apply_history_request(&ctx);
        assert_eq!(app.prefs.theme, Theme::Dark);
        assert!(app.history.can_redo());

        let before = app.snapshot();
        app.prefs.master_volume = 0.4;
        app.history.record(before, &app.snapshot(), None);
        assert!(!app.history.can_redo(), "the redo step is now stale");
    }

    #[test]
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn undo_of_a_playback_setting_marks_the_music_for_a_rebuild() {
        let (mut app, ctx) = history_app();
        // An unstarted session renders on the next Play; the test must not
        // block on a full render just to prove the flag was set.
        assert!(!app.started);
        let before = app.snapshot();
        app.prefs.accompaniment = AccompanimentInstrument::Guitar;
        app.history.record(before, &app.snapshot(), None);

        app.pending_history = Some(HistoryAction::Undo);
        app.apply_history_request(&ctx);

        assert_eq!(app.prefs.accompaniment, AccompanimentInstrument::Piano);
        assert!(app.needs_render, "the restored mix must be rendered again");
        assert!(
            app.pending_audio,
            "the rebuild is queued for the next playback frame"
        );
        assert!(!app.started, "undo must not start a stopped session");
    }

    #[test]
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn undo_of_a_dance_selection_stops_the_old_music() {
        let (mut app, ctx) = history_app();
        app.prefs.last_dance = Some(app.dance.source.clone());
        let before = app.snapshot();

        let mut other = app.dance.clone();
        other.source =
            contra_core::SourceRef::new("contradb", "9999", "https://example.test/dances/9999");
        other.title = "Another Dance".into();
        app.dance = other;
        app.prefs.last_dance = Some(app.dance.source.clone());
        app.rebuild_timeline();
        app.history.record(before.clone(), &app.snapshot(), None);

        // Pretend the session was running on the new dance.
        app.started = true;
        app.audio.play();
        assert!(app.audio.is_playing());

        app.pending_history = Some(HistoryAction::Undo);
        app.apply_history_request(&ctx);

        assert_eq!(app.dance.source.id, before.dance.source.id);
        assert_eq!(
            app.prefs
                .last_dance
                .as_ref()
                .map(|source| source.id.clone()),
            Some(before.dance.source.id.clone()),
            "the restored dance is the one a restart would load"
        );
        assert!(!app.started);
        assert!(!app.audio.is_playing(), "the old dance's music stops");
        assert!(
            app.needs_render,
            "the restored tune renders on the next Play"
        );
    }
}

// Each screen lives in its own file under `app/ui/`.
mod ui;

// Undo/redo history over the preferences and the selected dance and tune.
mod history;

#[cfg(feature = "headless")]
pub mod headless;
