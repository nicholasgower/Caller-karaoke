//! Application state and UI.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use contra_core::{
    AccompanimentInstrument, AliasTable, Dance, MelodyInstrument, MoveKind, PracticeTimeline,
    Preferences, RolePreset, RoleTerms, Theme, TimelineConfig, Tune,
};
use contra_music::{Arrangement, RenderRequest, export_midi, parse_abc, render};
use contra_sources::{
    Cache, DanceQuery, DanceSearchPage, DanceSort, DanceSummary, FileCache, TuneQuery,
    TuneSearchPage, TuneSummary,
};
use eframe::egui;
use rustysynth::SoundFont;

use crate::audio::AudioEngine;
use crate::defaults;
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

pub struct ContraApp {
    prefs: Preferences,
    prefs_path: PathBuf,
    audio: AudioEngine,
    audio_error: Option<String>,
    speech: Speech,
    source: SourceClient,
    source_rx: Receiver<SourceEvent>,
    screen: Screen,
    dance: Dance,
    tune: Tune,
    timeline: PracticeTimeline,
    arrangement: Option<Arrangement>,
    soundfont: Option<Arc<SoundFont>>,
    soundfont_error: Option<String>,
    needs_render: bool,
    render_loading: bool,
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
    status: String,
    error: Option<String>,
    last_spoken: Option<usize>,
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
            let base = std::env::temp_dir().join("contra-karaoke");
            (base.join("config"), base.join("cache"))
        };
        let _ = std::fs::create_dir_all(&config_dir);
        let _ = std::fs::create_dir_all(&cache_dir);
        let prefs_path = config_dir.join("preferences.json");
        let mut prefs = std::fs::read_to_string(&prefs_path)
            .ok()
            .and_then(|json| Preferences::from_json(&json).ok())
            .unwrap_or_default();
        prefs.validate();

        let cache = FileCache::new(cache_dir.join("sources"))
            .map(Cache::new)
            .unwrap_or_else(|_| Cache::new(contra_sources::MemoryCache::default()));
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

        let soundfont = prefs
            .soundfont
            .as_ref()
            .and_then(|setting| setting.path.as_ref())
            .and_then(|path| std::fs::read(path).ok())
            .and_then(|bytes| contra_music::soundfont::load_soundfont(&bytes).ok());

        theme::apply(&cc.egui_ctx, prefs.theme);

        Self {
            prefs,
            prefs_path,
            audio: audio.0,
            audio_error: audio.1,
            speech,
            source,
            source_rx,
            screen: Screen::Session,
            dance,
            tune,
            timeline,
            arrangement: None,
            soundfont,
            soundfont_error: None,
            needs_render: true,
            render_loading: false,
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
            dance_selected: None,
            tune_selected: None,
            search_loading: false,
            preview_loading: false,
            status: "Ready".into(),
            error: None,
            last_spoken: None,
            last_beat: 0.0,
            cache_clear_confirm: false,
            alias_filter: String::new(),
            custom_key: String::new(),
            custom_alias: String::new(),
        }
    }

    fn save_prefs(&self) {
        if let Ok(json) = self.prefs.to_json() {
            let _ = std::fs::write(&self.prefs_path, json);
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
                self.error = Some(format!("Could not parse tune: {error}"));
                return;
            }
        };
        let arrangement = Arrangement::from_score(&score, 64.0);
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
        self.status = "Playing".into();
    }

    fn use_dance(&mut self, dance: Dance) {
        self.error = None;
        self.dance = dance;
        self.prefs.last_dance = Some(self.dance.source.clone());
        self.dance_preview = None;
        self.dance_selected = None;
        self.screen = Screen::Session;
        self.needs_render = true;
        self.audio.stop();
        self.rebuild_timeline();
        self.save_prefs();
        self.status = "Dance selected".into();
    }

    fn use_tune(&mut self, tune: Tune) {
        self.error = None;
        self.tune = tune;
        self.prefs.last_tune = Some(self.tune.source.clone());
        self.tune_preview = None;
        self.tune_selected = None;
        self.screen = Screen::Session;
        self.needs_render = true;
        self.audio.stop();
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
        let active = self
            .timeline
            .calls
            .iter()
            .enumerate()
            .find(|(_, call)| beat >= call.call_start_beat && beat < call.execute_beat)
            .map(|(index, _)| index);
        if let Some(index) = active
            && self.last_spoken != Some(index)
        {
            let text = &self.timeline.calls[index].text;
            self.speech.speak(text);
            self.last_spoken = Some(index);
        }
    }

    fn current_beat(&self) -> f64 {
        if self.audio.has_buffer() {
            self.audio.position_beat()
        } else {
            0.0
        }
    }

    fn export_midi(&mut self) {
        let Some(arrangement) = &self.arrangement else {
            self.error = Some("Render the music before exporting MIDI".into());
            return;
        };
        let default_name = format!("{}.mid", safe_filename(&self.tune.title));
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(default_name)
            .add_filter("MIDI", &["mid"])
            .save_file()
        else {
            return;
        };
        let bytes = export_midi(
            arrangement,
            self.prefs.melody,
            self.prefs.accompaniment,
            self.prefs.tempo_bpm,
        );
        match std::fs::write(&path, bytes) {
            Ok(()) => self.status = format!("Saved MIDI to {}", path.display()),
            Err(error) => self.error = Some(format!("Could not save MIDI: {error}")),
        }
    }

    fn load_soundfont(&mut self) {
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
        theme::apply(&ctx, self.prefs.theme);
        self.handle_source_events();
        let beat = self.current_beat();
        self.update_speech(beat);

        egui::Panel::top("tabs")
            .frame(egui::Frame::default().inner_margin(egui::Margin::symmetric(12, 8)))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.heading("Contra Karaoke");
                    ui.add_space(12.0);
                    if tab(ui, self.screen == Screen::Session, "Session") {
                        self.screen = Screen::Session;
                    }
                    if tab(ui, self.screen == Screen::DancePicker, "Select dance") {
                        self.screen = Screen::DancePicker;
                    }
                    if tab(ui, self.screen == Screen::TunePicker, "Select Tune") {
                        self.screen = Screen::TunePicker;
                    }
                    if tab(ui, self.screen == Screen::Preferences, "Preferences") {
                        self.screen = Screen::Preferences;
                    }
                    if ui.button("Play").clicked() {
                        self.play();
                    }
                    if ui.button("Pause").clicked() {
                        self.pause();
                    }
                    if ui.button("Restart").clicked() {
                        self.restart();
                    }
                });
            });

        if let Some(error) = self.error.clone() {
            egui::Panel::bottom("error").show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(220, 120, 120), error);
                    if ui.button("Dismiss").clicked() {
                        self.error = None;
                    }
                });
            });
        }

        egui::CentralPanel::default().show(ui, |ui| match self.screen {
            Screen::Session => self.session_ui(ui, beat),
            Screen::DancePicker => self.dance_picker_ui(ui),
            Screen::TunePicker => self.tune_picker_ui(ui),
            Screen::Preferences => self.preferences_ui(ui),
            Screen::Aliases => self.aliases_ui(ui),
        });

        if self.audio.is_playing() || self.render_loading {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_prefs();
    }
}

fn tab(ui: &mut egui::Ui, active: bool, label: &str) -> bool {
    ui.selectable_label(active, label).clicked()
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

// The rest of the UI is implemented in `ui.rs` to keep this file navigable.
include!("ui.rs");
