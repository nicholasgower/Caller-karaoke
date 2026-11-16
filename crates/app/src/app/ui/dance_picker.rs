use super::*;
use crate::app::*;
use eframe::egui;

impl ContraApp {
    pub(in crate::app) fn dance_download_ui(&mut self, ui: &mut egui::Ui) {
        let compact = compact(ui);
        apply_phone_spacing(ui, compact);
        if compact {
            // The search form, results, and preview stack into one scrolling
            // page on a phone; two columns at 390 points leave the result
            // titles and the preview unreadably narrow.
            egui::ScrollArea::vertical()
                .id_salt("dance-download-page")
                .auto_shrink([false, false])
                .show(ui, |ui| self.dance_download_body(ui, true));
        } else {
            self.dance_download_body(ui, false);
        }
    }

    fn dance_download_body(&mut self, ui: &mut egui::Ui, compact: bool) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if compact {
                    "< Back"
                } else {
                    "< Back to saved dances"
                })
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
            .num_columns(if compact { 1 } else { 2 })
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                form_row(ui, compact, "Title", |ui| {
                    submit |= submit_text_edit(ui, &mut self.picker.dance_query.title);
                });
                ui.end_row();
                form_row(ui, compact, "Choreographer", |ui| {
                    submit |= submit_text_edit(ui, &mut self.picker.dance_query.choreographer);
                });
                ui.end_row();
                form_row(ui, compact, "User", |ui| {
                    submit |= submit_text_edit(ui, &mut self.picker.dance_query.user);
                });
                ui.end_row();
                form_row(ui, compact, "Hook", |ui| {
                    submit |= submit_text_edit(ui, &mut self.picker.dance_query.hook);
                });
                ui.end_row();
                form_row(ui, compact, "Formation", |ui| {
                    let combo = egui::ComboBox::from_id_salt("formation");
                    responsive_combo(combo, ui, compact)
                        .selected_text(formation_label(
                            self.picker.dance_query.formation.as_deref(),
                        ))
                        .show_ui(ui, |ui| {
                            for (value, label) in [
                                ("", "Any formation"),
                                ("improper", "Improper"),
                                ("becket", "Becket"),
                                ("proper", "Proper"),
                                ("everything else", "Everything else"),
                            ] {
                                let selected =
                                    self.picker.dance_query.formation.as_deref().unwrap_or("")
                                        == value;
                                if ui.selectable_label(selected, label).clicked() {
                                    self.picker.dance_query.formation = if value.is_empty() {
                                        None
                                    } else {
                                        Some(value.to_string())
                                    };
                                }
                            }
                        });
                });
                ui.end_row();
                form_row(ui, compact, "Figure type", |ui| {
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
                        .picker
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
                    let combo = egui::ComboBox::from_id_salt("figure-move");
                    responsive_combo(combo, ui, compact)
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            for (value, label) in figure_options {
                                let is_selected =
                                    self.picker.dance_query.figure_move.as_deref().unwrap_or("")
                                        == value;
                                if ui.selectable_label(is_selected, label).clicked() {
                                    self.picker.dance_query.figure_move = if value.is_empty() {
                                        None
                                    } else {
                                        Some(value.to_string())
                                    };
                                }
                            }
                        });
                });
                ui.end_row();
                form_row(ui, compact, "Figure contains", |ui| {
                    submit |= submit_text_edit(ui, &mut self.picker.dance_query.figure_text);
                });
                ui.end_row();
                if compact {
                    // Let the label itself toggle the checkbox; a bare square
                    // is a small target on a phone.
                    ui.checkbox(&mut self.picker.dance_query.verified, "Verified only");
                } else {
                    form_row(ui, compact, "Verified only", |ui| {
                        ui.checkbox(&mut self.picker.dance_query.verified, "");
                    });
                }
                ui.end_row();
                form_row(ui, compact, "Sort", |ui| {
                    let combo = egui::ComboBox::from_id_salt("dance-sort");
                    responsive_combo(combo, ui, compact)
                        .selected_text(self.picker.dance_query.sort.label())
                        .show_ui(ui, |ui| {
                            for sort in DanceSort::ALL {
                                ui.selectable_value(
                                    &mut self.picker.dance_query.sort,
                                    sort,
                                    sort.label(),
                                );
                            }
                        });
                });
                ui.end_row();
            });
        if submit {
            self.search_dances();
        }
        ui.horizontal_wrapped(|ui| {
            let search = ui
                .add_enabled(
                    !self.picker.search_loading,
                    egui::Button::new("Search dances"),
                )
                .on_hover_text("Search dances (Enter)");
            if search.clicked() {
                self.search_dances();
            }
            if self.picker.search_loading {
                ui.spinner();
                ui.label("Searching…");
            }
            if let Some(results) = &self.picker.dance_results {
                ui.label(
                    egui::RichText::new(format!("{} matches", results.total))
                        .color(theme::muted(ui)),
                );
            }
        });

        ui.add_space(6.0);
        match picker_pane_height(compact, ui.available_height(), PAGINATION_ROW_HEIGHT) {
            None => {
                // The phone page scrolls: the stacked list and preview keep
                // their natural height instead of reserving an empty box.
                ui.label(egui::RichText::new("Results").strong());
                self.dance_results_list(ui, true);
                ui.separator();
                ui.label(egui::RichText::new("Preview").strong());
                self.dance_preview_panel(ui, true);
            }
            Some(height) => {
                ui.columns(2, |columns| {
                    egui::ScrollArea::vertical()
                        .id_salt("dance-results")
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(&mut columns[0], |ui| self.dance_results_list(ui, false));
                    egui::ScrollArea::vertical()
                        .id_salt("dance-preview")
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(&mut columns[1], |ui| self.dance_preview_panel(ui, false));
                });
            }
        }
        self.pagination_ui(ui, true);
    }

    fn dance_results_list(&mut self, ui: &mut egui::Ui, compact: bool) {
        let animate = self.prefs.animations_enabled;
        let Some(results) = self.picker.dance_results.clone() else {
            return;
        };
        for dance in results.dances {
            let selected = self
                .picker
                .dance_selected
                .as_ref()
                .is_some_and(|value| value.id == dance.id);
            let selected_t =
                animate_amount(ui.ctx(), ("dance-selected", &dance.id), selected, animate);
            let downloaded = self.dance_library.contains("contradb", &dance.id);
            let mut preview = false;
            egui::Frame::group(ui.style())
                .fill(lerp_color(
                    theme::call_upcoming_bg(ui),
                    theme::call_active_bg(ui),
                    selected_t,
                ))
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    if compact {
                        ui.set_width(ui.available_width());
                        dance_summary(ui, &dance, downloaded);
                        if ui.button("Preview").clicked() {
                            preview = true;
                        }
                    } else {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| dance_summary(ui, &dance, downloaded));
                            if ui.small_button("Preview").clicked() {
                                preview = true;
                            }
                        });
                    }
                });
            if preview {
                self.picker.dance_selected = Some(dance.clone());
                self.source.load_dance(dance.id.clone());
                self.picker.preview_loading = true;
            }
        }
    }

    fn dance_preview_panel(&mut self, ui: &mut egui::Ui, compact: bool) {
        if self.picker.preview_loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Loading dance…");
            });
        }
        if let Some(dance) = self.picker.dance_preview.clone() {
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
            // The preview actions share the column on a desktop and stack at
            // full width on a phone, so every row is a 44-point target.
            let width = action_width(
                ui.available_width(),
                ui.spacing().item_spacing.x,
                3,
                compact,
            );
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_sized(
                        [width, theme::TOUCH_TARGET],
                        egui::Button::new(if downloaded {
                            "Download again"
                        } else {
                            "Download dance"
                        })
                        .wrap(),
                    )
                    .on_hover_text("Save this card to your local library")
                    .clicked()
                {
                    self.download_dance(dance.clone());
                }
                if ui
                    .add_sized(
                        [width, theme::TOUCH_TARGET],
                        egui::Button::new("Use this dance").wrap(),
                    )
                    .clicked()
                {
                    self.use_dance(dance.clone());
                }
                if ui
                    .add_sized(
                        [width, theme::TOUCH_TARGET],
                        egui::Button::new("View saved dances").wrap(),
                    )
                    .clicked()
                {
                    self.screen = Screen::DancePicker;
                }
            });
            if downloaded {
                ui.label(
                    egui::RichText::new("already downloaded")
                        .small()
                        .color(theme::muted(ui)),
                );
            }
            ui.separator();
            for figure in &dance.figures {
                ui.label(format!(
                    "{}  {:.0}b  {}",
                    figure.phrase.as_deref().unwrap_or(""),
                    figure.beats,
                    figure.text
                ));
            }
        } else if !self.picker.preview_loading {
            ui.label(
                egui::RichText::new("Preview a dance to see its card.").color(theme::muted(ui)),
            );
        }
    }
}

fn dance_summary(ui: &mut egui::Ui, dance: &contra_sources::DanceSummary, downloaded: bool) {
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
    if downloaded {
        ui.label(
            egui::RichText::new("downloaded")
                .small()
                .color(theme::muted(ui)),
        );
    }
}
