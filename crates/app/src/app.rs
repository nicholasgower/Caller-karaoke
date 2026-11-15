//! Application state and UI.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use contra_core::{
    AccompanimentInstrument, AliasTable, Dance, MelodyInstrument, MoveKind, PracticeTimeline,
    Preferences, RolePreset, RoleTerms, Theme, TimelineConfig, Tune,
};
use contra_music::{Arrangement, LengthMismatch, RenderRequest, export_midi, parse_abc, render};
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
use crate::sheet::SheetPreview;
use crate::source::{SourceClient, SourceEvent};
use crate::theme;
use crate::tts::Speech;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Session,
    DancePicker,
    TunePicker,
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

pub struct ContraApp {
    prefs: Preferences,
    prefs_path: PathBuf,
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
    dance: Dance,
    tune: Tune,
    timeline: PracticeTimeline,
    arrangement: Option<Arrangement>,
    soundfont: Option<Arc<SoundFont>>,
    soundfont_error: Option<String>,
    needs_render: bool,
    render_loading: bool,
    started: bool,
    dance_query: DanceQuery,
    tune_query: TuneQuery,
    dance_results: Option<DanceSearchPage>,
    tune_results: Option<TuneSearchPage>,
    dance_preview: Option<Dance>,
    tune_preview: Option<Tune>,
    tune_view: TuneView,
    tune_sheet: Option<Result<SheetPreview, String>>,
    dance_selected: Option<DanceSummary>,
    tune_selected: Option<TuneSummary>,
    search_loading: bool,
    preview_loading: bool,
    status: String,
    error: Option<String>,
    length_warning: Option<String>,
    last_spoken: Option<f64>,
    last_beat: f64,
    cache_clear_confirm: bool,
    alias_filter: String,
    custom_key: String,
    custom_alias: String,
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

        let mut speech = Speech::new();
        speech.set_rate(prefs.tts_rate);

        let dance = defaults::default_dance();
        let tune = defaults::default_tune();
        let timeline = build_timeline(&dance, &prefs).unwrap_or_else(|_| empty_timeline());
        let length_warning = length_warning_for(&dance, &tune);

        let soundfont = prefs
            .soundfont
            .as_ref()
            .and_then(|setting| setting.path.as_ref())
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| contra_music::soundfont::load_soundfont(&bytes).ok());

        theme::apply(&cc.egui_ctx, prefs.theme, prefs.animations_enabled);

        Self {
            prefs,
            prefs_path,
            audio: audio.0,
            audio_error: audio.1,
            speech,
            source,
            source_rx,
            screen: Screen::Session,
            screen_anim: 1.0,
            screen_anim_screen: Screen::Session,
            dance,
            tune,
            timeline,
            arrangement: None,
            soundfont,
            soundfont_error: None,
            needs_render: true,
            render_loading: false,
            started: false,
            dance_query: DanceQuery {
                page_size: 10,
                ..DanceQuery::default()
            },
            tune_query: TuneQuery {
                page_size: 10,
                ..TuneQuery::default()
            },
            dance_results: None,
            tune_results: None,
            dance_preview: None,
            tune_preview: None,
            tune_view: TuneView::default(),
            tune_sheet: None,
            dance_selected: None,
            tune_selected: None,
            search_loading: false,
            preview_loading: false,
            status: "Ready".into(),
            error: None,
            length_warning,
            last_spoken: None,
            last_beat: 0.0,
            cache_clear_confirm: false,
            alias_filter: String::new(),
            custom_key: String::new(),
            custom_alias: String::new(),
        }
    }

    /// Advance the screen-entry animation and return its eased progress.
    fn screen_transition(&mut self, ctx: &egui::Context) -> f32 {
        if self.screen_anim_screen != self.screen {
            self.screen_anim_screen = self.screen;
            self.screen_anim = 0.0;
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

    fn save_prefs(&self) {
        if let Ok(json) = self.prefs.to_json() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = std::fs::write(&self.prefs_path, json);
            }
            #[cfg(target_arch = "wasm32")]
            {
                let _ = &self.prefs_path;
                crate::web_store::write_prefs(&json);
            }
        }
    }

    fn rebuild_timeline(&mut self) {
        match build_timeline(&self.dance, &self.prefs) {
            Ok(timeline) => {
                self.timeline = timeline;
                self.last_spoken = None;
                self.last_beat = 0.0;
            }
            Err(error) => {
                self.error = Some(error);
            }
        }
    }

    fn rebuild_music(&mut self) {
        let score = match parse_abc(&self.tune.abc) {
            Ok(score) => score,
            Err(error) => {
                self.length_warning = None;
                self.error = Some(format!("Could not parse tune: {error}"));
                return;
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
        match render(&request, self.soundfont.as_ref()) {
            Ok(buffer) => {
                self.audio.set_buffer(buffer);
                self.arrangement = Some(arrangement);
                self.audio.set_looping(self.prefs.loop_enabled);
                self.audio.set_volume(self.prefs.master_volume);
                self.needs_render = false;
                self.status = "Music ready".into();
            }
            Err(error) => {
                self.error = Some(format!("Could not render music: {error}"));
            }
        }
    }

    fn ensure_ready(&mut self) {
        if self.needs_render || !self.audio.has_buffer() {
            self.rebuild_timeline();
            self.rebuild_music();
        }
    }

    fn play(&mut self) {
        self.ensure_ready();
        if self.audio.has_buffer() {
            self.audio.play();
            self.started = true;
            self.status = "Playing".into();
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
        self.status = "Playing".into();
    }

    fn use_dance(&mut self, dance: Dance) {
        self.error = None;
        self.dance = dance;
        self.length_warning = length_warning_for(&self.dance, &self.tune);
        self.prefs.last_dance = Some(self.dance.source.clone());
        self.dance_preview = None;
        self.dance_selected = None;
        self.screen = Screen::Session;
        self.needs_render = true;
        self.arrangement = None;
        self.audio.stop();
        self.speech.stop();
        self.started = false;
        self.rebuild_timeline();
        self.save_prefs();
        self.status = "Dance selected".into();
    }

    fn use_tune(&mut self, tune: Tune) {
        self.error = None;
        self.tune = tune;
        self.length_warning = length_warning_for(&self.dance, &self.tune);
        self.prefs.last_tune = Some(self.tune.source.clone());
        self.tune_preview = None;
        self.tune_sheet = None;
        self.tune_selected = None;
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
                    self.search_loading = false;
                    self.dance_query = query;
                    match result {
                        Ok(page) => {
                            self.dance_results = Some(page);
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                SourceEvent::Dance { result } => {
                    self.preview_loading = false;
                    match result {
                        Ok(dance) => {
                            self.dance_preview = Some(dance);
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                SourceEvent::TuneSearch { query, result } => {
                    self.search_loading = false;
                    self.tune_query = query;
                    match result {
                        Ok(page) => {
                            self.tune_results = Some(page);
                            self.error = None;
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                SourceEvent::Tune { result } => {
                    self.preview_loading = false;
                    match result {
                        Ok(tune) => {
                            self.tune_preview = Some(tune);
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
        if beat + 0.25 < self.last_beat {
            self.last_spoken = None;
        }
        self.last_beat = beat;
        if !self.prefs.tts_enabled || !self.speech.available() || !self.audio.is_playing() {
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
        theme::apply(&ctx, self.prefs.theme, self.prefs.animations_enabled);
        self.handle_source_events();

        if !ctx.egui_wants_keyboard_input() {
            if ui.input(|input| input.key_pressed(egui::Key::Space)) {
                if self.audio.is_playing() {
                    self.pause();
                } else {
                    self.play();
                }
            }
            if ui.input(|input| input.key_pressed(egui::Key::R)) {
                self.restart();
            }
            if ui.input(|input| input.key_pressed(egui::Key::Escape))
                && self.screen != Screen::Session
            {
                self.screen = Screen::Session;
            }
        }

        let beat = self.current_beat();
        self.update_speech(beat);

        egui::Panel::top("tabs")
            .frame(egui::Frame::default().inner_margin(egui::Margin::symmetric(12, 8)))
            .show(ui, |ui| {
                let animate = self.prefs.animations_enabled;
                ui.horizontal_wrapped(|ui| {
                    ui.heading("Contra Karaoke");
                    ui.add_space(12.0);
                    if tab(ui, self.screen == Screen::Session, "Session", animate) {
                        self.screen = Screen::Session;
                    }
                    if tab(
                        ui,
                        self.screen == Screen::DancePicker,
                        "Select dance",
                        animate,
                    ) {
                        self.screen = Screen::DancePicker;
                    }
                    if tab(
                        ui,
                        self.screen == Screen::TunePicker,
                        "Select Tune",
                        animate,
                    ) {
                        self.screen = Screen::TunePicker;
                    }
                    if tab(
                        ui,
                        self.screen == Screen::Preferences,
                        "Preferences",
                        animate,
                    ) {
                        self.screen = Screen::Preferences;
                    }
                });
            });

        if let Some(error) = self.error.clone() {
            egui::Panel::bottom("error").show(ui, |ui| {
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
            match self.screen {
                Screen::Session => self.session_ui(ui, beat),
                Screen::DancePicker => self.dance_picker_ui(ui),
                Screen::TunePicker => self.tune_picker_ui(ui),
                Screen::Preferences => self.preferences_ui(ui),
                Screen::Aliases => self.aliases_ui(ui),
            }
        });

        if self.audio.is_playing() || self.render_loading {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_prefs();
    }
}

/// Duration of the fade/slide when the active screen changes.
const SCREEN_ANIM_SECONDS: f32 = 0.22;
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
/// fade in and out; egui's selectable label switches instantly.
fn tab(ui: &mut egui::Ui, active: bool, label: &str, animate: bool) -> bool {
    let ctx = ui.ctx().clone();
    let id = ui.make_persistent_id(("tab", label));
    let font = egui::TextStyle::Button.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, egui::Color32::PLACEHOLDER);
    let desired = egui::vec2(
        galley.size().x + 24.0,
        (galley.size().y + 16.0).max(ui.spacing().interact_size.y),
    );
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());

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

    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, label));
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

fn build_timeline(dance: &Dance, prefs: &Preferences) -> Result<PracticeTimeline, String> {
    PracticeTimeline::build(
        dance,
        &prefs.aliases,
        &prefs.role_terms,
        &prefs.lead,
        prefs.tempo(),
        TimelineConfig {
            count_in_beats: if prefs.count_in_enabled { 3.0 } else { 0.0 },
            music_loop_beats: 64.0,
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

// The rest of the UI is implemented in `ui.rs` to keep this file navigable.
include!("ui.rs");
