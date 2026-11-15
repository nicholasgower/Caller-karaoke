use super::*;
use crate::app::*;
use eframe::egui;

impl ContraApp {
    pub(in crate::app) fn tune_download_ui(&mut self, ui: &mut egui::Ui) {
        let animate = self.prefs.animations_enabled;
        ui.horizontal(|ui| {
            if ui
                .button("< Back to saved tunes")
                .on_hover_text("Back to saved tunes (Esc)")
                .clicked()
            {
                self.screen = Screen::TunePicker;
            }
            ui.heading("Download tune");
        });
        ui.label(
            egui::RichText::new(
                "Search ABC Session Tunes, preview the sheet music, and download the tune to your saved tunes. \
                 Downloaded tunes stay available without a network connection.",
            )
            .color(theme::muted(ui)),
        );
        if let Some(status) = &self.library_status {
            ui.label(egui::RichText::new(status).small().color(theme::muted(ui)));
        }
        ui.add_space(6.0);

        let mut submit = false;
        egui::Grid::new("tune-search-grid")
            .num_columns(2)
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                ui.label("Search");
                submit |= submit_text_edit(ui, &mut self.picker.tune_query.search);
                ui.end_row();
                ui.label("Composer");
                submit |= submit_text_edit(ui, &mut self.picker.tune_query.composer);
                ui.end_row();
                ui.label("Key");
                combo_option(
                    ui,
                    "tune-key",
                    &mut self.picker.tune_query.key,
                    &[
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
                    ],
                );
                ui.end_row();
                ui.label("Meter");
                combo_option(
                    ui,
                    "tune-meter",
                    &mut self.picker.tune_query.meter,
                    &[
                        ("", "Any meter"),
                        ("4/4", "4/4"),
                        ("2/2", "2/2"),
                        ("2/4", "2/4"),
                        ("6/8", "6/8"),
                        ("9/8", "9/8"),
                        ("12/8", "12/8"),
                        ("3/4", "3/4"),
                    ],
                );
                ui.end_row();
                ui.label("Genre");
                combo_option(
                    ui,
                    "tune-rhythm",
                    &mut self.picker.tune_query.rhythm,
                    &[
                        ("", "Any genre"),
                        ("reel", "Reel"),
                        ("jig", "Jig"),
                        ("slip jig", "Slip jig"),
                        ("hornpipe", "Hornpipe"),
                        ("polka", "Polka"),
                        ("slide", "Slide"),
                        ("strathspey", "Strathspey"),
                        ("waltz", "Waltz"),
                    ],
                );
                ui.end_row();
            });
        if submit {
            self.search_tunes();
        }
        ui.horizontal(|ui| {
            let search = ui
                .add_enabled(
                    !self.picker.search_loading,
                    egui::Button::new("Search tunes"),
                )
                .on_hover_text("Search tunes (Enter)");
            if search.clicked() {
                self.search_tunes();
            }
            if self.picker.search_loading {
                ui.spinner();
                ui.label("Searching…");
            }
            if let Some(results) = &self.picker.tune_results {
                ui.label(
                    egui::RichText::new(format!("{} matches", results.total))
                        .color(theme::muted(ui)),
                );
            }
        });

        ui.add_space(6.0);
        ui.columns(2, |columns| {
            egui::ScrollArea::vertical()
                .id_salt("tune-results")
                .max_height(420.0)
                .show(&mut columns[0], |ui| {
                    let Some(results) = self.picker.tune_results.clone() else {
                        return;
                    };
                    for tune in results.tunes {
                        let selected = self.picker
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
                                        if self
                                            .tune_library
                                            .contains("abcsessiontunes", &tune.id)
                                        {
                                            ui.label(
                                                egui::RichText::new("downloaded")
                                                    .small()
                                                    .color(theme::muted(ui)),
                                            );
                                        }
                                    });
                                    if ui.small_button("Preview").clicked() {
                                        self.picker.tune_selected = Some(tune.clone());
                                        self.source.load_tune(tune.id.clone());
                                        self.picker.preview_loading = true;
                                    }
                                });
                            });
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("tune-preview")
                .max_height(420.0)
                .show(&mut columns[1], |ui| {
                    if self.picker.preview_loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading tune…");
                        });
                    }
                    if let Some(tune) = self.picker.tune_preview.clone() {
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
                        ui.add_space(4.0);
                        let downloaded = self
                            .tune_library
                            .contains(&tune.source.provider, &tune.source.id);
                        let in_rotation = self.tune_in_rotation(&tune);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .button(if downloaded {
                                    "Download again"
                                } else {
                                    "Download tune"
                                })
                                .on_hover_text("Save this tune to your local library")
                                .clicked()
                            {
                                self.download_tune(tune.clone());
                            }
                            if ui.button("Use this tune").clicked() {
                                self.use_tune(tune.clone());
                            }
                            if ui
                                .button(if in_rotation {
                                    "Remove from tune set"
                                } else {
                                    "Add to tune set"
                                })
                                .on_hover_text(if in_rotation {
                                    "Remove this tune from the set"
                                } else {
                                    "Include this tune when Cycle tunes is on"
                                })
                                .clicked()
                            {
                                self.toggle_rotation_tune(&tune);
                            }
                            if ui.button("View saved tunes").clicked() {
                                self.screen = Screen::TunePicker;
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
                    } else if !self.picker.preview_loading {
                        ui.label(
                            egui::RichText::new("Preview a tune to see its sheet music.")
                                .color(theme::muted(ui)),
                        );
                    }
                });
        });
        self.pagination_ui(ui, false);
    }
}
