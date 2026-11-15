use super::*;
use crate::app::*;
use eframe::egui;

impl ContraApp {
    pub(in crate::app) fn dance_library_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .button("< Back to session")
                .on_hover_text("Back to session (Esc)")
                .clicked()
            {
                self.screen = Screen::Session;
            }
            ui.heading("Select dance");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Download dance")
                    .on_hover_text("Search ContraDB and add a card to your saved dances")
                    .clicked()
                {
                    self.picker.dance_preview = None;
                    self.picker.dance_selected = None;
                    self.screen = Screen::DanceDownload;
                }
            });
        });
        ui.label(
            egui::RichText::new(
                "Your saved dances, available offline. Search them here, or use Download dance \
                 to fetch another card from ContraDB.",
            )
            .color(theme::muted(ui)),
        );
        ui.add_space(4.0);

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
            let shown = self
                .dance_library
                .search(&self.dance_library_query, &self.dance_library_folder)
                .len();
            ui.label(
                egui::RichText::new(format!(
                    "{shown} of {} shown · {}",
                    self.dance_library.len(),
                    self.dance_library_folder.label()
                ))
                .small()
                .color(theme::muted(ui)),
            );
        });
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
            ui.horizontal_top(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("dance-folder-panel")
                    .max_height(480.0)
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
                        },
                    );
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
                    saved_dance_row(ui, entry, selected_id.as_deref(), &folders, animate)
                {
                    action = Some(next);
                }
            }
            ui.separator();
            if let Some(dance) = &preview {
                if let Some(next) = saved_dance_preview(ui, dance) {
                    action = Some(next);
                }
            } else {
                ui.label(
                    egui::RichText::new("Preview a saved dance to see its card.")
                        .color(theme::muted(ui)),
                );
            }
        } else if ui.available_width() >= 560.0 {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("saved-dance-list")
                    .max_height(480.0)
                    .show(&mut columns[0], |ui| {
                        if !empty_message.is_empty() {
                            ui.label(egui::RichText::new(empty_message).color(theme::muted(ui)));
                        }
                        for entry in &entries {
                            if let Some(next) = saved_dance_row(
                                ui,
                                entry,
                                selected_id.as_deref(),
                                &folders,
                                animate,
                            ) {
                                action = Some(next);
                            }
                        }
                    });
                egui::ScrollArea::vertical()
                    .id_salt("saved-dance-preview")
                    .max_height(480.0)
                    .show(&mut columns[1], |ui| {
                        if let Some(dance) = &preview {
                            if let Some(next) = saved_dance_preview(ui, dance) {
                                action = Some(next);
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("Preview a saved dance to see its card.")
                                    .color(theme::muted(ui)),
                            );
                        }
                    });
            });
        } else {
            egui::ScrollArea::vertical()
                .id_salt("saved-dance-list-narrow")
                .max_height(300.0)
                .show(ui, |ui| {
                    if !empty_message.is_empty() {
                        ui.label(egui::RichText::new(empty_message).color(theme::muted(ui)));
                    }
                    for entry in &entries {
                        if let Some(next) =
                            saved_dance_row(ui, entry, selected_id.as_deref(), &folders, animate)
                        {
                            action = Some(next);
                        }
                    }
                });
            ui.separator();
            if let Some(dance) = &preview {
                if let Some(next) = saved_dance_preview(ui, dance) {
                    action = Some(next);
                }
            } else {
                ui.label(
                    egui::RichText::new("Preview a saved dance to see its card.")
                        .color(theme::muted(ui)),
                );
            }
        }

        if let Some(action) = action {
            self.apply_library_action(action);
        }
    }

    pub(in crate::app) fn tune_library_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .button("< Back to session")
                .on_hover_text("Back to session (Esc)")
                .clicked()
            {
                self.screen = Screen::Session;
            }
            ui.heading("Select tune");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Download tune")
                    .on_hover_text("Search ABC Session Tunes and add a tune to your saved tunes")
                    .clicked()
                {
                    self.picker.tune_preview = None;
                    self.picker.tune_selected = None;
                    self.tune_sheet = None;
                    self.screen = Screen::TuneDownload;
                }
            });
        });
        ui.label(
            egui::RichText::new(
                "Your saved tunes, available offline. Search them here, or use Download tune \
                 to fetch another tune from ABC Session Tunes.",
            )
            .color(theme::muted(ui)),
        );
        ui.add_space(4.0);

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
            let shown = self
                .tune_library
                .search(&self.tune_library_query, &self.tune_library_folder)
                .len();
            ui.label(
                egui::RichText::new(format!(
                    "{shown} of {} shown · {}",
                    self.tune_library.len(),
                    self.tune_library_folder.label()
                ))
                .small()
                .color(theme::muted(ui)),
            );
        });
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
            ui.horizontal_top(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("tune-folder-panel")
                    .max_height(480.0)
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
                        },
                    );
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
                if let Some(next) =
                    saved_tune_row(ui, entry, selected_id.as_deref(), &folders, animate)
                {
                    action = Some(next);
                }
            }
            ui.separator();
            if let Some(tune) = &preview {
                if let Some(next) = self.saved_tune_preview(ui, tune) {
                    action = Some(next);
                }
            } else {
                ui.label(
                    egui::RichText::new("Preview a saved tune to see its sheet music.")
                        .color(theme::muted(ui)),
                );
            }
        } else if ui.available_width() >= 560.0 {
            ui.columns(2, |columns| {
                egui::ScrollArea::vertical()
                    .id_salt("saved-tune-list")
                    .max_height(480.0)
                    .show(&mut columns[0], |ui| {
                        if !empty_message.is_empty() {
                            ui.label(egui::RichText::new(empty_message).color(theme::muted(ui)));
                        }
                        for entry in &entries {
                            if let Some(next) =
                                saved_tune_row(ui, entry, selected_id.as_deref(), &folders, animate)
                            {
                                action = Some(next);
                            }
                        }
                    });
                egui::ScrollArea::vertical()
                    .id_salt("saved-tune-preview")
                    .max_height(480.0)
                    .show(&mut columns[1], |ui| {
                        if let Some(tune) = &preview {
                            if let Some(next) = self.saved_tune_preview(ui, tune) {
                                action = Some(next);
                            }
                        } else {
                            ui.label(
                                egui::RichText::new("Preview a saved tune to see its sheet music.")
                                    .color(theme::muted(ui)),
                            );
                        }
                    });
            });
        } else {
            egui::ScrollArea::vertical()
                .id_salt("saved-tune-list-narrow")
                .max_height(300.0)
                .show(ui, |ui| {
                    if !empty_message.is_empty() {
                        ui.label(egui::RichText::new(empty_message).color(theme::muted(ui)));
                    }
                    for entry in &entries {
                        if let Some(next) =
                            saved_tune_row(ui, entry, selected_id.as_deref(), &folders, animate)
                        {
                            action = Some(next);
                        }
                    }
                });
            ui.separator();
            if let Some(tune) = &preview {
                if let Some(next) = self.saved_tune_preview(ui, tune) {
                    action = Some(next);
                }
            } else {
                ui.label(
                    egui::RichText::new("Preview a saved tune to see its sheet music.")
                        .color(theme::muted(ui)),
                );
            }
        }

        if let Some(action) = action {
            self.apply_library_action(action);
        }
    }

    /// Preview a tune that came from the local library, including its sheet
    /// music, and offer the library actions.
    fn saved_tune_preview(&mut self, ui: &mut egui::Ui, tune: &Tune) -> Option<LibraryAction> {
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
        ui.horizontal_wrapped(|ui| {
            if ui.button("Use this tune").clicked() {
                action = Some(LibraryAction::UseTune(tune.clone()));
            }
            if ui.small_button("Remove from saved tunes").clicked() {
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
    if ui
        .selectable_label(
            *panel.filter == FolderFilter::All,
            format!("{} ({})", panel.all_label, panel.all_count),
        )
        .clicked()
    {
        *panel.filter = FolderFilter::All;
        *panel.renaming = None;
    }
    if ui
        .selectable_label(
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
        if ui
            .selectable_label(is_selected, format!("{folder} ({count})"))
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
            ui.horizontal_wrapped(|ui| {
                if ui.button("Save name").clicked() || submit {
                    command = Some(FolderCommand::Rename {
                        old: folder.clone(),
                        new: panel.rename_text.clone(),
                    });
                }
                if ui.button("Cancel").clicked() {
                    *panel.renaming = None;
                }
            });
        } else {
            ui.vertical(|ui| {
                if ui
                    .small_button("Rename folder")
                    .on_hover_text("Rename this folder without moving its items out")
                    .clicked()
                {
                    *panel.renaming = Some(folder.clone());
                    *panel.rename_text = folder.clone();
                }
                if ui
                    .small_button("Delete folder")
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
    if ui
        .button("Create folder")
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
) -> Option<LibraryAction> {
    let selected = selected_id == Some(entry.item.source.id.as_str());
    let selected_t = animate_amount(
        ui.ctx(),
        ("saved-dance-selected", &entry.item.source.id),
        selected,
        animate,
    );
    let mut action = None;
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
                    let folder_label = entry
                        .folder
                        .clone()
                        .unwrap_or_else(|| "Unfiled".to_string());
                    ui.menu_button(format!("Folder: {folder_label}"), |ui| {
                        if ui
                            .selectable_label(entry.folder.is_none(), "Unfiled")
                            .clicked()
                        {
                            action = Some(LibraryAction::MoveDance {
                                id: entry.item.source.id.clone(),
                                folder: None,
                            });
                            ui.close();
                        }
                        for folder in folders {
                            let current = entry.folder.as_deref() == Some(folder.as_str());
                            if ui.selectable_label(current, folder).clicked() {
                                action = Some(LibraryAction::MoveDance {
                                    id: entry.item.source.id.clone(),
                                    folder: Some(folder.clone()),
                                });
                                ui.close();
                            }
                        }
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.small_button("Use").clicked() {
                        action = Some(LibraryAction::UseDance(entry.item.clone()));
                    }
                    if ui.small_button("Preview").clicked() {
                        action = Some(LibraryAction::PreviewDance(entry.item.clone()));
                    }
                });
            });
        });
    action
}

fn saved_dance_preview(ui: &mut egui::Ui, dance: &Dance) -> Option<LibraryAction> {
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
    ui.horizontal_wrapped(|ui| {
        if ui.button("Use this dance").clicked() {
            action = Some(LibraryAction::UseDance(dance.clone()));
        }
        if ui.small_button("Remove from saved dances").clicked() {
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
) -> Option<LibraryAction> {
    let selected = selected_id == Some(entry.item.source.id.as_str());
    let selected_t = animate_amount(
        ui.ctx(),
        ("saved-tune-selected", &entry.item.source.id),
        selected,
        animate,
    );
    let mut action = None;
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
                    let folder_label = entry
                        .folder
                        .clone()
                        .unwrap_or_else(|| "Unfiled".to_string());
                    ui.menu_button(format!("Folder: {folder_label}"), |ui| {
                        if ui
                            .selectable_label(entry.folder.is_none(), "Unfiled")
                            .clicked()
                        {
                            action = Some(LibraryAction::MoveTune {
                                id: entry.item.source.id.clone(),
                                folder: None,
                            });
                            ui.close();
                        }
                        for folder in folders {
                            let current = entry.folder.as_deref() == Some(folder.as_str());
                            if ui.selectable_label(current, folder).clicked() {
                                action = Some(LibraryAction::MoveTune {
                                    id: entry.item.source.id.clone(),
                                    folder: Some(folder.clone()),
                                });
                                ui.close();
                            }
                        }
                    });
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.small_button("Use").clicked() {
                        action = Some(LibraryAction::UseTune(entry.item.clone()));
                    }
                    if ui.small_button("Preview").clicked() {
                        action = Some(LibraryAction::PreviewTune(entry.item.clone()));
                    }
                });
            });
        });
    action
}
