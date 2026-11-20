use super::*;
use crate::app::*;
use eframe::egui;

impl CallerApp {
    pub(in crate::app) fn aliases_ui(&mut self, ui: &mut egui::Ui) {
        let compact = compact(ui);
        apply_phone_spacing(ui, compact);
        if compact {
            egui::ScrollArea::vertical()
                .id_salt("aliases-page")
                .auto_shrink([false, false])
                .show(ui, |ui| self.aliases_body(ui, true));
        } else {
            self.aliases_body(ui, false);
        }
    }

    fn aliases_body(&mut self, ui: &mut egui::Ui, compact: bool) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if compact {
                    "< Back"
                } else {
                    "< Back to preferences"
                })
                .on_hover_text("Back to preferences (Esc)")
                .clicked()
            {
                self.screen = Screen::Preferences;
            }
            ui.heading("Dance move aliases");
        });
        let placeholders = ui.label(
            egui::RichText::new(concat!(
                "These are the words shown and spoken during a session. Placeholders: ",
                "{actor}, {actor_singular}, {actor_plural}, {actor_object}, ",
                "{actor_object_plural}, {direction}, {count}."
            ))
            .color(theme::muted(ui)),
        );
        self.callout("alias-placeholders", placeholders.rect);
        ui.label(
            egui::RichText::new("Alternatives are used only when \"Shuffle alternative call aliases\" is on in Preferences.")
                .small()
                .color(theme::muted(ui)),
        );
        ui.add_space(6.0);
        let mut reset_aliases = false;
        if compact {
            ui.label(
                egui::RichText::new("Filter")
                    .small()
                    .color(theme::muted(ui)),
            );
            ui.add(egui::TextEdit::singleline(&mut self.alias_filter).desired_width(f32::INFINITY));
            let reset_all = ui.button("Reset all aliases");
            self.callout("alias-reset-all", reset_all.rect);
            if reset_all.clicked() {
                reset_aliases = true;
            }
        } else {
            ui.horizontal(|ui| {
                ui.label("Filter");
                ui.text_edit_singleline(&mut self.alias_filter);
                let reset_all = ui.button("Reset all aliases");
                self.callout("alias-reset-all", reset_all.rect);
                if reset_all.clicked() {
                    reset_aliases = true;
                }
            });
        }
        if reset_aliases {
            self.prefs.aliases = AliasTable::default();
            self.rebuild_timeline();
            self.needs_render = true;
            self.save_prefs();
        }

        if compact {
            self.alias_list(ui, true);
        } else {
            egui::ScrollArea::vertical()
                .max_height(460.0)
                .show(ui, |ui| self.alias_list(ui, false));
        }

        ui.separator();
        ui.heading("Custom figure aliases");
        ui.label(
            egui::RichText::new("Use this for figures the classifier does not recognise.")
                .small()
                .color(theme::muted(ui)),
        );
        let custom = self.prefs.aliases.custom.clone();
        for (key, alias) in custom {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(&key).monospace());
                ui.label("→");
                ui.label(&alias);
                let remove = ui.button("Remove");
                if remove.clicked() {
                    self.prefs.aliases.remove_custom(&key);
                    self.rebuild_timeline();
                    self.needs_render = true;
                    self.save_prefs();
                }
            });
        }
        if compact {
            ui.label(
                egui::RichText::new("Card phrase")
                    .small()
                    .color(theme::muted(ui)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.custom_key)
                    .hint_text("card phrase")
                    .desired_width(f32::INFINITY),
            );
            ui.label(
                egui::RichText::new("Call to say")
                    .small()
                    .color(theme::muted(ui)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.custom_alias)
                    .hint_text("call to say")
                    .desired_width(f32::INFINITY),
            );
            let clicked = ui.button("Add alias").on_hover_text("Add alias").clicked();
            if clicked {
                self.add_custom_alias();
            }
        } else {
            let custom_row = ui.horizontal(|ui| {
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
            self.callout("alias-custom", custom_row.response.rect);
        }
    }

    fn alias_list(&mut self, ui: &mut egui::Ui, compact: bool) {
        let filter = self.alias_filter.to_lowercase();
        let mut changed = false;
        let mut first_card = true;
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
            let card = egui::Frame::group(ui.style())
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
                    let field =
                        ui.add(egui::TextEdit::singleline(&mut value).desired_width(f32::INFINITY));
                    if first_card {
                        self.callout("alias-field", field.rect);
                    }
                    if field.changed() {
                        self.prefs.aliases.set_override(kind, value);
                        changed = true;
                    }
                    let reset = ui.button("Reset to default");
                    if first_card {
                        self.callout("alias-reset", reset.rect);
                    }
                    if reset.clicked() {
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
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("· {template}"))
                                        .small()
                                        .color(theme::muted(ui)),
                                );
                                let remove = ui.button("Remove");
                                if is_added && remove.clicked() {
                                    self.prefs.aliases.remove_alternative(kind, template);
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
                    if compact {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut input)
                                .hint_text("add an alternative call")
                                .desired_width(f32::INFINITY),
                        );
                        let submit = submitted(ui, &response);
                        let clicked = ui.button("Add alternative").clicked();
                        if clicked || submit {
                            self.prefs.aliases.add_alternative(kind, input.clone());
                            alternative_added = true;
                            changed = true;
                        }
                    } else {
                        ui.horizontal(|ui| {
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut input)
                                    .hint_text("add an alternative call")
                                    .desired_width(180.0),
                            );
                            let submit = submitted(ui, &response);
                            let clicked = ui
                                .button("Add alternative")
                                .on_hover_text("Add alternative (Enter)")
                                .clicked();
                            if clicked || submit {
                                self.prefs.aliases.add_alternative(kind, input.clone());
                                alternative_added = true;
                                changed = true;
                            }
                        });
                    }
                    self.alternative_inputs.insert(
                        kind.id().to_string(),
                        if alternative_added {
                            String::new()
                        } else {
                            input
                        },
                    );
                });
            if first_card {
                self.callout("alias-list", card.response.rect);
                first_card = false;
            }
        }
        if changed {
            self.rebuild_timeline();
            self.needs_render = true;
            self.save_prefs();
        }
    }
}
