//! Application state and UI.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use caller_core::{
    AccompanimentInstrument, AccompanimentRhythm, AliasTable, AudioEffects, BandPart, BandSlot,
    BandVoice, BowingOptions, COUNT_IN_BEATS, Countermelody, DEFAULT_DANCE_SOURCE, Dance,
    DancerSoundEvent, EndingOptions, FolderFilter, HumanizeOptions, InstrumentCopies, Library,
    MelodyInstrument, MoveKind, OutputMode, Percussion, PercussionRhythm, PracticeTimeline,
    Preferences, RolePreset, ScheduledCall, Theme, TimelineConfig, Tune, TuneSetSource,
    VariationOptions, dancer_sounds,
};
use caller_music::{
    Arrangement, AudioBuffer, LengthMismatch, RenderRequest, export_midi_with_band, parse_abc,
    render,
};
#[cfg(not(target_arch = "wasm32"))]
use caller_sources::FileCache;
use caller_sources::{
    Cache, DanceProvider, DanceQuery, DanceSearchPage, DanceSort, DanceSummary, FilterKind,
    TuneQuery, TuneSearchPage, TuneSummary, dance_provider,
};
use eframe::egui;
use rustysynth::SoundFont;

use crate::audio::AudioEngine;
use crate::defaults;
use crate::files;
use crate::library;
use crate::share::{self, SharePayload};
use crate::sheet::SheetPreview;
use crate::source::{SourceClient, SourceEvent};
use crate::theme;
use crate::tts::Speech;
use crate::tune_set::{ResolvedTuneSet, resolve_tune_set};
use history::{History, HistoryAction, MergeKey, Snapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Session,
    DancePicker,
    DanceDownload,
    TunePicker,
    TuneDownload,
    /// The plain-text ABC editor for a saved tune.
    AbcEditor,
    Preferences,
    Aliases,
    /// The in-app dance glossary, a top-level tab opened from its tab, a term,
    /// or the About section.
    Glossary,
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

/// Where a Back gesture (Escape or the Android system back button) leads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackAction {
    /// Leave for another screen.
    Screen(Screen),
    /// Stay in Preferences and return to its category list.
    PreferencesPage(ui::preferences::PreferencesPage),
}

/// Resolve one Back gesture.
///
/// A phone's Preferences drills into a category, so the first Back returns to
/// the category list and the next one to the session. A desktop shows the list
/// beside the settings, so Back leaves at once.
fn back_action(
    screen: Screen,
    page: ui::preferences::PreferencesPage,
    compact: bool,
) -> Option<BackAction> {
    if screen == Screen::Preferences
        && let Some(page) = page.back_target(compact)
    {
        return Some(BackAction::PreferencesPage(page));
    }
    screen.escape_target().map(BackAction::Screen)
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
        screens: &[Screen::TunePicker, Screen::TuneDownload, Screen::AbcEditor],
        label: "Select Tune",
        short_label: "Tune",
        hover: "Select Tune",
        shortcut: "3",
        key: egui::Key::Num3,
    },
    TabDef {
        screens: &[Screen::Glossary],
        label: "Glossary",
        short_label: "Terms",
        hover: "Dance glossary",
        shortcut: "4",
        key: egui::Key::Num4,
    },
];

/// The number key that opens Preferences.
///
/// Preferences is a header button rather than a tab, so its key follows the
/// four tabs in the row: 1-4 switch tabs and 5 opens the gear's screen.
const PREFERENCES_KEY: egui::Key = egui::Key::Num5;

/// The tabs in the row for the current glossary setting.
///
/// The glossary tab is part of the optional reference, so turning **Show dance
/// glossary** off removes it along with the underlined terms, the tooltips,
/// and the Dance card button. The remaining tabs keep their order and keys.
fn visible_tabs(glossary_enabled: bool) -> impl Iterator<Item = &'static TabDef> {
    TABS.iter()
        .filter(move |tab| glossary_enabled || tab.landing() != Screen::Glossary)
}

/// One screen: how to draw it and where Escape leads from it.
struct ScreenDef {
    screen: Screen,
    draw: fn(&mut CallerApp, &mut egui::Ui, f64),
    escape: Option<Screen>,
}

const SCREENS: [ScreenDef; 9] = [
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
        screen: Screen::AbcEditor,
        draw: draw_abc_editor,
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
    ScreenDef {
        screen: Screen::Glossary,
        draw: draw_glossary,
        // The glossary is a tab, so Back and Escape leave it like any other
        // tab and return to the session.
        escape: Some(Screen::Session),
    },
];

// The registry needs uniform draw functions; the screens themselves have
// different signatures.
fn draw_session(app: &mut CallerApp, ui: &mut egui::Ui, beat: f64) {
    app.session_ui(ui, beat);
}
fn draw_dance_library(app: &mut CallerApp, ui: &mut egui::Ui, _beat: f64) {
    app.dance_library_ui(ui);
}
fn draw_dance_download(app: &mut CallerApp, ui: &mut egui::Ui, _beat: f64) {
    app.dance_download_ui(ui);
}
fn draw_tune_library(app: &mut CallerApp, ui: &mut egui::Ui, _beat: f64) {
    app.tune_library_ui(ui);
}
fn draw_tune_download(app: &mut CallerApp, ui: &mut egui::Ui, _beat: f64) {
    app.tune_download_ui(ui);
}
fn draw_abc_editor(app: &mut CallerApp, ui: &mut egui::Ui, _beat: f64) {
    app.abc_editor_ui(ui);
}
fn draw_preferences(app: &mut CallerApp, ui: &mut egui::Ui, _beat: f64) {
    app.preferences_ui(ui);
}
fn draw_aliases(app: &mut CallerApp, ui: &mut egui::Ui, _beat: f64) {
    app.aliases_ui(ui);
}
fn draw_glossary(app: &mut CallerApp, ui: &mut egui::Ui, _beat: f64) {
    app.glossary_ui(ui);
}

/// Project links shown in the About section of Preferences.
///
/// The documentation site is published from `website/` to GitHub Pages; these
/// are the same addresses the release artifacts and the docs use. The constants
/// live with the GitHub fetch so the update check and the links cannot drift.
pub(crate) use crate::updates::{DOCUMENTATION_URL, REPOSITORY_URL};

/// Map the number-key shortcuts to screens.
fn screen_for_number_key(key: egui::Key, glossary_enabled: bool) -> Option<Screen> {
    visible_tabs(glossary_enabled)
        .find(|tab| tab.key == key)
        .map(|tab| tab.landing())
        .or_else(|| (key == PREFERENCES_KEY).then_some(Screen::Preferences))
}

/// The top-row tab that contains `screen`.
fn tab_index(screen: Screen, glossary_enabled: bool) -> usize {
    visible_tabs(glossary_enabled)
        .position(|tab| tab.is_active(screen))
        .unwrap_or(0)
}

/// The landing screen of the tab `step` places away from `current`.
///
/// The tab row wraps in both directions, so Left from Session reaches the last
/// tab and Right from the last tab returns to Session. Detail screens (a
/// download search, the alias editor) step from the tab that contains them.
fn tab_step(current: Screen, step: isize, glossary_enabled: bool) -> Screen {
    let count = visible_tabs(glossary_enabled).count() as isize;
    let index = tab_index(current, glossary_enabled) as isize;
    let next = (index + step).rem_euclid(count) as usize;
    visible_tabs(glossary_enabled)
        .nth(next)
        .map_or(current, TabDef::landing)
}

/// How far a horizontal swipe must travel before it changes tabs.
const SWIPE_MIN_DISTANCE: f32 = 60.0;
/// How much more horizontal than vertical a tab swipe must be.
const SWIPE_AXIS_RATIO: f32 = 1.5;

/// Whether a widget under the press owns the gesture instead of a tab swipe.
///
/// A clickable control (button, row, slider, checkbox, combo box) always owns
/// the press. A drag-only surface normally does too, but a wide vertical
/// `ScrollArea` also senses drags across its whole content; a horizontal drag
/// there cannot scroll a vertical list, so it may still change tabs. Narrow
/// drag handles (a pane splitter) keep the gesture.
fn swipe_owns_press(sense: egui::Sense, width: f32) -> bool {
    sense.senses_click() || (sense.senses_drag() && width <= 24.0)
}

/// Where the pointer press in progress began, recorded once per press.
///
/// Both facts are read while the press is fresh and reused for every frame of
/// the drag, because neither can be reconstructed later: the widget under the
/// pointer changes as the finger travels, and the touch event belongs to the
/// frame that started the press.
#[derive(Clone, Copy, Debug, Default)]
struct PressOrigin {
    /// The press landed on an interactive control, which owns the gesture.
    on_control: bool,
    /// The press came from a finger rather than a mouse.
    from_touch: bool,
}

/// Whether this frame's input puts a finger on the glass.
///
/// egui's pointer is a mouse and a touch at once: `egui-winit` and the web
/// backend both translate a touch into a primary pointer press, so the press
/// itself cannot say which device sent it. The touch event is the only record
/// of the device, and it arrives in the same frame as the press derived from
/// it, which is why a tab swipe needs both.
fn touch_started(events: &[egui::Event]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            egui::Event::Touch {
                phase: egui::TouchPhase::Start,
                ..
            }
        )
    })
}

/// The screen a horizontal swipe selects, or `None` when it is not a swipe.
///
/// A swipe left (negative x) moves to the next tab and a swipe right moves to
/// the previous one. A mostly vertical drag is a scroll, not a tab change, and
/// a short drag is a tap or a slider adjustment.
fn swipe_target(current: Screen, dx: f32, dy: f32, glossary_enabled: bool) -> Option<Screen> {
    if dx.abs() <= SWIPE_MIN_DISTANCE || dx.abs() < dy.abs() * SWIPE_AXIS_RATIO {
        return None;
    }
    Some(tab_step(
        current,
        if dx < 0.0 { 1 } else { -1 },
        glossary_enabled,
    ))
}

/// The screen selected by a number-key shortcut pressed this frame, if any.
///
/// The key must actually have been pressed: resolving the first tab's
/// shortcut regardless of input would pin the app to the session screen every
/// frame and undo any click on another tab.
fn number_key_shortcut(input: &egui::InputState, glossary_enabled: bool) -> Option<Screen> {
    let key = TABS
        .iter()
        .map(|tab| tab.key)
        .chain(std::iter::once(PREFERENCES_KEY))
        .find(|key| input.key_pressed(*key))?;
    screen_for_number_key(key, glossary_enabled)
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

pub struct CallerApp {
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
    /// Which page of Preferences is in front: the category list or one
    /// category's settings. A desktop shows the list and the selected category
    /// side by side; a phone shows one page at a time.
    prefs_page: ui::preferences::PreferencesPage,
    /// The Preferences search field. While it holds text, the category list
    /// and settings give way to the settings that match, from every category.
    prefs_query: String,
    /// The section a search result asked to reveal. The next category frame
    /// scrolls it into view and clears this, so a later visit does not jump.
    prefs_reveal: Option<&'static str>,
    /// A count-in change that must wait for the running count-in to finish
    /// before the timeline and the rendered music can change together.
    pending_count_in: Option<bool>,
    /// Progress (0..1) of the fade/slide used when the active screen changes.
    screen_anim: f32,
    /// The screen the animation progress belongs to; a mismatch restarts it.
    screen_anim_screen: Screen,
    /// True for `caller-headless`: ignore global shortcuts so the renderer can
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
    /// The glossary filter text and the entry a term link asked to reveal.
    glossary_query: String,
    glossary_focus: Option<String>,
    /// Whether the focused entry has already been scrolled to. The emphasis
    /// stays on the entry, but a later filter edit must not keep pulling the
    /// list back to it.
    glossary_focus_scrolled: bool,
    /// Bumped when the glossary opens so its scroll area starts at the top
    /// instead of where the previous visit left it.
    glossary_epoch: u64,
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
    /// The preferences the current music buffer was rendered with.
    ///
    /// A preference edit only dirties the render while the net value differs
    /// from this snapshot. Changing a control and changing it back before the
    /// queued render runs therefore leaves no pending work behind.
    music_prefs: Preferences,
    /// The render is dirty because of a preference edit, rather than a new
    /// dance, tune, or SoundFont. Only the preference part may be cleared by a
    /// reversed edit.
    prefs_audio_dirty: bool,
    /// A dance, tune, or SoundFont change needs a render even when the
    /// preferences match [`Self::music_prefs`].
    render_dirty_other: bool,
    /// The preferences that produced the current call timeline.
    timeline_prefs: Preferences,
    /// The alias seed used to build the current call timeline.
    timeline_alias_seed: u64,
    /// The timeline and call bookkeeping from before the current run of
    /// timeline edits, kept so an edit that is changed back restores what was
    /// already spoken.
    timeline_basis: Option<TimelineBasis>,
    /// A custom role-term field has focus, so the timeline rebuild is deferred
    /// until the edit settles.
    role_edit_active: bool,
    /// Custom role terms changed while the field was focused and still need
    /// the deferred rebuild.
    role_edit_pending: bool,
    render_loading: bool,
    started: bool,
    /// A Play press arrived while the startup prefetch was still running. The
    /// finished buffer starts the transport immediately instead of making the
    /// user press Play again.
    pending_play: bool,
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
    /// Desktop split-pane widths. Session-only: they are a view preference, not
    /// saved content, so they reset with the window rather than the library.
    panes: PaneWidths,
    tune_view: TuneView,
    tune_sheet: Option<Result<SheetPreview, String>>,
    /// The draft and preview of the ABC editor, when it is open.
    abc_editor: Option<ui::AbcEditorState>,
    /// An Android export waiting to be shared, if any.
    share_request: Option<files::ShareRequest>,
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
    /// The share link panel on the web session screen, when it is open.
    share: Option<SharePanel>,
    /// Bumped whenever a setting changes; prepared buffers from an older
    /// generation are discarded.
    render_generation: u64,
    /// The last loop rendered from the current tune and audio settings. A
    /// second Play, a restart, or an undo back to the same settings installs
    /// this buffer instead of synthesizing the loop again.
    render_cache: Option<CachedRender>,
    /// A render started before the user pressed Play, so the first press can
    /// take the cache instead of blocking the UI.
    prefetch: Option<Prefetch>,
    /// A prefetch that failed for this key; do not retry it every frame.
    prefetch_failed_key: Option<RenderKey>,
    /// Undo/redo steps over preferences and the selected dance and tune.
    history: History,
    /// An undo or redo asked for during this frame; it is applied after the
    /// frame's own edits have been recorded, so the edit just made is on the
    /// stack when the request arrives.
    pending_history: Option<HistoryAction>,
    /// The result of the last undo or redo, shown under the Interface buttons.
    history_status: Option<String>,
    /// The preference values last written to disk, so a preference pass that
    /// changed nothing does not rewrite the file.
    last_saved_prefs: Option<Preferences>,
    /// The last memory table built for Preferences → Maintenance.
    memory_report: Option<crate::memory::MemoryReport>,
    /// The GitHub release check behind the Preferences header's **Check for
    /// updates** button.
    updates: crate::updates::UpdateCheck,
    /// Number of preference files actually written. Test-only so a
    /// reversibility test can prove a no-op edit did not touch the file.
    #[cfg(test)]
    prefs_save_count: usize,
}

/// Transient state of the two download screens.
///
/// The dance and tune pickers have the same shape, so they share one struct: a
/// new search or preview field belongs here instead of being threaded through
/// the app struct and both constructors.
#[derive(Clone, Debug, PartialEq)]
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

/// Widths, in points, of the desktop split panes.
///
/// A width is the size of the left/top pane; the other pane takes the rest.
/// Widths are clamped at draw time, so a value left over from a wide window is
/// harmless after the window shrinks.
#[derive(Clone, Copy, Debug, PartialEq)]
struct PaneWidths {
    /// Folder sidebar in **Select dance** and **Select tune**.
    sidebar: f32,
    /// Saved-item list beside the preview in either library.
    library_list: f32,
    /// Search results beside the preview on **Download dance**.
    dance_picker_list: f32,
    /// Search results beside the preview on **Download tune**.
    tune_picker_list: f32,
}

impl Default for PaneWidths {
    fn default() -> Self {
        Self {
            sidebar: 230.0,
            library_list: 380.0,
            dance_picker_list: 400.0,
            tune_picker_list: 400.0,
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

/// Everything that changes the rendered audio.
///
/// The key holds the tune's ABC text and the captured render settings, not the
/// whole preference file: a theme, alias, or glossary change rebuilds calls but
/// must not invalidate a perfectly good audio buffer.
#[derive(Clone, PartialEq, Debug)]
struct RenderKey {
    tune_abc: String,
    settings: RenderSettings,
}

/// The last rendered loop, shared with the audio engine.
struct CachedRender {
    key: RenderKey,
    buffer: Arc<AudioBuffer>,
    arrangement: Arrangement,
}

/// A render running on a worker thread before the user asks for it.
struct Prefetch {
    key: RenderKey,
    rx: Receiver<Result<PreparedTune, String>>,
}

/// An open share-link panel: the encoded link and its last status line.
struct SharePanel {
    link: String,
    message: String,
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

/// The timeline and spoken-call bookkeeping from before a run of timeline
/// edits.
///
/// Timeline settings are applied immediately, but a user who tries a value and
/// changes it straight back should still hear the call that was already being
/// spoken. Keeping one basis restores the old timeline instead of treating the
/// round trip as two fresh timelines.
struct TimelineBasis {
    prefs: Preferences,
    alias_seed: u64,
    timeline: PracticeTimeline,
    last_spoken: Option<f64>,
    last_beat: f64,
    first_call_pending: bool,
}

/// The values a render needs, captured so a worker can render without the app.
#[derive(Clone, PartialEq, Debug)]
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
    bowing_options: BowingOptions,
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
    let arrangement =
        Arrangement::from_score_with_bowing(&score, 64.0, settings.bowing_options.clone());
    let buffer = render(&settings.request(arrangement.clone()), soundfont.as_ref())
        .map_err(|error| format!("Could not render music: {error}"))?;
    Ok(PreparedTune {
        generation,
        tune: tune.clone(),
        buffer,
        arrangement,
    })
}

impl CallerApp {
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
            let project = directories::ProjectDirs::from("org", "callerkaraoke", "CallerKaraoke");
            if let Some(project) = project {
                (
                    project.config_dir().to_path_buf(),
                    project.cache_dir().to_path_buf(),
                )
            } else {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let base = std::env::temp_dir().join("caller-karaoke");
                    (base.join("config"), base.join("cache"))
                }
                #[cfg(target_arch = "wasm32")]
                {
                    (
                        PathBuf::from("caller-karaoke-config"),
                        PathBuf::from("caller-karaoke-cache"),
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
                audio.set_caller_volume(prefs.caller_volume);
                (audio, None)
            }
            Err(error) => {
                let audio = AudioEngine::silent(48_000);
                audio.set_looping(prefs.loop_enabled);
                audio.set_volume(prefs.master_volume);
                audio.set_caller_volume(prefs.caller_volume);
                (audio, Some(error))
            }
        };

        let mut speech = Speech::new(cache_dir.join("tts"), audio.0.sample_rate());
        speech.set_rate(prefs.speech_rate());
        speech.set_voice(&prefs.tts_voice);
        speech.set_tone(prefs.tts_tone);
        speech.set_deliveries(prefs.tts_deliveries);
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
        let timeline = build_timeline(&dance, &prefs, 0)
            .unwrap_or_else(|_| empty_timeline(count_in_for(&prefs)));
        speech.prewarm(call_texts(&timeline));
        let length_warning = length_warning_for(&dance, &tune);
        let music_prefs = prefs.clone();
        let timeline_prefs = prefs.clone();

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
                Ok(bytes) => match caller_music::soundfont::load_soundfont(&bytes) {
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
            prefs_page: ui::preferences::PreferencesPage::Categories,
            prefs_query: String::new(),
            prefs_reveal: None,
            pending_count_in: None,
            screen_anim: 1.0,
            screen_anim_screen: Screen::Session,
            headless: false,
            scale_guard: crate::window::ScaleGuard::default(),
            launch_focus: crate::window::LaunchFocus::default(),
            media_keys: crate::media_keys::MediaKeys::new(cc),
            about_reveal: false,
            glossary_query: String::new(),
            glossary_focus: None,
            glossary_focus_scrolled: false,
            glossary_epoch: 0,
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
            music_prefs,
            prefs_audio_dirty: false,
            render_dirty_other: true,
            timeline_prefs,
            timeline_alias_seed: 0,
            timeline_basis: None,
            role_edit_active: false,
            role_edit_pending: false,
            render_loading: false,
            started: false,
            pending_play: false,
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
            panes: PaneWidths::default(),

            tune_view: TuneView::default(),
            tune_sheet: None,
            abc_editor: None,
            share_request: None,

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
            share: None,
            render_generation: 0,
            render_cache: None,
            prefetch: None,
            prefetch_failed_key: None,
            history: History::default(),
            pending_history: None,
            history_status: None,
            last_saved_prefs: None,
            memory_report: None,
            updates: crate::updates::UpdateCheck::default(),
            #[cfg(test)]
            prefs_save_count: 0,
        }
    }

    /// Browser entry point.
    ///
    /// The page may have been opened from a share link. A valid `?set=`
    /// argument replaces the startup session before the first frame; an invalid
    /// one leaves the bundled defaults in place and explains why.
    #[cfg(target_arch = "wasm32")]
    pub fn new_web(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::new_with_data_dir(cc, None);
        let Some(code) = crate::web_store::share_code_from_location() else {
            return app;
        };
        if let Err(error) = app.apply_share_code(&code) {
            app.error = Some(format!("Could not open the shared link: {error}"));
        }
        app
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
                // Escape and the Android system back button share one action.
                // eframe forwards Android's back gesture as BrowserBack on the
                // platforms that have it, and as Escape when the OS maps it.
                input.key_pressed(egui::Key::Escape) || input.key_pressed(egui::Key::BrowserBack),
                number_key_shortcut(input, self.prefs.glossary_enabled),
                input.key_pressed(egui::Key::ArrowLeft),
                input.key_pressed(egui::Key::ArrowRight),
            )
        });

        if let Some(screen) = tab {
            self.screen = screen;
        } else if next_tab {
            self.screen = tab_step(self.screen, 1, self.prefs.glossary_enabled);
        } else if previous_tab {
            self.screen = tab_step(self.screen, -1, self.prefs.glossary_enabled);
        }
        if space {
            self.toggle_playback();
        }
        if restart {
            self.restart();
        }
        if escape {
            let compact = compact_layout(ui.available_width());
            match back_action(self.screen, self.prefs_page, compact) {
                Some(BackAction::PreferencesPage(page)) => self.prefs_page = page,
                Some(BackAction::Screen(screen)) => self.screen = screen,
                None => {}
            }
        }
    }

    /// Switch tabs on a horizontal swipe of a finger in the screen content.
    ///
    /// The gesture is read from the raw pointer rather than from a widget, so
    /// it works over plain labels and frames. It is a **touch gesture only**:
    /// a mouse drag must not change tabs, because text selection, list
    /// reordering and slider drags all begin with the same press, and a mouse
    /// has the tab row, the arrow keys and the number keys as its own way to
    /// switch screens.
    ///
    /// A swipe must also never compete with a control the user is actually
    /// using: a press that starts on any interactive widget (a button, slider,
    /// folder row, text field, scroll area, or popup) belongs to that widget,
    /// so the tab does not change even if the finger then travels sideways. A
    /// press on empty content can still swipe. One press can change tabs once;
    /// releasing the pointer arms the next swipe.
    fn tab_swipe(&mut self, ui: &egui::Ui, content: egui::Rect) {
        let fired_id = egui::Id::unique("tab-swipe-fired");
        let origin_id = egui::Id::unique("tab-swipe-press-origin");

        // Record where this press began. A scroll area's drag surface is
        // deliberately not a control: a horizontal drag there does not scroll a
        // vertical list, so it may still change tabs. A clickable widget, or a
        // narrow drag handle such as a pane splitter, owns the press and must
        // not be pre-empted.
        if ui.input(|input| input.pointer.primary_pressed()) {
            let hovered: Vec<egui::Id> = ui
                .ctx()
                .interaction_snapshot(|snapshot| snapshot.hovered.iter().copied().collect());
            let on_control = hovered.iter().any(|id| {
                ui.ctx().read_response(*id).is_some_and(|response| {
                    response.enabled()
                        && swipe_owns_press(response.sense, response.interact_rect.width())
                })
            });
            let from_touch = ui.input(|input| touch_started(&input.events));
            ui.data_mut(|data| {
                data.insert_temp(
                    origin_id,
                    PressOrigin {
                        on_control,
                        from_touch,
                    },
                );
            });
        }
        if !ui.input(|input| input.pointer.primary_down()) {
            ui.data_mut(|data| data.remove::<PressOrigin>(origin_id));
        }
        let origin = ui
            .data(|data| data.get_temp::<PressOrigin>(origin_id))
            .unwrap_or_default();

        let gesture = ui.input(|input| {
            let start = input.pointer.press_origin()?;
            let pointer = input.pointer.interact_pos()?;
            input
                .pointer
                .primary_down()
                .then_some((start, pointer - start))
        });
        let Some((start, delta)) = gesture else {
            ui.data_mut(|data| data.insert_temp(fired_id, false));
            return;
        };
        if self.headless
            || !origin.from_touch
            || origin.on_control
            || ui.ctx().any_popup_open()
            || ui.ctx().text_edit_focused()
            || !content.contains(start)
            || ui::press_started_on_slider(ui, start)
            || ui.data(|data| data.get_temp::<bool>(fired_id).unwrap_or(false))
        {
            return;
        }
        if let Some(screen) =
            swipe_target(self.screen, delta.x, delta.y, self.prefs.glossary_enabled)
        {
            self.screen = screen;
            ui.data_mut(|data| data.insert_temp(fired_id, true));
        }
    }

    /// The dance provider selected in preferences.
    ///
    /// An id this build does not know (a hand-edited file or a preference from
    /// a newer build) falls back to ContraDB instead of breaking the screen.
    fn dance_source(&self) -> &'static dyn DanceProvider {
        dance_provider(&self.prefs.dance_source).unwrap_or(&caller_sources::CONTRADB)
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

    /// Take every file the platform's import dialog produced.
    ///
    /// The browser and Android pickers are asynchronous, so this runs every
    /// frame; the native dialogs queue their files and return at once.
    fn poll_imported_files(&mut self) {
        for (kind, file) in files::take_imports() {
            match kind {
                files::ImportKind::Abc => self.import_abc_file(file),
                files::ImportKind::DanceJson => self.import_dance_file(file),
            }
        }
    }

    /// Open the picker for ABC tunes or dance JSON files.
    fn choose_import(&mut self, kind: files::ImportKind) {
        if let Err(error) = files::choose_files(kind) {
            self.error = Some(error);
        }
        self.poll_imported_files();
    }

    fn import_abc_file(&mut self, file: files::ImportedFile) {
        let import = match files::tune_from_abc(&file.name, &file.text) {
            Ok(import) => import,
            Err(error) => {
                self.error = Some(format!("Could not import {}: {error}", file.name));
                return;
            }
        };
        let tune = import.tune;
        let refreshed = self
            .tune_library
            .contains(&tune.source.provider, &tune.source.id);
        let title = tune.title.clone();
        self.tune_library
            .insert(tune.clone(), None, library::now_seconds());
        self.save_tune_library();
        let extra = match import.additional_tunes {
            0 => String::new(),
            1 => " The file holds one more tune; only the first was imported.".to_string(),
            count => format!(" The file holds {count} more tunes; only the first was imported."),
        };
        self.library_status = Some(format!(
            "{} \"{title}\" from {}.{extra}",
            if refreshed { "Updated" } else { "Imported" },
            file.name
        ));
        self.picker.tune_preview = Some(tune);
        self.picker.tune_selected = None;
        self.tune_sheet = None;
        self.tune_library_folder = FolderFilter::All;
        self.tune_library_query.clear();
        self.screen = Screen::TunePicker;
    }

    fn import_dance_file(&mut self, file: files::ImportedFile) {
        let dance = match files::dance_from_json(&file.text) {
            Ok(dance) => dance,
            Err(error) => {
                self.error = Some(format!("Could not import {}: {error}", file.name));
                return;
            }
        };
        let refreshed = self
            .dance_library
            .contains(&dance.source.provider, &dance.source.id);
        let title = dance.title.clone();
        self.dance_library
            .insert(dance.clone(), None, library::now_seconds());
        self.save_dance_library();
        self.library_status = Some(format!(
            "{} \"{title}\" from {}.",
            if refreshed { "Updated" } else { "Imported" },
            file.name
        ));
        self.picker.dance_preview = Some(dance);
        self.picker.dance_selected = None;
        self.dance_library_folder = FolderFilter::All;
        self.dance_library_query.clear();
        self.screen = Screen::DancePicker;
    }

    /// Export a tune's ABC text with the platform's save mechanism.
    fn export_tune_abc(&mut self, tune: &Tune) {
        let (name, text) = files::tune_export(tune);
        self.export_text(files::ExportKind::Abc, &name, &text, &tune.title);
    }

    /// Export a dance as the versioned JSON exchange document.
    fn export_dance_json(&mut self, dance: &Dance) {
        let text = match files::dance_json(dance) {
            Ok(text) => text,
            Err(error) => {
                self.error = Some(error);
                return;
            }
        };
        let name = format!("{}.json", files::safe_filename(&dance.title));
        self.export_text(files::ExportKind::DanceJson, &name, &text, &dance.title);
    }

    fn export_text(&mut self, kind: files::ExportKind, name: &str, text: &str, label: &str) {
        // Android writes into the app's config directory; the desktop dialog and
        // the browser download choose their own destination.
        let export_dir = self
            .library_dir
            .as_deref()
            .and_then(std::path::Path::parent);
        match files::export_text(kind, name, text, export_dir) {
            Ok(files::ExportOutcome::Cancelled) => {}
            Ok(files::ExportOutcome::Saved(path)) => {
                let noun = match kind {
                    files::ExportKind::Abc => "ABC",
                    files::ExportKind::DanceJson => "JSON",
                };
                self.library_status = Some(format!("Exported \"{label}\" as {noun} to {path}."));
                #[cfg(target_os = "android")]
                {
                    self.share_request = Some(files::ShareRequest {
                        name: name.to_string(),
                        text: text.to_string(),
                        kind,
                    });
                }
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// Open a saved tune in the ABC editor.
    fn edit_tune_abc(&mut self, tune: &Tune) {
        self.abc_editor = Some(ui::AbcEditorState::new(tune));
        self.share_request = None;
        self.library_status = None;
        self.screen = Screen::AbcEditor;
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
            "Deleted \"{}\" from your saved dances.",
            entry.item.title
        ));
    }

    fn remove_saved_tune(&mut self, id: &str) {
        let Some(entry) = self
            .tune_library
            .entries
            .iter()
            .find(|entry| entry.item.source.id == id)
            .map(|entry| entry.item.clone())
        else {
            return;
        };
        self.tune_library.remove(&entry.source.provider, id);
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
            "Deleted \"{}\" from your saved tunes.",
            entry.title
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
        let Some(provider) = self
            .tune_library
            .entries
            .iter()
            .find(|entry| entry.item.source.id == id)
            .map(|entry| entry.item.source.provider.clone())
        else {
            return;
        };
        match self.tune_library.set_folder(&provider, id, folder) {
            Ok(()) => {
                self.save_tune_library();
                self.library_status = Some(format!("Moved the tune to {label}."));
                // A folder-set rotation follows its folder live, so a tune
                // moved in or out must discard a tune already rendered from
                // the old membership.
                if matches!(
                    self.prefs.tune_rotation.source,
                    caller_core::TuneSetSource::Folder(_)
                ) {
                    self.rotation_settings_changed();
                }
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
        self.save_prefs();
    }

    /// Advance the screen-entry animation and return its eased progress.
    fn screen_transition(&mut self, ctx: &egui::Context) -> f32 {
        if self.screen_anim_screen != self.screen {
            // The editor reports its save or cancel result on the library it
            // returns to, so that one transition keeps the status line.
            let keep_status =
                self.screen_anim_screen == Screen::AbcEditor && self.screen == Screen::TunePicker;
            self.screen_anim_screen = self.screen;
            self.screen_anim = 0.0;
            if !keep_status {
                self.library_status = None;
            }
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
    /// network client, or user preference file. Used by `caller-headless` and
    /// screenshot tests.
    #[cfg(any(feature = "headless", test))]
    pub fn new_headless(cc: &eframe::CreationContext<'_>) -> Self {
        let (source_tx, source_rx): (Sender<SourceEvent>, Receiver<SourceEvent>) = channel();
        let mut prefs = Preferences::default();
        prefs.validate();
        // Screenshot frames must not be captured mid-transition.
        prefs.animations_enabled = false;

        let audio = AudioEngine::headless_clock(48_000, prefs.tempo_bpm);
        audio.set_looping(prefs.loop_enabled);
        audio.set_volume(prefs.master_volume);
        audio.set_caller_volume(prefs.caller_volume);

        let mut speech = Speech::stub();
        speech.set_rate(prefs.speech_rate());
        speech.set_tone(prefs.tts_tone);

        let source = SourceClient::new(
            source_tx,
            Cache::new(caller_sources::MemoryCache::default()),
            None,
            None,
        );

        let dance = defaults::default_dance();
        let tune = defaults::default_tune();
        let mut dance_library = Library::default();
        dance_library.insert(dance.clone(), None, 0);
        let mut tune_library = Library::default();
        tune_library.insert(tune.clone(), None, 0);
        let timeline = build_timeline(&dance, &prefs, 0)
            .unwrap_or_else(|_| empty_timeline(count_in_for(&prefs)));
        let length_warning = length_warning_for(&dance, &tune);
        let music_prefs = prefs.clone();
        let timeline_prefs = prefs.clone();

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
            prefs_page: ui::preferences::PreferencesPage::Categories,
            prefs_query: String::new(),
            prefs_reveal: None,
            pending_count_in: None,
            screen_anim: 1.0,
            screen_anim_screen: Screen::Session,
            headless: true,
            scale_guard: crate::window::ScaleGuard::default(),
            launch_focus: crate::window::LaunchFocus::default(),
            media_keys: None,
            about_reveal: false,
            glossary_query: String::new(),
            glossary_focus: None,
            glossary_focus_scrolled: false,
            glossary_epoch: 0,
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
            music_prefs,
            prefs_audio_dirty: false,
            render_dirty_other: true,
            timeline_prefs,
            timeline_alias_seed: 0,
            timeline_basis: None,
            role_edit_active: false,
            role_edit_pending: false,
            render_loading: false,
            started: false,
            pending_play: false,
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
            panes: PaneWidths::default(),

            tune_view: TuneView::default(),
            tune_sheet: None,
            abc_editor: None,
            share_request: None,

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
            share: None,
            render_generation: 0,
            render_cache: None,
            prefetch: None,
            prefetch_failed_key: None,
            history: History::default(),
            pending_history: None,
            history_status: None,
            last_saved_prefs: None,
            memory_report: None,
            updates: crate::updates::UpdateCheck::default(),
            #[cfg(test)]
            prefs_save_count: 0,
        }
    }

    fn save_prefs(&mut self) {
        // Preferences are saved after every preference frame. A round trip
        // that ends on the value already on disk must not rewrite the file;
        // flash storage and the web's localStorage both pay for pointless
        // writes.
        if self.last_saved_prefs.as_ref() == Some(&self.prefs) {
            return;
        }
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
            #[cfg(test)]
            {
                self.prefs_save_count += 1;
            }
        }
        self.last_saved_prefs = Some(self.prefs.clone());
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
        self.share = None;
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
        self.timeline_basis = None;
        if dance_changed || tune_changed || soundfont_changed {
            self.render_dirty_other = true;
        }
        // An undo is a new state of the world; never keep a count-in change
        // waiting against a transport it was planned for.
        self.pending_count_in = None;

        // Settings that apply immediately rather than through a rebuild. These
        // mirror the Preferences screen, so undo sounds like choosing the old
        // value again.
        self.audio.set_looping(self.prefs.loop_enabled);
        self.audio.set_volume(self.prefs.master_volume);
        self.audio.set_caller_volume(self.prefs.caller_volume);
        self.source.set_proxy(
            self.prefs.web_proxy_url.clone(),
            self.prefs.web_proxy_token.clone(),
        );
        self.speech.set_rate(self.prefs.speech_rate());
        self.speech.set_voice(&self.prefs.tts_voice);
        self.speech.set_tone(self.prefs.tts_tone);
        self.speech.set_deliveries(self.prefs.tts_deliveries);
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
            self.mark_render_pending_other();
            self.pending_audio = false;
            self.arrangement = None;
            self.audio.stop();
            self.speech.stop();
            self.started = false;
            self.rotation.passes = 0;
            self.rotation.last_beat = 0.0;
            self.rotation_settings_changed();
            self.rebuild_timeline();
            self.length_warning = length_warning_for(&self.dance, &self.tune);
        } else if changes.any() {
            if changes.timeline {
                self.rebuild_timeline();
            }
            if changes.audio {
                self.refresh_render_flags();
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
        // The bundled font is parsed lazily on the next render. Dropping the
        // parsed copy here keeps a user bank and the bundled subset from being
        // resident at the same time.
        self.bundled_soundfont = None;
        self.bundled_soundfont_checked = false;
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
                Ok(bytes) => match caller_music::soundfont::load_soundfont(&bytes) {
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

    /// Recompute the render flags from the preferences and the buffer.
    ///
    /// `needs_render` is the union of a preference edit and a new dance, tune,
    /// or SoundFont. Only the preference part can be cleared by changing the
    /// control back before the queued render runs.
    fn refresh_render_flags(&mut self) {
        self.prefs_audio_dirty = self.prefs.changes_since(&self.music_prefs).audio;
        self.needs_render = self.prefs_audio_dirty || self.render_dirty_other;
        self.pending_audio = self.prefs_audio_dirty;
    }

    /// Mark a render that no preference reversal can cancel.
    fn mark_render_pending_other(&mut self) {
        self.render_dirty_other = true;
        self.needs_render = true;
        // The tune, dance, or SoundFont changed; a buffer or prefetch from the
        // old source must not be installed.
        self.render_cache = None;
        self.prefetch = None;
        self.prefetch_failed_key = None;
        self.pending_play = false;
    }

    /// The current buffer and preferences agree again.
    fn clear_render_pending(&mut self) {
        self.render_dirty_other = false;
        self.prefs_audio_dirty = false;
        self.needs_render = false;
        self.pending_audio = false;
    }

    /// Apply the consequences of a Preferences-screen edit.
    ///
    /// The screen passes the preferences from before the frame; the test suite
    /// calls this same method after changing a control directly, so the
    /// reversibility tests exercise the production rule instead of a copy.
    fn reconcile_preferences(&mut self, before: Preferences) {
        self.prefs.validate();
        if self.prefs == before {
            return;
        }
        let changes = self.prefs.changes_since(&before);
        if self.prefs.count_in_enabled != before.count_in_enabled {
            // The timeline, the rendered count-in, and the transport state
            // change together; the helper also decides whether a count-in
            // already playing may finish first.
            self.apply_count_in_change();
        } else if changes.timeline && !self.role_edit_active {
            // A rebuild that returns to the previous timeline restores the
            // spoken call itself (see `rebuild_timeline`).
            self.rebuild_timeline();
        }
        // The timeline rebuild already reset the spoken call when the calls
        // changed; when the round trip restored the old timeline, keep the
        // restored bookkeeping instead of clearing it here.
        self.refresh_render_flags();
        if !self.prefs.tts_enabled {
            self.speech.stop();
        }
        if self.prefs.tts_enabled != before.tts_enabled {
            self.speech.set_enabled(self.prefs.tts_enabled);
        }
        if self.prefs.tts_rate != before.tts_rate {
            self.speech.set_rate(self.prefs.speech_rate());
        }
        if self.prefs.tts_voice != before.tts_voice {
            self.speech.set_voice(&self.prefs.tts_voice);
        }
        if self.prefs.tune_rotation != before.tune_rotation {
            self.rotation_settings_changed();
        }
        if changes.tone {
            self.speech.set_tone(self.prefs.tts_tone);
        }
        if changes.deliveries {
            self.speech.set_deliveries(self.prefs.tts_deliveries);
        }
        self.audio.set_looping(self.prefs.loop_enabled);
        self.audio.set_volume(self.prefs.master_volume);
        self.audio.set_caller_volume(self.prefs.caller_volume);
        self.save_prefs();
    }

    fn rebuild_timeline(&mut self) {
        let prefs = self.prefs.clone();
        let seed = self.alias_seed;
        // While a count-in change waits for the loop start, the installed
        // timeline (and the buffer already playing) keep the old count-in, so
        // an unrelated rebuild must not apply the new setting early.
        let count_in_beats = if self.pending_count_in.is_some() {
            self.timeline.count_in_beats
        } else {
            count_in_for(&self.prefs)
        };
        match build_timeline_with_count_in(&self.dance, &self.prefs, seed, count_in_beats) {
            Ok(timeline) => {
                if timeline == self.timeline {
                    // The edit did not change any call, so the caller's
                    // bookkeeping still describes this timeline exactly.
                    self.timeline_prefs = prefs;
                    self.timeline_alias_seed = seed;
                    return;
                }
                let restore = self
                    .timeline_basis
                    .as_ref()
                    .is_some_and(|basis| {
                        basis.prefs == prefs && basis.alias_seed == seed && !self.audio.is_playing()
                    })
                    .then(|| self.timeline_basis.take())
                    .flatten();
                if let Some(basis) = restore {
                    // The settings are back on the value that produced the
                    // timeline before this run of edits. Restore that timeline
                    // and what had already been spoken, instead of treating
                    // the round trip as a fresh session.
                    self.timeline = basis.timeline;
                    self.timeline_prefs = prefs;
                    self.timeline_alias_seed = seed;
                    self.speech.prewarm(call_texts(&self.timeline));
                    self.last_spoken = basis.last_spoken;
                    self.last_beat = basis.last_beat;
                    self.first_call_pending = basis.first_call_pending;
                    return;
                }
                if self.timeline_basis.is_none() {
                    self.timeline_basis = Some(TimelineBasis {
                        prefs: self.timeline_prefs.clone(),
                        alias_seed: self.timeline_alias_seed,
                        timeline: self.timeline.clone(),
                        last_spoken: self.last_spoken,
                        last_beat: self.last_beat,
                        first_call_pending: self.first_call_pending,
                    });
                }
                self.timeline = timeline;
                self.timeline_prefs = prefs;
                self.timeline_alias_seed = seed;
                self.speech.prewarm(call_texts(&self.timeline));
                self.last_spoken = None;
                self.last_beat = 0.0;
                self.first_call_pending = should_announce_first_call(
                    &self.timeline,
                    self.current_beat(),
                    self.last_spoken,
                );
            }
            Err(error) => {
                // Keep the last good timeline; the error is shown, and the
                // fallback constructed at launch already used the caller's
                // count-in rather than a hardcoded four beats.
                self.error = Some(error);
            }
        }
    }

    /// Apply a count-in change to the timeline, the music, and the transport
    /// together.
    ///
    /// `prefs.count_in_enabled` is read, not written: the checkbox already
    /// updated it. Shortening a count-in that is playing is deferred to the loop
    /// start (see [`plan_count_in_change`]) so it cannot restart the dance; every
    /// other change re-renders now, keeping the musical position. The caller's
    /// first call is re-evaluated either way, so it is not lost when the
    /// count-in disappears and not spoken twice when the count-in just ran.
    fn apply_count_in_change(&mut self) {
        let desired = self.prefs.count_in_enabled;
        let installed = self.timeline.count_in_beats > 0.0;
        // Toggling back while a change is waiting needs no work: the installed
        // timeline already has the wanted state.
        if self.pending_count_in.is_some() && desired == installed {
            self.pending_count_in = None;
            self.save_prefs();
            return;
        }

        let old_count_in = self.timeline.count_in_beats;
        let new_count_in = count_in_for(&self.prefs);
        let beat = self.current_beat();
        let plan = plan_count_in_change(self.audio.is_playing(), beat, old_count_in, new_count_in);
        match plan {
            CountInPlan::DeferUntilLoop => {
                self.pending_count_in = Some(desired);
                self.status = "The count-in will change when the dance starts".into();
                self.save_prefs();
            }
            CountInPlan::ApplyNow { resume_beat } => {
                self.pending_count_in = None;
                self.rebuild_timeline_with_count_in(new_count_in);
                // Pre-rendered rotation work used the old count-in.
                self.render_generation = self.render_generation.wrapping_add(1);
                self.rotation.prepared = None;
                self.rotation.rx = None;
                self.rotation.loading_generation = 0;
                if self.started {
                    let was_playing = self.audio.is_playing();
                    let previous_spoken = self.last_spoken;
                    let Some((buffer, arrangement)) = self.render_music() else {
                        return;
                    };
                    // The old buffer played while this rendered, so sample its
                    // position now; clamp it into the count-in the new buffer
                    // actually has.
                    let resume = if was_playing {
                        count_in_resume_beat(self.audio.position_beat(), new_count_in)
                    } else {
                        resume_beat.unwrap_or(count_in_resume_beat(beat, new_count_in))
                    };
                    self.install_music(buffer, arrangement, Some(resume), false);
                    if was_playing {
                        // A re-render in the same run keeps what was already
                        // said; only a new run may announce the first call.
                        self.last_spoken = previous_spoken.or(self.last_spoken);
                        self.audio.play();
                        self.status = "Playing".into();
                    } else {
                        self.status = "Paused".into();
                    }
                    self.first_call_pending = should_announce_first_call(
                        &self.timeline,
                        self.current_beat(),
                        self.last_spoken,
                    );
                } else {
                    // Nothing is playing: the next Play renders the new
                    // count-in. The timeline already agrees with the setting.
                    self.needs_render = true;
                    self.pending_audio = false;
                }
                self.save_prefs();
            }
        }
    }

    /// Rebuild the timeline with an explicit count-in.
    ///
    /// A failed build falls back to an empty timeline with the count-in the
    /// caller asked for. Keeping the previous timeline would leave the UI
    /// advertising a four-beat count-in the music no longer has.
    fn rebuild_timeline_with_count_in(&mut self, count_in_beats: f64) {
        let prefs = self.prefs.clone();
        let seed = self.alias_seed;
        match build_timeline_with_count_in(&self.dance, &self.prefs, seed, count_in_beats) {
            Ok(timeline) => {
                if timeline == self.timeline {
                    // The edit did not change any call, so the caller's
                    // bookkeeping still describes this timeline exactly.
                    self.timeline_prefs = prefs;
                    self.timeline_alias_seed = seed;
                    return;
                }
                let restore = self
                    .timeline_basis
                    .as_ref()
                    .is_some_and(|basis| {
                        basis.prefs == prefs && basis.alias_seed == seed && !self.audio.is_playing()
                    })
                    .then(|| self.timeline_basis.take())
                    .flatten();
                if let Some(basis) = restore {
                    // The settings are back on the value that produced the
                    // timeline before this run of edits. Restore that timeline
                    // and what had already been spoken, instead of treating
                    // the round trip as a fresh session.
                    self.timeline = basis.timeline;
                    self.timeline_prefs = prefs;
                    self.timeline_alias_seed = seed;
                    self.speech.prewarm(call_texts(&self.timeline));
                    self.last_spoken = basis.last_spoken;
                    self.last_beat = basis.last_beat;
                    self.first_call_pending = basis.first_call_pending;
                    return;
                }
                if self.timeline_basis.is_none() {
                    self.timeline_basis = Some(TimelineBasis {
                        prefs: self.timeline_prefs.clone(),
                        alias_seed: self.timeline_alias_seed,
                        timeline: self.timeline.clone(),
                        last_spoken: self.last_spoken,
                        last_beat: self.last_beat,
                        first_call_pending: self.first_call_pending,
                    });
                }
                self.timeline = timeline;
                self.timeline_prefs = prefs;
                self.timeline_alias_seed = seed;
                self.speech.prewarm(call_texts(&self.timeline));
                self.last_spoken = None;
                self.last_beat = 0.0;
                self.first_call_pending = should_announce_first_call(
                    &self.timeline,
                    self.current_beat(),
                    self.last_spoken,
                );
            }
            Err(error) => {
                self.error = Some(error);
                self.timeline = empty_timeline(count_in_beats);
                self.first_call_pending = false;
            }
        }
    }

    /// Apply a deferred count-in change once the loop is reached.
    fn resolve_pending_count_in(&mut self, beat: f64) {
        if self.pending_count_in.is_none() || beat < 0.0 {
            return;
        }
        self.pending_count_in = None;
        self.apply_count_in_change();
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
            bowing_options: self.prefs.bowing_options.clone(),
            humanize_options: self.prefs.humanize_options.clone(),
            ending_options: self.prefs.ending_options.clone(),
            dancer_sounds: self.prefs.dancer_sounds,
            dancer_sound_volume: self.prefs.dancer_sound_volume,
            dancer_sound_events: if self.prefs.dancer_sounds {
                dancer_sounds(&self.dance)
            } else {
                Vec::new()
            },
            percussion: self.prefs.percussion,
            percussion_rhythm: self.prefs.percussion_rhythm,
            melody_volume: self.prefs.melody_volume,
            accompaniment_volume: self.prefs.accompaniment_volume,
            percussion_volume: self.prefs.percussion_volume,
            effects: self.prefs.effects,
            count_in_beats,
            sample_rate: self.audio.sample_rate(),
            variation_passes: if self.prefs.accompaniment_variations {
                caller_music::VARIATION_PASSES
            } else {
                1
            },
            humanize_timing: self.prefs.humanize_timing,
        }
    }

    /// The values that identify one rendered loop.
    fn render_key(&self) -> RenderKey {
        RenderKey {
            tune_abc: self.tune.abc.clone(),
            settings: self.render_settings(self.timeline.count_in_beats),
        }
    }

    /// Whether the RAM cache already holds the loop for the current settings.
    fn cached_render_matches(&self) -> bool {
        self.render_cache
            .as_ref()
            .is_some_and(|cache| cache.key == self.render_key())
    }

    /// Start synthesizing the current loop before the user presses Play.
    ///
    /// The worker only fills the in-memory cache; the transport and the
    /// installed buffer are untouched. A Play press while it runs becomes a
    /// pending play and starts as soon as the cache is ready, rather than
    /// synthesizing the same buffer again on the UI thread.
    fn start_prefetch(&mut self) {
        if self.prefetch.is_some() || self.pending_count_in.is_some() {
            return;
        }
        // A live settings change is allowed to prefetch while the old buffer
        // keeps playing; an ordinary running session is not.
        if self.audio.is_playing() && !self.pending_audio {
            return;
        }
        let key = self.render_key();
        if self
            .render_cache
            .as_ref()
            .is_some_and(|cache| cache.key == key)
        {
            return;
        }
        if self.prefetch_failed_key.as_ref() == Some(&key) {
            return;
        }

        #[cfg(target_arch = "wasm32")]
        {
            // The web build has no render worker; its first Play keeps the
            // existing synchronous path.
            let _ = key;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let settings = key.settings.clone();
            let soundfont = self.active_soundfont();
            let tune = self.tune.clone();
            let generation = self.render_generation;
            let (tx, rx) = channel();
            std::thread::spawn(move || {
                let _ = tx.send(prepare_tune(&tune, &settings, soundfont, generation));
            });
            self.prefetch = Some(Prefetch { key, rx });
        }
    }

    /// Collect a finished prefetch into the render cache.
    fn poll_prefetch(&mut self) {
        let Some(prefetch) = self.prefetch.take() else {
            return;
        };
        match prefetch.rx.try_recv() {
            Ok(Ok(prepared)) => {
                let play_when_ready = std::mem::take(&mut self.pending_play);
                if prepared.generation == self.render_generation
                    && prefetch.key == self.render_key()
                {
                    self.render_cache = Some(CachedRender {
                        key: prefetch.key,
                        buffer: Arc::new(prepared.buffer),
                        arrangement: prepared.arrangement,
                    });
                    self.prefetch_failed_key = None;
                }
                if play_when_ready {
                    self.play();
                }
            }
            Ok(Err(error)) => {
                let play_when_ready = std::mem::take(&mut self.pending_play);
                if prefetch.key == self.render_key() {
                    self.prefetch_failed_key = Some(prefetch.key);
                    self.status = format!("Could not prepare the music: {error}");
                }
                if play_when_ready {
                    // Fall back to the synchronous path so a failed prefetch
                    // does not swallow the press.
                    self.play();
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => self.prefetch = Some(prefetch),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                let play_when_ready = std::mem::take(&mut self.pending_play);
                if prefetch.key == self.render_key() {
                    self.prefetch_failed_key = Some(prefetch.key);
                }
                if play_when_ready {
                    self.play();
                }
            }
        }
    }

    /// Parse and render the current tune with the current playback settings.
    fn render_music(&mut self) -> Option<(Arc<AudioBuffer>, Arrangement)> {
        let key = self.render_key();
        if let Some(cache) = &self.render_cache
            && cache.key == key
        {
            self.length_warning = cache
                .arrangement
                .check_dance_length(self.dance.total_beats())
                .err()
                .map(format_length_mismatch);
            return Some((Arc::clone(&cache.buffer), cache.arrangement.clone()));
        }

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
        let settings = key.settings.clone();
        let soundfont = self.active_soundfont();
        let buffer = match render(&settings.request(arrangement.clone()), soundfont.as_ref()) {
            Ok(buffer) => Arc::new(buffer),
            Err(error) => {
                self.error = Some(format!("Could not render music: {error}"));
                return None;
            }
        };
        self.render_cache = Some(CachedRender {
            key,
            buffer: Arc::clone(&buffer),
            arrangement: arrangement.clone(),
        });
        self.prefetch_failed_key = None;
        Some((buffer, arrangement))
    }

    /// Install a freshly rendered loop without restarting the transport.
    ///
    /// `resume_beat` is the position captured from the old buffer just before
    /// the swap, so a settings change keeps the dance where it was.
    /// `keep_speech` leaves a call that straddles a tune change audible; a
    /// settings change stops stale speech instead.
    fn install_music(
        &mut self,
        buffer: Arc<AudioBuffer>,
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
        self.audio.set_buffer_shared(buffer);
        self.arrangement = Some(arrangement);
        self.audio.set_looping(self.prefs.loop_enabled);
        self.audio.set_volume(self.prefs.master_volume);
        self.audio.set_caller_volume(self.prefs.caller_volume);
        self.music_prefs = self.prefs.clone();
        self.clear_render_pending();
        self.timeline_basis = None;
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

    /// The saved tunes the rotation currently plays, in play order.
    ///
    /// The set is resolved from the library on every read: a folder set picks
    /// up a tune as soon as it is saved into that folder, and a deleted folder
    /// falls back to the explicit list instead of stalling the rotation.
    fn resolved_tune_set(&self) -> ResolvedTuneSet {
        resolve_tune_set(&self.tune_library, &self.prefs.tune_rotation)
    }

    /// The saved tunes in the rotation, in play order.
    fn rotation_tunes(&self) -> Vec<Tune> {
        self.resolved_tune_set().tunes
    }

    /// A non-blocking notice when the rotation's folder has disappeared.
    fn tune_set_notice(&self) -> Option<String> {
        self.resolved_tune_set().missing_folder.map(|name| {
            format!(
                "The folder \"{name}\" for the tune set no longer exists; using the chosen tunes."
            )
        })
    }

    /// Point the rotation at a folder or back at the explicit list.
    ///
    /// Cycle tunes itself is unchanged: this only chooses which saved tunes it
    /// plays through.
    fn set_tune_set_source(&mut self, source: TuneSetSource) {
        if self.prefs.tune_rotation.source == source {
            return;
        }
        self.library_status = Some(match &source {
            TuneSetSource::List => "Tune set: the chosen tunes.".to_string(),
            TuneSetSource::Folder(name) => {
                format!("Tune set: every tune in the folder \"{name}\".")
            }
        });
        self.prefs.tune_rotation.source = source;
        self.rotation_settings_changed();
        self.save_prefs();
    }

    /// The tune after the one now playing, or `None` when cycling cannot run.
    fn next_rotation_tune(&self) -> Option<Tune> {
        let tunes = self.rotation_tunes();
        if !caller_core::TuneRotation::can_rotate(tunes.len()) {
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
        // The prepared tune replaces the old source; cached or in-flight work
        // for the previous tune must not be installed later.
        self.render_cache = None;
        self.prefetch = None;
        self.prefetch_failed_key = None;
        self.pending_play = false;
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
        self.install_music(Arc::new(buffer), arrangement, Some(0.0), true);
        self.last_spoken = None;
        self.last_beat = 0.0;
        self.status = format!("Tune change: {}", self.tune.title);
    }

    /// The tune set or its interval changed: discard work rendered from the
    /// old setup. The current tune keeps playing.
    fn rotation_settings_changed(&mut self) {
        self.share = None;
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
        match &self.prefs.tune_rotation.source {
            TuneSetSource::Folder(_) => self.resolved_tune_set().tunes.iter().any(|saved| {
                saved.source.provider == tune.source.provider && saved.source.id == tune.source.id
            }),
            TuneSetSource::List => self.prefs.tune_rotation.contains(&tune.source),
        }
    }

    /// Add or remove a tune from the rotation set.
    fn toggle_rotation_tune(&mut self, tune: &Tune) {
        if matches!(self.prefs.tune_rotation.source, TuneSetSource::Folder(_)) {
            // The set follows a folder, so editing one tune into the explicit
            // list would have no audible effect. Point at the folder controls
            // instead of silently doing nothing.
            self.library_status = Some(
                "The tune set follows a folder. Choose the chosen tunes to add single tunes."
                    .to_string(),
            );
            return;
        }
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
        // A prefetch already holds the loop; let the worker finish instead of
        // synthesizing the same buffer again on the UI thread. The completion
        // poll calls Play again and installs the cache.
        if !self.headless
            && self.prefetch.is_some()
            && (self.needs_render || !self.audio.has_buffer())
        {
            self.pending_play = true;
            self.status = "Preparing music…".into();
            return;
        }
        self.timeline_basis = None;
        let from_finished = self.audio.is_finished();
        self.ensure_ready();
        if self.audio.has_buffer() {
            self.audio.play();
            self.started = true;
            self.status = "Playing".into();
            if self.prefs.tts_enabled {
                // Pausing cancels the hidden delivery passes; resume them now
                // that the session is running again. Prepared calls are reused.
                self.speech.prewarm(call_texts(&self.timeline));
            }
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
        self.pending_play = false;
        self.timeline_basis = None;
        self.audio.pause();
        self.speech.stop();
        self.status = "Paused".into();
    }

    /// Start, resume, or pause the session.
    ///
    /// The single Play/Pause button and the Space key both come here, so the
    /// button's icon and the key can never disagree about what a press does.
    fn toggle_playback(&mut self) {
        if self.audio.is_playing() {
            self.pause();
        } else {
            self.play();
        }
    }

    fn restart(&mut self) {
        self.pending_play = false;
        self.timeline_basis = None;
        // A restart is a new run: a count-in change that was waiting for the
        // current count-in to finish applies now, so this run honours it.
        if self.pending_count_in.is_some() {
            self.pending_count_in = None;
            self.rebuild_timeline_with_count_in(count_in_for(&self.prefs));
            self.needs_render = true;
        }
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
        self.share = None;
        self.pending_count_in = None;
        self.download_dance(dance.clone());
        self.dance = dance;
        self.length_warning = length_warning_for(&self.dance, &self.tune);
        self.prefs.last_dance = Some(self.dance.source.clone());
        self.picker.dance_preview = None;
        self.picker.dance_selected = None;
        self.screen = Screen::Session;
        self.mark_render_pending_other();
        self.arrangement = None;
        self.audio.stop();
        self.speech.stop();
        self.started = false;
        self.rotation.passes = 0;
        self.rotation.last_beat = 0.0;
        // Give a newly selected dance a fresh shuffle of alternative aliases.
        self.alias_seed = self.alias_seed.wrapping_add(1);
        self.timeline_basis = None;
        self.rebuild_timeline();
        self.save_prefs();
        self.status = "Dance selected".into();
    }

    fn use_tune(&mut self, tune: Tune) {
        self.error = None;
        self.share = None;
        self.pending_count_in = None;
        self.download_tune(tune.clone());
        self.tune = tune;
        self.length_warning = length_warning_for(&self.dance, &self.tune);
        self.prefs.last_tune = Some(self.tune.source.clone());
        self.picker.tune_preview = None;
        self.tune_sheet = None;
        self.picker.tune_selected = None;
        self.screen = Screen::Session;
        self.mark_render_pending_other();
        self.arrangement = None;
        self.audio.stop();
        self.speech.stop();
        self.started = false;
        self.rotation.passes = 0;
        self.rotation.last_beat = 0.0;
        self.timeline_basis = None;
        self.rotation_settings_changed();
        self.save_prefs();
        self.status = "Tune selected".into();
    }

    /// The payload a share link carries: the dance and the ordered tune set.
    ///
    /// The current tune is prepended when it is not a member of the set, so the
    /// recipient hears the same opening tune even when the caller was playing
    /// something outside their rotation.
    fn share_payload(&self) -> SharePayload {
        let mut tunes = self.rotation_tunes();
        let mut current = tunes.iter().position(|tune| same_tune(tune, &self.tune));
        if current.is_none() {
            tunes.insert(0, self.tune.clone());
            current = Some(0);
        }
        SharePayload {
            schema_version: share::SCHEMA_VERSION,
            dance: self.dance.clone(),
            tunes,
            current: current.unwrap_or(0),
            rotation: self.prefs.tune_rotation.enabled,
            passes_per_tune: self.prefs.tune_rotation.passes_per_tune,
        }
    }

    /// Build the page link for the current session.
    fn share_link(&self) -> Result<String, String> {
        let code = share::encode(&self.share_payload())?;
        Ok(share_link_for(&code))
    }

    /// Open the panel and copy the new link to the clipboard.
    fn open_share_link(&mut self) {
        match self.share_link() {
            Ok(link) => {
                self.share = Some(SharePanel {
                    link,
                    message: String::new(),
                });
                self.copy_share_link();
            }
            Err(error) => {
                self.error = Some(format!("Could not create a share link: {error}"));
            }
        }
    }

    /// Copy the open panel's link, reporting the result inside the panel.
    fn copy_share_link(&mut self) {
        let Some(panel) = self.share.as_mut() else {
            return;
        };
        panel.message = match copy_link_text(&panel.link) {
            Ok(()) => "Link copied to the clipboard.".to_string(),
            Err(error) => {
                format!("Could not copy automatically: {error}. Select the link and copy it.")
            }
        };
    }

    /// Load a share argument: rebuild the dance, libraries, and tune set.
    ///
    /// Returns the status line shown to the user. A rejected link leaves the
    /// current session exactly as it was, because decoding and validation
    /// finish before anything is changed.
    ///
    /// Only the browser build can receive a URL; the tests call it directly.
    #[cfg(any(target_arch = "wasm32", test, feature = "headless"))]
    fn apply_share_code(&mut self, code: &str) -> Result<String, String> {
        let payload = share::decode(code)?;
        self.apply_share_payload(payload)
    }

    #[cfg(any(target_arch = "wasm32", test, feature = "headless"))]
    fn apply_share_payload(&mut self, payload: SharePayload) -> Result<String, String> {
        payload.validate()?;
        let SharePayload {
            dance,
            tunes,
            current,
            rotation,
            passes_per_tune,
            ..
        } = payload;

        // A link is hand-editable, so resolve the opening tune before dropping
        // repeated source references; the index could otherwise move.
        let opening = tunes.get(current).map(|tune| tune.source.clone());
        let mut seen = std::collections::BTreeSet::new();
        let tunes: Vec<Tune> = tunes
            .into_iter()
            .filter(|tune| seen.insert((tune.source.provider.clone(), tune.source.id.clone())))
            .collect();
        if tunes.is_empty() {
            return Err("The shared session has no tunes.".to_string());
        }
        let chosen = opening
            .and_then(|source| tunes.iter().find(|tune| tune.source == source).cloned())
            .unwrap_or_else(|| tunes[0].clone());

        let now = library::now_seconds();
        for tune in &tunes {
            self.tune_library.insert(tune.clone(), None, now);
        }
        self.save_tune_library();

        self.prefs.tune_rotation.tunes.clear();
        for tune in &tunes {
            self.prefs.tune_rotation.add(tune.source.clone());
        }
        self.prefs.tune_rotation.source = TuneSetSource::List;
        self.prefs.tune_rotation.enabled = rotation && tunes.len() >= 2;
        self.prefs.tune_rotation.passes_per_tune = passes_per_tune;
        self.prefs.tune_rotation.validate();

        // These save the library again and reset the transport, timeline, and
        // render state the same way a picker selection does.
        self.use_dance(dance);
        self.use_tune(chosen);
        self.rotation_settings_changed();
        self.save_prefs();

        let message = format!(
            "Opened a shared session: \"{}\" with {} tune{}.",
            self.dance.title,
            tunes.len(),
            if tunes.len() == 1 { "" } else { "s" }
        );
        self.error = None;
        self.status = message.clone();
        self.library_status = Some(message.clone());
        Ok(message)
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
            self.timeline_basis = None;
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
            let _ = export_midi_with_band;
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
            let arrangement = Arrangement::from_score_with_bowing(
                &score,
                64.0,
                self.prefs.bowing_options.clone(),
            );
            let default_name = format!("{}.mid", files::safe_filename(&self.tune.title));
            let Some(path) = rfd::FileDialog::new()
                .set_file_name(default_name)
                .add_filter("MIDI", &["mid"])
                .save_file()
            else {
                return;
            };
            let variation_passes = if self.prefs.accompaniment_variations {
                caller_music::VARIATION_PASSES
            } else {
                1
            };
            let bytes = export_midi_with_band(
                &arrangement,
                &self.prefs.band_slots(),
                self.prefs.accompaniment_rhythm,
                self.prefs.percussion_rhythm,
                self.prefs.tempo_bpm,
                variation_passes,
                &self.prefs.variation_options,
                self.prefs.effects,
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
                Ok(bytes) => match caller_music::soundfont::load_soundfont(&bytes) {
                    Ok(soundfont) => {
                        let name = path
                            .file_stem()
                            .and_then(|name| name.to_str())
                            .unwrap_or("SoundFont")
                            .to_string();
                        self.install_user_soundfont(
                            soundfont,
                            name,
                            Some(path.to_string_lossy().to_string()),
                        );
                    }
                    Err(error) => self.soundfont_error = Some(error.to_string()),
                },
                Err(error) => self.soundfont_error = Some(error.to_string()),
            }
        }
    }

    /// Install an already parsed user bank.
    ///
    /// Kept separate from the file dialog so tests can exercise the memory
    /// consequences (the bundled font is dropped and reloaded lazily) without
    /// opening a picker. The file picker only exists on desktop, so the
    /// platforms without one do not call this.
    #[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
    fn install_user_soundfont(
        &mut self,
        soundfont: Arc<SoundFont>,
        name: String,
        path: Option<String>,
    ) {
        self.soundfont = Some(soundfont);
        self.prefs.soundfont = Some(caller_core::SoundFontSetting { name, path });
        self.mark_render_pending_other();
        // The bundled font is still lazily reloadable; keeping it parsed while
        // a user bank is active would retain two sample sets in memory.
        self.bundled_soundfont = None;
        self.bundled_soundfont_checked = false;
        self.soundfont_error = None;
        self.save_prefs();
    }

    fn remove_soundfont(&mut self) {
        self.soundfont = None;
        self.prefs.soundfont = None;
        self.mark_render_pending_other();
        // `active_soundfont` parses the bundled subset on the next render.
        self.bundled_soundfont = None;
        self.bundled_soundfont_checked = false;
        self.save_prefs();
    }
}

impl CallerApp {
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
        self.updates.poll();
        self.poll_imported_files();
        self.poll_prefetch();
        self.handle_media_keys();

        // The audio position is the only clock: speech is triggered from the
        // rendered timeline, never from wall-clock time.
        let mut beat = self.current_beat();
        if self.pending_count_in.is_some() {
            // A count-in toggle that waited for the loop start applies both
            // halves at the boundary, before this frame's call decision.
            self.resolve_pending_count_in(beat);
            beat = self.current_beat();
        }
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
        if self.pending_audio && self.pending_count_in.is_none() && self.started && !dragging {
            // A settings change can render on a worker while the old loop
            // keeps playing. The next tick installs the cached buffer at the
            // current beat, so the control never freezes the UI. The web
            // build has no worker and keeps the synchronous path.
            let can_defer = !self.headless && cfg!(not(target_arch = "wasm32"));
            if can_defer && !self.cached_render_matches() {
                if self.prefetch.is_none() {
                    let key = self.render_key();
                    if self.prefetch_failed_key.as_ref() == Some(&key) {
                        // The background attempt failed; report the real error
                        // through the synchronous path.
                        self.apply_audio_settings();
                    } else {
                        self.start_prefetch();
                        self.status = "Preparing the new settings…".into();
                    }
                }
            } else {
                self.apply_audio_settings();
            }
        }

        self.sync_media_session();

        // Keep the loop warm before the user asks for it. The headless
        // renderer stays deterministic and skips this: it drives the same app
        // without an audio device and its tests stage the buffer themselves.
        #[cfg(not(target_arch = "wasm32"))]
        if !self.headless
            && self.needs_render
            && !self.audio.is_playing()
            && self.pending_count_in.is_none()
        {
            self.start_prefetch();
        }

        // Keep the event loop waking while there is something to advance. A
        // hidden window is throttled by eframe to one pass per 100 ms, but each
        // pass still reads the audio clock, so the caller stays on the beat.
        if self.audio.is_playing()
            || self.render_loading
            || self.speech.is_busy()
            || self.prefetch.is_some()
            || self.updates.is_checking()
        {
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
        let previous_screen = self.screen;
        let previous_prefs_page = self.prefs_page;

        if !self.headless && !ctx.egui_wants_keyboard_input() {
            self.handle_shortcuts(ui);
        }

        let beat = self.current_beat();

        let compact = compact_layout(ui.available_width());
        let header = header_ui(
            ui,
            self.screen,
            self.prefs.animations_enabled,
            compact,
            self.prefs.glossary_enabled,
        );
        if let Some(screen) = header.screen {
            self.screen = screen;
        }
        if header.preferences {
            self.screen = Screen::Preferences;
        }
        if !self.headless
            && self.screen == Screen::Preferences
            && previous_screen != Screen::Preferences
        {
            // Entering Preferences starts at the category list on a phone and
            // at the first category on a desktop, never halfway down a page the
            // user left earlier. A search belongs to one visit, so its results
            // do not hide the categories the next time Preferences opens.
            self.prefs_page = ui::preferences::PreferencesPage::Categories;
            self.prefs_query.clear();
            self.prefs_reveal = None;
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
        // for this frame is complete. A press that landed anywhere but on a
        // slider puts its guard back, so the next swipe cannot move a level.
        ui::finish_slider_frame(ui);
        self.tab_swipe(ui, content_rect);

        // An armed slider belongs to the page that showed it. Leaving the
        // screen or the Preferences category restores every guard.
        if self.screen != previous_screen || self.prefs_page != previous_prefs_page {
            ui::disarm_sliders(ui);
        }

        // A touch definition card belongs to the screen that opened it. Keep it
        // out of the next screen and draw it last so no scroll area clips it.
        if self.screen != previous_screen || !self.prefs.glossary_enabled {
            ui::clear_tooltip(&ctx);
        }
        if self.prefs.glossary_enabled {
            ui::glossary_tooltip(&ctx, header.rect.bottom() + 8.0);
        }

        // Leaving the editor by any route discards the draft. Save and Restore
        // write the library themselves before changing the screen.
        if self.screen != Screen::AbcEditor {
            self.abc_editor = None;
        }

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

    /// Build the per-category memory table shown in Preferences → Maintenance.
    ///
    /// Everything the app can read a length from is measured; font atlases and
    /// library serialization are labelled estimates. The table is cheap enough
    /// to rebuild on demand, and is not rebuilt per frame.
    pub fn memory_report(&self, ctx: Option<&egui::Context>) -> crate::memory::MemoryReport {
        use crate::memory::{MemoryReport, sample_bytes, vec_bytes};

        let mut report = MemoryReport::new();
        report.add(
            "bundled_soundfont_file",
            crate::defaults::default_soundfont_bytes(),
            false,
            "Fluid R3 GM subset in the executable image",
        );

        let mut parsed_font = 0u64;
        for font in [self.soundfont.as_ref(), self.bundled_soundfont.as_ref()]
            .into_iter()
            .flatten()
        {
            // rustysynth stores the sample data as i16 and keeps the decoded
            // instrument/preset tables alongside it.
            parsed_font += (font.get_wave_data().len() as u64) * 2;
            parsed_font += (font.get_sample_headers().len() as u64)
                * std::mem::size_of::<rustysynth::SampleHeader>() as u64;
            parsed_font += (font.get_presets().len() as u64)
                * std::mem::size_of::<rustysynth::Preset>() as u64;
        }
        report.add(
            "soundfont_samples",
            parsed_font,
            false,
            "decoded 16-bit samples and preset tables",
        );

        report.add(
            "audio_buffer",
            sample_bytes(self.audio.buffer_samples()),
            false,
            "count-in + variation passes, interleaved stereo f32",
        );
        report.add(
            "prepared_tune",
            self.rotation
                .prepared
                .as_ref()
                .map_or(0, |prepared| sample_bytes(prepared.buffer.samples.len())),
            false,
            "next tune rendered ahead for Cycle tunes",
        );
        report.add(
            "speech_audio",
            sample_bytes(self.audio.speech_buffer_samples()),
            false,
            "the call currently mixed into playback",
        );
        report.add(
            "speech_cache",
            self.speech.speech_cache_bytes(),
            true,
            "prepared deliveries for repeated calls; the disk copy is separate",
        );

        let arrangement = self.arrangement.as_ref().map_or(0, |arrangement| {
            vec_bytes(&arrangement.melody)
                + vec_bytes(&arrangement.pickup)
                + vec_bytes(&arrangement.chords)
                + arrangement
                    .warnings
                    .iter()
                    .map(|warning| warning.len() as u64)
                    .sum::<u64>()
        });
        report.add(
            "arrangement",
            arrangement,
            false,
            "parsed notes and chords for the current tune",
        );

        let calls = &self.timeline.calls;
        let timeline = vec_bytes(calls)
            + calls
                .iter()
                .map(|call| (call.text.len() + call.original_text.len()) as u64)
                .sum::<u64>()
            + vec_bytes(&self.timeline.phrases)
            + self
                .timeline
                .phrases
                .iter()
                .map(|phrase| phrase.label.len() as u64)
                .sum::<u64>();
        report.add(
            "call_timeline",
            timeline,
            false,
            "scheduled calls, phrases, and their text",
        );

        let library = serde_json::to_vec(&self.dance_library)
            .map(|json| json.len() as u64)
            .unwrap_or(0)
            + serde_json::to_vec(&self.tune_library)
                .map(|json| json.len() as u64)
                .unwrap_or(0);
        report.add(
            "saved_libraries",
            library,
            true,
            "serialized dance and tune cards; allocation overhead is not included",
        );

        let prefs = serde_json::to_vec(&self.prefs)
            .map(|json| json.len() as u64)
            .unwrap_or(0);
        report.add(
            "preferences",
            prefs,
            true,
            "preferences and alias table, serialized size",
        );

        let picker = if self.picker.dance_results.is_some() {
            u64::try_from(self.picker.dance_query.page_size).unwrap_or(0) * 256
        } else {
            0
        } + if self.picker.tune_results.is_some() {
            u64::try_from(self.picker.tune_query.page_size).unwrap_or(0) * 256
        } else {
            0
        } + serde_json::to_vec(&self.picker.dance_preview)
            .map(|json| json.len() as u64)
            .unwrap_or(0)
            + serde_json::to_vec(&self.picker.tune_preview)
                .map(|json| json.len() as u64)
                .unwrap_or(0);
        report.add(
            "picker_state",
            picker,
            true,
            "search result summaries and the selected preview",
        );

        let mut fonts = 0u64;
        if let Some(ctx) = ctx {
            let (atlas, font_data) = ctx.fonts(|fonts| {
                let [width, height] = fonts.font_image_size();
                let data = fonts
                    .definitions()
                    .font_data
                    .values()
                    .map(|font| font.font.as_ref().as_ref().len() as u64)
                    .sum::<u64>();
                ((width as u64) * (height as u64) * 4, data)
            });
            fonts = atlas + font_data;
        }
        report.add(
            "fonts",
            fonts,
            false,
            "glyph atlas (RGBA) and embedded font files",
        );

        let sheet = self
            .tune_sheet
            .as_ref()
            .and_then(|sheet| sheet.as_ref().ok())
            .map_or(0, crate::sheet::SheetPreview::texture_bytes);
        report.add(
            "sheet_music_texture",
            sheet,
            false,
            "rasterized preview of the selected tune",
        );

        report.add(
            "undo_history",
            self.history.estimated_bytes(),
            true,
            "preferences and card snapshots on the undo and redo stacks",
        );
        report.add(
            "source_cache",
            self.source.cache_memory_bytes() as u64,
            false,
            "in-memory source responses; the desktop file cache is disk",
        );
        report.add(
            "neural_tts",
            self.speech.neural_working_set_bytes(),
            true,
            "quantized Kokoro model, runtime, and per-inference arenas",
        );
        report
    }
}

impl eframe::App for CallerApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        #[cfg(target_os = "android")]
        {
            // The activity forwards the system keyboard through JNI. Events
            // land here, before egui starts the frame, so the focused text
            // field applies them exactly once.
            raw_input.events.extend(crate::android_ime::drain_events());
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

        // Focus is settled by the end of the frame, so this is where a field
        // that was just tapped asks the activity for the system keyboard.
        // The bridge is the only Android IME controller; egui's own IME output
        // would make winit show and hide the same keyboard again (see
        // `suppress_backend_ime`).
        #[cfg(target_os = "android")]
        {
            crate::android_ime::suppress_backend_ime(ui.ctx());
            crate::android_ime::sync_text_input(ui.ctx());
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
        #[cfg(target_os = "android")]
        let _ = crate::media_keys::set_text_input_active(false);
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
    /// Wide layout: dance information and the beat meter on the left, the
    /// dance card on the right, after the controls.
    split: bool,
}

fn session_layout(width: f32, height: f32) -> SessionLayout {
    let compact = compact_layout(width);
    let page_scroll =
        (compact && height < SESSION_PAGE_SCROLL_HEIGHT) || height < SESSION_SHORT_HEIGHT;
    SessionLayout {
        compact,
        page_scroll,
        split: !compact && !page_scroll,
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
    /// The Preferences button carries its text label, not just the gear.
    preferences_label: bool,
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
    /// The Preferences button's rect. Test-only.
    #[cfg(test)]
    preferences_rect: Option<egui::Rect>,
}

impl Default for HeaderResponse {
    fn default() -> Self {
        Self {
            screen: None,
            preferences: false,
            preferences_label: false,
            rect: egui::Rect::NOTHING,
            heading: None,
            #[cfg(test)]
            tabs: Vec::new(),
            #[cfg(test)]
            preferences_rect: None,
        }
    }
}

/// Draw the tab row: the pinned Preferences button, the wordmark, and the tabs.
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
/// Where room is short, the header gives up its least useful parts in order:
/// the Preferences text first, then the decorative wordmark. The tabs and the
/// gear always keep one row, so the header height stays the same from the
/// phone minimum to a large desktop.
fn header_ui(
    ui: &mut egui::Ui,
    active: Screen,
    animate: bool,
    compact: bool,
    glossary_enabled: bool,
) -> HeaderResponse {
    let mut header = HeaderResponse::default();
    let available_width = (ui.available_width() - 24.0).max(0.0);
    let spacing = ui.spacing().item_spacing.x;
    let gear_side = ui.spacing().interact_size.y;
    let tabs: Vec<&TabDef> = visible_tabs(glossary_enabled).collect();
    let heading_width = if compact {
        0.0
    } else {
        let font = egui::TextStyle::Heading.resolve(ui.style());
        ui.painter()
            .layout_no_wrap(
                "Caller Karaoke".to_owned(),
                font,
                egui::Color32::PLACEHOLDER,
            )
            .size()
            .x
            + 12.0
    };
    let tab_widths: Vec<f32> = tabs
        .iter()
        .map(|tab_def| {
            let label = if compact {
                tab_def.short_label
            } else {
                tab_def.label
            };
            tab_desired_size(ui, label, compact).x
        })
        .collect();
    let label_extra = preferences_label_width(ui) + PREFERENCES_LABEL_PADDING;
    let show_preferences_label = preferences_label_fits(
        compact,
        available_width,
        header_content_width(heading_width, &tab_widths, gear_side, spacing),
        label_extra,
        spacing,
    );
    // The wordmark is decorative and the tabs are navigation, so a row that
    // cannot hold both drops the wordmark rather than wrapping onto a second
    // row. This is the same trade the phone layout makes, and it keeps the
    // four tabs plus the gear on one line at the narrowest desktop width.
    let heading_width = if compact
        || header_content_width(heading_width, &tab_widths, gear_side, spacing) > available_width
    {
        0.0
    } else {
        heading_width
    };
    header.preferences_label = show_preferences_label;
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
                // the gear button so four short tabs fit beside it. This is
                // the panel's own ui, so the tweak ends with the header.
                ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
                ui.spacing_mut().button_padding.x = 8.0;
            }
            // The Preferences gear is pinned to the top-right corner and the
            // wordmark and tabs share the remaining width.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let gear = preferences_gear(ui, active, show_preferences_label);
                if gear.clicked() {
                    header.preferences = true;
                }
                #[cfg(test)]
                {
                    header.preferences_rect = Some(gear.rect);
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        // At phone width the four tabs, not the wordmark, are
                        // the useful part of the header. Dropping the heading
                        // and the long tab labels keeps the navigation to one
                        // row and leaves that height for the prompt and card.
                        if heading_width > 0.0 {
                            let heading = ui.heading("Caller Karaoke");
                            header.heading = Some(heading.rect);
                            ui.add_space(12.0);
                        }
                        for tab_def in &tabs {
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

/// The size a tab needs for `label`.
///
/// The header measures the row with this before laying it out, and `tab` uses
/// it to allocate, so the decision to show the Preferences text and the drawn
/// row cannot disagree.
fn tab_desired_size(ui: &egui::Ui, label: &str, compact: bool) -> egui::Vec2 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, egui::Color32::PLACEHOLDER);
    // Narrower tabs leave room for the pinned Preferences gear on a phone.
    let horizontal_padding = if compact { 16.0 } else { 24.0 };
    egui::vec2(
        galley.size().x + horizontal_padding,
        (galley.size().y + 16.0).max(ui.spacing().interact_size.y),
    )
}

/// The width of the header row without the Preferences button's text.
fn header_content_width(wordmark: f32, tabs: &[f32], gear: f32, spacing: f32) -> f32 {
    let gaps = tabs.len() + usize::from(wordmark > 0.0);
    wordmark + tabs.iter().sum::<f32>() + gear + spacing * gaps as f32
}

/// Whether the "Preferences" text fits beside the tabs and wordmark.
///
/// The label is only added when the whole row still fits the panel: a wrapped
/// header steals height from the session and, at 640 points, can push the tabs
/// onto a second row. Compact widths keep the icon-only button.
fn preferences_label_fits(
    compact: bool,
    available_width: f32,
    content_without_label: f32,
    label_extra: f32,
    spacing: f32,
) -> bool {
    !compact && content_without_label + label_extra + spacing <= available_width
}

/// The word "Preferences" in the header button's font.
fn preferences_label_width(ui: &egui::Ui) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    ui.painter()
        .layout_no_wrap(
            PREFERENCES_LABEL.to_owned(),
            font,
            egui::Color32::PLACEHOLDER,
        )
        .size()
        .x
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
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        egui::TextStyle::Button.resolve(ui.style()),
        egui::Color32::PLACEHOLDER,
    );
    let desired = tab_desired_size(ui, label, compact);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    // A tooltip needs a hovering mouse, and a mouse cannot swipe a tab: only
    // offer the gesture once the app has seen a touch screen.
    let hint = if ui.input(|input| input.has_touch_screen()) {
        "swipe or use ← → to change tabs"
    } else {
        "use ← → to change tabs"
    };
    let response = response.on_hover_text(format!("{hover} ({shortcut}) · {hint}"));

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
/// Text shown in the header's Preferences button when it fits.
const PREFERENCES_LABEL: &str = "Preferences";
/// Space between the text and the gear, and the button's end padding.
const PREFERENCES_LABEL_PADDING: f32 = 10.0;

fn preferences_gear(ui: &mut egui::Ui, active: Screen, show_label: bool) -> egui::Response {
    // Square and one row high: 40 points on the desktop, and the full
    // 44-point touch target in the phone header, which raises
    // `interact_size.y`.
    let side = ui.spacing().interact_size.y;
    let label = show_label.then(|| {
        ui.painter().layout_no_wrap(
            PREFERENCES_LABEL.to_owned(),
            egui::TextStyle::Button.resolve(ui.style()),
            egui::Color32::PLACEHOLDER,
        )
    });
    let label_width = label
        .as_ref()
        .map(|galley| galley.size().x + PREFERENCES_LABEL_PADDING)
        .unwrap_or(0.0);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(side + label_width, side), egui::Sense::click());
    let response = response.on_hover_text("Preferences (5)");
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
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() - side, rect.top()),
            egui::vec2(side, side),
        );
        if let Some(galley) = label {
            let text_pos = egui::pos2(
                rect.left() + PREFERENCES_LABEL_PADDING / 2.0,
                rect.center().y - galley.size().y / 2.0,
            );
            ui.painter()
                .galley_with_override_text_color(text_pos, galley, visuals.fg_stroke.color);
        }
        // A glyph would not do: the bundled monochrome emoji font has no
        // U+2699 GEAR, so a "⚙" label renders as an empty box everywhere.
        paint_gear(ui, icon_rect, visuals.fg_stroke.color);
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
            .unwrap_or_else(|_| Cache::new(caller_sources::MemoryCache::default()))
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

/// The count-in length the preferences ask for, in dance beats.
fn count_in_for(prefs: &Preferences) -> f64 {
    if prefs.count_in_enabled {
        COUNT_IN_BEATS
    } else {
        0.0
    }
}

/// How a count-in change is applied to a transport.
#[derive(Clone, Copy, Debug, PartialEq)]
enum CountInPlan {
    /// Rebuild the timeline and re-render the music now, resuming at this beat.
    ApplyNow {
        /// Position to resume at, or `None` when the transport has no material
        /// to re-render yet and the next Play will render it.
        resume_beat: Option<f64>,
    },
    /// The old count-in is already running. Let it finish, then change both the
    /// timeline and the music at the loop start instead of cutting it short or
    /// starting the dance at beat 0 in the middle of the count.
    DeferUntilLoop,
}

/// Decide how a count-in change meets the transport.
///
/// The only case that cannot be applied at once is shortening the count-in while
/// its beats are already playing: seeking into a shorter (or absent) count-in
/// clamps to the loop start, which would restart the dance mid-count. Waiting
/// for the loop start applies both halves together and keeps what the dancers
/// already heard.
fn plan_count_in_change(
    playing: bool,
    beat: f64,
    old_count_in: f64,
    new_count_in: f64,
) -> CountInPlan {
    if playing && beat < 0.0 && new_count_in < old_count_in {
        CountInPlan::DeferUntilLoop
    } else {
        CountInPlan::ApplyNow {
            resume_beat: Some(count_in_resume_beat(beat, new_count_in)),
        }
    }
}

/// The nearest beat at which a buffer with `count_in_beats` can resume.
///
/// A loop position is preserved exactly. A pre-loop position is kept while the
/// new count-in is long enough and otherwise moves forward to the loop start;
/// it never rewinds to the beginning of a count-in.
fn count_in_resume_beat(beat: f64, count_in_beats: f64) -> f64 {
    if beat >= 0.0 {
        beat
    } else {
        beat.max(-count_in_beats.max(0.0))
    }
}

/// Whether a rebuilt timeline should announce the first call at `beat`.
///
/// A rebuild can move the first call's lead into the past or drop the count-in
/// around it. Only the top of the run is eligible, and only while that call has
/// not already been spoken, so the first move is neither lost nor repeated.
fn should_announce_first_call(
    timeline: &PracticeTimeline,
    beat: f64,
    last_spoken: Option<f64>,
) -> bool {
    beat <= 0.0
        && timeline.starting_call(beat).is_some()
        && timeline
            .calls
            .first()
            .is_none_or(|call| last_spoken != Some(call.call_start_beat))
}

fn build_timeline(
    dance: &Dance,
    prefs: &Preferences,
    alias_seed: u64,
) -> Result<PracticeTimeline, String> {
    build_timeline_with_count_in(dance, prefs, alias_seed, count_in_for(prefs))
}

fn build_timeline_with_count_in(
    dance: &Dance,
    prefs: &Preferences,
    alias_seed: u64,
    count_in_beats: f64,
) -> Result<PracticeTimeline, String> {
    PracticeTimeline::build(
        dance,
        &prefs.aliases,
        &prefs.role_terms,
        &prefs.lead,
        prefs.tempo(),
        TimelineConfig {
            count_in_beats,
            music_loop_beats: 64.0,
            shuffle_alternatives: prefs.shuffle_alternatives,
            alias_seed,
        },
    )
    .map_err(|error| format!("Could not build the call timeline: {error}"))
}

/// A fallback timeline for a card that cannot produce calls.
///
/// The count-in comes from the caller's setting, never a hardcoded four beats:
/// a failed build must not silently turn the count-in back on.
fn empty_timeline(count_in_beats: f64) -> PracticeTimeline {
    PracticeTimeline {
        calls: Vec::new(),
        phrases: Vec::new(),
        total_beats: 64.0,
        music_loop_beats: 64.0,
        count_in_beats: count_in_beats.max(0.0),
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

/// Whether two tunes are the same saved item.
fn same_tune(a: &Tune, b: &Tune) -> bool {
    a.source.provider == b.source.provider && a.source.id == b.source.id
}

/// Turn a code into the address the user shares.
///
/// The browser build keeps the page's own origin and path, so a bundle hosted
/// under a subpath still produces a working link. Other builds cannot know the
/// address; they return the bare argument, which the tests use.
fn share_link_for(code: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(base) = crate::web_store::page_url() {
            return share::link_for(&base, code);
        }
    }
    share::link_for("", code)
}

/// Put the link on the clipboard. Only the browser build has one.
fn copy_link_text(link: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        crate::web_store::copy_text(link)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = link;
        Err("the clipboard is only available in the web build".to_string())
    }
}

#[cfg(test)]
mod tests_layout;
#[cfg(test)]
mod tests_memory;
#[cfg(test)]
mod tests_share;
#[cfg(test)]
mod tests_state;

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
        prefs.role_terms = caller_core::RoleTerms {
            left_singular: "port".into(),
            left_plural: "ports".into(),
            right_singular: "starboard".into(),
            right_plural: "starboards".into(),
        };
        let timeline = build_timeline(&dance, &prefs, 0).expect("timeline");
        // A lady is a robin-side dancer, so the card's "ladles" uses the
        // custom right-side term, not the left-side one.
        assert!(
            timeline
                .calls
                .iter()
                .any(|call| call.text.contains("starboards allemande")),
            "{:?}",
            call_texts(&timeline)
        );
        assert!(
            !timeline
                .calls
                .iter()
                .any(|call| call.text.contains("robins")),
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
        const ALL: [Screen; 9] = [
            Screen::Session,
            Screen::DancePicker,
            Screen::DanceDownload,
            Screen::TunePicker,
            Screen::TuneDownload,
            Screen::AbcEditor,
            Screen::Preferences,
            Screen::Aliases,
            Screen::Glossary,
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
            assert_eq!(screen_for_number_key(tab.key, true), Some(tab.landing()));
        }
    }

    /// The glossary is a top-level reference tab to the right of Select tune,
    /// and Preferences stays a header button.
    #[test]
    fn the_glossary_is_a_top_tab_and_preferences_is_a_header_button() {
        assert_eq!(
            TABS.len(),
            4,
            "the top row is Session, dance, tune, and the glossary"
        );
        assert_eq!(TABS[3].landing(), Screen::Glossary);
        for tab in &TABS {
            assert!(
                !tab.is_active(Screen::Preferences),
                "{} must not cover Preferences",
                tab.label
            );
        }
        assert_eq!(
            screen_for_number_key(PREFERENCES_KEY, true),
            Some(Screen::Preferences)
        );
    }

    #[test]
    fn number_keys_select_top_tabs() {
        assert_eq!(
            screen_for_number_key(egui::Key::Num1, true),
            Some(Screen::Session)
        );
        assert_eq!(
            screen_for_number_key(egui::Key::Num2, true),
            Some(Screen::DancePicker)
        );
        assert_eq!(
            screen_for_number_key(egui::Key::Num3, true),
            Some(Screen::TunePicker)
        );
        assert_eq!(
            screen_for_number_key(egui::Key::Num4, true),
            Some(Screen::Glossary)
        );
        assert_eq!(
            screen_for_number_key(egui::Key::Num5, true),
            Some(Screen::Preferences)
        );
        assert_eq!(screen_for_number_key(egui::Key::Num6, true), None);
        assert_eq!(screen_for_number_key(egui::Key::Space, true), None);
    }

    /// Turning the glossary off removes its tab, so the number key does not
    /// reopen a screen the rest of the reference is no longer offering.
    #[test]
    fn the_glossary_tab_disappears_when_the_reference_is_off() {
        assert_eq!(visible_tabs(false).count(), 3);
        assert!(visible_tabs(false).all(|tab| !tab.is_active(Screen::Glossary)));
        assert_eq!(screen_for_number_key(egui::Key::Num4, false), None);
        assert_eq!(
            screen_for_number_key(egui::Key::Num5, false),
            Some(Screen::Preferences)
        );
        // The row wraps, so Right from Select tune returns to Session instead
        // of an unreachable glossary tab.
        assert_eq!(tab_step(Screen::TunePicker, 1, false), Screen::Session);
        assert_eq!(tab_step(Screen::Session, -1, false), Screen::TunePicker);
    }

    #[test]
    fn arrow_keys_step_through_tabs_and_wrap() {
        assert_eq!(tab_step(Screen::Session, 1, true), Screen::DancePicker);
        assert_eq!(tab_step(Screen::DancePicker, 1, true), Screen::TunePicker);
        assert_eq!(tab_step(Screen::TunePicker, 1, true), Screen::Glossary);
        assert_eq!(tab_step(Screen::Glossary, 1, true), Screen::Session);
        assert_eq!(tab_step(Screen::Session, -1, true), Screen::Glossary);
        assert_eq!(tab_step(Screen::Glossary, -1, true), Screen::TunePicker);
        assert_eq!(tab_step(Screen::TunePicker, -1, true), Screen::DancePicker);
        // A detail screen steps from the tab that contains it.
        assert_eq!(tab_step(Screen::DanceDownload, 1, true), Screen::TunePicker);
        assert_eq!(tab_step(Screen::DanceDownload, -1, true), Screen::Session);
        // The ABC editor belongs to the Select tune tab as well.
        assert_eq!(tab_step(Screen::AbcEditor, 1, true), Screen::Glossary);
        assert_eq!(tab_step(Screen::AbcEditor, -1, true), Screen::DancePicker);
        // The alias editor is reached from the Preferences gear, not a tab;
        // stepping from it starts at the first tab.
        assert_eq!(tab_step(Screen::Aliases, 1, true), Screen::DancePicker);
    }

    #[test]
    fn a_press_on_a_control_owns_the_gesture() {
        assert!(swipe_owns_press(egui::Sense::click(), 400.0));
        assert!(swipe_owns_press(egui::Sense::click_and_drag(), 400.0));
        assert!(
            swipe_owns_press(egui::Sense::drag(), 10.0),
            "a pane splitter keeps its drag"
        );
        assert!(
            !swipe_owns_press(egui::Sense::drag(), 400.0),
            "a wide scroll area still allows a horizontal tab swipe"
        );
        assert!(!swipe_owns_press(egui::Sense::hover(), 400.0));
    }

    #[test]
    fn only_a_finger_starts_a_tab_swipe() {
        let touch = |phase, pos| egui::Event::Touch {
            device_id: egui::TouchDeviceId(1),
            id: egui::TouchId(2),
            phase,
            pos,
            force: None,
        };
        // A touch screen reports the finger itself next to the pointer press it
        // derives, so the start event is what identifies a swipe.
        assert!(touch_started(&[
            touch(egui::TouchPhase::Start, egui::pos2(100.0, 100.0)),
            egui::Event::PointerButton {
                pos: egui::pos2(100.0, 100.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]));
        // A mouse press has no touch event, and the moves of a gesture that is
        // already underway are not new presses.
        assert!(!touch_started(&[egui::Event::PointerButton {
            pos: egui::pos2(100.0, 100.0),
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        }]));
        assert!(!touch_started(&[touch(
            egui::TouchPhase::Move,
            egui::pos2(200.0, 100.0)
        )]));
        assert!(!touch_started(&[]));
    }

    #[test]
    fn a_horizontal_swipe_selects_an_adjacent_tab() {
        // Swipe left goes forward, swipe right goes back, and both wrap.
        assert_eq!(
            swipe_target(Screen::Session, -100.0, 10.0, true),
            Some(Screen::DancePicker)
        );
        assert_eq!(
            swipe_target(Screen::Session, 100.0, 10.0, true),
            Some(Screen::Glossary)
        );
        assert_eq!(
            swipe_target(Screen::TunePicker, -100.0, 10.0, true),
            Some(Screen::Glossary)
        );
        assert_eq!(
            swipe_target(Screen::Glossary, -100.0, 10.0, true),
            Some(Screen::Session)
        );
        assert_eq!(
            swipe_target(Screen::TunePicker, 100.0, -10.0, true),
            Some(Screen::DancePicker)
        );
        assert_eq!(
            swipe_target(Screen::Session, 100.0, 10.0, false),
            Some(Screen::TunePicker)
        );
        // A short drag is a tap or slider adjustment, not a swipe.
        assert_eq!(swipe_target(Screen::Session, 60.0, 0.0, true), None);
        assert_eq!(swipe_target(Screen::Session, 59.0, 0.0, true), None);
        // A mostly vertical drag is a scroll.
        assert_eq!(swipe_target(Screen::Session, 80.0, 60.0, true), None);
    }

    /// Run one egui frame with `raw_input` and read the number-key shortcut it
    /// saw.
    fn number_shortcut_for(raw_input: egui::RawInput) -> Option<Screen> {
        let ctx = egui::Context::default();
        let mut screen = None;
        let mut output = ctx.run_ui(raw_input, |ui| {
            screen = ui.input(|input| number_key_shortcut(input, true));
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
            Some(Screen::Glossary)
        );
        assert_eq!(
            number_shortcut_for(key_press(egui::Key::Num5)),
            Some(Screen::Preferences)
        );
        assert_eq!(number_shortcut_for(key_press(egui::Key::Num6)), None);
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
            let response = preferences_gear(ui, *screen, false);
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
            size = preferences_gear(ui, screen, false).rect.size();
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
            std::ptr::fn_addr_eq(last.draw, ABOUT_SECTION),
            "About must be the last Preferences section"
        );
        let occurrences = SECTIONS
            .iter()
            .filter(|section| std::ptr::fn_addr_eq(section.draw, ABOUT_SECTION))
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
        assert_eq!(Screen::AbcEditor.escape_target(), Some(Screen::TunePicker));
        assert_eq!(Screen::Preferences.escape_target(), Some(Screen::Session));
        assert_eq!(Screen::Aliases.escape_target(), Some(Screen::Preferences));
        // The glossary is a tab, so it leaves like the other tabs.
        assert_eq!(Screen::Glossary.escape_target(), Some(Screen::Session));
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
                page_scroll: false,
                split: false
            }
        );
        assert_eq!(
            session_layout(639.0, 900.0),
            SessionLayout {
                compact: true,
                page_scroll: false,
                split: false
            }
        );
        assert_eq!(
            session_layout(390.0, 768.0),
            SessionLayout {
                compact: true,
                page_scroll: true,
                split: false
            }
        );
        assert_eq!(
            session_layout(640.0, 800.0),
            SessionLayout {
                compact: false,
                page_scroll: false,
                split: true
            }
        );
        assert_eq!(
            session_layout(1280.0, 800.0),
            SessionLayout {
                compact: false,
                page_scroll: false,
                split: true
            }
        );
    }

    #[test]
    fn short_phone_sessions_scroll_as_one_page() {
        let layout = session_layout(390.0, 560.0);
        assert!(layout.compact);
        assert!(layout.page_scroll);
        assert!(!layout.split, "a short phone keeps one scrolling column");
        // A landscape phone keeps the wide control row, but its short height
        // still makes the whole page scroll so the dance card is reachable.
        let landscape = session_layout(844.0, 390.0);
        assert!(!landscape.compact);
        assert!(landscape.page_scroll);
        assert!(!landscape.split, "a short wide window cannot split");
    }

    #[test]
    fn wide_sessions_split_into_information_and_card_boxes() {
        for width in [640.0, 1280.0, 1920.0] {
            let layout = session_layout(width, 800.0);
            assert!(!layout.compact, "{width}px is a desktop width");
            assert!(!layout.page_scroll);
            assert!(layout.split, "{width}px should use the two-box session");
        }
        // A tall phone stays stacked even though it does not need to scroll.
        assert!(!session_layout(390.0, 900.0).split);
    }

    #[test]
    fn a_shrinking_count_in_waits_for_the_loop_but_growing_it_does_not() {
        // Turning the count-in off mid-count-in must not seek to the loop start
        // (which would restart the dance two beats early): both changes wait for
        // beat 0 instead.
        assert_eq!(
            plan_count_in_change(true, -2.0, 4.0, 0.0),
            CountInPlan::DeferUntilLoop
        );
        // Turning it on mid-count-in keeps the full count-in.
        assert_eq!(
            plan_count_in_change(true, -2.0, 0.0, 4.0),
            CountInPlan::ApplyNow {
                resume_beat: Some(-2.0)
            }
        );
        // Outside the count-in the loop position is preserved either way.
        assert_eq!(
            plan_count_in_change(true, 12.0, 4.0, 0.0),
            CountInPlan::ApplyNow {
                resume_beat: Some(12.0)
            }
        );
        assert_eq!(
            plan_count_in_change(true, 32.0, 0.0, 4.0),
            CountInPlan::ApplyNow {
                resume_beat: Some(32.0)
            }
        );
        // A paused pre-loop transport has nothing to cut short; it applies now
        // at the loop start rather than replaying part of the count-in.
        assert_eq!(
            plan_count_in_change(false, -2.0, 4.0, 0.0),
            CountInPlan::ApplyNow {
                resume_beat: Some(0.0)
            }
        );
    }

    #[test]
    fn a_resume_beat_never_rewinds_into_a_count_in() {
        assert_eq!(count_in_resume_beat(-2.0, 4.0), -2.0);
        assert_eq!(count_in_resume_beat(-5.0, 2.0), -2.0);
        assert_eq!(count_in_resume_beat(-2.0, 0.0), 0.0);
        assert_eq!(count_in_resume_beat(7.0, 4.0), 7.0);
        assert_eq!(count_in_resume_beat(7.0, 0.0), 7.0);
    }

    #[test]
    fn the_count_in_setting_builds_the_timeline_and_the_fallback() {
        let mut prefs = Preferences::default();
        let dance = crate::defaults::default_dance();
        assert_eq!(
            build_timeline(&dance, &prefs, 0)
                .expect("timeline")
                .count_in_beats,
            COUNT_IN_BEATS
        );
        prefs.count_in_enabled = false;
        assert_eq!(
            build_timeline(&dance, &prefs, 0)
                .expect("timeline")
                .count_in_beats,
            0.0
        );
        // A failed build falls back with the caller's setting, not four beats.
        assert_eq!(empty_timeline(count_in_for(&prefs)).count_in_beats, 0.0);
        prefs.count_in_enabled = true;
        assert_eq!(
            empty_timeline(count_in_for(&prefs)).count_in_beats,
            COUNT_IN_BEATS
        );
    }

    #[test]
    fn a_timeline_without_a_count_in_still_announces_the_first_call_once() {
        let dance = crate::defaults::default_dance();
        let prefs = Preferences {
            count_in_enabled: false,
            ..Preferences::default()
        };
        let timeline = build_timeline(&dance, &prefs, 0).expect("timeline");
        let first = timeline.calls.first().expect("calls");
        assert!(first.call_start_beat < 0.0);

        // At the loop start the normal lead window is over, so the rebuild has
        // to mark the first call pending or the dance opens without a call.
        assert!(should_announce_first_call(&timeline, 0.0, None));
        // Once it has been spoken, a second rebuild must not repeat it.
        assert!(!should_announce_first_call(
            &timeline,
            0.0,
            Some(first.call_start_beat)
        ));
        // The announcement belongs to the top of the run only.
        assert!(!should_announce_first_call(&timeline, first.end_beat, None));
    }

    #[test]
    fn a_failed_timeline_build_keeps_the_old_one() {
        let mut dance = crate::defaults::default_dance();
        dance.figures.clear();
        let prefs = Preferences::default();
        assert!(build_timeline(&dance, &prefs, 0).is_err());
    }

    #[test]
    fn back_from_every_screen_and_preferences_page() {
        let categories = ui::preferences::PreferencesPage::Categories;
        let category = ui::preferences::PreferencesPage::Category(
            ui::preferences::PreferencesCategory::Playback,
        );

        // A phone drills out of a category before it leaves Preferences.
        assert_eq!(
            back_action(Screen::Preferences, category, true),
            Some(BackAction::PreferencesPage(categories))
        );
        assert_eq!(
            back_action(Screen::Preferences, categories, true),
            Some(BackAction::Screen(Screen::Session))
        );
        // A desktop has the category list on screen; Back leaves at once.
        assert_eq!(
            back_action(Screen::Preferences, category, false),
            Some(BackAction::Screen(Screen::Session))
        );
        // Download screens return to their picker; the picker to the session.
        assert_eq!(
            back_action(Screen::DanceDownload, categories, true),
            Some(BackAction::Screen(Screen::DancePicker))
        );
        assert_eq!(
            back_action(Screen::DancePicker, categories, true),
            Some(BackAction::Screen(Screen::Session))
        );
        assert_eq!(
            back_action(Screen::TuneDownload, categories, true),
            Some(BackAction::Screen(Screen::TunePicker))
        );
        assert_eq!(
            back_action(Screen::Aliases, categories, true),
            Some(BackAction::Screen(Screen::Preferences))
        );
        assert_eq!(back_action(Screen::Session, categories, true), None);
        // The glossary is a tab, so it leaves to the session like any tab.
        assert_eq!(
            back_action(Screen::Glossary, categories, true),
            Some(BackAction::Screen(Screen::Session))
        );
    }

    #[test]
    fn the_preferences_text_only_joins_the_gear_when_the_header_fits() {
        let spacing = 8.0;
        let gear = 40.0;
        let label = 78.0;
        let tabs = [96.0, 118.0, 98.0];
        let wordmark = 142.0;
        let content = header_content_width(wordmark, &tabs, gear, spacing);

        // 640 is the widest phone width and the first desktop width. A 640
        // header without the label is the narrowest desktop case; the text is
        // added only when the whole row still fits.
        assert!(!preferences_label_fits(
            true, 640.0, content, label, spacing
        ));
        assert!(!preferences_label_fits(
            false,
            content + label,
            content,
            label,
            spacing
        ));
        assert!(preferences_label_fits(
            false,
            content + label + spacing,
            content,
            label,
            spacing
        ));

        // The real row widths use the same helper as the drawn tabs.
        let ctx = egui::Context::default();
        theme::apply(&ctx, Theme::Dark, false);
        let mut measured = Vec::new();
        let mut output = ctx.run_ui(ui_input(Vec::new()), |ui| {
            measured = TABS
                .iter()
                .map(|tab_def| tab_desired_size(ui, tab_def.label, false).x)
                .collect();
        });
        output.textures_delta.clear();
        assert_eq!(measured.len(), TABS.len());
        assert!(measured.iter().all(|width| *width > 40.0));
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
    fn hidden_app() -> (CallerApp, egui::Context) {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = CallerApp::new_headless(&cc);
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
                    true,
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

        // The phone layout, then an intermediate width. With four tabs and
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
            let gear = header
                .preferences_rect
                .expect("the Preferences button is drawn");
            assert!(
                header.rect.contains_rect(gear) && gear.right() <= width,
                "{width}px: the Preferences button overflows: {gear:?} in {:?}",
                header.rect
            );
            if width < COMPACT_WIDTH {
                assert!(
                    !header.preferences_label,
                    "{width}px: a phone header keeps the icon-only button"
                );
            }
        }
    }

    #[test]
    fn wide_headers_show_the_preferences_text_without_wrapping() {
        let ctx = egui::Context::default();
        theme::apply(&ctx, Theme::Dark, false);

        let wide = run_header(&ctx, egui::vec2(1280.0, 800.0), 3);
        assert!(
            wide[2].preferences_label,
            "a 1280-point header has room for the word"
        );
        let gear = wide[2].preferences_rect.expect("Preferences button");
        assert!(
            gear.width() > gear.height(),
            "the labeled button is wider than its height: {gear:?}"
        );

        let phone = run_header(&ctx, egui::vec2(390.0, 844.0), 3);
        assert!(!phone[2].preferences_label);
        let phone_gear = phone[2].preferences_rect.expect("Preferences button");
        assert!(
            phone_gear.width() <= theme::TOUCH_TARGET + 0.01,
            "the phone button stays square: {phone_gear:?}"
        );

        // At every desktop width the row keeps one line: the header height is
        // the same as a fresh launch at that width, and no item wraps.
        for width in [640.0, 800.0, 1280.0, 1920.0] {
            let frames = run_header(&ctx, egui::vec2(width, 800.0), 3);
            let header = &frames[2];
            assert!(
                header.rect.height() <= 80.0,
                "{width}px: the header wrapped to {:.0} points",
                header.rect.height()
            );
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

    #[test]
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn turning_the_count_in_off_before_play_rebuilds_without_one() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = CallerApp::new_headless(&cc);
        app.needs_render = false;

        app.prefs.count_in_enabled = false;
        app.apply_count_in_change();
        assert_eq!(app.timeline.count_in_beats, 0.0);
        assert!(
            app.needs_render,
            "the next Play must render the tune without a count-in"
        );
        // With no count-in the transport starts on the first execute beat, so
        // the rebuilt timeline still has to announce the first call itself.
        assert!(should_announce_first_call(&app.timeline, 0.0, None));

        app.prefs.count_in_enabled = true;
        app.apply_count_in_change();
        assert_eq!(app.timeline.count_in_beats, COUNT_IN_BEATS);
        assert!(
            app.needs_render,
            "turning it back on renders a count-in on the next run"
        );
    }

    #[test]
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn a_count_in_toggle_mid_count_in_keeps_timeline_and_music_together() {
        let (mut app, _ctx) = hidden_app();
        app.audio.pause();
        app.audio.seek_beat(-2.0);

        // Shortening a count-in that is already playing waits for the loop.
        app.audio.play();
        app.prefs.count_in_enabled = false;
        app.apply_count_in_change();
        assert_eq!(app.pending_count_in, Some(false));
        assert_eq!(
            app.timeline.count_in_beats, COUNT_IN_BEATS,
            "the installed timeline keeps the running count-in"
        );
        assert!(
            app.audio.position_beat() < 0.0,
            "the running count-in is not rewound or restarted: {}",
            app.audio.position_beat()
        );
        // An unrelated rebuild while waiting must not apply it early.
        app.rebuild_timeline();
        assert_eq!(app.timeline.count_in_beats, COUNT_IN_BEATS);

        // At the loop start both the timeline and the music change together.
        app.audio.stop();
        app.started = false;
        app.resolve_pending_count_in(0.0);
        assert_eq!(app.pending_count_in, None);
        assert_eq!(app.timeline.count_in_beats, 0.0);
        assert!(app.needs_render);
    }

    #[test]
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn a_failed_count_in_rebuild_falls_back_without_a_count_in() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = CallerApp::new_headless(&cc);
        app.needs_render = false;
        app.prefs.count_in_enabled = false;
        // A card that cannot produce calls fails the build. The fallback must
        // honour the setting instead of quietly restoring four beats.
        app.dance.figures.clear();
        app.rebuild_timeline_with_count_in(0.0);
        assert!(app.error.is_some());
        assert_eq!(app.timeline.count_in_beats, 0.0);
    }

    #[test]
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn toggling_the_count_in_while_paused_keeps_timeline_and_audio_agreeing() {
        let (mut app, _ctx) = hidden_app();
        app.audio.pause();
        app.audio.seek_beat(12.0);
        app.prefs.count_in_enabled = false;
        app.apply_count_in_change();
        assert_eq!(app.timeline.count_in_beats, 0.0);
        // The installed music has no count-in region: a negative position
        // clamps to the loop start.
        app.audio.seek_beat(-1.0);
        assert_eq!(app.audio.position_beat(), 0.0);
        assert!(!app.audio.is_playing(), "a paused session stays paused");

        app.prefs.count_in_enabled = true;
        app.apply_count_in_change();
        assert_eq!(app.timeline.count_in_beats, COUNT_IN_BEATS);
        // The re-rendered buffer has the count-in the timeline names.
        app.audio.seek_beat(-1.0);
        assert!((app.audio.position_beat() + 1.0).abs() < 1e-6);
        assert!(!app.audio.is_playing());
    }

    #[test]
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn switching_the_dance_source_clears_the_old_results() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = CallerApp::new_headless(&cc);
        let other = caller_sources::dance_providers()
            .iter()
            .find(|provider| provider.id() != app.prefs.dance_source)
            .map(|provider| provider.id().to_string())
            .expect("a second dance provider is registered");
        app.picker.dance_results = Some(DanceSearchPage {
            total: 1,
            page: 1,
            page_size: 10,
            dances: Vec::new(),
        });
        app.picker.dance_query.formation = Some("improper".into());
        app.picker.search_loading = true;

        app.set_dance_source(&other);

        assert_eq!(app.prefs.dance_source, other);
        assert!(app.picker.dance_results.is_none());
        assert!(app.picker.dance_preview.is_none());
        assert!(!app.picker.search_loading);
        assert_eq!(app.picker.dance_query.page, 1);
        assert!(app.picker.dance_query.formation.is_none());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn a_render_key_ignores_ui_only_preferences_and_tracks_audio() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx);
        let mut app = CallerApp::new_headless(&cc);
        let base = app.render_key();

        app.prefs.theme = if app.prefs.theme == Theme::Dark {
            Theme::Light
        } else {
            Theme::Dark
        };
        app.prefs.animations_enabled = !app.prefs.animations_enabled;
        assert_eq!(
            app.render_key(),
            base,
            "a theme or animation change must not invalidate the audio cache"
        );

        app.prefs.tempo_bpm += 10.0;
        assert_ne!(
            app.render_key(),
            base,
            "a tempo change must produce a different render key"
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn a_finished_prefetch_fills_the_cache_and_starts_a_pending_play() {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx);
        let mut app = CallerApp::new_headless(&cc);
        app.headless = false;

        let score = parse_abc(&app.tune.abc).expect("the bundled tune parses");
        let arrangement = Arrangement::from_score(&score, 64.0);
        let samples_per_beat = app.audio.sample_rate() as f64 * 0.5;
        let pass_samples = (64.0 * samples_per_beat).round() as u64;
        let buffer = AudioBuffer {
            samples: vec![0.0; (pass_samples as usize) * 2],
            sample_rate: app.audio.sample_rate(),
            channels: 2,
            count_in_samples: 0,
            loop_start_sample: 0,
            loop_end_sample: pass_samples,
            loop_pass_samples: pass_samples,
            samples_per_beat,
            ending_from_beat: 62.0,
            ending_start_sample: None,
        };
        let prepared = PreparedTune {
            generation: app.render_generation,
            tune: app.tune.clone(),
            buffer,
            arrangement,
        };
        let (tx, rx) = channel();
        app.prefetch = Some(Prefetch {
            key: app.render_key(),
            rx,
        });

        app.play();
        assert!(app.pending_play, "Play waits for the running prefetch");
        assert!(!app.audio.is_playing(), "nothing starts before it is ready");

        tx.send(Ok(prepared))
            .expect("the prefetch result is delivered");
        app.poll_prefetch();

        assert!(!app.pending_play);
        assert!(app.audio.is_playing(), "the pending Play starts the loop");
        assert!(app.render_cache.is_some());

        let first = app.render_music().expect("the cache holds the loop");
        let second = app.render_music().expect("the cache holds the loop");
        assert!(
            Arc::ptr_eq(&first.0, &second.0),
            "a cache hit must reuse the installed buffer, not synthesize again"
        );
    }

    /// A headless app with an empty history and nothing rendered yet.
    #[cfg(all(
        feature = "headless",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    fn history_app() -> (CallerApp, egui::Context) {
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut app = CallerApp::new_headless(&cc);
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
            caller_core::SourceRef::new("contradb", "9999", "https://example.test/dances/9999");
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

// Development-only CPU profiler for the screens and user-facing controls.
#[cfg(feature = "headless")]
pub mod perf;
