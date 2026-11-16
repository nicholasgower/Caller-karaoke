use super::about::about;
use super::{apply_phone_spacing, compact, form_row, inline_field, responsive_combo, slider_row};
use crate::app::*;
use contra_core::{ChorusPreset, EchoPreset, ReverbPreset};
use eframe::egui;

/// Every preferences section, in display order.
///
/// A new setting belongs in the matching section function, so unrelated
/// branches edit different functions instead of the same 200-line body. About
/// is last because it is the least frequently needed section.
pub(in crate::app) const SECTIONS: &[fn(&mut ContraApp, &mut egui::Ui, bool)] = &[
    role_terms,
    instruments,
    appearance,
    calling_lead,
    speech,
    playback,
    soundfont,
    effects,
    web_proxy,
    maintenance,
    about,
];

/// The About section, named so the shell tests can assert that the version,
/// author, and project links are drawn from Preferences and nowhere else.
#[cfg(test)]
pub(in crate::app) const ABOUT_SECTION: fn(&mut ContraApp, &mut egui::Ui, bool) = about;

impl ContraApp {
    pub(in crate::app) fn preferences_ui(&mut self, ui: &mut egui::Ui) {
        let compact = compact(ui);
        apply_phone_spacing(ui, compact);
        let before = self.prefs.clone();
        // Recomputed by the custom role fields each frame. While a field is
        // focused the timeline rebuild is deferred; see `role_terms`.
        self.role_edit_active = false;
        ui.heading("Preferences");
        ui.label(
            egui::RichText::new("Changes are saved automatically. Playback settings are applied without restarting the music; role and alias changes rebuild the call timeline.")
                .color(theme::muted(ui)),
        );
        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            for section in SECTIONS {
                section(self, ui, compact);
            }
        });

        // One place applies a change to the subsystem it belongs to; see
        // Preferences::changes_since.
        self.prefs.validate();
        if self.prefs != before {
            let changes = self.prefs.changes_since(&before);
            if changes.timeline && !self.role_edit_active {
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

fn role_terms(app: &mut ContraApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Role terms");
    // The stored selection, not an inference from the terms: choosing Custom
    // keeps the editable fields on screen even when the words still match a
    // named preset.
    let mut preset = app.prefs.role_preset;
    let combo = if compact {
        egui::ComboBox::from_id_salt("role-terms")
    } else {
        egui::ComboBox::from_label("Role terms")
    };
    responsive_combo(combo, ui, compact)
        .selected_text(preset.label())
        .show_ui(ui, |ui| {
            for option in RolePreset::ALL {
                ui.selectable_value(&mut preset, option, option.label());
            }
        });
    if preset != app.prefs.role_preset {
        let before = app.prefs.role_terms.clone();
        app.prefs.select_role_preset(preset);
        // A named preset replaces the terms, so the change is complete now.
        // Custom keeps the current words and waits for an edit.
        app.role_edit_pending = false;
        if app.prefs.role_terms != before {
            app.needs_render = true;
        }
    }
    if app.prefs.role_preset == RolePreset::Custom {
        egui::Grid::new("custom-roles")
            .num_columns(if compact { 1 } else { 2 })
            .show(ui, |ui| {
                form_row(ui, compact, "Left singular", |ui| {
                    edit_role_term(app, ui, RoleField::LeftSingular);
                });
                ui.end_row();
                form_row(ui, compact, "Left plural", |ui| {
                    edit_role_term(app, ui, RoleField::LeftPlural);
                });
                ui.end_row();
                form_row(ui, compact, "Right singular", |ui| {
                    edit_role_term(app, ui, RoleField::RightSingular);
                });
                ui.end_row();
                form_row(ui, compact, "Right plural", |ui| {
                    edit_role_term(app, ui, RoleField::RightPlural);
                });
                ui.end_row();
            });
        // The example below and the next session frame read the edited terms
        // immediately, but the deferred rebuild waits until the field loses
        // focus so typing a long word does not rebuild the timeline and
        // restart speech on every keystroke.
        if app.role_edit_pending && !app.role_edit_active {
            app.commit_role_terms();
        }
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

/// One editable custom role term.
///
/// Editing a term always means Custom: it marks the selection, remembers that
/// a rebuild is pending, and lets the field lose focus before the timeline is
/// rebuilt. The active flag is cleared at the start of every preferences
/// frame, so a stale focus state cannot defer the rebuild forever.
fn edit_role_term(app: &mut ContraApp, ui: &mut egui::Ui, field: RoleField) {
    let terms = &mut app.prefs.role_terms;
    let buffer = match field {
        RoleField::LeftSingular => &mut terms.left_singular,
        RoleField::LeftPlural => &mut terms.left_plural,
        RoleField::RightSingular => &mut terms.right_singular,
        RoleField::RightPlural => &mut terms.right_plural,
    };
    let response = ui.text_edit_singleline(buffer);
    if response.changed() {
        app.prefs.role_preset = RolePreset::Custom;
        app.role_edit_pending = true;
        app.role_edit_active = true;
    }
    if response.has_focus() || response.lost_focus() {
        app.role_edit_active = true;
    }
    if response.lost_focus() {
        app.commit_role_terms();
    }
}

#[derive(Clone, Copy)]
enum RoleField {
    LeftSingular,
    LeftPlural,
    RightSingular,
    RightPlural,
}

fn instruments(app: &mut ContraApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Instruments");
    if compact {
        inline_field(ui, true, "Melody", |ui| {
            melody_combo(ui, true, &mut app.prefs);
        });
        copies_field(
            ui,
            true,
            "Melody",
            &mut app.prefs.instrument_copies.melody,
            InstrumentCopies::MAX,
        );
        inline_field(ui, true, "Accompaniment", |ui| {
            accompaniment_combo(ui, true, &mut app.prefs);
        });
        inline_field(ui, true, "Rhythm", |ui| {
            rhythm_combo(ui, true, &mut app.prefs);
        });
        copies_field(
            ui,
            true,
            "Accompaniment",
            &mut app.prefs.instrument_copies.accompaniment,
            InstrumentCopies::MAX,
        );
        inline_field(ui, true, "Countermelody", |ui| {
            countermelody_combo(ui, true, &mut app.prefs);
        });
        copies_field(
            ui,
            true,
            "Countermelody",
            &mut app.prefs.instrument_copies.countermelody,
            InstrumentCopies::MAX,
        );
        inline_field(ui, true, "Percussion", |ui| {
            percussion_combo(ui, true, &mut app.prefs);
        });
        inline_field(ui, true, "Percussion rhythm", |ui| {
            percussion_rhythm_combo(ui, true, &mut app.prefs);
        });
        copies_field(
            ui,
            true,
            "Percussion",
            &mut app.prefs.instrument_copies.percussion,
            InstrumentCopies::MAX_PERCUSSION,
        );
    } else {
        egui::Grid::new("instruments")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Melody");
                ui.horizontal(|ui| {
                    melody_combo(ui, false, &mut app.prefs);
                    copies_field(
                        ui,
                        false,
                        "Melody",
                        &mut app.prefs.instrument_copies.melody,
                        InstrumentCopies::MAX,
                    );
                });
                ui.end_row();
                ui.label("Accompaniment");
                ui.horizontal(|ui| {
                    accompaniment_combo(ui, false, &mut app.prefs);
                    ui.label("Rhythm");
                    rhythm_combo(ui, false, &mut app.prefs);
                    copies_field(
                        ui,
                        false,
                        "Accompaniment",
                        &mut app.prefs.instrument_copies.accompaniment,
                        InstrumentCopies::MAX,
                    );
                });
                ui.end_row();
                ui.label("Countermelody");
                ui.horizontal(|ui| {
                    countermelody_combo(ui, false, &mut app.prefs);
                    copies_field(
                        ui,
                        false,
                        "Countermelody",
                        &mut app.prefs.instrument_copies.countermelody,
                        InstrumentCopies::MAX,
                    );
                });
                ui.end_row();
                ui.label("Percussion");
                ui.horizontal(|ui| {
                    percussion_combo(ui, false, &mut app.prefs);
                    ui.label("Rhythm");
                    percussion_rhythm_combo(ui, false, &mut app.prefs);
                    copies_field(
                        ui,
                        false,
                        "Percussion",
                        &mut app.prefs.instrument_copies.percussion,
                        InstrumentCopies::MAX_PERCUSSION,
                    );
                });
                ui.end_row();
            });
    }
    ui.label(
        egui::RichText::new(
            "The melody, the chordal accompaniment, and a countermelody make the default trio. \
             Choose Off for a duo; the countermelody follows the accompaniment volume.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.label(
        egui::RichText::new(
            "Copies add players to a part, up to four (three for percussion). Every copy is a \
             separate player with its own pan, a few milliseconds of spread, and a slight tuning \
             difference, so the section sounds wider instead of just louder. One copy is the \
             single player and sounds exactly as before.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.label(
        egui::RichText::new(
            "Percussion is optional: foot taps play a one-and-a gallop (eighth + two sixteenths \
             per dance beat), or a simple drum kit adds kick, snare, and hi-hat. The rhythm menu \
             changes the pattern; it starts with the count-in.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    slider_row(
        ui,
        compact,
        "Melody volume",
        &mut app.prefs.melody_volume,
        |ui, value| ui.add(egui::Slider::new(value, 0.0..=1.5)),
    );
    slider_row(
        ui,
        compact,
        "Accompaniment volume",
        &mut app.prefs.accompaniment_volume,
        |ui, value| ui.add(egui::Slider::new(value, 0.0..=1.5)),
    );
    let percussion_enabled = app.prefs.percussion.is_enabled();
    ui.add_enabled_ui(percussion_enabled, |ui| {
        slider_row(
            ui,
            compact,
            "Percussion volume",
            &mut app.prefs.percussion_volume,
            |ui, value| ui.add(egui::Slider::new(value, 0.0..=1.5)),
        );
    });
    ui.separator();
}

fn melody_combo(ui: &mut egui::Ui, compact: bool, prefs: &mut Preferences) {
    let combo = egui::ComboBox::from_id_salt("melody");
    responsive_combo(combo, ui, compact)
        .selected_text(prefs.melody.label())
        .show_ui(ui, |ui| {
            for option in MelodyInstrument::ALL {
                ui.selectable_value(&mut prefs.melody, option, option.label());
            }
        });
}

fn accompaniment_combo(ui: &mut egui::Ui, compact: bool, prefs: &mut Preferences) {
    let combo = egui::ComboBox::from_id_salt("accompaniment");
    responsive_combo(combo, ui, compact)
        .selected_text(prefs.accompaniment.label())
        .show_ui(ui, |ui| {
            for option in AccompanimentInstrument::ALL {
                ui.selectable_value(&mut prefs.accompaniment, option, option.label());
            }
        });
}

fn rhythm_combo(ui: &mut egui::Ui, compact: bool, prefs: &mut Preferences) {
    let combo = egui::ComboBox::from_id_salt("accompaniment_rhythm");
    responsive_combo(combo, ui, compact)
        .selected_text(prefs.accompaniment_rhythm.label())
        .show_ui(ui, |ui| {
            for option in AccompanimentRhythm::ALL {
                ui.selectable_value(&mut prefs.accompaniment_rhythm, option, option.label());
            }
        });
}

fn countermelody_combo(ui: &mut egui::Ui, compact: bool, prefs: &mut Preferences) {
    let combo = egui::ComboBox::from_id_salt("second_accompaniment");
    responsive_combo(combo, ui, compact)
        .selected_text(prefs.second_accompaniment.label())
        .show_ui(ui, |ui| {
            for option in Countermelody::ALL {
                ui.selectable_value(&mut prefs.second_accompaniment, option, option.label());
            }
        });
}

/// A labelled copies stepper: `Copies  −  1  +`.
///
/// Every part gets its own stepper; the buttons carry the part's name in their
/// accessible label ("Fewer melody copies") so an agent can tell the four
/// apart in the widget tree. On a phone both buttons are full 44-point touch
/// targets.
fn copies_field(ui: &mut egui::Ui, compact: bool, part: &str, copies: &mut u8, max: u8) {
    inline_field(ui, compact, "Copies", |ui| {
        copies_stepper(ui, compact, part, copies, max);
    });
}

fn copies_stepper(ui: &mut egui::Ui, compact: bool, part: &str, copies: &mut u8, max: u8) {
    let name = format!("{} copies", part.to_lowercase());
    let fewer = format!("Fewer {name}");
    let more = format!("More {name}");
    let size = if compact {
        egui::vec2(theme::TOUCH_TARGET, theme::TOUCH_TARGET)
    } else {
        egui::vec2(28.0, 20.0)
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let decrease = ui
            .add_enabled(*copies > 1, egui::Button::new("−").min_size(size))
            .accessible_name(&fewer);
        if decrease.clicked() {
            *copies = (*copies).saturating_sub(1).max(1);
        }
        ui.add_sized(
            [size.x, size.y],
            egui::Label::new(egui::RichText::new(copies.to_string()).monospace()),
        )
        .accessible_name(format!("{name}: {copies}"));
        let increase = ui
            .add_enabled(*copies < max, egui::Button::new("+").min_size(size))
            .accessible_name(&more);
        if increase.clicked() {
            *copies = (*copies + 1).min(max);
        }
    });
}

fn percussion_combo(ui: &mut egui::Ui, compact: bool, prefs: &mut Preferences) {
    let combo = egui::ComboBox::from_id_salt("percussion");
    responsive_combo(combo, ui, compact)
        .selected_text(prefs.percussion.label())
        .show_ui(ui, |ui| {
            for option in Percussion::ALL {
                ui.selectable_value(&mut prefs.percussion, option, option.label());
            }
        });
}

fn percussion_rhythm_combo(ui: &mut egui::Ui, compact: bool, prefs: &mut Preferences) {
    let combo = egui::ComboBox::from_id_salt("percussion_rhythm");
    responsive_combo(combo, ui, compact)
        .selected_text(prefs.percussion_rhythm.label())
        .show_ui(ui, |ui| {
            for option in PercussionRhythm::ALL {
                ui.selectable_value(&mut prefs.percussion_rhythm, option, option.label());
            }
        });
}

fn appearance(app: &mut ContraApp, ui: &mut egui::Ui, _compact: bool) {
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
                    "Keyboard: Space play/pause · R restart · Esc back · 1-3 switch tabs · 4 opens Preferences · ←/→ previous or next tab · Enter starts a search, creates a folder, or adds an alias.",
                )
                .small()
                .color(theme::muted(ui)),
            );
    ui.separator();
}

fn calling_lead(app: &mut ContraApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Calling lead");
    ui.checkbox(&mut app.prefs.lead.auto, "Automatic syllable-based lead");
    if app.prefs.lead.auto {
        slider_row(
            ui,
            compact,
            "Minimum speech lead",
            &mut app.prefs.lead.min_beats,
            |ui, value| ui.add(egui::Slider::new(value, 0.5..=4.0).text("beats")),
        );
        slider_row(
            ui,
            compact,
            "Maximum speech lead",
            &mut app.prefs.lead.max_beats,
            |ui, value| ui.add(egui::Slider::new(value, 1.0..=6.0).text("beats")),
        );
        slider_row(
            ui,
            compact,
            "Speech rate",
            &mut app.prefs.lead.syllables_per_second,
            |ui, value| ui.add(egui::Slider::new(value, 3.0..=10.0).text("syllables/s")),
        );
    } else {
        slider_row(
            ui,
            compact,
            "Manual speech lead",
            &mut app.prefs.lead.manual_beats,
            |ui, value| ui.add(egui::Slider::new(value, 0.5..=6.0).text("beats")),
        );
    }
    slider_row(
        ui,
        compact,
        "Safety padding",
        &mut app.prefs.lead.padding_seconds,
        |ui, value| {
            ui.add(
                egui::Slider::new(
                    value,
                    contra_core::MIN_PADDING_SECONDS..=contra_core::MAX_PADDING_SECONDS,
                )
                .step_by(0.1)
                .fixed_decimals(1)
                .text("s"),
            )
        },
    );
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

fn speech(app: &mut ContraApp, ui: &mut egui::Ui, compact: bool) {
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
    inline_field(ui, compact, "Voice", |ui| {
        ui.add_enabled_ui(neural, |ui| {
            let selected = crate::tts::voice(&app.prefs.tts_voice);
            let combo = egui::ComboBox::from_id_salt("tts_voice");
            responsive_combo(combo, ui, compact)
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
    let rate_response = slider_row(ui, compact, "Rate", &mut app.prefs.tts_rate, |ui, rate| {
        ui.add(
            egui::Slider::new(rate, contra_core::MIN_TTS_RATE..=contra_core::MAX_TTS_RATE)
                .step_by(0.1)
                .fixed_decimals(1),
        )
    });
    if rate_response.changed() {
        app.speech.set_rate(app.prefs.speech_rate());
    }
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

fn playback(app: &mut ContraApp, ui: &mut egui::Ui, _compact: bool) {
    ui.checkbox(&mut app.prefs.loop_enabled, "Loop the dance and music");
    ui.checkbox(&mut app.prefs.count_in_enabled, "Four-beat count-in");
    ui.checkbox(
        &mut app.prefs.accompaniment_variations,
        "Vary the music on each loop",
    );
    ui.label(
        egui::RichText::new(
            "Adds melody ornaments, emphasis notes, and the occasional bar with melody alone.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.checkbox(&mut app.prefs.show_original, "Show original card text");
    ui.checkbox(&mut app.prefs.humanize_timing, "Humanize timing");
    ui.label(
        egui::RichText::new(
            "Each player drifts by a fraction of a percent and a controller gently settles it back on \
             the tempo; chord notes start a few milliseconds apart. The count-in, the 64-beat grid, \
             and the calls stay exact.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.separator();
}

fn soundfont(app: &mut ContraApp, ui: &mut egui::Ui, _compact: bool) {
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

fn effects(app: &mut ContraApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Effects");
    ui.label(
        egui::RichText::new(
            "Applied to playback without restarting the music, and written into exported MIDI \
             files. Exported MIDI carries General MIDI sends, so its sound also depends on the player.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    inline_field(ui, compact, "Reverb", |ui| {
        let combo = egui::ComboBox::from_id_salt("effect_reverb");
        responsive_combo(combo, ui, compact)
            .selected_text(app.prefs.effects.reverb.label())
            .show_ui(ui, |ui| {
                for option in ReverbPreset::ALL {
                    ui.selectable_value(&mut app.prefs.effects.reverb, option, option.label());
                }
            });
    });
    ui.label(
        egui::RichText::new(app.prefs.effects.reverb.detail())
            .small()
            .color(theme::muted(ui)),
    );
    inline_field(ui, compact, "Echo", |ui| {
        let combo = egui::ComboBox::from_id_salt("effect_echo");
        responsive_combo(combo, ui, compact)
            .selected_text(app.prefs.effects.echo.label())
            .show_ui(ui, |ui| {
                for option in EchoPreset::ALL {
                    ui.selectable_value(&mut app.prefs.effects.echo, option, option.label());
                }
            });
    });
    ui.label(
        egui::RichText::new(
            "A short slapback or a longer echo in playback and the exported file. Exported MIDI \
             uses the General MIDI delay send; players that do not implement it ignore the echo.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    inline_field(ui, compact, "Chorus", |ui| {
        let combo = egui::ComboBox::from_id_salt("effect_chorus");
        responsive_combo(combo, ui, compact)
            .selected_text(app.prefs.effects.chorus.label())
            .show_ui(ui, |ui| {
                for option in ChorusPreset::ALL {
                    ui.selectable_value(&mut app.prefs.effects.chorus, option, option.label());
                }
            });
    });
    ui.label(
        egui::RichText::new("A light shimmer or a wide chorus on playback and the exported file.")
            .small()
            .color(theme::muted(ui)),
    );
    ui.separator();
}

fn web_proxy(app: &mut ContraApp, ui: &mut egui::Ui, compact: bool) {
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
    egui::Grid::new("proxy")
        .num_columns(if compact { 1 } else { 2 })
        .show(ui, |ui| {
            form_row(ui, compact, "Proxy URL", |ui| {
                if ui.text_edit_singleline(&mut proxy).changed() {
                    proxy_changed = true;
                }
            });
            ui.end_row();
            form_row(ui, compact, "Token", |ui| {
                if ui.text_edit_singleline(&mut token).changed() {
                    proxy_changed = true;
                }
            });
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

fn maintenance(app: &mut ContraApp, ui: &mut egui::Ui, _compact: bool) {
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
