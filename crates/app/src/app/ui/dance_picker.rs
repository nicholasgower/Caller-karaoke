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
        let provider = self.dance_source();
        let display_name = provider.display_name();
        ui.label(
            egui::RichText::new(format!(
                "Search {display_name}, preview the card, and download it to your saved \
                 dances. Downloaded cards stay available without a network connection."
            ))
            .color(theme::muted(ui)),
        );
        // The source belongs beside the search it controls: a saved dance keeps
        // the source it came from, so this only chooses where the next download
        // comes from.
        let mut chosen = None;
        let current = self.dance_source();
        if compact {
            ui.label(
                egui::RichText::new("Dance source")
                    .small()
                    .color(theme::muted(ui)),
            );
            let combo = egui::ComboBox::from_id_salt("download-dance-source");
            responsive_combo(combo, ui, true)
                .selected_text(current.display_name())
                .show_ui(ui, |ui| {
                    for provider in contra_sources::dance_providers() {
                        if ui
                            .selectable_label(
                                provider.id() == current.id(),
                                provider.display_name(),
                            )
                            .clicked()
                        {
                            chosen = Some(provider.id());
                        }
                    }
                });
        } else {
            ui.horizontal(|ui| {
                ui.label("Dance source");
                let combo = egui::ComboBox::from_id_salt("download-dance-source");
                combo
                    .selected_text(current.display_name())
                    .show_ui(ui, |ui| {
                        for provider in contra_sources::dance_providers() {
                            if ui
                                .selectable_label(
                                    provider.id() == current.id(),
                                    provider.display_name(),
                                )
                                .clicked()
                            {
                                chosen = Some(provider.id());
                            }
                        }
                    });
            });
        }
        if let Some(id) = chosen {
            self.set_dance_source(id);
        }
        let provider = self.dance_source();
        let display_name = provider.display_name();
        let mut description = format!("{} — {}", display_name, provider.base_url());
        match provider.result_limit() {
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
        ui.hyperlink_to("Open the source site", provider.base_url());
        if let Some(status) = &self.library_status {
            ui.label(egui::RichText::new(status).small().color(theme::muted(ui)));
        }
        ui.add_space(6.0);

        let specs = provider.filter_specs();
        let mut submit = false;
        egui::Grid::new("dance-search-grid")
            .num_columns(if compact { 1 } else { 2 })
            .spacing([10.0, 8.0])
            .show(ui, |ui| {
                // A source advertises the fields it understands; the screen
                // draws only those, so a site without a "User" or "Verified"
                // filter does not show a field that would be ignored.
                for key in [
                    "title",
                    "choreographer",
                    "user",
                    "hook",
                    "formation",
                    "figure_move",
                    "figure_text",
                    "verified",
                ] {
                    let Some(spec) = specs.iter().find(|spec| spec.key == key) else {
                        continue;
                    };
                    match &spec.kind {
                        FilterKind::Text => {
                            if let Some(buffer) = query_text(&mut self.picker.dance_query, spec.key)
                            {
                                form_row(ui, compact, spec.label, |ui| {
                                    submit |= submit_text_edit(ui, buffer);
                                });
                            }
                            ui.end_row();
                        }
                        FilterKind::Checkbox => {
                            if let Some(flag) = query_flag(&mut self.picker.dance_query, spec.key) {
                                if compact {
                                    ui.checkbox(flag, spec.label);
                                } else {
                                    form_row(ui, compact, spec.label, |ui| {
                                        ui.checkbox(flag, "");
                                    });
                                }
                                ui.end_row();
                            }
                        }
                        FilterKind::Select(options) => {
                            let current = query_select_value(&self.picker.dance_query, spec.key)
                                .cloned()
                                .unwrap_or_default();
                            let selected_label = options
                                .iter()
                                .find(|(value, _)| *value == current)
                                .map(|(_, label)| *label)
                                .unwrap_or(spec.label);
                            form_row(ui, compact, spec.label, |ui| {
                                let combo =
                                    egui::ComboBox::from_id_salt(("dance-filter", spec.key));
                                responsive_combo(combo, ui, compact)
                                    .selected_text(selected_label)
                                    .show_ui(ui, |ui| {
                                        for (value, label) in options {
                                            if ui
                                                .selectable_label(current == *value, *label)
                                                .clicked()
                                            {
                                                set_query_select_value(
                                                    &mut self.picker.dance_query,
                                                    spec.key,
                                                    value,
                                                );
                                            }
                                        }
                                    });
                            });
                            ui.end_row();
                        }
                    }
                }
                if provider.supports_sort() {
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
                }
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
                let label =
                    if provider.result_limit().is_some() && results.total > results.dances.len() {
                        format!(
                            "{} matches · first {} shown",
                            results.total,
                            results.dances.len()
                        )
                    } else {
                        format!("{} matches", results.total)
                    };
                ui.label(egui::RichText::new(label).color(theme::muted(ui)));
            }
        });
        if let Some(limit) = provider.result_limit() {
            ui.label(
                egui::RichText::new(format!(
                    "{display_name} returns up to {limit} matches per search and has no \
                     further pages. Add a title, author, or figure phrase to narrow a \
                     larger result."
                ))
                .small()
                .color(theme::muted(ui)),
            );
        }
        if !compact
            && ui
                .small_button("Reset panes")
                .on_hover_text("Restore the results and preview widths")
                .clicked()
        {
            self.panes.dance_picker_list = PaneWidths::default().dance_picker_list;
        }

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
                let available = ui.available_width();
                let list_width = clamp_pane_width(self.panes.dance_picker_list, available);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(list_width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(list_width);
                            ui.set_min_height(height);
                            ui.set_max_height(height);
                            egui::ScrollArea::vertical()
                                .id_salt("dance-results")
                                .max_height(height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.dance_results_list(ui, false));
                        },
                    );
                    let splitter = pane_splitter(
                        ui,
                        "dance-results-splitter",
                        height,
                        "Resize the dance search results",
                    );
                    if splitter.dragged() {
                        self.panes.dance_picker_list = clamp_pane_width(
                            self.panes.dance_picker_list + splitter.drag_delta().x,
                            available,
                        );
                    }
                    let preview_width = ui.available_width();
                    ui.allocate_ui_with_layout(
                        egui::vec2(preview_width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(preview_width);
                            ui.set_min_height(height);
                            ui.set_max_height(height);
                            egui::ScrollArea::vertical()
                                .id_salt("dance-preview")
                                .max_height(height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| self.dance_preview_panel(ui, false));
                        },
                    );
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
            let downloaded = self
                .dance_library
                .contains(&self.picker.dance_provider, &dance.id);
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
                        dance_summary(ui, &dance, downloaded);
                    })
                    .response;
                let response = ui
                    .interact(
                        body.rect,
                        ui.scope_id().with(("preview-result-dance", &dance.id)),
                        egui::Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Preview this dance");
                let label = format!("Preview {}", dance.title);
                response
                    .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
                (row_activated(ui, &response), response.has_focus())
            };
            if focused {
                prepared.frame.stroke = ui.visuals().selection.stroke;
            }
            prepared.end(ui);
            if preview {
                self.picker.dance_selected = Some(dance.clone());
                self.source
                    .load_dance(&self.picker.dance_provider, dance.id.clone());
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
            ui.hyperlink_to(
                format!("Open on {}", source_label(&dance.source.provider)),
                &dance.source.url,
            );
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
            ui.scope(|ui| {
                // A compact card packs its figures closer, matching the
                // session's dance card and the saved-dance preview.
                ui.spacing_mut().item_spacing.y = self.prefs.card_padding.gap();
                for figure in &dance.figures {
                    ui.label(format!(
                        "{}  {:.0}b  {}",
                        figure.phrase.as_deref().unwrap_or(""),
                        figure.beats,
                        figure.text
                    ));
                }
            });
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

/// The text field behind one source-neutral filter key.
fn query_text<'a>(query: &'a mut DanceQuery, key: &str) -> Option<&'a mut String> {
    match key {
        "title" => Some(&mut query.title),
        "choreographer" => Some(&mut query.choreographer),
        "user" => Some(&mut query.user),
        "hook" => Some(&mut query.hook),
        "figure_text" => Some(&mut query.figure_text),
        _ => None,
    }
}

/// The checkbox behind one source-neutral filter key.
fn query_flag<'a>(query: &'a mut DanceQuery, key: &str) -> Option<&'a mut bool> {
    (key == "verified").then_some(&mut query.verified)
}

/// The current value of a select filter; an empty value means "any".
fn query_select_value<'a>(query: &'a DanceQuery, key: &str) -> Option<&'a String> {
    match key {
        "formation" => query.formation.as_ref(),
        "figure_move" => query.figure_move.as_ref(),
        _ => None,
    }
}

/// Store a select filter, mapping the empty option back to `None`.
fn set_query_select_value(query: &mut DanceQuery, key: &str, value: &str) {
    let target = match key {
        "formation" => &mut query.formation,
        "figure_move" => &mut query.figure_move,
        _ => return,
    };
    *target = (!value.is_empty()).then(|| value.to_string());
}
