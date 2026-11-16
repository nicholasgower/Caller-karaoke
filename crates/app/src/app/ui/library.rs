use super::*;
use crate::app::*;
use eframe::egui;

impl ContraApp {
    pub(in crate::app) fn dance_library_ui(&mut self, ui: &mut egui::Ui) {
        let compact = compact(ui);
        apply_phone_spacing(ui, compact);
        let mut open_download = false;
        if compact {
            ui.heading("Select dance");
            // At phone width the heading and the action do not fit on one row.
            let width = ui.available_width();
            if ui
                .add_sized(
                    [width, theme::TOUCH_TARGET],
                    egui::Button::new("Download dance"),
                )
                .on_hover_text("Search ContraDB and add a card to your saved dances")
                .clicked()
            {
                open_download = true;
            }
        } else {
            ui.horizontal(|ui| {
                ui.heading("Select dance");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("Download dance")
                        .on_hover_text("Search ContraDB and add a card to your saved dances")
                        .clicked()
                    {
                        open_download = true;
                    }
                });
            });
        }
        if open_download {
            self.picker.dance_preview = None;
            self.picker.dance_selected = None;
            self.screen = Screen::DanceDownload;
        }
        ui.label(
            egui::RichText::new(
                "Your saved dances, available offline. Search them here, or use Download dance \
                 to fetch another card from ContraDB.",
            )
            .color(theme::muted(ui)),
        );
        ui.add_space(4.0);

        let shown = self
            .dance_library
            .search(&self.dance_library_query, &self.dance_library_folder)
            .len();
        let count_label = format!(
            "{shown} of {} shown · {}",
            self.dance_library.len(),
            self.dance_library_folder.label()
        );
        if compact {
            ui.label(
                egui::RichText::new("Search saved")
                    .small()
                    .color(theme::muted(ui)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.dance_library_query)
                    .hint_text("title, choreographer, formation, or figure")
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal_wrapped(|ui| {
                if !self.dance_library_query.is_empty() && ui.button("Clear").clicked() {
                    self.dance_library_query.clear();
                }
                ui.label(
                    egui::RichText::new(count_label)
                        .small()
                        .color(theme::muted(ui)),
                );
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label("Search saved");
                ui.add(
                    egui::TextEdit::singleline(&mut self.dance_library_query)
                        .hint_text("title, choreographer, formation, or figure")
                        .desired_width(340.0),
                );
                if !self.dance_library_query.is_empty() && ui.small_button("Clear").clicked() {
                    self.dance_library_query.clear();
                }
                ui.label(
                    egui::RichText::new(count_label)
                        .small()
                        .color(theme::muted(ui)),
                );
            });
        }
        if let Some(status) = &self.library_status {
            ui.label(egui::RichText::new(status).small().color(theme::muted(ui)));
        }
        ui.add_space(4.0);

        let folders = self.dance_library.folders.clone();
        let counts: Vec<usize> = folders
            .iter()
            .map(|folder| {
                self.dance_library
                    .count(&FolderFilter::Folder(folder.clone()))
            })
            .collect();
        let all_count = self.dance_library.count(&FolderFilter::All);
        let unfiled_count = self.dance_library.count(&FolderFilter::Unfiled);
        let wide = ui.available_width() >= 760.0;
        let mut command = None;

        if wide {
            // The folder panel and both saved-list columns share one height so
            // they end at the same bottom edge as the window.
            let pane_height = remaining_pane_height(ui.available_height(), 0.0);
            ui.horizontal_top(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("dance-folder-panel")
                    .max_height(pane_height)
                    // Shrink horizontally to the 230-point panel so the list
                    // and preview keep the rest of the row; fill vertically so
                    // the panel ends at the same bottom edge as the columns.
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(230.0);
                            command = folder_panel_ui(
                                ui,
                                FolderPanel {
                                    all_label: "All dances",
                                    folders: &folders,
                                    counts: &counts,
                                    all_count,
                                    unfiled_count,
                                    filter: &mut self.dance_library_folder,
                                    new_folder: &mut self.new_dance_folder,
                                    renaming: &mut self.renaming_dance_folder,
                                    rename_text: &mut self.folder_rename_text,
                                },
                            );
                        });
                    });
                ui.separator();
                self.dance_library_body(ui, false);
            });
        } else {
            egui::ScrollArea::vertical()
                .id_salt("dance-library-narrow-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // Folders are a filter, not the main content: keep them
                    // collapsed on a phone so the saved cards start on screen.
                    command = egui::CollapsingHeader::new(format!(
                        "Folders · {}",
                        self.dance_library_folder.label()
                    ))
                    .default_open(false)
                    .show(ui, |ui| {
                        folder_panel_ui(
                            ui,
                            FolderPanel {
                                all_label: "All dances",
                                folders: &folders,
                                counts: &counts,
                                all_count,
                                unfiled_count,
                                filter: &mut self.dance_library_folder,
                                new_folder: &mut self.new_dance_folder,
                                renaming: &mut self.renaming_dance_folder,
                                rename_text: &mut self.folder_rename_text,
                            },
                        )
                    })
                    .body_returned
                    .flatten();
                    ui.separator();
                    self.dance_library_body(ui, true);
                });
        }

        if let Some(command) = command {
            self.apply_dance_folder_command(command);
        }
    }

    fn dance_library_body(&mut self, ui: &mut egui::Ui, compact: bool) {
        let animate = self.prefs.animations_enabled;
        let mut entries: Vec<contra_core::LibraryEntry<Dance>> = self
            .dance_library
            .search(&self.dance_library_query, &self.dance_library_folder)
            .into_iter()
            .cloned()
            .collect();
        entries.sort_by_key(|entry| entry.item.title.to_lowercase());
        let folders = self.dance_library.folders.clone();
        let selected_id = self
            .picker
            .dance_preview
            .as_ref()
            .map(|dance| dance.source.id.clone())
            .filter(|id| self.dance_library.contains("contradb", id));
        let preview = selected_id.as_deref().and_then(|id| {
            entries
                .iter()
                .find(|entry| entry.item.source.id == id)
                .map(|entry| entry.item.clone())
        });
        let empty_message = if self.dance_library.is_empty() {
            "No saved dances yet. Use Download dance to fetch one from ContraDB."
        } else if entries.is_empty() {
            "No saved dances match this search and folder."
        } else {
            ""
        };
        let mut action = None;

        if compact {
            if !empty_message.is_empty() {
                ui.label(egui::RichText::new(empty_message).color(theme::muted(ui)));
            }
            for entry in &entries {
                if let Some(next) =
                    saved_dance_row(ui, entry, selected_id.as_deref(), &folders, animate, true)
                {
                    action = Some(next);
                }
            }
            ui.separator();
            if let Some(dance) = &preview {
                if let Some(next) = saved_dance_preview(ui, dance, true) {
                    action = Some(next);
                }
            } else {
                ui.label(
                    egui::RichText::new("Select a saved dance to see its card.")
                        .color(theme::muted(ui)),
                );
            }
        } else {
            // The saved-library page does not scroll as a whole, so the panes
            // take every point left below the search and status rows.
            let height = remaining_pane_height(ui.available_height(), 0.0);
            if ui.available_width() >= 560.0 {
                ui.columns(2, |columns| {
                    egui::ScrollArea::vertical()
                        .id_salt("saved-dance-list")
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(&mut columns[0], |ui| {
                            if !empty_message.is_empty() {
                                ui.label(
                                    egui::RichText::new(empty_message).color(theme::muted(ui)),
                                );
                            }
                            for entry in &entries {
                                if let Some(next) = saved_dance_row(
                                    ui,
                                    entry,
                                    selected_id.as_deref(),
                                    &folders,
                                    animate,
                                    false,
                                ) {
                                    action = Some(next);
                                }
                            }
                        });
                    egui::ScrollArea::vertical()
                        .id_salt("saved-dance-preview")
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(&mut columns[1], |ui| {
                            if let Some(dance) = &preview {
                                if let Some(next) = saved_dance_preview(ui, dance, false) {
                                    action = Some(next);
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new("Select a saved dance to see its card.")
                                        .color(theme::muted(ui)),
                                );
                            }
                        });
                });
            } else {
                // Too narrow for two readable columns: one scrolling pane
                // holds the list and preview and still ends at the same bottom
                // edge as the folder panel. The pane is vertical even though
                // its parent row is horizontal, or the card and preview would
                // flow side by side inside the scroll content.
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("saved-dance-list-narrow")
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if !empty_message.is_empty() {
                                ui.label(
                                    egui::RichText::new(empty_message).color(theme::muted(ui)),
                                );
                            }
                            for entry in &entries {
                                if let Some(next) = saved_dance_row(
                                    ui,
                                    entry,
                                    selected_id.as_deref(),
                                    &folders,
                                    animate,
                                    false,
                                ) {
                                    action = Some(next);
                                }
                            }
                            ui.separator();
                            if let Some(dance) = &preview {
                                if let Some(next) = saved_dance_preview(ui, dance, false) {
                                    action = Some(next);
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new("Select a saved dance to see its card.")
                                        .color(theme::muted(ui)),
                                );
                            }
                        });
                });
            }
        }

        if let Some(action) = action {
            self.apply_library_action(action);
        }
    }

    pub(in crate::app) fn tune_library_ui(&mut self, ui: &mut egui::Ui) {
        let compact = compact(ui);
        apply_phone_spacing(ui, compact);
        let mut open_download = false;
        if compact {
            ui.heading("Select tune");
            let width = ui.available_width();
            if ui
                .add_sized(
                    [width, theme::TOUCH_TARGET],
                    egui::Button::new("Download tune"),
                )
                .on_hover_text("Search ABC Session Tunes and add a tune to your saved tunes")
                .clicked()
            {
                open_download = true;
            }
        } else {
            ui.horizontal(|ui| {
                ui.heading("Select tune");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("Download tune")
                        .on_hover_text(
                            "Search ABC Session Tunes and add a tune to your saved tunes",
                        )
                        .clicked()
                    {
                        open_download = true;
                    }
                });
            });
        }
        if open_download {
            self.picker.tune_preview = None;
            self.picker.tune_selected = None;
            self.tune_sheet = None;
            self.screen = Screen::TuneDownload;
        }
        ui.label(
            egui::RichText::new(
                "Your saved tunes, available offline. Search them here, or use Download tune \
                 to fetch another tune from ABC Session Tunes.",
            )
            .color(theme::muted(ui)),
        );
        if self.prefs.tune_rotation.enabled {
            let count = self.rotation_tunes().len();
            ui.label(
                egui::RichText::new(format!(
                    "Tune set: {count} saved tunes. Use Add to set, then turn on Cycle tunes in the session controls."
                ))
                .small()
                .color(theme::muted(ui)),
            );
        }
        ui.add_space(4.0);

        let shown = self
            .tune_library
            .search(&self.tune_library_query, &self.tune_library_folder)
            .len();
        let count_label = format!(
            "{shown} of {} shown · {}",
            self.tune_library.len(),
            self.tune_library_folder.label()
        );
        if compact {
            ui.label(
                egui::RichText::new("Search saved")
                    .small()
                    .color(theme::muted(ui)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.tune_library_query)
                    .hint_text("title, composer, key, meter, or genre")
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal_wrapped(|ui| {
                if !self.tune_library_query.is_empty() && ui.button("Clear").clicked() {
                    self.tune_library_query.clear();
                }
                ui.label(
                    egui::RichText::new(count_label)
                        .small()
                        .color(theme::muted(ui)),
                );
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label("Search saved");
                ui.add(
                    egui::TextEdit::singleline(&mut self.tune_library_query)
                        .hint_text("title, composer, key, meter, or genre")
                        .desired_width(340.0),
                );
                if !self.tune_library_query.is_empty() && ui.small_button("Clear").clicked() {
                    self.tune_library_query.clear();
                }
                ui.label(
                    egui::RichText::new(count_label)
                        .small()
                        .color(theme::muted(ui)),
                );
            });
        }
        if let Some(status) = &self.library_status {
            ui.label(egui::RichText::new(status).small().color(theme::muted(ui)));
        }
        ui.add_space(4.0);

        let folders = self.tune_library.folders.clone();
        let counts: Vec<usize> = folders
            .iter()
            .map(|folder| {
                self.tune_library
                    .count(&FolderFilter::Folder(folder.clone()))
            })
            .collect();
        let all_count = self.tune_library.count(&FolderFilter::All);
        let unfiled_count = self.tune_library.count(&FolderFilter::Unfiled);
        let wide = ui.available_width() >= 760.0;
        let mut command = None;

        if wide {
            // The folder panel and both saved-list columns share one height so
            // they end at the same bottom edge as the window.
            let pane_height = remaining_pane_height(ui.available_height(), 0.0);
            ui.horizontal_top(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("tune-folder-panel")
                    .max_height(pane_height)
                    // Shrink horizontally to the 230-point panel so the list
                    // and preview keep the rest of the row; fill vertically so
                    // the panel ends at the same bottom edge as the columns.
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(230.0);
                            command = folder_panel_ui(
                                ui,
                                FolderPanel {
                                    all_label: "All tunes",
                                    folders: &folders,
                                    counts: &counts,
                                    all_count,
                                    unfiled_count,
                                    filter: &mut self.tune_library_folder,
                                    new_folder: &mut self.new_tune_folder,
                                    renaming: &mut self.renaming_tune_folder,
                                    rename_text: &mut self.folder_rename_text,
                                },
                            );
                        });
                    });
                ui.separator();
                self.tune_library_body(ui, false);
            });
        } else {
            egui::ScrollArea::vertical()
                .id_salt("tune-library-narrow-scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    command = egui::CollapsingHeader::new(format!(
                        "Folders · {}",
                        self.tune_library_folder.label()
                    ))
                    .default_open(false)
                    .show(ui, |ui| {
                        folder_panel_ui(
                            ui,
                            FolderPanel {
                                all_label: "All tunes",
                                folders: &folders,
                                counts: &counts,
                                all_count,
                                unfiled_count,
                                filter: &mut self.tune_library_folder,
                                new_folder: &mut self.new_tune_folder,
                                renaming: &mut self.renaming_tune_folder,
                                rename_text: &mut self.folder_rename_text,
                            },
                        )
                    })
                    .body_returned
                    .flatten();
                    ui.separator();
                    self.tune_library_body(ui, true);
                });
        }

        if let Some(command) = command {
            self.apply_tune_folder_command(command);
        }
    }

    fn tune_library_body(&mut self, ui: &mut egui::Ui, compact: bool) {
        let animate = self.prefs.animations_enabled;
        let mut entries: Vec<contra_core::LibraryEntry<Tune>> = self
            .tune_library
            .search(&self.tune_library_query, &self.tune_library_folder)
            .into_iter()
            .cloned()
            .collect();
        entries.sort_by_key(|entry| entry.item.title.to_lowercase());
        let folders = self.tune_library.folders.clone();
        let selected_id = self
            .picker
            .tune_preview
            .as_ref()
            .map(|tune| tune.source.id.clone())
            .filter(|id| self.tune_library.contains("abcsessiontunes", id));
        let preview = selected_id.as_deref().and_then(|id| {
            entries
                .iter()
                .find(|entry| entry.item.source.id == id)
                .map(|entry| entry.item.clone())
        });
        let empty_message = if self.tune_library.is_empty() {
            "No saved tunes yet. Use Download tune to fetch one from ABC Session Tunes."
        } else if entries.is_empty() {
            "No saved tunes match this search and folder."
        } else {
            ""
        };
        let mut action = None;

        if compact {
            if !empty_message.is_empty() {
                ui.label(egui::RichText::new(empty_message).color(theme::muted(ui)));
            }
            for entry in &entries {
                if let Some(next) = saved_tune_row(
                    ui,
                    entry,
                    selected_id.as_deref(),
                    &folders,
                    animate,
                    self.tune_in_rotation(&entry.item),
                    true,
                ) {
                    action = Some(next);
                }
            }
            ui.separator();
            if let Some(tune) = &preview {
                if let Some(next) = self.saved_tune_preview(ui, tune, true) {
                    action = Some(next);
                }
            } else {
                ui.label(
                    egui::RichText::new("Select a saved tune to see its sheet music.")
                        .color(theme::muted(ui)),
                );
            }
        } else {
            // The saved-library page does not scroll as a whole, so the panes
            // take every point left below the search and status rows.
            let height = remaining_pane_height(ui.available_height(), 0.0);
            if ui.available_width() >= 560.0 {
                ui.columns(2, |columns| {
                    egui::ScrollArea::vertical()
                        .id_salt("saved-tune-list")
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(&mut columns[0], |ui| {
                            if !empty_message.is_empty() {
                                ui.label(
                                    egui::RichText::new(empty_message).color(theme::muted(ui)),
                                );
                            }
                            for entry in &entries {
                                if let Some(next) = saved_tune_row(
                                    ui,
                                    entry,
                                    selected_id.as_deref(),
                                    &folders,
                                    animate,
                                    self.tune_in_rotation(&entry.item),
                                    false,
                                ) {
                                    action = Some(next);
                                }
                            }
                        });
                    egui::ScrollArea::vertical()
                        .id_salt("saved-tune-preview")
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(&mut columns[1], |ui| {
                            if let Some(tune) = &preview {
                                if let Some(next) = self.saved_tune_preview(ui, tune, false) {
                                    action = Some(next);
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new(
                                        "Select a saved tune to see its sheet music.",
                                    )
                                    .color(theme::muted(ui)),
                                );
                            }
                        });
                });
            } else {
                // Too narrow for two readable columns: one scrolling pane
                // holds the list and preview and still ends at the same bottom
                // edge as the folder panel. The pane is vertical even though
                // its parent row is horizontal, or the card and preview would
                // flow side by side inside the scroll content.
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("saved-tune-list-narrow")
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if !empty_message.is_empty() {
                                ui.label(
                                    egui::RichText::new(empty_message).color(theme::muted(ui)),
                                );
                            }
                            for entry in &entries {
                                if let Some(next) = saved_tune_row(
                                    ui,
                                    entry,
                                    selected_id.as_deref(),
                                    &folders,
                                    animate,
                                    self.tune_in_rotation(&entry.item),
                                    false,
                                ) {
                                    action = Some(next);
                                }
                            }
                            ui.separator();
                            if let Some(tune) = &preview {
                                if let Some(next) = self.saved_tune_preview(ui, tune, false) {
                                    action = Some(next);
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new(
                                        "Select a saved tune to see its sheet music.",
                                    )
                                    .color(theme::muted(ui)),
                                );
                            }
                        });
                });
            }
        }

        if let Some(action) = action {
            self.apply_library_action(action);
        }
    }

    /// Preview a tune that came from the local library, including its sheet
    /// music, and offer the library actions.
    fn saved_tune_preview(
        &mut self,
        ui: &mut egui::Ui,
        tune: &Tune,
        compact: bool,
    ) -> Option<LibraryAction> {
        let mut action = None;
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
        let in_rotation = self.tune_in_rotation(tune);
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
                    egui::Button::new("Use this tune").wrap(),
                )
                .clicked()
            {
                action = Some(LibraryAction::UseTune(tune.clone()));
            }
            if ui
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
                action = Some(LibraryAction::ToggleRotation(tune.clone()));
            }
            if ui
                .add_sized(
                    [width, theme::TOUCH_TARGET],
                    egui::Button::new("Remove from saved tunes").wrap(),
                )
                .clicked()
            {
                action = Some(LibraryAction::RemoveTune(tune.source.id.clone()));
            }
        });
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
        ui.add_space(6.0);
        action
    }

    fn apply_library_action(&mut self, action: LibraryAction) {
        match action {
            LibraryAction::PreviewDance(dance) => {
                self.picker.dance_preview = Some(dance);
                self.picker.dance_selected = None;
            }
            LibraryAction::UseDance(dance) => self.use_dance(dance),
            LibraryAction::RemoveDance(id) => self.remove_saved_dance(&id),
            LibraryAction::MoveDance { id, folder } => self.move_saved_dance(&id, folder),
            LibraryAction::PreviewTune(tune) => {
                if self
                    .picker
                    .tune_preview
                    .as_ref()
                    .is_none_or(|current| current.source != tune.source)
                {
                    self.tune_sheet = None;
                }
                self.picker.tune_preview = Some(tune);
                self.picker.tune_selected = None;
            }
            LibraryAction::UseTune(tune) => self.use_tune(tune),
            LibraryAction::RemoveTune(id) => self.remove_saved_tune(&id),
            LibraryAction::ToggleRotation(tune) => self.toggle_rotation_tune(&tune),
            LibraryAction::MoveTune { id, folder } => self.move_saved_tune(&id, folder),
        }
    }

    fn apply_dance_folder_command(&mut self, command: FolderCommand) {
        match apply_folder_command(&mut self.dance_library, command) {
            Ok(FolderOutcome::Created(name)) => {
                self.new_dance_folder.clear();
                self.save_dance_library();
                self.library_status = Some(format!("Created folder \"{name}\"."));
            }
            Ok(FolderOutcome::Renamed(new)) => {
                self.renaming_dance_folder = None;
                self.dance_library_folder = FolderFilter::Folder(new.clone());
                self.save_dance_library();
                self.library_status = Some(format!("Renamed the folder to \"{new}\"."));
            }
            Ok(FolderOutcome::Deleted(name)) => {
                if self.dance_library_folder == FolderFilter::Folder(name.clone()) {
                    self.dance_library_folder = FolderFilter::Unfiled;
                }
                self.renaming_dance_folder = None;
                self.save_dance_library();
                self.library_status = Some(format!(
                    "Deleted folder \"{name}\". Its dances are now unfiled."
                ));
            }
            Err(error) => self.library_status = Some(error.to_string()),
        }
    }

    fn apply_tune_folder_command(&mut self, command: FolderCommand) {
        match apply_folder_command(&mut self.tune_library, command) {
            Ok(FolderOutcome::Created(name)) => {
                self.new_tune_folder.clear();
                self.save_tune_library();
                self.library_status = Some(format!("Created folder \"{name}\"."));
            }
            Ok(FolderOutcome::Renamed(new)) => {
                self.renaming_tune_folder = None;
                self.tune_library_folder = FolderFilter::Folder(new.clone());
                self.save_tune_library();
                self.library_status = Some(format!("Renamed the folder to \"{new}\"."));
            }
            Ok(FolderOutcome::Deleted(name)) => {
                if self.tune_library_folder == FolderFilter::Folder(name.clone()) {
                    self.tune_library_folder = FolderFilter::Unfiled;
                }
                self.renaming_tune_folder = None;
                self.save_tune_library();
                self.library_status = Some(format!(
                    "Deleted folder \"{name}\". Its tunes are now unfiled."
                ));
            }
            Err(error) => self.library_status = Some(error.to_string()),
        }
    }
}

/// Shared folder sidebar for the saved-dance and saved-tune pickers.
///
/// It returns nothing and mutates only `filter`; the callers apply folder
/// commands after the list is drawn so the library is never borrowed twice.
fn folder_panel_ui(ui: &mut egui::Ui, panel: FolderPanel<'_>) -> Option<FolderCommand> {
    let mut command = None;
    let selected_folder = match &*panel.filter {
        FolderFilter::Folder(name) => Some(name.clone()),
        _ => None,
    };

    ui.label(egui::RichText::new("Folders").strong());
    if block_selectable(
        ui,
        *panel.filter == FolderFilter::All,
        format!("{} ({})", panel.all_label, panel.all_count),
    )
    .clicked()
    {
        *panel.filter = FolderFilter::All;
        *panel.renaming = None;
    }
    if block_selectable(
        ui,
        *panel.filter == FolderFilter::Unfiled,
        format!("Unfiled ({})", panel.unfiled_count),
    )
    .clicked()
    {
        *panel.filter = FolderFilter::Unfiled;
        *panel.renaming = None;
    }
    for (folder, count) in panel.folders.iter().zip(panel.counts.iter()) {
        let is_selected = matches!(&*panel.filter, FolderFilter::Folder(name) if name == folder);
        if block_selectable(ui, is_selected, format!("{folder} ({count})"))
            .on_hover_text("Show only this folder")
            .clicked()
        {
            *panel.filter = FolderFilter::Folder(folder.clone());
            *panel.renaming = None;
        }
    }

    ui.separator();
    if let Some(folder) = selected_folder {
        if panel.renaming.as_deref() == Some(folder.as_str()) {
            ui.label(
                egui::RichText::new("New folder name")
                    .small()
                    .color(theme::muted(ui)),
            );
            let response =
                ui.add(egui::TextEdit::singleline(panel.rename_text).desired_width(f32::INFINITY));
            let submit = submitted(ui, &response);
            ui.vertical(|ui| {
                if block_button(ui, "Save name").clicked() || submit {
                    command = Some(FolderCommand::Rename {
                        old: folder.clone(),
                        new: panel.rename_text.clone(),
                    });
                }
                if block_button(ui, "Cancel").clicked() {
                    *panel.renaming = None;
                }
            });
        } else {
            ui.vertical(|ui| {
                if block_button(ui, "Rename folder")
                    .on_hover_text("Rename this folder without moving its items out")
                    .clicked()
                {
                    *panel.renaming = Some(folder.clone());
                    *panel.rename_text = folder.clone();
                }
                if block_button(ui, "Delete folder")
                    .on_hover_text("Delete this folder; its items become unfiled")
                    .clicked()
                {
                    command = Some(FolderCommand::Delete(folder.clone()));
                }
            });
            ui.label(
                egui::RichText::new("Deleting a folder keeps its items; they become unfiled.")
                    .small()
                    .color(theme::muted(ui)),
            );
        }
        ui.separator();
    }

    ui.label(
        egui::RichText::new("New folder")
            .small()
            .color(theme::muted(ui)),
    );
    let response = ui.add(
        egui::TextEdit::singleline(panel.new_folder)
            .hint_text("folder name")
            .desired_width(f32::INFINITY),
    );
    let submit = submitted(ui, &response);
    if block_button(ui, "Create folder")
        .on_hover_text("Create folder (Enter)")
        .clicked()
        || submit
    {
        command = Some(FolderCommand::Create(panel.new_folder.clone()));
    }

    command
}

fn apply_folder_command<T: contra_core::LibraryItem>(
    library: &mut Library<T>,
    command: FolderCommand,
) -> Result<FolderOutcome, contra_core::FolderError> {
    match command {
        FolderCommand::Create(name) => {
            library.create_folder(&name)?;
            Ok(FolderOutcome::Created(name.trim().to_string()))
        }
        FolderCommand::Rename { old, new } => {
            let new = new.trim().to_string();
            library.rename_folder(&old, &new)?;
            Ok(FolderOutcome::Renamed(new))
        }
        FolderCommand::Delete(name) => {
            library.delete_folder(&name);
            Ok(FolderOutcome::Deleted(name))
        }
    }
}

fn saved_dance_row(
    ui: &mut egui::Ui,
    entry: &contra_core::LibraryEntry<Dance>,
    selected_id: Option<&str>,
    folders: &[String],
    animate: bool,
    compact: bool,
) -> Option<LibraryAction> {
    let selected = selected_id == Some(entry.item.source.id.as_str());
    let selected_t = animate_amount(
        ui.ctx(),
        ("saved-dance-selected", &entry.item.source.id),
        selected,
        animate,
    );
    let mut action = None;
    let mut prepared = egui::Frame::group(ui.style())
        .fill(lerp_color(
            theme::call_upcoming_bg(ui),
            theme::call_active_bg(ui),
            selected_t,
        ))
        .inner_margin(egui::Margin::same(8))
        .begin(ui);
    let hovered = {
        let ui = &mut prepared.content_ui;
        // The card body is the preview target. It is drawn before the action
        // buttons so a button click is never swallowed by the card body.
        let body = dance_row_body(ui, entry, compact);
        if body.clicked() {
            action = Some(LibraryAction::PreviewDance(entry.item.clone()));
        }
        // "Use" and the folder menu split the card width evenly so the row is
        // made of two large targets instead of two small inline buttons.
        let width = action_width(
            ui.available_width(),
            ui.spacing().item_spacing.x,
            2,
            compact,
        );
        ui.horizontal_wrapped(|ui| {
            if row_button(ui, width, "Use").clicked() {
                action = Some(LibraryAction::UseDance(entry.item.clone()));
            }
            folder_menu(ui, width, entry, folders, &mut action);
        });
        body.hovered()
    };
    if hovered {
        prepared.frame.fill = lerp_color(
            prepared.frame.fill,
            ui.visuals().widgets.hovered.weak_bg_fill,
            0.35,
        );
    }
    prepared.end(ui);
    action
}

/// Draw the title and metadata of a saved card, and return the whole body as
/// one click target so the card previews the dance when it is clicked.
fn dance_row_body(
    ui: &mut egui::Ui,
    entry: &contra_core::LibraryEntry<Dance>,
    compact: bool,
) -> egui::Response {
    let body = ui
        .vertical(|ui| {
            ui.set_width(ui.available_width());
            if compact {
                ui.set_min_height(theme::TOUCH_TARGET);
            }
            ui.label(egui::RichText::new(&entry.item.title).strong());
            ui.label(
                egui::RichText::new(format!(
                    "{} · {}",
                    empty_label(&entry.item.choreographer, "unknown"),
                    empty_label(&entry.item.formation, "unknown")
                ))
                .small()
                .color(theme::muted(ui)),
            );
        })
        .response;
    let response = ui
        .interact(
            body.rect,
            ui.scope_id()
                .with(("preview-dance", entry.item.source.id.as_str())),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Preview this dance");
    let label = format!("Preview {}", entry.item.title);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    response
}

fn folder_menu(
    ui: &mut egui::Ui,
    width: f32,
    entry: &contra_core::LibraryEntry<Dance>,
    folders: &[String],
    action: &mut Option<LibraryAction>,
) {
    let folder_label = entry
        .folder
        .clone()
        .unwrap_or_else(|| "Unfiled".to_string());
    row_menu_button(ui, width, format!("Folder: {folder_label}"), |ui| {
        if ui
            .selectable_label(entry.folder.is_none(), "Unfiled")
            .clicked()
        {
            *action = Some(LibraryAction::MoveDance {
                id: entry.item.source.id.clone(),
                folder: None,
            });
            ui.close();
        }
        for folder in folders {
            let current = entry.folder.as_deref() == Some(folder.as_str());
            if ui.selectable_label(current, folder).clicked() {
                *action = Some(LibraryAction::MoveDance {
                    id: entry.item.source.id.clone(),
                    folder: Some(folder.clone()),
                });
                ui.close();
            }
        }
    });
}

fn saved_dance_preview(ui: &mut egui::Ui, dance: &Dance, compact: bool) -> Option<LibraryAction> {
    let mut action = None;
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
    ui.separator();
    for figure in &dance.figures {
        ui.label(format!(
            "{}  {:.0}b  {}",
            figure.phrase.as_deref().unwrap_or(""),
            figure.beats,
            figure.text
        ));
    }
    ui.add_space(6.0);
    let width = action_width(
        ui.available_width(),
        ui.spacing().item_spacing.x,
        2,
        compact,
    );
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_sized(
                [width, theme::TOUCH_TARGET],
                egui::Button::new("Use this dance").wrap(),
            )
            .clicked()
        {
            action = Some(LibraryAction::UseDance(dance.clone()));
        }
        if ui
            .add_sized(
                [width, theme::TOUCH_TARGET],
                egui::Button::new("Remove from saved dances").wrap(),
            )
            .clicked()
        {
            action = Some(LibraryAction::RemoveDance(dance.source.id.clone()));
        }
    });
    action
}

fn saved_tune_row(
    ui: &mut egui::Ui,
    entry: &contra_core::LibraryEntry<Tune>,
    selected_id: Option<&str>,
    folders: &[String],
    animate: bool,
    in_rotation: bool,
    compact: bool,
) -> Option<LibraryAction> {
    let selected = selected_id == Some(entry.item.source.id.as_str());
    let selected_t = animate_amount(
        ui.ctx(),
        ("saved-tune-selected", &entry.item.source.id),
        selected,
        animate,
    );
    let mut action = None;
    let mut prepared = egui::Frame::group(ui.style())
        .fill(lerp_color(
            theme::call_upcoming_bg(ui),
            theme::call_active_bg(ui),
            selected_t,
        ))
        .inner_margin(egui::Margin::same(8))
        .begin(ui);
    let hovered = {
        let ui = &mut prepared.content_ui;
        // The card body is the preview target. It is drawn before the action
        // buttons so a button click is never swallowed by the card body.
        let body = tune_row_body(ui, entry, compact);
        if body.clicked() {
            action = Some(LibraryAction::PreviewTune(entry.item.clone()));
        }
        // "Use", "Add to set", and the folder menu split the card width evenly
        // so the row is made of three large targets, not small inline buttons.
        let width = action_width(
            ui.available_width(),
            ui.spacing().item_spacing.x,
            3,
            compact,
        );
        ui.horizontal_wrapped(|ui| {
            if row_button(ui, width, "Use").clicked() {
                action = Some(LibraryAction::UseTune(entry.item.clone()));
            }
            let set_label = if in_rotation { "In set" } else { "Add to set" };
            if row_button(ui, width, set_label)
                .on_hover_text(if in_rotation {
                    "Remove this tune from the set"
                } else {
                    "Include this tune when Cycle tunes is on"
                })
                .clicked()
            {
                action = Some(LibraryAction::ToggleRotation(entry.item.clone()));
            }
            tune_folder_menu(ui, width, entry, folders, &mut action);
        });
        body.hovered()
    };
    if hovered {
        prepared.frame.fill = lerp_color(
            prepared.frame.fill,
            ui.visuals().widgets.hovered.weak_bg_fill,
            0.35,
        );
    }
    prepared.end(ui);
    action
}

/// Draw the title and metadata of a saved tune, and return the whole body as
/// one click target so the card previews the tune when it is clicked.
fn tune_row_body(
    ui: &mut egui::Ui,
    entry: &contra_core::LibraryEntry<Tune>,
    compact: bool,
) -> egui::Response {
    let body = ui
        .vertical(|ui| {
            ui.set_width(ui.available_width());
            if compact {
                ui.set_min_height(theme::TOUCH_TARGET);
            }
            ui.label(egui::RichText::new(&entry.item.title).strong());
            ui.label(
                egui::RichText::new(format!(
                    "{} · {} · {}",
                    empty_label(&entry.item.composer, "unknown"),
                    entry.item.display_key(),
                    entry.item.display_rhythm()
                ))
                .small()
                .color(theme::muted(ui)),
            );
        })
        .response;
    let response = ui
        .interact(
            body.rect,
            ui.scope_id()
                .with(("preview-tune", entry.item.source.id.as_str())),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Preview this tune");
    let label = format!("Preview {}", entry.item.title);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    response
}

fn tune_folder_menu(
    ui: &mut egui::Ui,
    width: f32,
    entry: &contra_core::LibraryEntry<Tune>,
    folders: &[String],
    action: &mut Option<LibraryAction>,
) {
    let folder_label = entry
        .folder
        .clone()
        .unwrap_or_else(|| "Unfiled".to_string());
    row_menu_button(ui, width, format!("Folder: {folder_label}"), |ui| {
        if ui
            .selectable_label(entry.folder.is_none(), "Unfiled")
            .clicked()
        {
            *action = Some(LibraryAction::MoveTune {
                id: entry.item.source.id.clone(),
                folder: None,
            });
            ui.close();
        }
        for folder in folders {
            let current = entry.folder.as_deref() == Some(folder.as_str());
            if ui.selectable_label(current, folder).clicked() {
                *action = Some(LibraryAction::MoveTune {
                    id: entry.item.source.id.clone(),
                    folder: Some(folder.clone()),
                });
                ui.close();
            }
        }
    });
}
