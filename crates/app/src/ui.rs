// Included from `app.rs`; these are the screen implementations.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CallPhase {
    Upcoming,
    Calling,
    Executing,
}

fn phase_at(call: &contra_core::ScheduledCall, beat: f64) -> CallPhase {
    if beat < call.start_beat {
        CallPhase::Upcoming
    } else if beat < call.execute_beat {
        CallPhase::Calling
    } else {
        CallPhase::Executing
    }
}

/// What the large prompt card is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CardMode {
    /// The call the caller is about to speak or is speaking now.
    CurrentCall,
    /// The figure is already being danced, so the card shows the move the
    /// caller must say next.
    UpcomingMove,
    /// The figure is being danced and there is no following move to call.
    FinalMove,
}

/// Pick the card content from the active call's phase and whether another
/// call follows it.
///
/// A caller looks ahead while the dancers are executing, so the card must stop
/// describing the figure in progress and start prompting the next move.
fn card_state(phase: Option<CallPhase>, has_next: bool) -> CardMode {
    match phase {
        Some(CallPhase::Executing) if has_next => CardMode::UpcomingMove,
        Some(CallPhase::Executing) => CardMode::FinalMove,
        _ => CardMode::CurrentCall,
    }
}

/// Prompt the caller with the move they must call next while the current
/// figure is being danced.
fn upcoming_move_card(
    ui: &mut egui::Ui,
    next: &contra_core::ScheduledCall,
    beat: f64,
    show_original: bool,
) {
    ui.label(
        egui::RichText::new("UPCOMING MOVE")
            .small()
            .color(theme::muted(ui)),
    );
    ui.label(
        egui::RichText::new(&next.call.text)
            .size(34.0)
            .strong()
            .color(theme::on_card(ui)),
    );
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "call in {:.1} beats",
                (next.start_beat - beat).max(0.0)
            ))
            .color(theme::muted(ui)),
        );
        if show_original {
            ui.label(
                egui::RichText::new(format!("· {}", next.call.original_text))
                    .small()
                    .color(theme::muted(ui)),
            );
        }
    });
}

impl ContraApp {
    fn session_ui(&mut self, ui: &mut egui::Ui, beat: f64) {
        let animate = self.prefs.animations_enabled;
        let mut timeline_dirty = false;
        let mut music_dirty = false;
        let mut save = false;
        let mut action: Option<&'static str> = None;

        ui.horizontal_wrapped(|ui| {
            ui.label("Tempo");
            if ui
                .add(
                    egui::Slider::new(&mut self.prefs.tempo_bpm, 120.0..=320.0)
                        .text("BPM")
                        .fixed_decimals(0),
                )
                .changed()
            {
                timeline_dirty = true;
                music_dirty = true;
            }
            if ui
                .button("Play")
                .on_hover_text("Play (Space)")
                .clicked()
            {
                action = Some("play");
            }
            if ui
                .button("Pause")
                .on_hover_text("Pause (Space)")
                .clicked()
            {
                action = Some("pause");
            }
            if ui
                .button("Restart")
                .on_hover_text("Restart (R)")
                .clicked()
            {
                action = Some("restart");
            }
            if ui
                .checkbox(&mut self.prefs.loop_enabled, "Loop")
                .changed()
            {
                self.audio.set_looping(self.prefs.loop_enabled);
                save = true;
            }
            if ui
                .checkbox(&mut self.prefs.count_in_enabled, "Count-in")
                .changed()
            {
                timeline_dirty = true;
                music_dirty = true;
            }
            if ui
                .checkbox(&mut self.prefs.tts_enabled, "Speak calls")
                .changed()
            {
                self.speech.set_enabled(self.prefs.tts_enabled);
                if !self.prefs.tts_enabled {
                    self.speech.stop();
                }
                save = true;
            }
            ui.label("Volume");
            if ui
                .add(egui::Slider::new(
                    &mut self.prefs.master_volume,
                    0.0..=1.0,
                ))
                .changed()
            {
                self.audio.set_volume(self.prefs.master_volume);
                save = true;
            }
        });

        if self.needs_render {
            ui.horizontal(|ui| {
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
            ui.colored_label(
                theme::error(ui),
                format!("Audio device: {error}"),
            );
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
                ui.label(egui::RichText::new(&status.message).small().color(theme::muted(ui)));
            }
        }

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        ui.heading(&self.dance.title);
        ui.label(
            egui::RichText::new(format!(
                "{} · {} · {} beats",
                empty_label(&self.dance.choreographer, "unknown choreographer"),
                empty_label(&self.dance.formation, "formation unknown"),
                self.dance.total_beats()
            ))
            .color(theme::muted(ui)),
        );
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
        let card_mode = card_state(
            active.as_ref().map(|call| phase_at(call, beat)),
            next.is_some(),
        );

        egui::Frame::group(ui.style())
            .fill(theme::call_active_bg(ui))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                if !self.started {
                    ui.label(
                        egui::RichText::new("READY")
                            .small()
                            .color(theme::muted(ui)),
                    );
                    ui.label(egui::RichText::new("Press Play to begin").size(30.0).strong());
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
                                .size(56.0)
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
                                upcoming_move_card(ui, upcoming, beat, self.prefs.show_original);
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
                                    .size(34.0)
                                    .strong()
                                    .color(theme::muted(ui)),
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new("final figure")
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
                        CardMode::CurrentCall => {
                            let phase = phase_at(call, beat);
                            let label = match phase {
                                CallPhase::Upcoming => "UPCOMING",
                                CallPhase::Calling => "CALL NOW",
                                CallPhase::Executing => "DANCING",
                            };
                            ui.label(
                                egui::RichText::new(label)
                                    .small()
                                    .color(if phase == CallPhase::Calling {
                                        theme::on_card(ui)
                                    } else {
                                        theme::muted(ui)
                                    }),
                            );
                            ui.label(
                                egui::RichText::new(&call.call.text)
                                    .size(34.0)
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
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Next:").color(theme::muted(ui)));
                ui.label(egui::RichText::new(&next.call.text).strong());
                ui.label(
                    egui::RichText::new(format!(
                        "at beat {:.0} (call at {:.1})",
                        next.execute_beat, next.start_beat
                    ))
                    .small()
                    .color(theme::muted(ui)),
                );
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
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
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
                            ui.horizontal_wrapped(|ui| {
                                if let Some(phrase) = &figure.phrase {
                                    ui.label(
                                        egui::RichText::new(phrase)
                                            .strong()
                                            .color(if active_figure {
                                                theme::on_card(ui)
                                            } else {
                                                theme::muted(ui)
                                            }),
                                    );
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
                                    .scheduled_call_for_index(
                                        *index,
                                        beat,
                                        self.prefs.loop_enabled,
                                    )
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
            });

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

    fn dance_picker_ui(&mut self, ui: &mut egui::Ui) {
        let animate = self.prefs.animations_enabled;
        ui.horizontal(|ui| {
            if ui
                .button("← Back to session")
                .on_hover_text("Back to session (Esc)")
                .clicked()
            {
                self.screen = Screen::Session;
            }
            ui.heading("Select dance");
        });
        ui.label(
            egui::RichText::new("Search ContraDB and preview the dance card before loading it.")
                .color(theme::muted(ui)),
        );
        ui.add_space(6.0);

        let mut submit = false;
        egui::Grid::new("dance-search-grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Title");
                submit |= submit_text_edit(ui, &mut self.dance_query.title);
                ui.end_row();
                ui.label("Choreographer");
                submit |= submit_text_edit(ui, &mut self.dance_query.choreographer);
                ui.end_row();
                ui.label("User");
                submit |= submit_text_edit(ui, &mut self.dance_query.user);
                ui.end_row();
                ui.label("Hook");
                submit |= submit_text_edit(ui, &mut self.dance_query.hook);
                ui.end_row();
                ui.label("Formation");
                egui::ComboBox::from_id_salt("formation")
                    .selected_text(formation_label(self.dance_query.formation.as_deref()))
                    .show_ui(ui, |ui| {
                        for (value, label) in [
                            ("", "Any formation"),
                            ("improper", "Improper"),
                            ("becket", "Becket"),
                            ("proper", "Proper"),
                            ("everything else", "Everything else"),
                        ] {
                            let selected = self
                                .dance_query
                                .formation
                                .as_deref()
                                .unwrap_or("")
                                == value;
                            if ui.selectable_label(selected, label).clicked() {
                                self.dance_query.formation = if value.is_empty() {
                                    None
                                } else {
                                    Some(value.to_string())
                                };
                            }
                        }
                    });
                ui.end_row();
                ui.label("Figure type");
                let figure_options = [
                    ("", "Any figure"),
                    ("swing", "Swing"),
                    ("allemande", "Allemande"),
                    ("do si do", "Do-si-do"),
                    ("balance", "Balance"),
                    ("chain", "Chain"),
                    ("circle", "Circle"),
                    ("star", "Star"),
                    ("hey", "Hey"),
                    ("gyre", "Gyre"),
                    ("promenade", "Promenade"),
                    ("long lines", "Long lines"),
                    ("right left through", "Right and left through"),
                    ("pass through", "Pass through"),
                    ("petronella", "Petronella"),
                    ("mad robin", "Mad robin"),
                ];
                let selected = self
                    .dance_query
                    .figure_move
                    .as_deref()
                    .and_then(|current| {
                        figure_options
                            .iter()
                            .find(|(value, _)| *value == current)
                            .map(|(_, label)| *label)
                    })
                    .unwrap_or("Any figure");
                egui::ComboBox::from_id_salt("figure-move")
                    .selected_text(selected)
                    .show_ui(ui, |ui| {
                        for (value, label) in figure_options {
                            let is_selected =
                                self.dance_query.figure_move.as_deref().unwrap_or("") == value;
                            if ui.selectable_label(is_selected, label).clicked() {
                                self.dance_query.figure_move = if value.is_empty() {
                                    None
                                } else {
                                    Some(value.to_string())
                                };
                            }
                        }
                    });
                ui.end_row();
                ui.label("Figure contains");
                submit |= submit_text_edit(ui, &mut self.dance_query.figure_text);
                ui.end_row();
                ui.label("Verified only");
                ui.checkbox(&mut self.dance_query.verified, "");
                ui.end_row();
                ui.label("Sort");
                egui::ComboBox::from_id_salt("dance-sort")
                    .selected_text(self.dance_query.sort.label())
                    .show_ui(ui, |ui| {
                        for sort in DanceSort::ALL {
                            ui.selectable_value(&mut self.dance_query.sort, sort, sort.label());
                        }
                    });
                ui.end_row();
            });
        if submit {
            self.search_dances();
        }
        ui.horizontal(|ui| {
            let search = ui
                .add_enabled(!self.search_loading, egui::Button::new("Search dances"))
                .on_hover_text("Search dances (Enter)");
            if search.clicked() {
                self.search_dances();
            }
            if self.search_loading {
                ui.spinner();
                ui.label("Searching…");
            }
            if let Some(results) = &self.dance_results {
                ui.label(
                    egui::RichText::new(format!("{} matches", results.total)).color(theme::muted(ui)),
                );
            }
        });

        ui.add_space(6.0);
        ui.columns(2, |columns| {
            egui::ScrollArea::vertical()
                .id_salt("dance-results")
                .max_height(420.0)
                .show(&mut columns[0], |ui| {
                    let Some(results) = self.dance_results.clone() else {
                        return;
                    };
                    for dance in results.dances {
                        let selected = self
                            .dance_selected
                            .as_ref()
                            .is_some_and(|value| value.id == dance.id);
                        let selected_t = animate_amount(
                            ui.ctx(),
                            ("dance-selected", &dance.id),
                            selected,
                            animate,
                        );
                        egui::Frame::group(ui.style())
                            .fill(lerp_color(
                                theme::call_upcoming_bg(ui),
                                theme::call_active_bg(ui),
                                selected_t,
                            ))
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(&dance.title).strong());
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} · {}",
                                                empty_label(&dance.choreographer, "unknown"),
                                                empty_label(&dance.formation, "unknown")
                                            ))
                                            .small()
                                            .color(theme::muted(ui)),
                                        );
                                    });
                                    if ui.small_button("Preview").clicked() {
                                        self.dance_selected = Some(dance.clone());
                                        self.source.load_dance(dance.id.clone());
                                        self.preview_loading = true;
                                    }
                                });
                            });
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("dance-preview")
                .max_height(420.0)
                .show(&mut columns[1], |ui| {
                    if self.preview_loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading dance…");
                        });
                    }
                    if let Some(dance) = self.dance_preview.clone() {
                        ui.heading(&dance.title);
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {}",
                                empty_label(&dance.choreographer, "unknown"),
                                empty_label(&dance.formation, "unknown")
                            ))
                            .color(theme::muted(ui)),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{} figures · {} beats",
                                dance.figures.len(),
                                dance.total_beats()
                            ))
                            .small()
                            .color(theme::muted(ui)),
                        );
                        ui.hyperlink_to("Open on ContraDB", &dance.source.url);
                        ui.separator();
                        for figure in &dance.figures {
                            ui.label(format!(
                                "{}  {:.0}b  {}",
                                figure.phrase.as_deref().unwrap_or(""),
                                figure.beats,
                                figure.text
                            ));
                        }
                        ui.add_space(6.0);
                        if ui.button("Use this dance").clicked() {
                            self.use_dance(dance);
                        }
                    } else if !self.preview_loading {
                        ui.label(
                            egui::RichText::new("Preview a dance to see its card.")
                                .color(theme::muted(ui)),
                        );
                    }
                });
        });
        self.pagination_ui(ui, true);
    }

    fn tune_picker_ui(&mut self, ui: &mut egui::Ui) {
        let animate = self.prefs.animations_enabled;
        ui.horizontal(|ui| {
            if ui
                .button("← Back to session")
                .on_hover_text("Back to session (Esc)")
                .clicked()
            {
                self.screen = Screen::Session;
            }
            ui.heading("Select Tune");
        });
        ui.label(
            egui::RichText::new(
                "Search ABC Session Tunes and preview the sheet music before loading it.",
            )
            .color(theme::muted(ui)),
        );
        ui.add_space(6.0);

        let mut submit = false;
        egui::Grid::new("tune-search-grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Search");
                submit |= submit_text_edit(ui, &mut self.tune_query.search);
                ui.end_row();
                ui.label("Composer");
                submit |= submit_text_edit(ui, &mut self.tune_query.composer);
                ui.end_row();
                ui.label("Key");
                combo_option(ui, "tune-key", &mut self.tune_query.key, &[
                    ("", "Any key"),
                    ("D", "D major"),
                    ("G", "G major"),
                    ("A", "A major"),
                    ("C", "C major"),
                    ("F", "F major"),
                    ("Bb", "B♭ major"),
                    ("Am", "A minor"),
                    ("Em", "E minor"),
                    ("Bm", "B minor"),
                    ("Dm", "D minor"),
                    ("Ador", "A dorian"),
                    ("Amix", "A mixolydian"),
                ]);
                ui.end_row();
                ui.label("Meter");
                combo_option(ui, "tune-meter", &mut self.tune_query.meter, &[
                    ("", "Any meter"),
                    ("4/4", "4/4"),
                    ("2/2", "2/2"),
                    ("2/4", "2/4"),
                    ("6/8", "6/8"),
                    ("9/8", "9/8"),
                    ("12/8", "12/8"),
                    ("3/4", "3/4"),
                ]);
                ui.end_row();
                ui.label("Genre");
                combo_option(ui, "tune-rhythm", &mut self.tune_query.rhythm, &[
                    ("", "Any genre"),
                    ("reel", "Reel"),
                    ("jig", "Jig"),
                    ("slip jig", "Slip jig"),
                    ("hornpipe", "Hornpipe"),
                    ("polka", "Polka"),
                    ("slide", "Slide"),
                    ("strathspey", "Strathspey"),
                    ("waltz", "Waltz"),
                ]);
                ui.end_row();
            });
        if submit {
            self.search_tunes();
        }
        ui.horizontal(|ui| {
            let search = ui
                .add_enabled(!self.search_loading, egui::Button::new("Search tunes"))
                .on_hover_text("Search tunes (Enter)");
            if search.clicked() {
                self.search_tunes();
            }
            if self.search_loading {
                ui.spinner();
                ui.label("Searching…");
            }
            if let Some(results) = &self.tune_results {
                ui.label(
                    egui::RichText::new(format!("{} matches", results.total)).color(theme::muted(ui)),
                );
            }
        });

        ui.add_space(6.0);
        ui.columns(2, |columns| {
            egui::ScrollArea::vertical()
                .id_salt("tune-results")
                .max_height(420.0)
                .show(&mut columns[0], |ui| {
                    let Some(results) = self.tune_results.clone() else {
                        return;
                    };
                    for tune in results.tunes {
                        let selected = self
                            .tune_selected
                            .as_ref()
                            .is_some_and(|value| value.id == tune.id);
                        let selected_t = animate_amount(
                            ui.ctx(),
                            ("tune-selected", &tune.id),
                            selected,
                            animate,
                        );
                        egui::Frame::group(ui.style())
                            .fill(lerp_color(
                                theme::call_upcoming_bg(ui),
                                theme::call_active_bg(ui),
                                selected_t,
                            ))
                            .inner_margin(egui::Margin::same(8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(egui::RichText::new(&tune.title).strong());
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} · {} · {}",
                                                empty_label(&tune.composer, "unknown"),
                                                tune.key,
                                                tune.meter
                                            ))
                                            .small()
                                            .color(theme::muted(ui)),
                                        );
                                    });
                                    if ui.small_button("Preview").clicked() {
                                        self.tune_selected = Some(tune.clone());
                                        self.source.load_tune(tune.id.clone());
                                        self.preview_loading = true;
                                    }
                                });
                            });
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("tune-preview")
                .max_height(420.0)
                .show(&mut columns[1], |ui| {
                    if self.preview_loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading tune…");
                        });
                    }
                    if let Some(tune) = self.tune_preview.clone() {
                        ui.heading(&tune.title);
                        ui.label(
                            egui::RichText::new(format!(
                                "{} · {} · {}",
                                empty_label(&tune.composer, "unknown"),
                                tune.display_key(),
                                tune.display_meter()
                            ))
                            .color(theme::muted(ui)),
                        );
                        ui.label(
                            egui::RichText::new(&tune.credits)
                                .small()
                                .color(theme::muted(ui)),
                        );
                        ui.horizontal_wrapped(|ui| {
                            ui.hyperlink_to("Open on ABC Session Tunes", &tune.source.url);
                            if ui.button(self.tune_view.toggle_label()).clicked() {
                                self.tune_view = self.tune_view.toggled();
                            }
                        });
                        ui.separator();
                        match self.tune_view {
                            TuneView::Sheet => {
                                if self.tune_sheet.is_none() {
                                    self.tune_sheet =
                                        Some(SheetPreview::render(ui.ctx(), &tune.abc));
                                }
                                if let Some(result) = &self.tune_sheet {
                                    match result {
                                        Ok(sheet) => sheet.ui(ui),
                                        Err(error) => {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "Could not render sheet music: {error}"
                                                ))
                                                .color(theme::warning(ui)),
                                            );
                                            ui.label(
                                                egui::RichText::new(
                                                    "The tune still loads; switch to ABC to read or copy the source.",
                                                )
                                                .small()
                                                .color(theme::muted(ui)),
                                            );
                                        }
                                    }
                                }
                            }
                            TuneView::Abc => {
                                let mut abc_preview = tune.abc.clone();
                                ui.add(
                                    egui::TextEdit::multiline(&mut abc_preview)
                                        .font(egui::TextStyle::Monospace)
                                        .desired_rows(12)
                                        .desired_width(f32::INFINITY),
                                );
                            }
                        }
                        ui.add_space(6.0);
                        if ui.button("Use this tune").clicked() {
                            self.use_tune(tune);
                        }
                    } else if !self.preview_loading {
                        ui.label(
                            egui::RichText::new("Preview a tune to see its sheet music.")
                                .color(theme::muted(ui)),
                        );
                    }
                });
        });
        self.pagination_ui(ui, false);
    }

    fn pagination_ui(&mut self, ui: &mut egui::Ui, dance: bool) {
        let (page, page_size, total) = if dance {
            let results = self.dance_results.as_ref();
            (
                self.dance_query.page.max(1),
                self.dance_query.page_size.max(1),
                results.map(|page| page.total).unwrap_or(0),
            )
        } else {
            let results = self.tune_results.as_ref();
            (
                self.tune_query.page.max(1),
                self.tune_query.page_size.max(1),
                results.map(|page| page.total).unwrap_or(0),
            )
        };
        let pages = total.div_ceil(page_size);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(page > 1, egui::Button::new("Previous"))
                .clicked()
            {
                if dance {
                    self.dance_query.page = page - 1;
                    self.source.search_dances(self.dance_query.clone());
                } else {
                    self.tune_query.page = page - 1;
                    self.source.search_tunes(self.tune_query.clone());
                }
                self.search_loading = true;
            }
            ui.label(format!("Page {page} of {}", pages.max(1)));
            if ui
                .add_enabled(page < pages, egui::Button::new("Next"))
                .clicked()
            {
                if dance {
                    self.dance_query.page = page + 1;
                    self.source.search_dances(self.dance_query.clone());
                } else {
                    self.tune_query.page = page + 1;
                    self.source.search_tunes(self.tune_query.clone());
                }
                self.search_loading = true;
            }
        });
    }

    fn preferences_ui(&mut self, ui: &mut egui::Ui) {
        let before = self.prefs.clone();
        ui.horizontal(|ui| {
            if ui
                .button("← Back to session")
                .on_hover_text("Back to session (Esc)")
                .clicked()
            {
                self.screen = Screen::Session;
            }
            ui.heading("Preferences");
        });
        ui.label(
            egui::RichText::new("Changes are saved automatically. Playback settings are applied without restarting the music; role and alias changes rebuild the call timeline.")
                .color(theme::muted(ui)),
        );
        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Role terms");
            let mut preset = RolePreset::detect(&self.prefs.role_terms);
            egui::ComboBox::from_label("Role terms")
                .selected_text(preset.label())
                .show_ui(ui, |ui| {
                    for option in RolePreset::ALL {
                        ui.selectable_value(&mut preset, option, option.label());
                    }
                });
            if preset != RolePreset::detect(&self.prefs.role_terms) {
                self.prefs.role_terms = RoleTerms::preset(preset);
                self.rebuild_timeline();
                self.needs_render = true;
                self.save_prefs();
            }
            if preset == RolePreset::Custom {
                egui::Grid::new("custom-roles")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Left singular");
                        ui.text_edit_singleline(&mut self.prefs.role_terms.left_singular);
                        ui.end_row();
                        ui.label("Left plural");
                        ui.text_edit_singleline(&mut self.prefs.role_terms.left_plural);
                        ui.end_row();
                        ui.label("Right singular");
                        ui.text_edit_singleline(&mut self.prefs.role_terms.right_singular);
                        ui.end_row();
                        ui.label("Right plural");
                        ui.text_edit_singleline(&mut self.prefs.role_terms.right_plural);
                        ui.end_row();
                    });
            }
            ui.label(
                egui::RichText::new(self.prefs.role_terms.apply(
                    "ladles allemande left 1½; gentlespoons do si do once",
                ))
                .italics()
                .color(theme::muted(ui)),
            );
            if ui.button("Edit dance move aliases…").clicked() {
                self.screen = Screen::Aliases;
            }
            ui.separator();

            ui.heading("Instruments");
            egui::Grid::new("instruments")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Melody");
                    egui::ComboBox::from_id_salt("melody")
                        .selected_text(self.prefs.melody.label())
                        .show_ui(ui, |ui| {
                            for option in MelodyInstrument::ALL {
                                ui.selectable_value(
                                    &mut self.prefs.melody,
                                    option,
                                    option.label(),
                                );
                            }
                        });
                    ui.end_row();
                    ui.label("Accompaniment");
                    egui::ComboBox::from_id_salt("accompaniment")
                        .selected_text(self.prefs.accompaniment.label())
                        .show_ui(ui, |ui| {
                            for option in AccompanimentInstrument::ALL {
                                ui.selectable_value(
                                    &mut self.prefs.accompaniment,
                                    option,
                                    option.label(),
                                );
                            }
                        });
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                ui.label("Melody volume");
                ui.add(egui::Slider::new(
                    &mut self.prefs.melody_volume,
                    0.0..=1.5,
                ));
            });
            ui.horizontal(|ui| {
                ui.label("Accompaniment volume");
                ui.add(egui::Slider::new(
                    &mut self.prefs.accompaniment_volume,
                    0.0..=1.5,
                ));
            });
            ui.separator();

            ui.heading("Theme");
            ui.horizontal(|ui| {
                for option in Theme::ALL {
                    ui.selectable_value(&mut self.prefs.theme, option, option.label());
                }
            });
            ui.heading("Interface");
            ui.checkbox(&mut self.prefs.animations_enabled, "Smooth animations");
            ui.label(
                egui::RichText::new(
                    "Fade and slide between screens and animate tabs and highlights. Turn off to reduce motion.",
                )
                .small()
                .color(theme::muted(ui)),
            );
            ui.label(
                egui::RichText::new(
                    "Keyboard: Space play/pause · R restart · Esc back · 1–4 switch tabs · Enter starts a search or adds an alias.",
                )
                .small()
                .color(theme::muted(ui)),
            );
            ui.separator();

            ui.heading("Calling lead");
            ui.checkbox(&mut self.prefs.lead.auto, "Automatic syllable-based lead");
            if self.prefs.lead.auto {
                ui.horizontal(|ui| {
                    ui.label("Minimum speech lead");
                    ui.add(
                        egui::Slider::new(&mut self.prefs.lead.min_beats, 0.5..=4.0)
                            .text("beats"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Maximum speech lead");
                    ui.add(
                        egui::Slider::new(&mut self.prefs.lead.max_beats, 1.0..=6.0)
                            .text("beats"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Speech rate");
                    ui.add(
                        egui::Slider::new(
                            &mut self.prefs.lead.syllables_per_second,
                            3.0..=10.0,
                        )
                        .text("syllables/s"),
                    );
                });
            } else {
                ui.horizontal(|ui| {
                    ui.label("Manual speech lead");
                    ui.add(
                        egui::Slider::new(&mut self.prefs.lead.manual_beats, 0.5..=6.0)
                            .text("beats"),
                    );
                });
            }
            ui.label(
                egui::RichText::new("Every call also gets a flat 0.5 s safety padding.")
                    .small()
                    .color(theme::muted(ui)),
            );
            if ui.button("Rebuild call timing").clicked() {
                self.rebuild_timeline();
                self.needs_render = true;
                self.save_prefs();
            }
            ui.separator();

            ui.heading("Text to speech");
            if ui
                .checkbox(&mut self.prefs.tts_enabled, "Speak calls")
                .changed()
            {
                self.speech.set_enabled(self.prefs.tts_enabled);
                if !self.prefs.tts_enabled {
                    self.speech.stop();
                }
            }
            ui.label(
                egui::RichText::new(
                    "Desktop builds use the local Kokoro neural voice and download the selected \
                     voice once, then cache it. Web and Android use the platform voice.",
                )
                .small()
                .color(theme::muted(ui)),
            );
            let neural = cfg!(all(
                not(target_arch = "wasm32"),
                not(target_os = "android")
            ));
            ui.horizontal(|ui| {
                ui.label("Voice");
                ui.add_enabled_ui(neural, |ui| {
                    let selected = crate::tts::voice(&self.prefs.tts_voice);
                    egui::ComboBox::from_id_salt("tts_voice")
                        .selected_text(selected.label)
                        .show_ui(ui, |ui| {
                            for voice in crate::tts::VOICES {
                                if ui
                                    .selectable_label(self.prefs.tts_voice == voice.id, voice.label)
                                    .clicked()
                                {
                                    self.prefs.tts_voice = voice.id.to_string();
                                    self.speech.set_voice(&self.prefs.tts_voice);
                                }
                            }
                        });
                });
            });
            let selected = crate::tts::voice(&self.prefs.tts_voice);
            ui.label(
                egui::RichText::new(if neural {
                    selected.detail
                } else {
                    "The platform voice is used in this build."
                })
                .small()
                .color(theme::muted(ui)),
            );
            ui.horizontal(|ui| {
                ui.label("Rate");
                if ui
                    .add(egui::Slider::new(&mut self.prefs.tts_rate, 0.5..=2.0))
                    .changed()
                {
                    self.speech.set_rate(self.prefs.tts_rate);
                }
            });
            ui.checkbox(
                &mut self.prefs.shuffle_alternatives,
                "Shuffle alternative call aliases",
            );
            ui.label(
                egui::RichText::new(
                    "Randomly uses an alternative wording, such as \"partner swing\" instead of \"swing your partner\", for moves that have one. The chosen wording is shown and spoken, and its timing is used.",
                )
                .small()
                .color(theme::muted(ui)),
            );
            let status = self.speech.status().clone();
            if let Some(progress) = status.progress {
                ui.add(
                    egui::ProgressBar::new(progress)
                        .show_percentage()
                        .text(status.message.clone()),
                );
            } else {
                let color = if status.error.is_some() {
                    theme::error(ui)
                } else {
                    theme::muted(ui)
                };
                ui.label(egui::RichText::new(&status.message).small().color(color));
            }
            if let Some(error) = &status.error {
                ui.label(
                    egui::RichText::new(format!("Using the platform voice: {error}"))
                        .small()
                        .color(theme::muted(ui)),
                );
            } else if !self.speech.available()
                && let Some(error) = self.speech.error()
            {
                ui.label(
                    egui::RichText::new(format!("TTS unavailable: {error}"))
                        .small()
                        .color(theme::muted(ui)),
                );
            }
            if status.error.is_some()
                && self.speech.can_retry()
                && ui.button("Retry neural voice").clicked()
            {
                self.speech.retry();
            }
            ui.checkbox(&mut self.prefs.loop_enabled, "Loop the dance and music");
            ui.checkbox(&mut self.prefs.count_in_enabled, "Three-beat count-in");
            ui.checkbox(
                &mut self.prefs.accompaniment_variations,
                "Vary the accompaniment on each loop",
            );
            ui.label(
                egui::RichText::new("Adds small emphasis notes and the occasional bar with melody alone.")
                    .small()
                    .color(theme::muted(ui)),
            );
            ui.checkbox(&mut self.prefs.show_original, "Show original card text");
            ui.separator();

            ui.heading("Soundfont");
            let current = self
                .prefs
                .soundfont
                .as_ref()
                .map(|setting| setting.name.clone())
                .unwrap_or_else(|| "None (built-in synth)".into());
            ui.label(current);
            if let Some(error) = &self.soundfont_error {
                ui.colored_label(theme::error(ui), error);
            }
            ui.horizontal(|ui| {
                if ui.button("Load .sf2…").clicked() {
                    self.load_soundfont();
                }
                if self.prefs.soundfont.is_some() && ui.button("Remove").clicked() {
                    self.remove_soundfont();
                }
            });
            ui.separator();

            ui.heading("Web data proxy");
            ui.label(
                egui::RichText::new("Only needed by the web build. Desktop and Android fetch sources directly.")
                    .small()
                    .color(theme::muted(ui)),
            );
            let mut proxy = self.prefs.web_proxy_url.clone().unwrap_or_default();
            let mut token = self.prefs.web_proxy_token.clone().unwrap_or_default();
            let mut proxy_changed = false;
            egui::Grid::new("proxy")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Proxy URL");
                    if ui.text_edit_singleline(&mut proxy).changed() {
                        proxy_changed = true;
                    }
                    ui.end_row();
                    ui.label("Token");
                    if ui.text_edit_singleline(&mut token).changed() {
                        proxy_changed = true;
                    }
                    ui.end_row();
                });
            if proxy_changed {
                self.prefs.web_proxy_url = (!proxy.trim().is_empty()).then_some(proxy);
                self.prefs.web_proxy_token = (!token.trim().is_empty()).then_some(token);
                self.source.set_proxy(
                    self.prefs.web_proxy_url.clone(),
                    self.prefs.web_proxy_token.clone(),
                );
                self.save_prefs();
            }
            ui.separator();

            ui.heading("Maintenance");
            ui.horizontal_wrapped(|ui| {
                if ui.button("Export MIDI…").clicked() {
                    self.export_midi();
                }
                if self.cache_clear_confirm {
                    if ui.button("Confirm clear cache").clicked() {
                        self.source.clear_cache();
                        self.cache_clear_confirm = false;
                        self.status = "Cache cleared".into();
                    }
                    if ui.button("Cancel").clicked() {
                        self.cache_clear_confirm = false;
                    }
                } else if ui.button("Clear source cache").clicked() {
                    self.cache_clear_confirm = true;
                }
            });
            ui.label(egui::RichText::new(&self.status).small().color(theme::muted(ui)));
        });

        self.prefs.validate();
        if self.prefs != before {
            let timeline_changed = self.prefs.role_terms != before.role_terms
                || self.prefs.aliases != before.aliases
                || self.prefs.lead != before.lead
                || self.prefs.tempo_bpm != before.tempo_bpm
                || self.prefs.count_in_enabled != before.count_in_enabled
                || self.prefs.shuffle_alternatives != before.shuffle_alternatives;
            let audio_changed = self.prefs.tempo_bpm != before.tempo_bpm
                || self.prefs.count_in_enabled != before.count_in_enabled
                || self.prefs.accompaniment_variations != before.accompaniment_variations
                || self.prefs.melody != before.melody
                || self.prefs.accompaniment != before.accompaniment
                || self.prefs.melody_volume != before.melody_volume
                || self.prefs.accompaniment_volume != before.accompaniment_volume
                || self.prefs.soundfont != before.soundfont;
            if timeline_changed {
                self.rebuild_timeline();
                self.last_spoken = None;
            }
            if audio_changed {
                // Apply on the next frame once the user releases the control.
                // Keep playing the old buffer in the meantime so changing a
                // setting never restarts playback.
                self.needs_render = true;
                self.pending_audio = true;
            }
            if !self.prefs.tts_enabled {
                self.speech.stop();
            }
            self.audio.set_looping(self.prefs.loop_enabled);
            self.audio.set_volume(self.prefs.master_volume);
            self.save_prefs();
        }
    }

    fn aliases_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .button("← Back to preferences")
                .on_hover_text("Back to preferences (Esc)")
                .clicked()
            {
                self.screen = Screen::Preferences;
            }
            ui.heading("Dance move aliases");
        });
        ui.label(
            egui::RichText::new(concat!(
                "These are the words shown and spoken during a session. Placeholders: ",
                "{actor}, {actor_singular}, {actor_plural}, {actor_object}, ",
                "{actor_object_plural}, {direction}, {count}."
            ))
            .color(theme::muted(ui)),
        );
        ui.label(
            egui::RichText::new("Alternatives are used only when \"Shuffle alternative call aliases\" is on in Preferences.")
                .small()
                .color(theme::muted(ui)),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label("Filter");
            ui.text_edit_singleline(&mut self.alias_filter);
            if ui.button("Reset all aliases").clicked() {
                self.prefs.aliases = AliasTable::default();
                self.rebuild_timeline();
                self.needs_render = true;
                self.save_prefs();
            }
        });

        let filter = self.alias_filter.to_lowercase();
        egui::ScrollArea::vertical()
            .max_height(460.0)
            .show(ui, |ui| {
                let mut changed = false;
                for kind in MoveKind::ALL {
                    if kind == MoveKind::Other {
                        continue;
                    }
                    if !filter.is_empty()
                        && !kind.label().to_lowercase().contains(&filter)
                        && !kind.id().contains(&filter)
                    {
                        continue;
                    }
                    egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::same(6))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(kind.label()).strong());
                                if self.prefs.aliases.is_customised(kind) {
                                    ui.label(
                                        egui::RichText::new("overridden")
                                            .small()
                                            .color(theme::muted(ui)),
                                    );
                                }
                            });
                            let mut value = self.prefs.aliases.template(kind).to_string();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut value)
                                        .desired_width(f32::INFINITY),
                                )
                                .changed()
                            {
                                self.prefs.aliases.set_override(kind, value);
                                changed = true;
                            }
                            if ui.small_button("Reset to default").clicked() {
                                self.prefs.aliases.reset(kind);
                                changed = true;
                            }

                            let alternatives = self.prefs.aliases.alternatives(kind);
                            let added = self.prefs.aliases.custom_alternatives_for(kind).to_vec();
                            if !alternatives.is_empty() {
                                ui.label(
                                    egui::RichText::new("Alternative calls")
                                        .small()
                                        .color(theme::muted(ui)),
                                );
                                for template in &alternatives {
                                    let is_added = added.iter().any(|value| value == template);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!("· {template}"))
                                                .small()
                                                .color(theme::muted(ui)),
                                        );
                                        if is_added && ui.small_button("Remove").clicked() {
                                            self.prefs
                                                .aliases
                                                .remove_alternative(kind, template);
                                            changed = true;
                                        }
                                    });
                                }
                            }

                            let mut input = self
                                .alternative_inputs
                                .get(kind.id())
                                .cloned()
                                .unwrap_or_default();
                            let mut alternative_added = false;
                            ui.horizontal(|ui| {
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut input)
                                        .hint_text("add an alternative call")
                                        .desired_width(180.0),
                                );
                                let submit = submitted(ui, &response);
                                let clicked = ui
                                    .small_button("Add alternative")
                                    .on_hover_text("Add alternative (Enter)")
                                    .clicked();
                                if clicked || submit {
                                    self.prefs.aliases.add_alternative(kind, input.clone());
                                    alternative_added = true;
                                    changed = true;
                                }
                            });
                            self.alternative_inputs.insert(
                                kind.id().to_string(),
                                if alternative_added {
                                    String::new()
                                } else {
                                    input
                                },
                            );
                        });
                }
                if changed {
                    self.rebuild_timeline();
                    self.needs_render = true;
                    self.save_prefs();
                }
            });

        ui.separator();
        ui.heading("Custom figure aliases");
        ui.label(
            egui::RichText::new("Use this for figures the classifier does not recognise.")
                .small()
                .color(theme::muted(ui)),
        );
        let custom = self.prefs.aliases.custom.clone();
        for (key, alias) in custom {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&key).monospace());
                ui.label("→");
                ui.label(&alias);
                if ui.small_button("Remove").clicked() {
                    self.prefs.aliases.remove_custom(&key);
                    self.rebuild_timeline();
                    self.needs_render = true;
                    self.save_prefs();
                }
            });
        }
        ui.horizontal(|ui| {
            let key = ui.add(
                egui::TextEdit::singleline(&mut self.custom_key)
                    .hint_text("card phrase")
                    .desired_width(220.0),
            );
            let alias = ui.add(
                egui::TextEdit::singleline(&mut self.custom_alias)
                    .hint_text("call to say")
                    .desired_width(220.0),
            );
            let submit = submitted(ui, &key) || submitted(ui, &alias);
            let clicked = ui
                .button("Add alias")
                .on_hover_text("Add alias (Enter)")
                .clicked();
            if clicked || submit {
                self.add_custom_alias();
            }
        });
    }
}

/// Whether a text field lost focus because Enter was pressed.
fn submitted(ui: &egui::Ui, response: &egui::Response) -> bool {
    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))
}

/// A single-line search field that reports Enter as a submit action.
fn submit_text_edit(ui: &mut egui::Ui, text: &mut String) -> bool {
    let response = ui.text_edit_singleline(text);
    submitted(ui, &response)
}

fn empty_label<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

fn formation_label(value: Option<&str>) -> &'static str {
    match value.unwrap_or("") {
        "improper" => "Improper",
        "becket" => "Becket",
        "proper" => "Proper",
        "everything else" => "Everything else",
        _ => "Any formation",
    }
}

fn combo_option(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut Option<String>,
    options: &[(&str, &str)],
) {
    let selected = value
        .as_deref()
        .and_then(|current| {
            options
                .iter()
                .find(|(option, _)| *option == current)
                .map(|(_, label)| *label)
        })
        .unwrap_or_else(|| options.first().map(|(_, label)| *label).unwrap_or(""));
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (option, label) in options {
                let is_selected = value.as_deref().unwrap_or("") == *option;
                if ui.selectable_label(is_selected, *label).clicked() {
                    *value = if option.is_empty() {
                        None
                    } else {
                        Some((*option).to_string())
                    };
                }
            }
        });
}
