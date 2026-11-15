use super::*;
use crate::app::*;
use crate::audio::AudioEngine;
use crate::theme;
use crate::tts::Speech;
use contra_core::{COUNT_IN_BEATS, Preferences};
use eframe::egui;

/// The tempo, transport, loop/count-in/speech, and volume controls.
///
/// On a phone the controls are stacked so each row owns the full width: a
/// full-width rail with a value box is usable with a thumb, while the desktop
/// wrapping row leaves the tempo slider barely a finger wide. The wide layout
/// keeps the original single wrapping row.
fn session_controls(
    ui: &mut egui::Ui,
    prefs: &mut Preferences,
    audio: &mut AudioEngine,
    speech: &mut Speech,
    compact: bool,
) -> SessionControlChanges {
    let mut changes = SessionControlChanges::default();

    if compact {
        ui.horizontal(|ui| {
            ui.label("Tempo");
            // Leave room for the value box and the " BPM" suffix.
            ui.spacing_mut().slider_width = (ui.available_width() - 100.0).max(96.0);
            let response = ui.add(
                egui::Slider::new(&mut prefs.tempo_bpm, 120.0..=320.0)
                    .fixed_decimals(0)
                    .suffix(" BPM"),
            );
            // A stable accessible name, so the slider can be found by name in
            // every layout (and by the MCP session tools).
            let response = response.accessible_name("Tempo");
            if response.changed() {
                changes.timeline_dirty = true;
                changes.music_dirty = true;
            }
        });

        // Equal-width transport buttons: no control hides behind hover and
        // each is a full touch target.
        ui.horizontal(|ui| {
            let spacing = ui.spacing().item_spacing.x;
            let width = ((ui.available_width() - 3.0 * spacing) / 4.0).max(62.0);
            for (label, hover, action) in [
                ("Play", "Play (Space)", "play"),
                ("Pause", "Pause (Space)", "pause"),
                ("Restart", "Restart (R)", "restart"),
            ] {
                if ui
                    .add_sized([width, theme::TOUCH_TARGET], egui::Button::new(label))
                    .on_hover_text(hover)
                    .clicked()
                {
                    changes.action = Some(action);
                }
            }
            let ending = audio.ending_requested();
            let in_ending = audio.in_ending();
            let can_end = !in_ending && !audio.is_finished() && audio.has_buffer();
            let end_label = if in_ending {
                "Ending…"
            } else if ending {
                "Cancel"
            } else {
                "End"
            };
            let response = ui
                .add_enabled_ui(can_end, |ui| {
                    ui.add_sized([width, theme::TOUCH_TARGET], egui::Button::new(end_label))
                })
                .inner
                .on_hover_text(if ending {
                    "Cancel the slow ending"
                } else {
                    "End the tune slowly after this time through"
                });
            if response.clicked() {
                changes.action = Some(if ending { "cancel-ending" } else { "end" });
            }
        });

        ui.horizontal_wrapped(|ui| {
            if ui.checkbox(&mut prefs.loop_enabled, "Loop").changed() {
                audio.set_looping(prefs.loop_enabled);
                changes.save = true;
            }
            if ui
                .checkbox(&mut prefs.tune_rotation.enabled, "Cycle tunes")
                .on_hover_text("Play through the tune set, changing at a repeat")
                .changed()
            {
                changes.rotation_dirty = true;
                changes.save = true;
            }
            if ui
                .checkbox(&mut prefs.count_in_enabled, "Count-in")
                .changed()
            {
                changes.timeline_dirty = true;
                changes.music_dirty = true;
            }
            if ui.checkbox(&mut prefs.tts_enabled, "Speak calls").changed() {
                speech.set_enabled(prefs.tts_enabled);
                if !prefs.tts_enabled {
                    speech.stop();
                }
                changes.save = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Volume");
            ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(96.0);
            let response =
                ui.add(egui::Slider::new(&mut prefs.master_volume, 0.0..=1.0).max_decimals(2));
            let response = response.accessible_name("Volume");
            if response.changed() {
                audio.set_volume(prefs.master_volume);
                changes.save = true;
            }
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            ui.label("Tempo");
            if ui
                .add(
                    egui::Slider::new(&mut prefs.tempo_bpm, 120.0..=320.0)
                        .text("BPM")
                        .fixed_decimals(0),
                )
                .accessible_name("Tempo")
                .changed()
            {
                changes.timeline_dirty = true;
                changes.music_dirty = true;
            }
            if ui.button("Play").on_hover_text("Play (Space)").clicked() {
                changes.action = Some("play");
            }
            if ui.button("Pause").on_hover_text("Pause (Space)").clicked() {
                changes.action = Some("pause");
            }
            if ui.button("Restart").on_hover_text("Restart (R)").clicked() {
                changes.action = Some("restart");
            }
            let ending = audio.ending_requested();
            let in_ending = audio.in_ending();
            let can_end = !in_ending && !audio.is_finished() && audio.has_buffer();
            let end_label = if in_ending {
                "Ending…"
            } else if ending {
                "Cancel ending"
            } else {
                "End tune"
            };
            if ui
                .add_enabled(can_end, egui::Button::new(end_label))
                .on_hover_text(if ending {
                    "Cancel the slow ending"
                } else {
                    "End the tune slowly after this time through"
                })
                .clicked()
            {
                changes.action = Some(if ending { "cancel-ending" } else { "end" });
            }
            if ui.checkbox(&mut prefs.loop_enabled, "Loop").changed() {
                audio.set_looping(prefs.loop_enabled);
                changes.save = true;
            }
            if ui
                .checkbox(&mut prefs.tune_rotation.enabled, "Cycle tunes")
                .on_hover_text("Play through the tune set, changing at a repeat")
                .changed()
            {
                changes.rotation_dirty = true;
                changes.save = true;
            }
            if ui
                .checkbox(&mut prefs.count_in_enabled, "Count-in")
                .changed()
            {
                changes.timeline_dirty = true;
                changes.music_dirty = true;
            }
            if ui.checkbox(&mut prefs.tts_enabled, "Speak calls").changed() {
                speech.set_enabled(prefs.tts_enabled);
                if !prefs.tts_enabled {
                    speech.stop();
                }
                changes.save = true;
            }
            ui.label("Volume");
            if ui
                .add(egui::Slider::new(&mut prefs.master_volume, 0.0..=1.0))
                .accessible_name("Volume")
                .changed()
            {
                audio.set_volume(prefs.master_volume);
                changes.save = true;
            }
        });
    }

    changes
}

/// The transport state named by the progress bar.
///
/// A caller can hear whether the music is running; without a visible state a
/// muted or unplugged session looks exactly like a paused one, and an agent
/// testing the session tab has nothing to assert against. The progress bar is
/// the one row that is always visible while the session runs, so it carries the
/// state as well as the beat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TransportState {
    Ready,
    CountIn,
    Playing,
    Paused,
    Ending,
    Finished,
}

impl TransportState {
    fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::CountIn => "Count-in",
            Self::Playing => "Playing",
            Self::Paused => "Paused",
            Self::Ending => "Ending",
            Self::Finished => "Finished",
        }
    }
}

/// Name the current transport state.
///
/// Finished and Ready win over Paused: a completed run and a session that never
/// started are not paused states, even though neither is playing.
fn transport_state(
    started: bool,
    playing: bool,
    finished: bool,
    ending: bool,
    beat: f64,
) -> TransportState {
    if finished {
        TransportState::Finished
    } else if !started {
        TransportState::Ready
    } else if !playing {
        TransportState::Paused
    } else if ending {
        TransportState::Ending
    } else if beat < -0.001 {
        TransportState::CountIn
    } else {
        TransportState::Playing
    }
}

/// Beats still to count before the loop starts.
///
/// Rounds up (the number a caller says) and caps at the configured four: the
/// audio clock can land a hair below `-4.0` at the very start, and `ceil` would
/// otherwise show five.
fn count_in_beats_left(beat: f64) -> u32 {
    ((-beat).ceil().max(1.0) as u32).min(COUNT_IN_BEATS as u32)
}

/// The progress bar text: the explicit transport state plus the position.
fn progress_text(state: TransportState, beat: f64, loop_beats: f64) -> String {
    match state {
        TransportState::CountIn => {
            let left = count_in_beats_left(beat);
            if left == 1 {
                "Count-in · 1 beat left".to_string()
            } else {
                format!("Count-in · {left} beats left")
            }
        }
        TransportState::Ending => "Ending · slow finish".to_string(),
        _ => format!(
            "{} · beat {:.0} / {:.0}",
            state.label(),
            beat.clamp(0.0, loop_beats),
            loop_beats
        ),
    }
}

impl ContraApp {
    pub(in crate::app) fn session_ui(&mut self, ui: &mut egui::Ui, beat: f64) {
        let layout = session_layout(ui.available_width(), ui.available_height());
        if layout.page_scroll {
            // A short phone cannot fit the controls, prompt, and dance card at
            // once. Scroll the whole page instead of squeezing the card into a
            // few pixels or hiding it below the fold.
            egui::ScrollArea::vertical()
                .id_salt("session-page")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.session_body(ui, beat, layout, false);
                    ui.add_space(24.0);
                });
        } else {
            self.session_body(ui, beat, layout, true);
        }
    }

    fn session_body(
        &mut self,
        ui: &mut egui::Ui,
        beat: f64,
        layout: SessionLayout,
        card_scroll: bool,
    ) {
        let compact = layout.compact;
        if compact {
            // The phone layout gives every interactive control a 44 pt row.
            ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
        }
        let SessionControlChanges {
            timeline_dirty,
            music_dirty,
            rotation_dirty,
            save,
            action,
        } = session_controls(
            ui,
            &mut self.prefs,
            &mut self.audio,
            &mut self.speech,
            compact,
        );
        let mut rotation_changed = rotation_dirty;

        if self.needs_render {
            ui.horizontal_wrapped(|ui| {
                let message = if self.started {
                    "Playback settings will apply without restarting; press Play to apply them now."
                } else {
                    "Press Play to prepare the music with these settings."
                };
                ui.label(egui::RichText::new(message).color(theme::muted(ui)));
            });
        }
        if let Some(warning) = &self.length_warning {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(warning)
                        .strong()
                        .color(theme::warning(ui)),
                );
            });
        }
        if let Some(arrangement) = &self.arrangement {
            for warning in &arrangement.warnings {
                ui.label(egui::RichText::new(warning).small().color(theme::muted(ui)));
            }
        }
        if let Some(error) = &self.audio_error {
            ui.colored_label(theme::error(ui), format!("Audio device: {error}"));
        }
        if self.prefs.tts_enabled {
            let status = self.speech.status();
            if let Some(error) = &status.error {
                ui.colored_label(
                    theme::error(ui),
                    format!("Neural voice: {error}. Using the platform voice."),
                );
            } else if !self.speech.available() {
                ui.colored_label(
                    theme::error(ui),
                    "Text-to-speech is not available on this system.",
                );
            } else if let Some(progress) = status.progress {
                ui.add(
                    egui::ProgressBar::new(progress)
                        .show_percentage()
                        .text(status.message.clone()),
                );
            } else if !status.ready {
                ui.label(
                    egui::RichText::new(&status.message)
                        .small()
                        .color(theme::muted(ui)),
                );
            }
        }

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        if compact {
            ui.label(egui::RichText::new(&self.dance.title).size(22.0).strong());
        } else {
            ui.heading(&self.dance.title);
        }
        ui.label(
            egui::RichText::new(format!(
                "{} · {} · {} beats",
                empty_label(&self.dance.choreographer, "unknown choreographer"),
                empty_label(&self.dance.formation, "formation unknown"),
                self.dance.total_beats()
            ))
            .color(theme::muted(ui)),
        );
        let instruments = match self.prefs.second_accompaniment.instrument() {
            Some(second) => format!(
                "Melody: {} · Accompaniment: {} · Second: {}",
                self.prefs.melody.label(),
                self.prefs.accompaniment.label(),
                second.label()
            ),
            None => format!(
                "Melody: {} · Accompaniment: {}",
                self.prefs.melody.label(),
                self.prefs.accompaniment.label()
            ),
        };
        if compact {
            // Two deliberate lines instead of one wrapped line, so the tune
            // and the instrument summary never break mid-phrase at phone width.
            ui.label(
                egui::RichText::new(format!(
                    "Tune: {} · {} · {}",
                    self.tune.title,
                    self.tune.display_key(),
                    self.tune.display_rhythm()
                ))
                .strong(),
            );
            ui.label(egui::RichText::new(&instruments).color(theme::muted(ui)));
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label("Tune:");
                ui.label(
                    egui::RichText::new(format!(
                        "{} · {} · {}",
                        self.tune.title,
                        self.tune.display_key(),
                        self.tune.display_rhythm()
                    ))
                    .strong(),
                );
                ui.label(egui::RichText::new(&instruments).color(theme::muted(ui)));
            });
        }

        if self.prefs.tune_rotation.enabled {
            let tunes = self.rotation_tunes();
            let position = self.rotation_position();
            let next = self.next_rotation_tune().map(|tune| tune.title);
            let interval = self.prefs.tune_rotation.passes_per_tune;
            let minutes = self
                .prefs
                .tune_rotation
                .seconds_per_tune(self.prefs.tempo_bpm)
                / 60.0;
            let summary = if tunes.len() < 2 {
                format!(
                    "Tune set: add at least two saved tunes in Select tune ({} now).",
                    tunes.len()
                )
            } else {
                let place = match position {
                    Some(index) => format!("Tune {} of {}", index + 1, tunes.len()),
                    None => "Current tune is not in the set".to_string(),
                };
                let next = next.unwrap_or_else(|| "the first tune".to_string());
                format!(
                    "{place} · next: {next} · changes every {interval} times through (≈{minutes:.1} min)"
                )
            };
            let mut edit = false;
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(summary).small().color(theme::muted(ui)));
                if ui.small_button("Edit tune set").clicked() {
                    edit = true;
                }
            });
            if tunes.len() >= 2 {
                // On a phone the label goes above the rail: a one-row label,
                // slider, value, and "times through" is wider than 360 points.
                let response = slider_row(ui, compact, "Change tune every", |ui| {
                    ui.add(
                        egui::Slider::new(
                            &mut self.prefs.tune_rotation.passes_per_tune,
                            contra_core::TuneRotation::MIN_PASSES
                                ..=contra_core::TuneRotation::MAX_PASSES,
                        )
                        .text("times through"),
                    )
                });
                if response.changed() {
                    rotation_changed = true;
                }
            }
            if edit {
                self.screen = Screen::TunePicker;
            }
        }

        ui.add_space(8.0);
        let active = if self.started {
            self.timeline
                .scheduled_call_at(beat, self.prefs.loop_enabled)
        } else {
            None
        };
        let next = if self.started {
            self.timeline
                .next_scheduled_call(beat, self.prefs.loop_enabled)
        } else {
            None
        };
        let phase = active.as_ref().map(|call| phase_at(call, beat));
        let card_mode = card_state(phase, next.is_some());
        let card_fill = match card_tone(self.started, phase, card_mode) {
            CardTone::CallNow => theme::call_now_bg(ui),
            CardTone::UpcomingMove => theme::upcoming_move_bg(ui),
            CardTone::Neutral => theme::call_active_bg(ui),
        };

        let call_size = if compact { 30.0 } else { 34.0 };
        let count_size = if compact { 48.0 } else { 56.0 };
        let count_in_label = if self.prefs.count_in_enabled {
            format!("{}-beat count-in", COUNT_IN_BEATS as u32)
        } else {
            "no count-in".to_string()
        };
        egui::Frame::group(ui.style())
            .fill(card_fill)
            .inner_margin(egui::Margin::same(if compact { 10 } else { 12 }))
            .show(ui, |ui| {
                if compact {
                    // A content-sized prompt card leaves a ragged strip of
                    // background on a phone; use the full width for the text.
                    ui.set_width(ui.available_width());
                }
                if !self.started {
                    ui.label(egui::RichText::new("READY").small().color(theme::muted(ui)));
                    ui.label(
                        egui::RichText::new("Press Play to begin")
                            .size(30.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} calls · {} beats · {}",
                            self.timeline.calls.len(),
                            self.timeline.music_loop_beats,
                            count_in_label
                        ))
                        .color(theme::muted(ui)),
                    );
                } else if beat < -0.001 {
                    ui.label(
                        egui::RichText::new("COUNT-IN")
                            .small()
                            .color(theme::muted(ui)),
                    );
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(count_in_beats_left(beat).to_string())
                                .size(count_size)
                                .strong(),
                        );
                        ui.vertical(|ui| {
                            if let Some(call) = &active {
                                ui.label(
                                    egui::RichText::new("CALL NOW")
                                        .small()
                                        .color(theme::on_card(ui)),
                                );
                                ui.label(
                                    egui::RichText::new(&call.call.text)
                                        .size(26.0)
                                        .strong()
                                        .color(theme::on_card(ui)),
                                );
                            } else {
                                ui.label(
                                    egui::RichText::new("Listen for the first chord").size(20.0),
                                );
                                if let Some(next) = &next {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "First call: {}",
                                            next.call.text
                                        ))
                                        .color(theme::muted(ui)),
                                    );
                                }
                            }
                        });
                    });
                } else if let Some(call) = &active {
                    match card_mode {
                        CardMode::UpcomingMove => {
                            if let Some(upcoming) = &next {
                                upcoming_move_card(
                                    ui,
                                    upcoming,
                                    beat,
                                    self.prefs.show_original,
                                    call_size,
                                );
                            }
                        }
                        CardMode::FinalMove => {
                            ui.label(
                                egui::RichText::new("DANCING")
                                    .small()
                                    .color(theme::muted(ui)),
                            );
                            ui.label(
                                egui::RichText::new(&call.call.text)
                                    .size(call_size)
                                    .strong()
                                    .color(theme::muted(ui)),
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new("final figure").color(theme::muted(ui)),
                                );
                                if self.prefs.show_original {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "· {}",
                                            call.call.original_text
                                        ))
                                        .small()
                                        .color(theme::muted(ui)),
                                    );
                                }
                            });
                        }
                        CardMode::CurrentCall => {
                            let phase = phase_at(call, beat);
                            let label = match phase {
                                CallPhase::Upcoming => "UPCOMING",
                                CallPhase::Calling => "CALL NOW",
                                CallPhase::Executing => "DANCING",
                            };
                            ui.label(egui::RichText::new(label).small().color(
                                if phase == CallPhase::Calling {
                                    theme::on_card(ui)
                                } else {
                                    theme::muted(ui)
                                },
                            ));
                            ui.label(
                                egui::RichText::new(&call.call.text)
                                    .size(call_size)
                                    .strong()
                                    .color(if phase == CallPhase::Executing {
                                        theme::muted(ui)
                                    } else {
                                        theme::on_card(ui)
                                    }),
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(if phase == CallPhase::Executing {
                                        "figure in progress".to_string()
                                    } else {
                                        format!(
                                            "executes in {:.1} beats",
                                            (call.execute_beat - beat).max(0.0)
                                        )
                                    })
                                    .color(theme::muted(ui)),
                                );
                                if self.prefs.show_original {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "· {}",
                                            call.call.original_text
                                        ))
                                        .small()
                                        .color(theme::muted(ui)),
                                    );
                                }
                            });
                        }
                    }
                } else if self.audio.is_finished() {
                    ui.label(egui::RichText::new("Dance complete").size(28.0).strong());
                    if self.audio.ending_requested() {
                        ui.label("The tune finished on a held last note.");
                    }
                    ui.label("Press Restart to run it again.");
                } else if self.audio.in_ending() {
                    ui.label(
                        egui::RichText::new("SLOW FINISH")
                            .small()
                            .color(theme::muted(ui)),
                    );
                    ui.label(egui::RichText::new(&self.tune.title).size(28.0).strong());
                    ui.label(
                        egui::RichText::new("Slowing down and holding the last note…")
                            .color(theme::muted(ui)),
                    );
                } else {
                    ui.label(
                        egui::RichText::new("Press Play to begin")
                            .size(28.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} calls · {} beats · {}",
                            self.timeline.calls.len(),
                            self.timeline.music_loop_beats,
                            count_in_label
                        ))
                        .color(theme::muted(ui)),
                    );
                }
            });

        if let Some(next) = &next
            && card_mode != CardMode::UpcomingMove
        {
            let timing = if compact {
                format!(
                    "beat {:.0} · call {:.1}",
                    next.execute_beat, next.start_beat
                )
            } else {
                format!(
                    "at beat {:.0} (call at {:.1})",
                    next.execute_beat, next.start_beat
                )
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Next:").color(theme::muted(ui)));
                ui.label(egui::RichText::new(&next.call.text).strong());
                ui.label(egui::RichText::new(timing).small().color(theme::muted(ui)));
            });
        }

        ui.add_space(6.0);
        let progress = if beat < 0.0 {
            0.0
        } else {
            (beat / self.timeline.music_loop_beats).clamp(0.0, 1.0)
        };
        let transport = transport_state(
            self.started,
            self.audio.is_playing(),
            self.audio.is_finished(),
            self.audio.in_ending(),
            beat,
        );
        let progress_text = progress_text(transport, beat, self.timeline.music_loop_beats);
        ui.add(
            egui::ProgressBar::new(progress as f32)
                .text(progress_text)
                .desired_width(f32::INFINITY),
        );

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Dance card").strong());
        if card_scroll {
            egui::ScrollArea::vertical()
                .id_salt("dance-card")
                .auto_shrink([false, false])
                .show(ui, |ui| self.dance_card_list(ui, beat, compact));
        } else {
            self.dance_card_list(ui, beat, compact);
        }

        if timeline_dirty && !(music_dirty && self.audio.is_playing()) {
            self.rebuild_timeline();
        }
        if music_dirty {
            // Re-render on the next frame after the control is released; the
            // old buffer keeps playing until then so playback does not restart.
            self.needs_render = true;
            self.pending_audio = true;
        }
        match action {
            Some("play") => self.play(),
            Some("pause") => self.pause(),
            Some("restart") => self.restart(),
            Some("end") | Some("cancel-ending") => self.end_tune(),
            _ => {}
        }
        if rotation_changed {
            self.rotation_settings_changed();
        }
        if save || timeline_dirty || music_dirty || rotation_changed {
            self.save_prefs();
        }
    }

    /// The dance card figures with their call text and beat counts.
    ///
    /// `full_width` makes each figure card span the panel on a phone, where a
    /// content-sized card leaves ragged edges and narrows the call text.
    fn dance_card_list(&self, ui: &mut egui::Ui, beat: f64, full_width: bool) {
        let animate = self.prefs.animations_enabled;
        let mut cursor = 0.0;
        for (figure_index, figure) in self.dance.figures.iter().enumerate() {
            let figure_end = cursor + figure.beats;
            let active_figure = beat >= cursor && beat < figure_end;
            let active_t = animate_amount(
                ui.ctx(),
                ("figure-active", figure_index),
                active_figure,
                animate,
            );
            let calls: Vec<(usize, contra_core::CallEvent)> = self
                .timeline
                .calls
                .iter()
                .enumerate()
                .filter(|(_, call)| call.figure_index == figure_index)
                .map(|(index, call)| (index, call.clone()))
                .collect();
            let fill = lerp_color(
                theme::call_upcoming_bg(ui),
                theme::call_active_bg(ui),
                active_t,
            );
            egui::Frame::group(ui.style())
                .fill(fill)
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    if full_width {
                        ui.set_width(ui.available_width());
                    }
                    ui.horizontal_wrapped(|ui| {
                        if let Some(phrase) = &figure.phrase {
                            ui.label(egui::RichText::new(phrase).strong().color(
                                if active_figure {
                                    theme::on_card(ui)
                                } else {
                                    theme::muted(ui)
                                },
                            ));
                        }
                        ui.label(
                            egui::RichText::new(format!("{} beats", figure.beats))
                                .small()
                                .color(theme::muted(ui)),
                        );
                    });
                    for (index, call) in &calls {
                        let phase = self
                            .timeline
                            .scheduled_call_for_index(*index, beat, self.prefs.loop_enabled)
                            .as_ref()
                            .map(|scheduled| phase_at(scheduled, beat))
                            .unwrap_or(CallPhase::Upcoming);
                        let color = match phase {
                            CallPhase::Upcoming => theme::muted(ui),
                            CallPhase::Calling => theme::on_card(ui),
                            CallPhase::Executing => theme::executing(ui),
                        };
                        ui.label(
                            egui::RichText::new(&call.text)
                                .size(if active_figure { 19.0 } else { 16.0 })
                                .color(color),
                        );
                        if self.prefs.show_original && call.original_text != call.text {
                            ui.label(
                                egui::RichText::new(&call.original_text)
                                    .small()
                                    .color(theme::subtle(ui)),
                            );
                        }
                    }
                });
            cursor = figure_end;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_state_names_every_session_state() {
        assert_eq!(
            transport_state(false, false, false, false, 0.0),
            TransportState::Ready
        );
        assert_eq!(
            transport_state(true, true, false, false, -2.0),
            TransportState::CountIn
        );
        assert_eq!(
            transport_state(true, true, false, false, 4.0),
            TransportState::Playing
        );
        assert_eq!(
            transport_state(true, false, false, false, 4.0),
            TransportState::Paused
        );
        assert_eq!(
            transport_state(true, true, false, true, 62.0),
            TransportState::Ending
        );
        assert_eq!(
            transport_state(true, false, true, false, 64.0),
            TransportState::Finished
        );
    }

    #[test]
    fn a_paused_run_is_not_hidden_by_the_count_in_or_a_pending_ending() {
        // Pausing in the count-in must read as Paused, or a test cannot tell a
        // stopped transport from one that is waiting for the first beat.
        assert_eq!(
            transport_state(true, false, false, false, -1.0),
            TransportState::Paused
        );
        // A finished run stays Finished even if the ending flag lingers.
        assert_eq!(
            transport_state(true, false, true, true, 64.0),
            TransportState::Finished
        );
    }

    #[test]
    fn progress_text_explains_the_state_and_position() {
        assert_eq!(
            progress_text(TransportState::Ready, 0.0, 64.0),
            "Ready · beat 0 / 64"
        );
        assert_eq!(
            progress_text(TransportState::Playing, 12.4, 64.0),
            "Playing · beat 12 / 64"
        );
        assert_eq!(
            progress_text(TransportState::Paused, 12.6, 64.0),
            "Paused · beat 13 / 64"
        );
        assert_eq!(
            progress_text(TransportState::Finished, 64.0, 64.0),
            "Finished · beat 64 / 64"
        );
        assert_eq!(
            progress_text(TransportState::Ending, 62.0, 64.0),
            "Ending · slow finish"
        );
    }

    #[test]
    fn count_in_text_counts_the_remaining_beats() {
        assert_eq!(
            progress_text(TransportState::CountIn, -4.0, 64.0),
            "Count-in · 4 beats left"
        );
        // The audio clock can land just past the first beat; the display must
        // still start at four, not five.
        assert_eq!(
            progress_text(TransportState::CountIn, -4.000_001, 64.0),
            "Count-in · 4 beats left"
        );
        assert_eq!(
            progress_text(TransportState::CountIn, -3.2, 64.0),
            "Count-in · 4 beats left"
        );
        assert_eq!(
            progress_text(TransportState::CountIn, -0.2, 64.0),
            "Count-in · 1 beat left"
        );
    }
}
