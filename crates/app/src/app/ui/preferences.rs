use crate::app::*;
use eframe::egui;

/// Every preferences section, in display order.
///
/// A new setting belongs in the matching section function, so unrelated
/// branches edit different functions instead of the same 200-line body.
const SECTIONS: &[fn(&mut ContraApp, &mut egui::Ui)] = &[
    role_terms,
    instruments,
    appearance,
    calling_lead,
    speech,
    playback,
    soundfont,
    web_proxy,
    maintenance,
];

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
            for section in SECTIONS {
                section(self, ui);
            }
        });

        // One place applies a change to the subsystem it belongs to; see
        // Preferences::changes_since.
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
            if changes.tone {
                self.speech.set_tone(self.prefs.tts_tone);
            }
            self.audio.set_looping(self.prefs.loop_enabled);
            self.audio.set_volume(self.prefs.master_volume);
            self.save_prefs();
        }
    }
}

fn role_terms(app: &mut ContraApp, ui: &mut egui::Ui) {
    ui.heading("Role terms");
    let mut preset = RolePreset::detect(&app.prefs.role_terms);
    egui::ComboBox::from_label("Role terms")
        .selected_text(preset.label())
        .show_ui(ui, |ui| {
            for option in RolePreset::ALL {
                ui.selectable_value(&mut preset, option, option.label());
            }
        });
    if preset != RolePreset::detect(&app.prefs.role_terms) {
        app.prefs.role_terms = RoleTerms::preset(preset);
        app.rebuild_timeline();
        app.needs_render = true;
        app.save_prefs();
    }
    if preset == RolePreset::Custom {
        egui::Grid::new("custom-roles")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Left singular");
                ui.text_edit_singleline(&mut app.prefs.role_terms.left_singular);
                ui.end_row();
                ui.label("Left plural");
                ui.text_edit_singleline(&mut app.prefs.role_terms.left_plural);
                ui.end_row();
                ui.label("Right singular");
                ui.text_edit_singleline(&mut app.prefs.role_terms.right_singular);
                ui.end_row();
                ui.label("Right plural");
                ui.text_edit_singleline(&mut app.prefs.role_terms.right_plural);
                ui.end_row();
            });
    }
    ui.label(
        egui::RichText::new(
            app.prefs
                .role_terms
                .apply("ladles allemande left 1½; gentlespoons do si do once"),
        )
        .italics()
        .color(theme::muted(ui)),
    );
    if ui.button("Edit dance move aliases…").clicked() {
        app.screen = Screen::Aliases;
    }
    ui.separator();
}

fn instruments(app: &mut ContraApp, ui: &mut egui::Ui) {
    ui.heading("Instruments");
    egui::Grid::new("instruments")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("Melody");
            egui::ComboBox::from_id_salt("melody")
                .selected_text(app.prefs.melody.label())
                .show_ui(ui, |ui| {
                    for option in MelodyInstrument::ALL {
                        ui.selectable_value(&mut app.prefs.melody, option, option.label());
                    }
                });
            ui.end_row();
            ui.label("Accompaniment");
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("accompaniment")
                    .selected_text(app.prefs.accompaniment.label())
                    .show_ui(ui, |ui| {
                        for option in AccompanimentInstrument::ALL {
                            ui.selectable_value(
                                &mut app.prefs.accompaniment,
                                option,
                                option.label(),
                            );
                        }
                    });
                ui.label("Rhythm");
                egui::ComboBox::from_id_salt("accompaniment_rhythm")
                    .selected_text(app.prefs.accompaniment_rhythm.label())
                    .show_ui(ui, |ui| {
                        for option in AccompanimentRhythm::ALL {
                            ui.selectable_value(
                                &mut app.prefs.accompaniment_rhythm,
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
        ui.add(egui::Slider::new(&mut app.prefs.melody_volume, 0.0..=1.5));
    });
    ui.horizontal(|ui| {
        ui.label("Accompaniment volume");
        ui.add(egui::Slider::new(
            &mut app.prefs.accompaniment_volume,
            0.0..=1.5,
        ));
    });
    ui.separator();
}

fn appearance(app: &mut ContraApp, ui: &mut egui::Ui) {
    ui.heading("Theme");
    ui.horizontal(|ui| {
        for option in Theme::ALL {
            ui.selectable_value(&mut app.prefs.theme, option, option.label());
        }
    });
    ui.heading("Interface");
    ui.checkbox(&mut app.prefs.animations_enabled, "Smooth animations");
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
}

fn calling_lead(app: &mut ContraApp, ui: &mut egui::Ui) {
    ui.heading("Calling lead");
    ui.checkbox(&mut app.prefs.lead.auto, "Automatic syllable-based lead");
    if app.prefs.lead.auto {
        ui.horizontal(|ui| {
            ui.label("Minimum speech lead");
            ui.add(egui::Slider::new(&mut app.prefs.lead.min_beats, 0.5..=4.0).text("beats"));
        });
        ui.horizontal(|ui| {
            ui.label("Maximum speech lead");
            ui.add(egui::Slider::new(&mut app.prefs.lead.max_beats, 1.0..=6.0).text("beats"));
        });
        ui.horizontal(|ui| {
            ui.label("Speech rate");
            ui.add(
                egui::Slider::new(&mut app.prefs.lead.syllables_per_second, 3.0..=10.0)
                    .text("syllables/s"),
            );
        });
    } else {
        ui.horizontal(|ui| {
            ui.label("Manual speech lead");
            ui.add(egui::Slider::new(&mut app.prefs.lead.manual_beats, 0.5..=6.0).text("beats"));
        });
    }
    ui.horizontal(|ui| {
        ui.label("Safety padding");
        let slider = egui::Slider::new(
            &mut app.prefs.lead.padding_seconds,
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
        app.rebuild_timeline();
        app.needs_render = true;
        app.save_prefs();
    }
    ui.separator();
}

fn speech(app: &mut ContraApp, ui: &mut egui::Ui) {
    ui.heading("Text to speech");
    if ui
        .checkbox(&mut app.prefs.tts_enabled, "Speak calls")
        .changed()
    {
        app.speech.set_enabled(app.prefs.tts_enabled);
        if !app.prefs.tts_enabled {
            app.speech.stop();
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
    let neural = cfg!(all(not(target_arch = "wasm32"), not(target_os = "android")));
    ui.horizontal(|ui| {
        ui.label("Voice");
        ui.add_enabled_ui(neural, |ui| {
            let selected = crate::tts::voice(&app.prefs.tts_voice);
            egui::ComboBox::from_id_salt("tts_voice")
                .selected_text(selected.label)
                .show_ui(ui, |ui| {
                    for voice in crate::tts::VOICES {
                        if ui
                            .selectable_label(app.prefs.tts_voice == voice.id, voice.label)
                            .clicked()
                        {
                            app.prefs.tts_voice = voice.id.to_string();
                            app.speech.set_voice(&app.prefs.tts_voice);
                        }
                    }
                });
        });
    });
    let selected = crate::tts::voice(&app.prefs.tts_voice);
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
            .add(egui::Slider::new(&mut app.prefs.tts_rate, 0.5..=2.0))
            .changed()
        {
            app.speech.set_rate(app.prefs.tts_rate);
        }
    });
    ui.horizontal(|ui| {
        ui.label("Tone");
        egui::ComboBox::from_id_salt("tts_tone")
            .selected_text(app.prefs.tts_tone.label())
            .show_ui(ui, |ui| {
                for tone in contra_core::CallTone::ALL {
                    ui.selectable_value(&mut app.prefs.tts_tone, tone, tone.label());
                }
            });
    });
    ui.label(
        egui::RichText::new(app.prefs.tts_tone.detail())
            .small()
            .color(theme::muted(ui)),
    );
    ui.checkbox(
        &mut app.prefs.shuffle_alternatives,
        "Shuffle alternative call aliases",
    );
    ui.label(
                egui::RichText::new(
                    "Randomly uses an alternative wording, such as \"partner swing\" instead of \"swing your partner\", for moves that have one. The chosen wording is shown and spoken, and its timing is used.",
                )
                .small()
                .color(theme::muted(ui)),
            );
    let status = app.speech.status().clone();
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
    } else if !app.speech.available()
        && let Some(error) = app.speech.error()
    {
        ui.label(
            egui::RichText::new(format!("TTS unavailable: {error}"))
                .small()
                .color(theme::muted(ui)),
        );
    }
    if status.error.is_some() && app.speech.can_retry() && ui.button("Retry neural voice").clicked()
    {
        app.speech.retry();
    }
}

fn playback(app: &mut ContraApp, ui: &mut egui::Ui) {
    ui.checkbox(&mut app.prefs.loop_enabled, "Loop the dance and music");
    ui.checkbox(&mut app.prefs.count_in_enabled, "Four-beat count-in");
    ui.checkbox(
        &mut app.prefs.accompaniment_variations,
        "Vary the accompaniment on each loop",
    );
    ui.label(
        egui::RichText::new("Adds small emphasis notes and the occasional bar with melody alone.")
            .small()
            .color(theme::muted(ui)),
    );
    ui.checkbox(&mut app.prefs.show_original, "Show original card text");
    ui.separator();
}

fn soundfont(app: &mut ContraApp, ui: &mut egui::Ui) {
    ui.heading("Soundfont");
    let current = app
        .prefs
        .soundfont
        .as_ref()
        .map(|setting| setting.name.clone())
        .unwrap_or_else(|| "Bundled: Fluid R3 GM".into());
    ui.label(current);
    ui.label(
        egui::RichText::new(
            "Sampled instruments from Fluid R3 GM (MIT, Frank Wen). Load an .sf2 to use a different bank.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    if let Some(error) = &app.soundfont_error {
        ui.colored_label(theme::error(ui), error);
    }
    ui.horizontal(|ui| {
        if ui.button("Load .sf2…").clicked() {
            app.load_soundfont();
        }
        if app.prefs.soundfont.is_some() && ui.button("Use bundled").clicked() {
            app.remove_soundfont();
        }
    });
    ui.separator();
}

fn web_proxy(app: &mut ContraApp, ui: &mut egui::Ui) {
    ui.heading("Web data proxy");
    ui.label(
        egui::RichText::new(
            "Only needed by the web build. Desktop and Android fetch sources directly.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let mut proxy = app.prefs.web_proxy_url.clone().unwrap_or_default();
    let mut token = app.prefs.web_proxy_token.clone().unwrap_or_default();
    let mut proxy_changed = false;
    egui::Grid::new("proxy").num_columns(2).show(ui, |ui| {
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
        app.prefs.web_proxy_url = (!proxy.trim().is_empty()).then_some(proxy);
        app.prefs.web_proxy_token = (!token.trim().is_empty()).then_some(token);
        app.source.set_proxy(
            app.prefs.web_proxy_url.clone(),
            app.prefs.web_proxy_token.clone(),
        );
        app.save_prefs();
    }
    ui.separator();
}

fn maintenance(app: &mut ContraApp, ui: &mut egui::Ui) {
    ui.heading("Maintenance");
    ui.horizontal_wrapped(|ui| {
        if ui.button("Export MIDI…").clicked() {
            app.export_midi();
        }
        if app.cache_clear_confirm {
            if ui.button("Confirm clear cache").clicked() {
                app.source.clear_cache();
                app.cache_clear_confirm = false;
                app.status = "Cache cleared".into();
            }
            if ui.button("Cancel").clicked() {
                app.cache_clear_confirm = false;
            }
        } else if ui.button("Clear source cache").clicked() {
            app.cache_clear_confirm = true;
        }
    });
    ui.label(
        egui::RichText::new(&app.status)
            .small()
            .color(theme::muted(ui)),
    );
}
