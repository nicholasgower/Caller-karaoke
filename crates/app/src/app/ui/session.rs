use super::*;
use crate::app::*;
use crate::audio::AudioEngine;
use crate::theme;
use crate::tts::Speech;
use contra_core::Preferences;
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
            let label = ui.label("Tempo");
            // Leave room for the value box and the " BPM" suffix.
            ui.spacing_mut().slider_width = (ui.available_width() - 100.0).max(96.0);
            let response = ui.add(
                egui::Slider::new(&mut prefs.tempo_bpm, 120.0..=320.0)
                    .fixed_decimals(0)
                    .suffix(" BPM"),
            );
            let response = response.labelled_by(label.id);
            if response.changed() {
                changes.timeline_dirty = true;
                changes.music_dirty = true;
            }
        });

        // Equal-width transport buttons: no control hides behind hover and
        // each is a full touch target.
        ui.horizontal(|ui| {
            let spacing = ui.spacing().item_spacing.x;
            let width = ((ui.available_width() - 2.0 * spacing) / 3.0).max(72.0);
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
        });

        ui.horizontal_wrapped(|ui| {
            if ui.checkbox(&mut prefs.loop_enabled, "Loop").changed() {
                audio.set_looping(prefs.loop_enabled);
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
            let label = ui.label("Volume");
            ui.spacing_mut().slider_width = (ui.available_width() - 80.0).max(96.0);
            let response =
                ui.add(egui::Slider::new(&mut prefs.master_volume, 0.0..=1.0).max_decimals(2));
            let response = response.labelled_by(label.id);
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
            if ui.checkbox(&mut prefs.loop_enabled, "Loop").changed() {
                audio.set_looping(prefs.loop_enabled);
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
                .changed()
            {
                audio.set_volume(prefs.master_volume);
                changes.save = true;
            }
        });
    }

    changes
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
            save,
            action,
        } = session_controls(
            ui,
            &mut self.prefs,
            &mut self.audio,
            &mut self.speech,
            compact,
        );

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
        if compact {
            // Two deliberate lines instead of one wrapped line, so the tune
            // and the instrument pair never break mid-phrase at phone width.
            ui.label(
                egui::RichText::new(format!(
                    "Tune: {} · {} · {}",
                    self.tune.title,
                    self.tune.display_key(),
                    self.tune.display_rhythm()
                ))
                .strong(),
            );
            ui.label(
                egui::RichText::new(format!(
                    "Melody: {} · Accompaniment: {}",
                    self.prefs.melody.label(),
                    self.prefs.accompaniment.label()
                ))
                .color(theme::muted(ui)),
            );
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
                ui.label(
                    egui::RichText::new(format!(
                        "Melody: {} · Accompaniment: {}",
                        self.prefs.melody.label(),
                        self.prefs.accompaniment.label()
                    ))
                    .color(theme::muted(ui)),
                );
            });
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
                            "{} calls · {} beats · three-beat count-in",
                            self.timeline.calls.len(),
                            self.timeline.music_loop_beats
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
                            egui::RichText::new(format!("{:.0}", (-beat).ceil()))
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
                    ui.label("Press Restart to run it again.");
                } else {
                    ui.label(
                        egui::RichText::new("Press Play to begin")
                            .size(28.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} calls · {} beats · 3-beat count-in",
                            self.timeline.calls.len(),
                            self.timeline.music_loop_beats
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
        ui.add(
            egui::ProgressBar::new(progress as f32)
                .text(format!(
                    "Beat {:.0} / {:.0}",
                    beat.clamp(0.0, self.timeline.music_loop_beats),
                    self.timeline.music_loop_beats
                ))
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
            _ => {}
        }
        if save || timeline_dirty || music_dirty {
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
