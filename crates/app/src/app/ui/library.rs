use super::gesture;
use super::*;
use crate::app::*;
use eframe::egui;

/// The folder indexes and pointer position of a live folder drag.
#[derive(Clone, Copy, Debug)]
struct FolderDrag {
    /// Index of the row being moved.
    from: usize,
    /// Insertion slot under the pointer, in `0..=folders.len()`.
    slot: usize,
    /// True once movement may reorder. A touch drag starts only after the
    /// row has been held; a mouse drag starts immediately.
    active: bool,
}

/// What the item context menu asked for.
enum RowMenuChoice {
    Move(Option<String>),
    Delete,
}

impl ContraApp {
    pub(in crate::app) fn dance_library_ui(&mut self, ui: &mut egui::Ui) {
        let compact = compact(ui);
        apply_phone_spacing(ui, compact);
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
        let chip = group_label(
            &self.dance_library_folder,
            "All dances",
            self.dance_library.count(&self.dance_library_folder),
        );
        let download_hint = format!(
            "Search {} and add a card to your saved dances",
            self.dance_source().display_name()
        );
        let mut open_download = false;

        if compact {
            ui.heading("Select dance");
            ui.horizontal_wrapped(|ui| {
                group_chip(ui, &chip);
            });
            let width = ui.available_width();
            if ui
                .add_sized(
                    [width, theme::TOUCH_TARGET],
                    egui::Button::new("Download dance"),
                )
                .on_hover_text(&download_hint)
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
                        .on_hover_text(&download_hint)
                        .clicked()
                    {
                        open_download = true;
                    }
                    group_chip(ui, &chip);
                });
            });
        }
        if open_download {
            self.picker.dance_preview = None;
            self.picker.dance_selected = None;
            self.screen = Screen::DanceDownload;
        }
        ui.label(
            egui::RichText::new(format!(
                "Your saved dances, available offline. Search them here, or use Download dance \
                 to fetch another card from {}.",
                self.dance_source().display_name()
            ))
            .color(theme::muted(ui)),
        );
        ui.add_space(4.0);

        let shown = self
            .dance_library
            .search(&self.dance_library_query, &self.dance_library_folder)
            .len();
        let count_label = format!("{shown} of {} shown", self.dance_library.len());
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

        let wide = ui.available_width() >= 760.0;
        let mut command = None;

        if wide {
            // The folder panel, list, and preview share one height so they end
            // at the same bottom edge as the window. The folder width is a
            // stored, drag-resizable pane; the list/preview split lives in the
            // body so both screens can resize theirs independently.
            let pane_height = remaining_pane_height(ui.available_height(), 0.0);
            let width = ui.available_width();
            let sidebar = clamp_pane_width(self.panes.sidebar, width);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(sidebar, pane_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(sidebar);
                        ui.set_max_height(pane_height);
                        egui::ScrollArea::vertical()
                            .id_salt("dance-folder-panel")
                            .max_height(pane_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
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
                                        drag_id: "dance-folder-drag",
                                        tune_set: None,
                                    },
                                );
                            });
                    },
                );
                let splitter = pane_splitter(
                    ui,
                    "dance-sidebar-splitter",
                    pane_height,
                    "Resize the dance folder list",
                );
                if splitter.dragged() {
                    self.panes.sidebar =
                        clamp_pane_width(self.panes.sidebar + splitter.drag_delta().x, width);
                }
                self.dance_library_body(ui, false, pane_height);
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
                    .id_salt("dance-folders")
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
                                drag_id: "dance-folder-drag",
                                tune_set: None,
                            },
                        )
                    })
                    .body_returned
                    .flatten();
                    ui.separator();
                    self.dance_library_body(ui, true, 0.0);
                });
        }

        if let Some(command) = command {
            self.apply_dance_folder_command(command);
        }
    }

    fn dance_library_body(&mut self, ui: &mut egui::Ui, compact: bool, pane_height: f32) {
        let animate = self.prefs.animations_enabled;
        let mut entries: Vec<contra_core::LibraryEntry<Dance>> = self
            .dance_library
            .search(&self.dance_library_query, &self.dance_library_folder)
            .into_iter()
            .cloned()
            .collect();
        entries.sort_by_key(|entry| entry.item.title.to_lowercase());
        let folders = self.dance_library.folders.clone();
        let selected = self
            .picker
            .dance_preview
            .as_ref()
            .filter(|dance| {
                self.dance_library
                    .contains(&dance.source.provider, &dance.source.id)
            })
            .map(|dance| (dance.source.provider.clone(), dance.source.id.clone()));
        let preview = selected.as_ref().and_then(|(provider, id)| {
            entries
                .iter()
                .find(|entry| {
                    entry.item.source.provider == *provider && entry.item.source.id == *id
                })
                .map(|entry| entry.item.clone())
        });
        let preview_folder = preview.as_ref().and_then(|dance| {
            self.dance_library
                .get(&dance.source.provider, &dance.source.id)
                .and_then(|entry| entry.folder.clone())
        });
        let selected_ref = selected
            .as_ref()
            .map(|(provider, id)| (provider.as_str(), id.as_str()));
        let empty_message = if self.dance_library.is_empty() {
            format!(
                "No saved dances yet. Use Download dance to fetch one from {}.",
                self.dance_source().display_name()
            )
        } else if entries.is_empty() {
            "No saved dances match this search and folder.".to_string()
        } else {
            String::new()
        };
        let mut action = None;

        if compact {
            if !empty_message.is_empty() {
                ui.label(egui::RichText::new(&empty_message).color(theme::muted(ui)));
            }
            for entry in &entries {
                if let Some(next) =
                    saved_dance_row(ui, entry, selected_ref, &folders, animate, true)
                {
                    action = Some(next);
                }
            }
            ui.separator();
            if let Some(dance) = &preview {
                if let Some(next) =
                    saved_dance_preview(ui, dance, &folders, preview_folder.as_deref(), true)
                {
                    action = Some(next);
                }
            } else {
                ui.label(
                    egui::RichText::new("Select a saved dance to see its card.")
                        .color(theme::muted(ui)),
                );
            }
        } else {
            let height = if pane_height > 0.0 {
                pane_height
            } else {
                remaining_pane_height(ui.available_height(), 0.0)
            };
            if ui.available_width() >= 560.0 {
                let available = ui.available_width();
                let list_width = clamp_pane_width(self.panes.library_list, available);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(list_width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(list_width);
                            ui.set_min_height(height);
                            ui.set_max_height(height);
                            egui::ScrollArea::vertical()
                                .id_salt("saved-dance-list")
                                .max_height(height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    if !empty_message.is_empty() {
                                        ui.label(
                                            egui::RichText::new(&empty_message)
                                                .color(theme::muted(ui)),
                                        );
                                    }
                                    for entry in &entries {
                                        if let Some(next) = saved_dance_row(
                                            ui,
                                            entry,
                                            selected_ref,
                                            &folders,
                                            animate,
                                            false,
                                        ) {
                                            action = Some(next);
                                        }
                                    }
                                });
                        },
                    );
                    let splitter = pane_splitter(
                        ui,
                        "saved-dance-splitter",
                        height,
                        "Resize the saved dance preview",
                    );
                    if splitter.dragged() {
                        self.panes.library_list = clamp_pane_width(
                            self.panes.library_list + splitter.drag_delta().x,
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
                                .id_salt("saved-dance-preview")
                                .max_height(height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    if let Some(dance) = &preview {
                                        if let Some(next) = saved_dance_preview(
                                            ui,
                                            dance,
                                            &folders,
                                            preview_folder.as_deref(),
                                            false,
                                        ) {
                                            action = Some(next);
                                        }
                                    } else {
                                        ui.label(
                                            egui::RichText::new(
                                                "Select a saved dance to see its card.",
                                            )
                                            .color(theme::muted(ui)),
                                        );
                                    }
                                });
                        },
                    );
                });
            } else {
                // Too narrow for two readable columns: one scrolling pane
                // holds the list and preview and still ends at the same bottom
                // edge as the folder panel.
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("saved-dance-list-narrow")
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if !empty_message.is_empty() {
                                ui.label(
                                    egui::RichText::new(&empty_message).color(theme::muted(ui)),
                                );
                            }
                            for entry in &entries {
                                if let Some(next) = saved_dance_row(
                                    ui,
                                    entry,
                                    selected_ref,
                                    &folders,
                                    animate,
                                    false,
                                ) {
                                    action = Some(next);
                                }
                            }
                            ui.separator();
                            if let Some(dance) = &preview {
                                if let Some(next) = saved_dance_preview(
                                    ui,
                                    dance,
                                    &folders,
                                    preview_folder.as_deref(),
                                    false,
                                ) {
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
        let chip = group_label(
            &self.tune_library_folder,
            "All tunes",
            self.tune_library.count(&self.tune_library_folder),
        );
        let mut open_download = false;

        if compact {
            ui.heading("Select tune");
            ui.horizontal_wrapped(|ui| {
                group_chip(ui, &chip);
            });
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
                    group_chip(ui, &chip);
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
        let source = &self.prefs.tune_rotation.source;
        let set_count = self.rotation_tunes().len();
        let cycle = if self.prefs.tune_rotation.enabled {
            "Cycle tunes is on."
        } else {
            "Turn on Cycle tunes in the session controls to play the set."
        };
        ui.label(
            egui::RichText::new(format!(
                "Tune set: {} · {set_count} tunes. {cycle}",
                source.label()
            ))
            .small()
            .color(theme::muted(ui)),
        );
        if let Some(notice) = self.tune_set_notice() {
            ui.label(
                egui::RichText::new(notice)
                    .small()
                    .color(theme::warning(ui)),
            );
        }
        ui.add_space(4.0);

        let shown = self
            .tune_library
            .search(&self.tune_library_query, &self.tune_library_folder)
            .len();
        let count_label = format!("{shown} of {} shown", self.tune_library.len());
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

        let wide = ui.available_width() >= 760.0;
        let mut command = None;
        let tune_set_source = self.prefs.tune_rotation.source.clone();

        if wide {
            let pane_height = remaining_pane_height(ui.available_height(), 0.0);
            let width = ui.available_width();
            let sidebar = clamp_pane_width(self.panes.sidebar, width);
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(sidebar, pane_height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(sidebar);
                        ui.set_max_height(pane_height);
                        egui::ScrollArea::vertical()
                            .id_salt("tune-folder-panel")
                            .max_height(pane_height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
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
                                        drag_id: "tune-folder-drag",
                                        tune_set: Some(TuneSetPanel {
                                            source: &tune_set_source,
                                            count: set_count,
                                        }),
                                    },
                                );
                            });
                    },
                );
                let splitter = pane_splitter(
                    ui,
                    "tune-sidebar-splitter",
                    pane_height,
                    "Resize the tune folder list",
                );
                if splitter.dragged() {
                    self.panes.sidebar =
                        clamp_pane_width(self.panes.sidebar + splitter.drag_delta().x, width);
                }
                self.tune_library_body(ui, false, pane_height);
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
                    .id_salt("tune-folders")
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
                                drag_id: "tune-folder-drag",
                                tune_set: Some(TuneSetPanel {
                                    source: &tune_set_source,
                                    count: set_count,
                                }),
                            },
                        )
                    })
                    .body_returned
                    .flatten();
                    ui.separator();
                    self.tune_library_body(ui, true, 0.0);
                });
        }

        if let Some(command) = command {
            self.apply_tune_folder_command(command);
        }
    }

    fn tune_library_body(&mut self, ui: &mut egui::Ui, compact: bool, pane_height: f32) {
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
            "No saved tunes yet. Use Download tune to fetch one from ABC Session Tunes.".to_string()
        } else if entries.is_empty() {
            "No saved tunes match this search and folder.".to_string()
        } else {
            String::new()
        };
        let mut action = None;

        if compact {
            if !empty_message.is_empty() {
                ui.label(egui::RichText::new(&empty_message).color(theme::muted(ui)));
            }
            for entry in &entries {
                if let Some(next) =
                    saved_tune_row(ui, entry, selected_id.as_deref(), &folders, animate, true)
                {
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
            let height = if pane_height > 0.0 {
                pane_height
            } else {
                remaining_pane_height(ui.available_height(), 0.0)
            };
            if ui.available_width() >= 560.0 {
                let available = ui.available_width();
                let list_width = clamp_pane_width(self.panes.library_list, available);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(list_width, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(list_width);
                            ui.set_min_height(height);
                            ui.set_max_height(height);
                            egui::ScrollArea::vertical()
                                .id_salt("saved-tune-list")
                                .max_height(height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    if !empty_message.is_empty() {
                                        ui.label(
                                            egui::RichText::new(&empty_message)
                                                .color(theme::muted(ui)),
                                        );
                                    }
                                    for entry in &entries {
                                        if let Some(next) = saved_tune_row(
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
                        },
                    );
                    let splitter = pane_splitter(
                        ui,
                        "saved-tune-splitter",
                        height,
                        "Resize the saved tune preview",
                    );
                    if splitter.dragged() {
                        self.panes.library_list = clamp_pane_width(
                            self.panes.library_list + splitter.drag_delta().x,
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
                                .id_salt("saved-tune-preview")
                                .max_height(height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    if let Some(tune) = &preview {
                                        if let Some(next) = self.saved_tune_preview(ui, tune, false)
                                        {
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
                        },
                    );
                });
            } else {
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("saved-tune-list-narrow")
                        .max_height(height)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if !empty_message.is_empty() {
                                ui.label(
                                    egui::RichText::new(&empty_message).color(theme::muted(ui)),
                                );
                            }
                            for entry in &entries {
                                if let Some(next) = saved_tune_row(
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
        let source = &self.prefs.tune_rotation.source;
        let set_count = self.rotation_tunes().len();
        ui.label(
            egui::RichText::new(format!("Tune set: {} · {set_count} tunes", source.label()))
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
        let folder_set = matches!(
            self.prefs.tune_rotation.source,
            contra_core::TuneSetSource::Folder(_)
        );
        let folders = self.tune_library.folders.clone();
        let folder = self
            .tune_library
            .get(&tune.source.provider, &tune.source.id)
            .and_then(|entry| entry.folder.clone());
        // Use, the set toggle, Delete, and the folder menu split the width so
        // every row is a 44-point target instead of a small inline button.
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
                    egui::Button::new("Use this tune").wrap(),
                )
                .clicked()
            {
                action = Some(LibraryAction::UseTune(tune.clone()));
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
                    action = Some(LibraryAction::UseChosenTuneSet);
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
                action = Some(LibraryAction::ToggleRotation(tune.clone()));
            }
            if ui
                .add_sized(
                    [width, theme::TOUCH_TARGET],
                    egui::Button::new("Delete tune").wrap(),
                )
                .on_hover_text("Delete this saved copy; the loaded tune keeps playing")
                .clicked()
            {
                action = Some(LibraryAction::RemoveTune(tune.source.id.clone()));
            }
            folder_menu(ui, width, folder.as_deref(), &folders, &mut |folder| {
                action = Some(LibraryAction::MoveTune {
                    id: tune.source.id.clone(),
                    folder,
                });
            });
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
            LibraryAction::RemoveDance { provider, id } => self.remove_saved_dance(&provider, &id),
            LibraryAction::MoveDance {
                provider,
                id,
                folder,
            } => self.move_saved_dance(&provider, &id, folder),
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
            LibraryAction::UseChosenTuneSet => {
                self.set_tune_set_source(contra_core::TuneSetSource::List)
            }
        }
    }

    fn apply_dance_folder_command(&mut self, command: FolderCommand) {
        match command {
            FolderCommand::ResetPanes => {
                self.panes = PaneWidths::default();
                self.library_status = Some("Reset the pane widths.".to_string());
                return;
            }
            // The dance library never offers tune-set controls.
            FolderCommand::UseTuneSet(_) => return,
            _ => {}
        }
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
            Ok(FolderOutcome::Reordered(index)) => {
                self.save_dance_library();
                self.library_status = Some(format!("Moved the folder to position {}.", index + 1));
            }
            Err(error) => self.library_status = Some(error.to_string()),
        }
    }

    fn apply_tune_folder_command(&mut self, command: FolderCommand) {
        match command {
            FolderCommand::ResetPanes => {
                self.panes = PaneWidths::default();
                self.library_status = Some("Reset the pane widths.".to_string());
                return;
            }
            FolderCommand::UseTuneSet(source) => {
                self.set_tune_set_source(source);
                return;
            }
            _ => {}
        }
        // Remember whether the folder operation touches the active tune set
        // before the library is mutated.
        let active_set = self.prefs.tune_rotation.source.clone();
        let renamed_from = match &command {
            FolderCommand::Rename { old, .. } => Some(old.clone()),
            _ => None,
        };
        match apply_folder_command(&mut self.tune_library, command) {
            Ok(FolderOutcome::Created(name)) => {
                self.new_tune_folder.clear();
                self.save_tune_library();
                self.library_status = Some(format!("Created folder \"{name}\"."));
            }
            Ok(FolderOutcome::Renamed(new)) => {
                self.renaming_tune_folder = None;
                self.tune_library_folder = FolderFilter::Folder(new.clone());
                let set_followed = renamed_from.as_deref().is_some_and(|old| {
                    matches!(&active_set, contra_core::TuneSetSource::Folder(name) if name == old)
                });
                if set_followed {
                    self.prefs.tune_rotation.source =
                        contra_core::TuneSetSource::Folder(new.clone());
                    self.rotation_settings_changed();
                    self.save_prefs();
                }
                self.save_tune_library();
                self.library_status = Some(format!("Renamed the folder to \"{new}\"."));
            }
            Ok(FolderOutcome::Deleted(name)) => {
                if self.tune_library_folder == FolderFilter::Folder(name.clone()) {
                    self.tune_library_folder = FolderFilter::Unfiled;
                }
                self.renaming_tune_folder = None;
                let set_followed = active_set == contra_core::TuneSetSource::Folder(name.clone());
                if set_followed {
                    self.prefs.tune_rotation.source = contra_core::TuneSetSource::List;
                    self.rotation_settings_changed();
                    self.save_prefs();
                }
                self.save_tune_library();
                self.library_status = Some(if set_followed {
                    format!(
                        "Deleted folder \"{name}\". Its tunes are now unfiled, and the tune set \
                         is back to the chosen tunes."
                    )
                } else {
                    format!("Deleted folder \"{name}\". Its tunes are now unfiled.")
                });
            }
            Ok(FolderOutcome::Reordered(index)) => {
                self.save_tune_library();
                self.library_status = Some(format!("Moved the folder to position {}.", index + 1));
            }
            Err(error) => self.library_status = Some(error.to_string()),
        }
    }
}

/// A short label for the current grouping, shown in the list header chip.
///
/// The count line keeps the search result count; the chip names the folder
/// filter and its total so the two never repeat each other.
fn group_label(filter: &FolderFilter, all_label: &str, count: usize) -> String {
    match filter {
        FolderFilter::All => format!("{all_label} · {count}"),
        FolderFilter::Unfiled => format!("Unfiled · {count}"),
        FolderFilter::Folder(name) => format!("{name} · {count}"),
    }
}

/// One folder row: a grip, the name, the count, and the hold progress line.
///
/// The response covers the whole row so a click selects the folder and a drag
/// moves it. Rows are never shorter than a touch target.
fn folder_row_ui(
    ui: &mut egui::Ui,
    folder: &str,
    count: usize,
    selected: bool,
    dragging: bool,
    sense: egui::Sense,
) -> (egui::Response, RowPress) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), theme::TOUCH_TARGET), sense);
    let press = long_press_row(ui, &response);
    let lit = dragging || press.held;
    let fill = if lit {
        theme::call_active_bg(ui)
    } else if selected {
        ui.visuals().selection.bg_fill
    } else if response.hovered() {
        ui.visuals().widgets.hovered.weak_bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    let painter = ui.painter();
    if fill != egui::Color32::TRANSPARENT {
        painter.rect_filled(rect, 4.0, fill);
    }
    if lit {
        painter.rect_stroke(
            rect,
            4.0,
            egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
            egui::StrokeKind::Inside,
        );
    } else if response.has_focus() {
        painter.rect_stroke(
            rect,
            4.0,
            ui.visuals().selection.stroke,
            egui::StrokeKind::Inside,
        );
    }
    let text_color = if selected {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().text_color()
    };
    painter.text(
        egui::pos2(rect.right() - 8.0, rect.center().y),
        egui::Align2::RIGHT_CENTER,
        count.to_string(),
        egui::FontId::proportional(12.0),
        theme::muted(ui),
    );
    // A painted grip rather than a glyph: the bundled fonts do not all carry
    // a handle character, and a fixed drawing also lines up at every size.
    for offset in [-4.0, 0.0, 4.0] {
        painter.hline(
            (rect.left() + 9.0)..=(rect.left() + 19.0),
            rect.center().y + offset,
            egui::Stroke::new(1.5, theme::muted(ui)),
        );
    }
    let name_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 26.0, rect.top()),
        egui::pos2(rect.right() - 34.0, rect.bottom()),
    );
    painter.with_clip_rect(name_rect).text(
        egui::pos2(name_rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        folder,
        egui::FontId::proportional(14.0),
        text_color,
    );
    if press.progress > 0.0 && !press.held {
        let width = rect.width() * press.progress;
        painter.hline(
            rect.left()..=(rect.left() + width),
            rect.bottom() - 2.0,
            egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
        );
    }
    let label = format!("Folder {folder}, {count} items");
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, selected, &label));
    let response = response
        .on_hover_cursor(if dragging {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        })
        .on_hover_text("Click to show this folder · drag to reorder");
    (response, press)
}

/// Shared folder sidebar for the saved-dance and saved-tune pickers.
///
/// It returns a command and mutates only its own fields; the callers apply the
/// command after the list is drawn so the library is never borrowed twice.
fn folder_panel_ui(ui: &mut egui::Ui, panel: FolderPanel<'_>) -> Option<FolderCommand> {
    let mut command = None;
    let selected_folder = match &*panel.filter {
        FolderFilter::Folder(name) => Some(name.clone()),
        _ => None,
    };
    let drag_id = egui::Id::unique(panel.drag_id);
    let mut drag = ui.data(|data| data.get_temp::<FolderDrag>(drag_id));
    let pointer = ui.input(|input| input.pointer.interact_pos());
    let touch = ui.input(|input| input.any_touches());

    ui.label(egui::RichText::new("Folders").strong());
    ui.label(
        egui::RichText::new("Drag folders to reorder · hold on touch")
            .small()
            .color(theme::muted(ui)),
    );
    if drag.is_some_and(|drag| drag.active) {
        ui.label(
            egui::RichText::new("Move over another folder, then release to place it.")
                .small()
                .color(ui.visuals().text_color()),
        );
    }
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

    let mut centers = Vec::with_capacity(panel.folders.len());
    let mut rects = Vec::with_capacity(panel.folders.len());
    for (index, (folder, count)) in panel.folders.iter().zip(panel.counts.iter()).enumerate() {
        let is_selected = matches!(&*panel.filter, FolderFilter::Folder(name) if name == folder);
        let dragging = drag.is_some_and(|drag| drag.from == index && drag.active);
        let sense = if touch {
            egui::Sense::click()
        } else {
            egui::Sense::click_and_drag()
        };
        let (response, press) = folder_row_ui(ui, folder, *count, is_selected, dragging, sense);
        centers.push(response.rect.center().y);
        rects.push(response.rect);

        if press.fired {
            drag = Some(FolderDrag {
                from: index,
                slot: index,
                active: true,
            });
        }
        if response.drag_started() {
            drag = Some(FolderDrag {
                from: index,
                slot: index,
                active: true,
            });
        }
        // A press that lifted the row is a move, not a selection; a quick tap
        // still selects the folder, and Enter does the same from the keyboard.
        if row_activated(ui, &response) && !press.suppress_click && !press.held {
            *panel.filter = FolderFilter::Folder(folder.clone());
            *panel.renaming = None;
        }
    }

    if let Some(mut drag) = drag {
        if let Some(pos) = pointer {
            drag.slot = gesture::insertion_slot(&centers, pos.y);
        }
        if drag.active
            && let Some(last) = rects.last()
        {
            let line_y = if drag.slot == 0 {
                rects[0].top()
            } else if drag.slot >= rects.len() {
                last.bottom()
            } else {
                rects[drag.slot].top()
            };
            ui.painter().hline(
                last.left()..=last.right(),
                line_y,
                egui::Stroke::new(2.0, ui.visuals().selection.stroke.color),
            );
            // Capture the drag above the rows so the surrounding scroll area
            // cannot also scroll while a row is moving.
            let capture = rects
                .iter()
                .fold(egui::Rect::NOTHING, |rect, row| rect.union(*row));
            ui.interact(capture, drag_id.with("capture"), egui::Sense::drag());
        }

        let released = !ui.input(|input| input.pointer.primary_down());
        if released {
            if drag.active
                && let Some(to) = gesture::drop_target(drag.from, drag.slot, panel.folders.len())
            {
                command = Some(FolderCommand::Reorder {
                    from: drag.from,
                    to,
                });
            }
            ui.data_mut(|data| data.remove::<FolderDrag>(drag_id));
        } else {
            ui.ctx().request_repaint();
            ui.data_mut(|data| data.insert_temp(drag_id, drag));
        }
    }

    ui.separator();
    if let Some(folder) = selected_folder {
        let index = panel
            .folders
            .iter()
            .position(|name| name == &folder)
            .unwrap_or(0);
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
                let up = ui.add_enabled(
                    index > 0,
                    egui::Button::new("Move up")
                        .min_size(egui::vec2(ui.available_width(), theme::TOUCH_TARGET)),
                );
                if up.clicked() {
                    command = Some(FolderCommand::Reorder {
                        from: index,
                        to: index - 1,
                    });
                }
                let down = ui.add_enabled(
                    index + 1 < panel.folders.len(),
                    egui::Button::new("Move down")
                        .min_size(egui::vec2(ui.available_width(), theme::TOUCH_TARGET)),
                );
                if down.clicked() {
                    command = Some(FolderCommand::Reorder {
                        from: index,
                        to: index + 1,
                    });
                }
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

        if let Some(tune_set) = &panel.tune_set {
            ui.separator();
            ui.label(
                egui::RichText::new(format!(
                    "Tune set: {} · {} tunes",
                    tune_set.source.label(),
                    tune_set.count
                ))
                .small()
                .color(theme::muted(ui)),
            );
            let folder_is_set = matches!(
                tune_set.source,
                contra_core::TuneSetSource::Folder(name) if *name == folder
            );
            if folder_is_set {
                if block_button(ui, "Use the chosen tunes")
                    .on_hover_text("Stop following this folder and use the chosen tune list")
                    .clicked()
                {
                    command = Some(FolderCommand::UseTuneSet(contra_core::TuneSetSource::List));
                }
            } else if block_button(ui, "Use this folder as the tune set")
                .on_hover_text("Every tune in this folder joins the tune set, now and later")
                .clicked()
            {
                command = Some(FolderCommand::UseTuneSet(
                    contra_core::TuneSetSource::Folder(folder.clone()),
                ));
            }
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

    ui.add_space(4.0);
    if block_button(ui, "Reset panes")
        .on_hover_text("Restore the folder, list, and preview widths")
        .clicked()
    {
        command = Some(FolderCommand::ResetPanes);
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
        FolderCommand::Reorder { from, to } => {
            if library.move_folder(from, to) {
                Ok(FolderOutcome::Reordered(to))
            } else {
                Err(contra_core::FolderError::Missing)
            }
        }
        // Handled by the app before the library is touched.
        FolderCommand::UseTuneSet(_) | FolderCommand::ResetPanes => {
            Err(contra_core::FolderError::Missing)
        }
    }
}

/// A menu for moving one saved item into a folder, shared by the previews.
fn folder_menu(
    ui: &mut egui::Ui,
    width: f32,
    current: Option<&str>,
    folders: &[String],
    on_move: &mut dyn FnMut(Option<String>),
) {
    let label = current.unwrap_or("Unfiled");
    row_menu_button(ui, width, "Move to folder", |ui| {
        ui.label(
            egui::RichText::new(format!("Now in: {label}"))
                .small()
                .color(theme::muted(ui)),
        );
        if ui.selectable_label(current.is_none(), "Unfiled").clicked() {
            on_move(None);
            ui.close();
        }
        for folder in folders {
            let is_current = current == Some(folder.as_str());
            if ui.selectable_label(is_current, folder).clicked() {
                on_move(Some(folder.clone()));
                ui.close();
            }
        }
    });
}

/// Open the item context menu and report the chosen action.
///
/// A secondary click is handled by egui's context menu; a long touch press is
/// fed to the same popup by `open`, so both paths share one menu and one set of
/// 44-point targets.
fn row_context_menu(
    response: &egui::Response,
    folders: &[String],
    current: Option<&str>,
    delete_label: &str,
    open: bool,
) -> Option<RowMenuChoice> {
    let mut choice = None;
    let mut popup = egui::Popup::context_menu(response);
    if open {
        popup = popup.open_memory(Some(egui::SetOpenCommand::Bool(true)));
    }
    popup.show(|ui| {
        ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
        ui.label(
            egui::RichText::new(format!(
                "Add to folder · now in {}",
                current.unwrap_or("Unfiled")
            ))
            .small()
            .color(theme::muted(ui)),
        );
        let width = ui.available_width().max(160.0);
        if ui
            .add_sized(
                [width, theme::TOUCH_TARGET],
                egui::Button::selectable(current.is_none(), "Unfiled"),
            )
            .clicked()
        {
            choice = Some(RowMenuChoice::Move(None));
            ui.close();
        }
        for folder in folders {
            let is_current = current == Some(folder.as_str());
            if ui
                .add_sized(
                    [width, theme::TOUCH_TARGET],
                    egui::Button::selectable(is_current, folder),
                )
                .clicked()
            {
                choice = Some(RowMenuChoice::Move(Some(folder.clone())));
                ui.close();
            }
        }
        ui.separator();
        if ui
            .add_sized(
                [width, theme::TOUCH_TARGET],
                egui::Button::new(delete_label),
            )
            .clicked()
        {
            choice = Some(RowMenuChoice::Delete);
            ui.close();
        }
    });
    choice
}

fn saved_dance_row(
    ui: &mut egui::Ui,
    entry: &contra_core::LibraryEntry<Dance>,
    selected: Option<(&str, &str)>,
    folders: &[String],
    animate: bool,
    compact: bool,
) -> Option<LibraryAction> {
    let selected = selected
        == Some((
            entry.item.source.provider.as_str(),
            entry.item.source.id.as_str(),
        ));
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
    let (hovered, focused) = {
        let ui = &mut prepared.content_ui;
        // The card body is the only click target: a saved row previews the
        // card, and every edit lives in the preview or the context menu.
        let body = dance_row_body(ui, entry, compact);
        let press = long_press_row(ui, &body);
        if let Some(choice) = row_context_menu(
            &body,
            folders,
            entry.folder.as_deref(),
            "Delete dance",
            press.suppress_click,
        ) {
            action = Some(match choice {
                RowMenuChoice::Move(folder) => LibraryAction::MoveDance {
                    provider: entry.item.source.provider.clone(),
                    id: entry.item.source.id.clone(),
                    folder,
                },
                RowMenuChoice::Delete => LibraryAction::RemoveDance {
                    provider: entry.item.source.provider.clone(),
                    id: entry.item.source.id.clone(),
                },
            });
        }
        if row_activated(ui, &body) && !press.suppress_click && !press.held {
            action = Some(LibraryAction::PreviewDance(entry.item.clone()));
        }
        (body.hovered(), body.has_focus())
    };
    if focused {
        prepared.frame.stroke = ui.visuals().selection.stroke;
    }
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
                    "{} · {} · {}",
                    empty_label(&entry.item.choreographer, "unknown"),
                    empty_label(&entry.item.formation, "unknown"),
                    empty_label(entry.folder.as_deref().unwrap_or("Unfiled"), "Unfiled"),
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
        .on_hover_text("Preview this dance · right-click for folder and delete");
    let label = format!("Preview {}", entry.item.title);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    response
}

/// Preview a saved dance and offer the library actions.
///
/// The saved list itself has no edit buttons: the preview owns Use,
/// Delete, and the folder menu, and the row's context menu mirrors the last
/// two for a quick edit.
fn saved_dance_preview(
    ui: &mut egui::Ui,
    dance: &Dance,
    folders: &[String],
    folder: Option<&str>,
    compact: bool,
) -> Option<LibraryAction> {
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
    ui.hyperlink_to(
        format!("Open on {}", source_label(&dance.source.provider)),
        &dance.source.url,
    );
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
        3,
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
                egui::Button::new("Delete dance").wrap(),
            )
            .on_hover_text("Delete this saved copy; the loaded dance is unchanged")
            .clicked()
        {
            action = Some(LibraryAction::RemoveDance {
                provider: dance.source.provider.clone(),
                id: dance.source.id.clone(),
            });
        }
        folder_menu(ui, width, folder, folders, &mut |folder| {
            action = Some(LibraryAction::MoveDance {
                provider: dance.source.provider.clone(),
                id: dance.source.id.clone(),
                folder,
            });
        });
    });
    action
}

fn saved_tune_row(
    ui: &mut egui::Ui,
    entry: &contra_core::LibraryEntry<Tune>,
    selected_id: Option<&str>,
    folders: &[String],
    animate: bool,
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
    let (hovered, focused) = {
        let ui = &mut prepared.content_ui;
        let body = tune_row_body(ui, entry, compact);
        let press = long_press_row(ui, &body);
        if let Some(choice) = row_context_menu(
            &body,
            folders,
            entry.folder.as_deref(),
            "Delete tune",
            press.suppress_click,
        ) {
            action = Some(match choice {
                RowMenuChoice::Move(folder) => LibraryAction::MoveTune {
                    id: entry.item.source.id.clone(),
                    folder,
                },
                RowMenuChoice::Delete => LibraryAction::RemoveTune(entry.item.source.id.clone()),
            });
        }
        if row_activated(ui, &body) && !press.suppress_click && !press.held {
            action = Some(LibraryAction::PreviewTune(entry.item.clone()));
        }
        (body.hovered(), body.has_focus())
    };
    if focused {
        prepared.frame.stroke = ui.visuals().selection.stroke;
    }
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
                    "{} · {} · {} · {}",
                    empty_label(&entry.item.composer, "unknown"),
                    entry.item.display_key(),
                    entry.item.display_rhythm(),
                    empty_label(entry.folder.as_deref().unwrap_or("Unfiled"), "Unfiled"),
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
        .on_hover_text("Preview this tune · right-click for folder and delete");
    let label = format!("Preview {}", entry.item.title);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_group_chip_names_the_current_folder_and_its_count() {
        assert_eq!(
            group_label(&FolderFilter::All, "All dances", 7),
            "All dances · 7"
        );
        assert_eq!(
            group_label(&FolderFilter::Unfiled, "All tunes", 2),
            "Unfiled · 2"
        );
        assert_eq!(
            group_label(&FolderFilter::Folder("Warmups".into()), "All dances", 3),
            "Warmups · 3"
        );
        // The count line keeps the search result count, so the chip is the
        // only place the grouping and its total appear.
        assert!(!group_label(&FolderFilter::All, "All tunes", 4).contains("shown"));
    }
}
