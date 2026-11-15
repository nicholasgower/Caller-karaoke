use crate::app::*;
use eframe::egui;

impl ContraApp {
    pub(in crate::app) fn preferences_ui(&mut self, ui: &mut egui::Ui) {
        let before = self.prefs.clone();
        ui.horizontal(|ui| {
            if ui
                .button("< Back to session")
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
                    ui.horizontal(|ui| {
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
                        ui.label("Rhythm");
                        egui::ComboBox::from_id_salt("accompaniment_rhythm")
                            .selected_text(self.prefs.accompaniment_rhythm.label())
                            .show_ui(ui, |ui| {
                                for option in AccompanimentRhythm::ALL {
                                    ui.selectable_value(
                                        &mut self.prefs.accompaniment_rhythm,
                                        option,
                                        option.label(),
                                    );
                                }
                            });
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
                    "Keyboard: Space play/pause · R restart · Esc back · 1–4 switch tabs · Enter starts a search, creates a folder, or adds an alias.",
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
            ui.horizontal(|ui| {
                ui.label("Safety padding");
                let slider = egui::Slider::new(
                    &mut self.prefs.lead.padding_seconds,
                    contra_core::MIN_PADDING_SECONDS..=contra_core::MAX_PADDING_SECONDS,
                )
                .step_by(0.1)
                .fixed_decimals(1)
                .text("s");
                ui.add(slider);
            });
            ui.label(
                egui::RichText::new(
                    "Added to every call after the speech-lead clamp. Zero removes the buffer.",
                )
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
            let changes = self.prefs.changes_since(&before);
            if changes.timeline {
                self.rebuild_timeline();
                self.last_spoken = None;
            }
            if changes.audio {
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
}
