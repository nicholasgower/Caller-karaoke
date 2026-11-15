use super::*;
use crate::app::*;
use eframe::egui;

impl ContraApp {
    pub(in crate::app) fn dance_download_ui(&mut self, ui: &mut egui::Ui) {
        let animate = self.prefs.animations_enabled;
        ui.horizontal(|ui| {
            if ui
                .button("< Back to saved dances")
                .on_hover_text("Back to saved dances (Esc)")
                .clicked()
            {
                self.screen = Screen::DancePicker;
            }
            ui.heading("Download dance");
        });
        ui.label(
            egui::RichText::new(
                "Search ContraDB, preview the card, and download it to your saved dances. \
                 Downloaded cards stay available without a network connection.",
            )
            .color(theme::muted(ui)),
        );
        if let Some(status) = &self.library_status {
            ui.label(egui::RichText::new(status).small().color(theme::muted(ui)));
        }
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
                            let selected =
                                self.dance_query.formation.as_deref().unwrap_or("") == value;
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
                    egui::RichText::new(format!("{} matches", results.total))
                        .color(theme::muted(ui)),
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
                                        if self.dance_library.contains("contradb", &dance.id) {
                                            ui.label(
                                                egui::RichText::new("downloaded")
                                                    .small()
                                                    .color(theme::muted(ui)),
                                            );
                                        }
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
                        ui.add_space(4.0);
                        let downloaded = self
                            .dance_library
                            .contains(&dance.source.provider, &dance.source.id);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .button(if downloaded {
                                    "Download again"
                                } else {
                                    "Download dance"
                                })
                                .on_hover_text("Save this card to your local library")
                                .clicked()
                            {
                                self.download_dance(dance.clone());
                            }
                            if ui.button("Use this dance").clicked() {
                                self.use_dance(dance.clone());
                            }
                            if ui.button("View saved dances").clicked() {
                                self.screen = Screen::DancePicker;
                            }
                            if downloaded {
                                ui.label(
                                    egui::RichText::new("already downloaded")
                                        .small()
                                        .color(theme::muted(ui)),
                                );
                            }
                        });
                        ui.separator();
                        for figure in &dance.figures {
                            ui.label(format!(
                                "{}  {:.0}b  {}",
                                figure.phrase.as_deref().unwrap_or(""),
                                figure.beats,
                                figure.text
                            ));
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
}
