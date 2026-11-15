use super::*;
use crate::app::*;
use eframe::egui;

impl ContraApp {
    pub(in crate::app) fn aliases_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .button("< Back to preferences")
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
