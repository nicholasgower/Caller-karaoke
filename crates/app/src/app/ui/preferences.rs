use super::about::about;
use super::{apply_phone_spacing, compact, form_row, inline_field, responsive_combo, slider_row};
use crate::app::history::HistoryAction;
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
    data_sources,
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
        // Preferences::changes_since. A value changed and changed back before
        // the queued render runs leaves no pending work.
        self.reconcile_preferences(before);
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
        app.prefs.select_role_preset(preset);
        // A named preset replaces the terms, so the change is complete now.
        // Custom keeps the current words and waits for an edit. Role terms
        // change the calls and the spoken text, never the music; the frame's
        // timeline reconciliation is the whole effect.
        app.role_edit_pending = false;
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
    ui.label(
        egui::RichText::new(
            "The band is a list. Add any number of instruments, choose what each one plays, \
             how many players stand in that section, and where it sits. The part chooses the \
             notes; the instrument chooses the voice. A second piano is its own row with its \
             own level and place, not a copy of the first.",
        )
        .small()
        .color(theme::muted(ui)),
    );

    ui.label(egui::RichText::new("Band").strong());
    let mut remove = None;
    let mut move_up = None;
    let mut move_down = None;
    let pitched_count = app
        .prefs
        .band
        .iter()
        .filter(|slot| slot.is_pitched())
        .count();
    let count = app.prefs.band.len();
    for (index, slot) in app.prefs.band.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{}. {} · {}",
                        index + 1,
                        slot.part.label(),
                        slot.voice.label()
                    ))
                    .strong(),
                );
                ui.label(
                    egui::RichText::new(slot.direction_label())
                        .small()
                        .color(theme::muted(ui)),
                );
            });
            if compact {
                ui.horizontal_wrapped(|ui| {
                    part_combo(ui, true, index, slot);
                    voice_combo(ui, true, index, slot);
                });
                ui.horizontal_wrapped(|ui| {
                    players_field(ui, true, index, slot);
                    direction_combo(ui, true, index, slot);
                });
                ui.horizontal(|ui| {
                    ui.label("Volume");
                    volume_slider(ui, compact, index, slot);
                });
            } else {
                ui.horizontal_wrapped(|ui| {
                    part_combo(ui, false, index, slot);
                    voice_combo(ui, false, index, slot);
                    players_field(ui, false, index, slot);
                    direction_combo(ui, false, index, slot);
                    ui.label("Volume");
                    volume_slider(ui, compact, index, slot);
                });
            }
            ui.horizontal_wrapped(|ui| {
                let can_remove = if slot.is_pitched() {
                    pitched_count > 1
                } else {
                    count > 1
                };
                let remove_button = ui
                    .add_enabled(can_remove, egui::Button::new("Remove"))
                    .on_hover_text(if can_remove {
                        "Remove this instrument from the band"
                    } else {
                        "A band needs at least one pitched instrument"
                    })
                    .accessible_name(format!("Remove instrument {}", index + 1));
                if remove_button.clicked() {
                    remove = Some(index);
                }
                let up = ui
                    .add_enabled(index > 0, egui::Button::new("Up"))
                    .accessible_name(format!("Move instrument {} up", index + 1));
                if up.clicked() {
                    move_up = Some(index);
                }
                let down = ui
                    .add_enabled(index + 1 < count, egui::Button::new("Down"))
                    .accessible_name(format!("Move instrument {} down", index + 1));
                if down.clicked() {
                    move_down = Some(index);
                }
            });
        });
    }
    if let Some(index) = remove {
        app.prefs.band.remove(index);
    }
    if let Some(index) = move_up
        && index > 0
    {
        app.prefs.band.swap(index, index - 1);
    }
    if let Some(index) = move_down
        && index + 1 < app.prefs.band.len()
    {
        app.prefs.band.swap(index, index + 1);
    }
    if ui
        .button("Add instrument")
        .on_hover_text("Add another instrument to the band")
        .accessible_name("Add instrument")
        .clicked()
    {
        app.prefs.band.push(BandSlot::default());
    }
    app.prefs.validate();

    ui.label(
        egui::RichText::new(
            "Percussion is optional: add a foot-tap or drumset row for a one-and-a gallop or a \
             simple kit. The rhythm menu changes the pattern; it starts with the count-in. \
             Removing every percussion row leaves the band silent of percussion.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.checkbox(
        &mut app.prefs.dancer_sounds,
        "Dancer sounds (stomps and claps)",
    )
    .on_hover_text(
        "Adds stomps on petronellas and long lines, and claps on balances, from the dance card",
    );
    ui.label(
        egui::RichText::new(
            "Adds a foot stomp on petronellas and long lines, and hand claps on balances, taken \
             from the dance card. The sounds sit under the band on the dance beat and follow every \
             loop. This is on by default.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let dancer_sounds = app.prefs.dancer_sounds;
    ui.add_enabled_ui(dancer_sounds, |ui| {
        slider_row(
            ui,
            compact,
            "Dancer sound volume",
            &mut app.prefs.dancer_sound_volume,
            |ui, value| ui.add(egui::Slider::new(value, 0.0..=1.5)),
        );
    });
    inline_field(ui, compact, "Percussion rhythm", |ui| {
        percussion_rhythm_combo(ui, compact, &mut app.prefs);
    });
    inline_field(ui, compact, "Accompaniment rhythm", |ui| {
        rhythm_combo(ui, compact, &mut app.prefs);
    });
    ui.label(
        egui::RichText::new(
            "The accompaniment rhythm changes how an accompaniment row realises the chords; it \
             never moves the dance grid or the calls.",
        )
        .small()
        .color(theme::muted(ui)),
    );

    ui.separator();
    ui.label(egui::RichText::new("Mix").strong());
    slider_row(
        ui,
        compact,
        "Caller volume",
        &mut app.prefs.caller_volume,
        |ui, value| ui.add(egui::Slider::new(value, 0.0..=1.5)),
    );
    ui.label(
        egui::RichText::new(
            "The spoken calls have their own level. Turning the band down never hides a call, \
             and turning the caller down never changes the music.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    slider_row(
        ui,
        compact,
        "Band volume",
        &mut app.prefs.master_volume,
        |ui, value| ui.add(egui::Slider::new(value, 0.0..=1.5)),
    );
    ui.label(
        egui::RichText::new(
            "The band master sits on top of each row's volume. Both levels apply immediately.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.separator();
}

fn part_combo(ui: &mut egui::Ui, compact: bool, index: usize, slot: &mut BandSlot) {
    ui.label(format!("Part {}", index + 1))
        .on_hover_text("What this instrument plays");
    let mut part = slot.part;
    let combo = egui::ComboBox::from_id_salt(("band-part", index));
    responsive_combo(combo, ui, compact)
        .selected_text(part.label())
        .show_ui(ui, |ui| {
            for option in BandPart::ALL {
                ui.selectable_value(&mut part, option, option.label());
            }
        });
    if part != slot.part {
        slot.part = part;
        if part == BandPart::Percussion && !slot.voice.is_percussion() {
            slot.voice = BandVoice::FootTaps;
        } else if part != BandPart::Percussion && slot.voice.is_percussion() {
            slot.voice = BandVoice::Fiddle;
        }
    }
}

fn voice_combo(ui: &mut egui::Ui, compact: bool, index: usize, slot: &mut BandSlot) {
    ui.label(format!("Instrument {}", index + 1))
        .on_hover_text("The voice this instrument uses");
    let percussion = slot.part == BandPart::Percussion;
    let combo = egui::ComboBox::from_id_salt(("band-voice", index));
    responsive_combo(combo, ui, compact)
        .selected_text(slot.voice.label())
        .show_ui(ui, |ui| {
            for option in BandVoice::ALL {
                if option.is_percussion() == percussion {
                    ui.selectable_value(&mut slot.voice, option, option.label());
                }
            }
        });
}

fn players_field(ui: &mut egui::Ui, compact: bool, index: usize, slot: &mut BandSlot) {
    let max = if slot.voice.is_percussion() {
        BandSlot::MAX_PERCUSSION_PLAYERS
    } else {
        BandSlot::MAX_PLAYERS
    };
    ui.label("Players");
    players_stepper(ui, compact, index, &mut slot.players, max);
}

/// A players stepper whose buttons carry the instrument number, so an agent
/// can tell the rows apart in the widget tree.
fn players_stepper(ui: &mut egui::Ui, compact: bool, index: usize, players: &mut u8, max: u8) {
    let name = format!("instrument {}", index + 1);
    let fewer = format!("Fewer players on {name}");
    let more = format!("More players on {name}");
    let size = if compact {
        egui::vec2(theme::TOUCH_TARGET, theme::TOUCH_TARGET)
    } else {
        egui::vec2(28.0, 20.0)
    };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let decrease = ui
            .add_enabled(
                *players > BandSlot::MIN_PLAYERS,
                egui::Button::new("−").min_size(size),
            )
            .accessible_name(&fewer);
        if decrease.clicked() {
            *players = (*players).saturating_sub(1).max(BandSlot::MIN_PLAYERS);
        }
        ui.add_sized(
            [size.x, size.y],
            egui::Label::new(egui::RichText::new(players.to_string()).monospace()),
        )
        .accessible_name(format!("Players on {name}: {players}"));
        let increase = ui
            .add_enabled(*players < max, egui::Button::new("+").min_size(size))
            .accessible_name(&more);
        if increase.clicked() {
            *players = (*players + 1).min(max);
        }
    });
}

/// The six places a slot can sit in.
const DIRECTIONS: [(&str, f32, f32); 6] = [
    ("Left front", -0.6, 0.0),
    ("Centre front", 0.0, 0.0),
    ("Right front", 0.6, 0.0),
    ("Left rear", -0.6, 0.6),
    ("Centre rear", 0.0, 0.6),
    ("Right rear", 0.6, 0.6),
];

fn direction_combo(ui: &mut egui::Ui, compact: bool, index: usize, slot: &mut BandSlot) {
    let combo = egui::ComboBox::from_id_salt(("band-direction", index));
    responsive_combo(combo, ui, compact)
        .selected_text(slot.direction_label())
        .show_ui(ui, |ui| {
            for (label, pan, depth) in DIRECTIONS {
                if ui
                    .selectable_label(slot.direction_label() == label, label)
                    .clicked()
                {
                    slot.pan = pan;
                    slot.depth = depth;
                }
            }
        });
    ui.label(format!("Direction {}", index + 1))
        .on_hover_text("Left/right and front/rear in the stereo or surround image");
}

fn volume_slider(ui: &mut egui::Ui, compact: bool, index: usize, slot: &mut BandSlot) {
    if compact {
        ui.spacing_mut().slider_width = (ui.available_width() - 90.0).max(96.0);
    }
    ui.add(
        egui::Slider::new(&mut slot.volume, 0.0..=1.5)
            .max_decimals(2)
            .show_value(true),
    )
    .accessible_name(format!("Volume for instrument {}", index + 1));
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

fn appearance(app: &mut ContraApp, ui: &mut egui::Ui, compact: bool) {
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
    undo_redo_buttons(app, ui, compact);
    if let Some(message) = &app.history_status {
        ui.label(egui::RichText::new(message).small().color(theme::muted(ui)));
    }
    ui.label(
                egui::RichText::new(
                    "Keyboard: Space play/pause · R restart · Esc back · 1-3 switch tabs · 4 opens Preferences · ←/→ previous or next tab · Ctrl+Z undo · Ctrl+Y or Ctrl+Shift+Z redo · Enter starts a search, creates a folder, or adds an alias.",
                )
                .small()
                .color(theme::muted(ui)),
            );
    ui.separator();
}

/// Undo and redo buttons for the last preference or picker change.
///
/// The keyboard chords do the same thing; the buttons exist so the feature is
/// discoverable and works on a touch screen, where there is no Ctrl key. Both
/// carry an accessible name and are disabled when their stack is empty.
fn undo_redo_buttons(app: &mut ContraApp, ui: &mut egui::Ui, compact: bool) {
    let size = if compact {
        egui::vec2(112.0, theme::TOUCH_TARGET)
    } else {
        egui::vec2(96.0, 24.0)
    };
    ui.horizontal_wrapped(|ui| {
        let undo = ui
            .add_enabled(
                app.history.can_undo(),
                egui::Button::new("Undo").min_size(size),
            )
            .on_hover_text("Undo the last change (Ctrl+Z)")
            .accessible_name("Undo");
        if undo.clicked() {
            app.pending_history = Some(HistoryAction::Undo);
        }
        let redo = ui
            .add_enabled(
                app.history.can_redo(),
                egui::Button::new("Redo").min_size(size),
            )
            .on_hover_text("Redo the change (Ctrl+Y or Ctrl+Shift+Z)")
            .accessible_name("Redo");
        if redo.clicked() {
            app.pending_history = Some(HistoryAction::Redo);
        }
    });
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

fn playback(app: &mut ContraApp, ui: &mut egui::Ui, compact: bool) {
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
    let variations = app.prefs.accompaniment_variations;
    ui.add_enabled_ui(variations, |ui| {
        ui.indent("variation_options", |ui| {
            let options = &mut app.prefs.variation_options;
            ui.checkbox(&mut options.articulation, "Vary note lengths");
            ui.checkbox(&mut options.double_stops, "Add chord-tone double stops");
            ui.checkbox(&mut options.grace_notes, "Add grace notes");
            ui.checkbox(&mut options.octave_lift, "Lift held notes an octave");
            ui.checkbox(&mut options.chord_stabs, "Add chord stabs");
            ui.checkbox(
                &mut options.melody_alone_bar,
                "Let the melody play a bar alone",
            );
        });
    });
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
    let humanize = app.prefs.humanize_timing;
    ui.add_enabled_ui(humanize, |ui| {
        ui.indent("humanize_options", |ui| {
            let options = &mut app.prefs.humanize_options;
            ui.checkbox(&mut options.tempo_drift, "Tempo drift per player");
            ui.checkbox(&mut options.chord_spread, "Spread chord notes");
            slider_row(
                ui,
                compact,
                "Human timing amount",
                &mut options.amount,
                |ui, value| ui.add(egui::Slider::new(value, 0.0..=1.0).fixed_decimals(2)),
            );
        });
    });
    ui.checkbox(&mut app.prefs.ending_options.gradual, "Gradual ending");
    ui.checkbox(
        &mut app.prefs.ending_options.final_chord_down_fifth,
        "Resolve the final chord a fifth lower",
    );
    ui.label(
        egui::RichText::new(
            "The last four dance beats ease and thin into a held final chord. Turning gradual ending \
             off replaces only the final bar. The final chord is a perfect fifth below the written \
             one unless the second option is off.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.heading("Output");
    inline_field(ui, compact, "Output mode", |ui| {
        let combo = egui::ComboBox::from_id_salt("output_mode");
        responsive_combo(combo, ui, compact)
            .selected_text(app.prefs.output_mode.label())
            .show_ui(ui, |ui| {
                for option in OutputMode::ALL {
                    ui.selectable_value(&mut app.prefs.output_mode, option, option.label());
                }
            });
    });
    ui.label(
        egui::RichText::new(
            "Mono folds the band to one channel. Stereo is the default left/right pair. \
             Surround renders five channels: front left, front right, centre, rear left, rear \
             right. An instrument's direction sets its side and whether it sits at the front or \
             behind the dancers; a surround render played on a stereo device is folded down, and a \
             mono or stereo render on a multichannel device keeps its channels and leaves the \
             extras silent.",
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
            "Subtle is the default: a short slapback that widens the band without smearing the \
             dance pulse. Strong is a longer echo; Off is completely dry. Exported MIDI uses the \
             General MIDI delay send; players that do not implement it ignore the echo.",
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

/// Choose the site **Download dance** searches.
///
/// Saved dances keep their own source, so switching this menu never rewrites
/// the library; it only changes where the next download comes from.
fn data_sources(app: &mut ContraApp, ui: &mut egui::Ui, compact: bool) {
    ui.heading("Data sources");
    ui.label(
        egui::RichText::new(
            "The Download dance screen searches the selected source. Saved dances keep the \
             source they came from, and their Open on… link returns there.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let current = app.dance_source();
    // Replace an id from an older or newer build with the provider this build
    // actually resolved, so the file and the menu agree.
    if app.prefs.dance_source != current.id() {
        app.prefs.dance_source = current.id().to_string();
    }
    let mut chosen = None;
    inline_field(ui, compact, "Dance source", |ui| {
        let combo = egui::ComboBox::from_id_salt("dance_source");
        responsive_combo(combo, ui, compact)
            .selected_text(current.display_name())
            .show_ui(ui, |ui| {
                for provider in contra_sources::dance_providers() {
                    if ui
                        .selectable_label(provider.id() == current.id(), provider.display_name())
                        .clicked()
                    {
                        chosen = Some(provider.id());
                    }
                }
            });
    });
    if let Some(id) = chosen {
        app.set_dance_source(id);
    }
    let selected = app.dance_source();
    let mut description = format!("{} — {}", selected.display_name(), selected.base_url());
    match selected.result_limit() {
        Some(limit) => description.push_str(&format!(
            ". Returns up to {limit} matches per search; narrow the search for a large result."
        )),
        None => description.push('.'),
    }
    ui.label(
        egui::RichText::new(description)
            .small()
            .color(theme::muted(ui)),
    );
    ui.hyperlink_to("Open the source site", selected.base_url());
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
    ui.add_space(8.0);
    memory_use(app, ui);
}

/// The measured memory table, with an explicit refresh.
///
/// The report walks the libraries and buffers, so it is built on demand rather
/// than every frame. It is a diagnostic: nothing in the session depends on it.
fn memory_use(app: &mut ContraApp, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("Memory use").strong());
    ui.label(
        egui::RichText::new(
            "Measured and estimated resident memory by category. Numbers vary by platform and \
             by which optional features have loaded.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    let mut refresh = app.memory_report.is_none();
    if ui.button("Refresh memory use").clicked() {
        refresh = true;
    }
    if refresh {
        let ctx = ui.ctx().clone();
        app.memory_report = Some(app.memory_report(Some(&ctx)));
    }
    if let Some(report) = &app.memory_report {
        let mut total = 0u64;
        // One category per block rather than a two-column grid: at 360 points
        // the category name and the long source note would otherwise force the
        // grid wider than the page.
        for (name, bytes, estimated, note) in report.rows() {
            total = total.saturating_add(bytes);
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(name.replace('_', " ")).small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1} MiB{}",
                        crate::memory::mebibytes(bytes),
                        if estimated { " (est.)" } else { "" }
                    ))
                    .small()
                    .color(theme::muted(ui)),
                );
            });
            if !note.is_empty() {
                ui.label(egui::RichText::new(note).small().color(theme::muted(ui)));
            }
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Total").strong());
            ui.label(
                egui::RichText::new(format!("{:.1} MiB", crate::memory::mebibytes(total))).strong(),
            );
        });
    }
}
