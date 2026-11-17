//! The ABC editor for saved tunes.
//!
//! The editor is deliberately small: a monospace text area, the same
//! sheet-music preview the tune picker uses, and Save, Cancel, and Restore.
//! Saving only changes the tune's `abc` text; the title, composer, key, and
//! source stay as they were imported or downloaded. That keeps an edit from
//! silently rewriting the metadata a caller searched for.
//!
//! The idea of editing ABC next to a rendered score comes from Michael Eskin's
//! [abctools](https://github.com/seisiuneer/abctools); this is an independent,
//! minimal implementation, not a port.

use std::time::Duration;

use web_time::Instant;

use super::*;
use crate::app::*;
use crate::files;
use crate::sheet::SheetPreview;
use eframe::egui;

/// How long the editor waits after a keystroke before engraving the draft.
///
/// Engraving is fast but not free, and re-rendering on every keystroke makes a
/// long tune feel sticky while typing. The parse error is still reported
/// immediately; only the staff waits.
const PREVIEW_DELAY: Duration = Duration::from_millis(250);

/// The tune being edited plus the draft and its preview.
pub(in crate::app) struct AbcEditorState {
    provider: String,
    id: String,
    title: String,
    /// The text in the editor.
    pub(in crate::app) text: String,
    /// The text the tune was saved with, for Cancel and the Restore check.
    original_abc: Option<String>,
    /// The current parse error, or `None` when the draft parses.
    pub(in crate::app) error: Option<String>,
    /// When the draft changed; the staff is re-engraved once this is old.
    preview_due: Option<Instant>,
    preview: Option<Result<SheetPreview, String>>,
    preview_text: String,
}

impl AbcEditorState {
    pub(in crate::app) fn new(tune: &Tune) -> Self {
        let mut state = Self {
            provider: tune.source.provider.clone(),
            id: tune.source.id.clone(),
            title: tune.title.clone(),
            text: tune.abc.clone(),
            original_abc: tune.original_abc.clone(),
            error: None,
            preview_due: Some(Instant::now() - PREVIEW_DELAY),
            preview: None,
            preview_text: String::new(),
        };
        state.error = parse_error(&state.text);
        state
    }

    fn edited(&mut self) {
        self.error = parse_error(&self.text);
        self.preview_due = Some(Instant::now());
        if self.error.is_some() {
            // Do not keep showing a staff that no longer describes the draft.
            self.preview = None;
            self.preview_text.clear();
        }
    }
}

/// The parse error in a draft, if any.
pub(in crate::app) fn parse_error(abc: &str) -> Option<String> {
    if abc.trim().is_empty() {
        return Some("The ABC text is empty.".to_string());
    }
    caller_music::parse_abc(abc)
        .err()
        .map(|error| error.to_string())
}

/// Whether the editor should offer Restore: the tune has a downloaded text and
/// the draft is not already that text.
pub(in crate::app) fn restore_available(original: Option<&str>, current: &str) -> bool {
    original.is_some_and(|original| original != current)
}

/// Write `abc` into the saved tune, but only after it parses.
///
/// The library is left untouched when the text is malformed, so the caller can
/// keep the editor open and show the error. Title, composer, key, meter, and
/// source are not re-read from the text.
pub(in crate::app) fn save_abc(
    library: &mut Library<Tune>,
    provider: &str,
    id: &str,
    abc: &str,
) -> Result<(), String> {
    if let Some(error) = parse_error(abc) {
        return Err(error);
    }
    let entry = library
        .entries
        .iter_mut()
        .find(|entry| entry.item.source.provider == provider && entry.item.source.id == id)
        .ok_or_else(|| "This tune is no longer in your saved tunes.".to_string())?;
    entry.item.abc = abc.to_string();
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorAction {
    Save,
    Cancel,
    Restore,
}

impl CallerApp {
    pub(in crate::app) fn abc_editor_ui(&mut self, ui: &mut egui::Ui) {
        let compact = compact(ui);
        apply_phone_spacing(ui, compact);
        if compact {
            egui::ScrollArea::vertical()
                .id_salt("abc-editor-page")
                .auto_shrink([false, false])
                .show(ui, |ui| self.abc_editor_body(ui, true));
        } else {
            self.abc_editor_body(ui, false);
        }
    }

    fn abc_editor_body(&mut self, ui: &mut egui::Ui, compact: bool) {
        let Some(mut editor) = self.abc_editor.take() else {
            ui.heading("Edit ABC");
            ui.label(egui::RichText::new("No tune is open in the editor.").color(theme::muted(ui)));
            if block_button(ui, "Back to saved tunes").clicked() {
                self.screen = Screen::TunePicker;
            }
            return;
        };
        let mut action = None;

        ui.horizontal_wrapped(|ui| {
            if ui
                .button(if compact {
                    "< Back"
                } else {
                    "< Back to saved tunes"
                })
                .on_hover_text("Leave the editor (Esc, or Cancel)")
                .clicked()
            {
                action = Some(EditorAction::Cancel);
            }
            ui.heading("Edit ABC");
        });
        ui.label(
            egui::RichText::new(format!(
                "{} · editing the saved ABC text only",
                editor.title
            ))
            .color(theme::muted(ui)),
        );
        ui.label(
            egui::RichText::new(
                "Saving changes the ABC used for this saved tune. The title, composer, \
                 key, meter, and source stay as they were. Malformed ABC is not saved.",
            )
            .small()
            .color(theme::muted(ui)),
        );
        ui.add_space(6.0);

        let editor_width = ui.available_width();
        let response = ui.add(
            egui::TextEdit::multiline(&mut editor.text)
                .font(egui::TextStyle::Monospace)
                .desired_rows(if compact { 10 } else { 14 })
                .desired_width(editor_width)
                .hint_text("ABC source"),
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::MultilineTextInput, true, "ABC source")
        });
        if response.changed() {
            editor.edited();
        }

        ui.horizontal_wrapped(|ui| {
            if let Some(error) = &editor.error {
                ui.label(
                    egui::RichText::new(format!("ABC error: {error}")).color(theme::error(ui)),
                );
            } else {
                ui.label(
                    egui::RichText::new("ABC parses.")
                        .small()
                        .color(theme::muted(ui)),
                );
            }
        });
        if let Some(error) = &editor
            .preview
            .as_ref()
            .and_then(|preview| preview.as_ref().err())
        {
            ui.label(
                egui::RichText::new(format!("Sheet music: {error}"))
                    .small()
                    .color(theme::warning(ui)),
            );
        }
        ui.add_space(4.0);

        let width = action_width(
            ui.available_width(),
            ui.spacing().item_spacing.x,
            if restore_available(editor.original_abc.as_deref(), &editor.text) {
                3
            } else {
                2
            },
            compact,
        );
        ui.horizontal_wrapped(|ui| {
            let save = ui
                .add_enabled(
                    editor.error.is_none(),
                    egui::Button::new("Save").min_size(egui::vec2(width, theme::TOUCH_TARGET)),
                )
                .on_hover_text("Save this ABC text to the tune library");
            if save.clicked() {
                action = Some(EditorAction::Save);
            }
            if ui
                .add_sized([width, theme::TOUCH_TARGET], egui::Button::new("Cancel"))
                .on_hover_text("Discard the draft and return to the saved tunes")
                .clicked()
            {
                action = Some(EditorAction::Cancel);
            }
            if restore_available(editor.original_abc.as_deref(), &editor.text)
                && ui
                    .add_sized(
                        [width, theme::TOUCH_TARGET],
                        egui::Button::new("Restore original"),
                    )
                    .on_hover_text("Put the downloaded ABC back")
                    .clicked()
            {
                action = Some(EditorAction::Restore);
            }
        });
        if let Some(status) = &self.library_status {
            ui.label(egui::RichText::new(status).small().color(theme::muted(ui)));
        }
        ui.separator();

        // Engrave the draft only when it parses and the typing has paused.
        if editor.error.is_none() && editor.preview_text != editor.text {
            let due = editor.preview_due.unwrap_or_else(Instant::now);
            if due.elapsed() >= PREVIEW_DELAY {
                editor.preview = Some(SheetPreview::render(ui.ctx(), &editor.text));
                editor.preview_text = editor.text.clone();
                editor.preview_due = None;
            } else {
                ui.ctx()
                    .request_repaint_after(PREVIEW_DELAY - due.elapsed());
            }
        }
        match &editor.preview {
            Some(Ok(sheet)) => sheet.ui(ui),
            Some(Err(error)) => {
                ui.label(
                    egui::RichText::new(format!(
                        "The sheet-music preview could not render this draft: {error}"
                    ))
                    .color(theme::warning(ui)),
                );
            }
            None => {
                ui.label(
                    egui::RichText::new("The sheet music updates after you stop typing.")
                        .small()
                        .color(theme::muted(ui)),
                );
            }
        }

        let source = (editor.provider.clone(), editor.id.clone());
        let title = editor.title.clone();
        match action {
            Some(EditorAction::Cancel) => {
                self.screen = Screen::TunePicker;
                self.library_status = Some("Cancelled the ABC edit.".to_string());
            }
            Some(EditorAction::Save) => {
                match save_abc(&mut self.tune_library, &source.0, &source.1, &editor.text) {
                    Ok(()) => {
                        self.save_tune_library();
                        self.reload_edited_tune(&source.0, &source.1, &editor.text);
                        self.library_status = Some(format!("Saved ABC changes to \"{title}\"."));
                        self.screen = Screen::TunePicker;
                    }
                    Err(error) => {
                        editor.error = Some(error);
                        self.abc_editor = Some(editor);
                    }
                }
            }
            Some(EditorAction::Restore) => {
                let Some(original) = editor.original_abc.clone() else {
                    self.abc_editor = Some(editor);
                    return;
                };
                match save_abc(&mut self.tune_library, &source.0, &source.1, &original) {
                    Ok(()) => {
                        self.save_tune_library();
                        self.reload_edited_tune(&source.0, &source.1, &original);
                        editor.text = original;
                        editor.error = None;
                        editor.preview = None;
                        editor.preview_text.clear();
                        editor.preview_due = Some(Instant::now() - PREVIEW_DELAY);
                        self.library_status =
                            Some(format!("Restored the downloaded ABC for \"{title}\"."));
                        self.abc_editor = Some(editor);
                    }
                    Err(error) => {
                        editor.error = Some(error);
                        self.abc_editor = Some(editor);
                    }
                }
            }
            None => self.abc_editor = Some(editor),
        }
    }

    /// Use edited ABC in the session when the edited tune is the loaded one.
    fn reload_edited_tune(&mut self, provider: &str, id: &str, abc: &str) {
        // The preview belongs to the tune that was just edited; its texture was
        // engraved from the old text.
        self.tune_sheet = None;
        if self.tune.source.provider != provider || self.tune.source.id != id {
            return;
        }
        self.tune.abc = abc.to_string();
        self.needs_render = true;
        self.arrangement = None;
        self.rotation_settings_changed();
    }
}

/// The Android export prompt: the file is already saved, and the share sheet is
/// one tap away. Other platforms never set the request.
pub(in crate::app) fn share_prompt(app: &mut CallerApp, ui: &mut egui::Ui) {
    let Some(share) = app.share_request.clone() else {
        return;
    };
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(egui::RichText::new(format!("Exported \"{}\".", share.name)).strong());
        ui.horizontal_wrapped(|ui| {
            if block_button(ui, "Share").clicked() {
                match files::share_text(&share) {
                    Ok(()) => {
                        app.library_status = Some("Opened the Android share sheet.".to_string());
                        app.share_request = None;
                    }
                    Err(error) => app.error = Some(error),
                }
            }
            if block_button(ui, "Not now").clicked() {
                app.share_request = None;
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use caller_core::{Library, SourceRef};

    fn tune() -> Tune {
        Tune {
            source: SourceRef::new("local", "abc123", ""),
            title: "Test Tune".into(),
            composer: "Composer".into(),
            key: "D".into(),
            meter: "4/4".into(),
            rhythm: "reel".into(),
            abc: "X:1\nT:Test Tune\nM:4/4\nL:1/8\nK:D\nDEFG ABcd|\n".into(),
            original_abc: Some("X:1\nT:Test Tune\nM:4/4\nL:1/8\nK:D\nDEFG ABcd|\n".into()),
            credits: String::new(),
        }
    }

    fn library() -> Library<Tune> {
        let mut library = Library::default();
        library.insert(tune(), Some("Warmups".into()), 1);
        library
    }

    #[test]
    fn a_valid_edit_changes_only_the_abc_text() {
        let mut library = library();
        let edited = "X:1\nT:Test Tune\nM:4/4\nL:1/8\nK:D\nGABc defg|\n";
        save_abc(&mut library, "local", "abc123", edited).expect("save");
        let entry = library.get("local", "abc123").expect("still saved");
        assert_eq!(entry.item.abc, edited);
        assert_eq!(entry.item.title, "Test Tune");
        assert_eq!(entry.item.composer, "Composer");
        assert_eq!(entry.item.key, "D");
        assert_eq!(entry.item.source.id, "abc123");
        assert_eq!(entry.folder.as_deref(), Some("Warmups"));
        assert_eq!(entry.item.original_abc, tune().original_abc);
    }

    #[test]
    fn malformed_abc_keeps_the_saved_text() {
        let mut library = library();
        let before = library
            .get("local", "abc123")
            .expect("saved")
            .item
            .abc
            .clone();
        let error =
            save_abc(&mut library, "local", "abc123", "not abc at all").expect_err("reject");
        assert!(!error.is_empty());
        assert_eq!(
            library.get("local", "abc123").expect("saved").item.abc,
            before
        );
    }

    #[test]
    fn saving_an_unknown_tune_is_an_error() {
        let mut library = library();
        let error =
            save_abc(&mut library, "local", "missing", &tune().abc).expect_err("unknown tune");
        assert!(error.contains("no longer"));
    }

    #[test]
    fn restore_is_offered_only_when_the_draft_differs_from_the_original() {
        let original = tune().original_abc.expect("original");
        assert!(!restore_available(Some(&original), &original));
        assert!(restore_available(Some(&original), "X:1\nK:G\nGABc|"));
        assert!(!restore_available(None, "X:1\nK:G\nGABc|"));
    }

    #[test]
    fn restore_puts_the_downloaded_text_back() {
        let mut library = library();
        let original = tune().original_abc.expect("original");
        let edited = "X:1\nT:Test Tune\nM:4/4\nL:1/8\nK:D\nGABc|\n";
        save_abc(&mut library, "local", "abc123", edited).expect("save");
        save_abc(&mut library, "local", "abc123", &original).expect("restore");
        assert_eq!(
            library.get("local", "abc123").expect("saved").item.abc,
            original
        );
    }

    #[test]
    fn cancel_discards_the_draft_because_the_library_is_untouched() {
        let library = library();
        let before = library.get("local", "abc123").expect("saved").item.clone();
        let mut state = AbcEditorState::new(&before);
        state.text = "X:1\nT:Test Tune\nM:4/4\nL:1/8\nK:D\nGABc|\n".to_string();
        state.edited();
        // Cancel only drops the state; no save helper ran, so the library is
        // exactly as it was.
        assert_eq!(library.get("local", "abc123").expect("saved").item, before);
    }

    #[test]
    fn parse_error_reports_malformed_and_empty_text() {
        assert!(parse_error("").is_some());
        assert!(parse_error("just words").is_some());
        assert!(parse_error(&tune().abc).is_none());
        assert!(parse_error("X:1\nT:No body\nM:4/4\nL:1/8\nK:D\n").is_some());
    }

    #[test]
    fn the_editor_state_starts_with_the_saved_text_and_original() {
        let tune = tune();
        let state = AbcEditorState::new(&tune);
        assert_eq!(state.text, tune.abc);
        assert_eq!(state.original_abc, tune.original_abc);
        assert!(state.error.is_none());
        assert_eq!(state.title, "Test Tune");
    }

    #[test]
    fn the_editor_can_edit_a_tune_without_an_original() {
        let mut tune = tune();
        tune.original_abc = None;
        let mut state = AbcEditorState::new(&tune);
        assert!(!restore_available(
            state.original_abc.as_deref(),
            &state.text
        ));
        state.text = "not abc".into();
        state.edited();
        assert!(state.error.is_some());
    }
}
