use super::*;
use crate::app::*;
use eframe::egui;

impl CallerApp {
    pub(in crate::app) fn tune_download_ui(&mut self, ui: &mut egui::Ui) {
        let compact = compact(ui);
        apply_phone_spacing(ui, compact);
        if compact {
            // Stack the search form, results, and sheet music into one
            // scrolling page on a phone.
            egui::ScrollArea::vertical()
                .id_salt("tune-download-page")
                .auto_shrink([false, false])
                .show(ui, |ui| self.tune_download_body(ui, true));
        } else {
            self.tune_download_body(ui, false);
        }
    }

    fn tune_download_body(&mut self, ui: &mut egui::Ui, compact: bool) {
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if compact {
                    "< Back"
                } else {
                    "< Back to saved tunes"
                })
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
        let mut search_rect = None;
        let mut composer_rect = None;
        let mut key_rect = None;
        let mut meter_rect = None;
        let mut genre_rect = None;
        egui::Grid::new("tune-search-grid")
            .num_columns(if compact { 1 } else { 2 })
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                form_row(ui, compact, "Search", |ui| {
                    let field = ui.text_edit_singleline(&mut self.picker.tune_query.search);
                    search_rect = Some(field.rect);
                    submit |= submitted(ui, &field);
                });
                ui.end_row();
                form_row(ui, compact, "Composer", |ui| {
                    let field = ui.text_edit_singleline(&mut self.picker.tune_query.composer);
                    composer_rect = Some(field.rect);
                    submit |= submitted(ui, &field);
                });
                ui.end_row();
                form_row(ui, compact, "Key", |ui| {
                    key_rect = Some(combo_option(
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
                        compact,
                    ));
                });
                ui.end_row();
                form_row(ui, compact, "Meter", |ui| {
                    meter_rect = Some(combo_option(
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
                        compact,
                    ));
                });
                ui.end_row();
                form_row(ui, compact, "Genre", |ui| {
                    genre_rect = Some(combo_option(
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
                        compact,
                    ));
                });
                ui.end_row();
            });
        if let Some(rect) = search_rect {
            self.callout("tune-search", rect);
        }
        if let (Some(search), Some(composer)) = (search_rect, composer_rect) {
            self.callout("tune-text-fields", search.union(composer));
        }
        if let Some(rect) = key_rect {
            self.callout("tune-key", rect);
        }
        if let Some(rect) = meter_rect {
            self.callout("tune-meter", rect);
        }
        if let Some(rect) = genre_rect {
            self.callout("tune-genre", rect);
        }
        if submit {
            self.search_tunes();
        }
        ui.horizontal_wrapped(|ui| {
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

        if !compact
            && ui
                .button("Reset panes")
                .on_hover_text("Restore the results and preview widths")
                .clicked()
        {
            self.panes.tune_picker_list = PaneWidths::default().tune_picker_list;
        }

        ui.add_space(6.0);
        match picker_pane_height(compact, ui.available_height(), PAGINATION_ROW_HEIGHT) {
            None => {
                // The phone page scrolls: the stacked list and sheet music
                // keep their natural height instead of a fixed empty box.
                ui.label(egui::RichText::new("Results").strong());
                let results = ui.scope(|ui| self.tune_results_list(ui, true));
                self.callout("tune-results", results.response.rect);
                ui.separator();
                ui.label(egui::RichText::new("Preview").strong());
                let preview = ui.scope(|ui| self.tune_preview_panel(ui, true));
                self.callout("tune-preview", preview.response.rect);
            }
            Some(height) => {
                let available = ui.available_width();
                let list_width = clamp_pane_width(self.panes.tune_picker_list, available);
                ui.horizontal_top(|ui| {
                    let list_pane = ui.allocate_ui_with_layout(
                        egui::vec2(list_width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(list_width);
                            ui.set_min_height(height);
                            ui.set_max_height(height);
                            egui::ScrollArea::vertical()
                                .id_salt("tune-results")
                                .max_height(height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.tune_results_list(ui, false));
                        },
                    );
                    self.callout("tune-results", list_pane.response.rect);
                    let splitter = pane_splitter(
                        ui,
                        "tune-results-splitter",
                        height,
                        "Resize the tune search results",
                    );
                    if splitter.dragged() {
                        self.panes.tune_picker_list = clamp_pane_width(
                            self.panes.tune_picker_list + splitter.drag_delta().x,
                            available,
                        );
                    }
                    let preview_width = ui.available_width();
                    let preview_pane = ui.allocate_ui_with_layout(
                        egui::vec2(preview_width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(preview_width);
                            ui.set_min_height(height);
                            ui.set_max_height(height);
                            egui::ScrollArea::vertical()
                                .id_salt("tune-preview")
                                .max_height(height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.tune_preview_panel(ui, false));
                        },
                    );
                    self.callout("tune-preview", preview_pane.response.rect);
                });
            }
        }
        self.pagination_ui(ui, false);
    }

    fn tune_results_list(&mut self, ui: &mut egui::Ui, compact: bool) {
        let animate = self.prefs.animations_enabled;
        let Some(results) = self.picker.tune_results.clone() else {
            return;
        };
        for tune in results.tunes {
            let selected = self
                .picker
                .tune_selected
                .as_ref()
                .is_some_and(|value| value.id == tune.id);
            let selected_t =
                animate_amount(ui.ctx(), ("tune-selected", &tune.id), selected, animate);
            let downloaded = self.tune_library.contains("abcsessiontunes", &tune.id);
            let mut prepared = egui::Frame::group(ui.style())
                .fill(lerp_color(
                    theme::call_upcoming_bg(ui),
                    theme::call_active_bg(ui),
                    selected_t,
                ))
                .inner_margin(egui::Margin::same(8))
                .begin(ui);
            let (preview, focused) = {
                let ui = &mut prepared.content_ui;
                ui.set_width(ui.available_width());
                let body = ui
                    .vertical(|ui| {
                        if compact {
                            ui.set_min_height(theme::TOUCH_TARGET);
                        }
                        tune_summary(ui, &tune, downloaded);
                    })
                    .response;
                let response = ui
                    .interact(
                        body.rect,
                        ui.scope_id().with(("preview-result-tune", &tune.id)),
                        egui::Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Preview this tune");
                let label = format!("Preview {}", tune.title);
                response
                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
                (row_activated(ui, &response), response.has_focus())
            };
            if focused {
                prepared.frame.stroke = ui.visuals().selection.stroke;
            }
            prepared.end(ui);
            if preview {
                self.picker.tune_selected = Some(tune.clone());
                self.source.load_tune(tune.id.clone());
                self.picker.preview_loading = true;
            }
        }
    }

    fn tune_preview_panel(&mut self, ui: &mut egui::Ui, compact: bool) {
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
            let folder_set = matches!(
                self.prefs.tune_rotation.source,
                caller_core::TuneSetSource::Folder(_)
            );
            // The preview actions share the column on a desktop and stack at
            // full width on a phone, so every row is a 44-point target.
            let width = action_width(
                ui.available_width(),
                ui.spacing().item_spacing.x,
                4,
                compact,
            );
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_sized(
                        [width, theme::TOUCH_TARGET],
                        egui::Button::new(if downloaded {
                            "Download again"
                        } else {
                            "Download tune"
                        })
                        .wrap(),
                    )
                    .on_hover_text("Save this tune to your local library")
                    .clicked()
                {
                    self.download_tune(tune.clone());
                }
                if ui
                    .add_sized(
                        [width, theme::TOUCH_TARGET],
                        egui::Button::new("Use this tune").wrap(),
                    )
                    .clicked()
                {
                    self.use_tune(tune.clone());
                }
                if folder_set {
                    if ui
                        .add_sized(
                            [width, theme::TOUCH_TARGET],
                            egui::Button::new("Use chosen tunes").wrap(),
                        )
                        .on_hover_text("Switch the tune set from the folder to the chosen tunes")
                        .clicked()
                    {
                        self.set_tune_set_source(caller_core::TuneSetSource::List);
                    }
                } else if ui
                    .add_sized(
                        [width, theme::TOUCH_TARGET],
                        egui::Button::new(if in_rotation {
                            "Remove from tune set"
                        } else {
                            "Add to tune set"
                        })
                        .wrap(),
                    )
                    .on_hover_text(if in_rotation {
                        "Remove this tune from the set"
                    } else {
                        "Include this tune when Cycle tunes is on"
                    })
                    .clicked()
                {
                    self.toggle_rotation_tune(&tune);
                }
                if ui
                    .add_sized(
                        [width, theme::TOUCH_TARGET],
                        egui::Button::new("View saved tunes").wrap(),
                    )
                    .clicked()
                {
                    self.screen = Screen::TunePicker;
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
            match self.tune_view {
                TuneView::Sheet => {
                    if self.tune_sheet.is_none() {
                        self.tune_sheet = Some(SheetPreview::render(ui.ctx(), &tune.abc));
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
    }
}

fn tune_summary(ui: &mut egui::Ui, tune: &caller_sources::TuneSummary, downloaded: bool) {
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
    if downloaded {
        ui.label(
            egui::RichText::new("downloaded")
                .small()
                .color(theme::muted(ui)),
        );
    }
}
