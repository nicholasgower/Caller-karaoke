//! Screen implementations for the app shell.
//!
//! Each screen owns one file; shared card helpers stay here so the session
//! screen and the state-machine tests see the same definitions.

mod about;
mod aliases;
mod dance_picker;
mod library;
mod preferences;
mod session;
mod tune_picker;
mod widgets;

pub(in crate::app) use widgets::{
    combo_option, empty_label, formation_label, submit_text_edit, submitted,
};

use crate::theme;
use contra_core::{Dance, FolderFilter, Tune};
use eframe::egui;

use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CallPhase {
    Upcoming,
    Calling,
    Executing,
}

pub(super) fn phase_at(call: &contra_core::ScheduledCall, beat: f64) -> CallPhase {
    if beat < call.start_beat {
        CallPhase::Upcoming
    } else if beat < call.execute_beat {
        CallPhase::Calling
    } else {
        CallPhase::Executing
    }
}

/// What the large prompt card is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CardMode {
    /// The call the caller is about to speak or is speaking now.
    CurrentCall,
    /// The figure is already being danced, so the card shows the move the
    /// caller must say next.
    UpcomingMove,
    /// The figure is being danced and there is no following move to call.
    FinalMove,
}

/// Pick the card content from the active call's phase and whether another
/// call follows it.
///
/// A caller looks ahead while the dancers are executing, so the card must stop
/// describing the figure in progress and start prompting the next move.
pub(super) fn card_state(phase: Option<CallPhase>, has_next: bool) -> CardMode {
    match phase {
        Some(CallPhase::Executing) if has_next => CardMode::UpcomingMove,
        Some(CallPhase::Executing) => CardMode::FinalMove,
        _ => CardMode::CurrentCall,
    }
}

/// Background cue for the prompt card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CardTone {
    /// Nothing to speak yet: ready, count-in, dancing the last figure, done.
    Neutral,
    /// The caller has to say the shown call now.
    CallNow,
    /// The dancers are executing, so prompt the move to call next.
    UpcomingMove,
}

/// Green while the caller is speaking, red while the card looks ahead, grey
/// otherwise. The hue is a glanceable cue on top of the text, never the only
/// signal: the labels still say CALL NOW and UPCOMING MOVE.
pub(super) fn card_tone(started: bool, phase: Option<CallPhase>, mode: CardMode) -> CardTone {
    if !started {
        return CardTone::Neutral;
    }
    match (phase, mode) {
        (Some(CallPhase::Calling), _) => CardTone::CallNow,
        (_, CardMode::UpcomingMove) => CardTone::UpcomingMove,
        _ => CardTone::Neutral,
    }
}

/// A change requested by one of the saved-library lists.
pub(super) enum LibraryAction {
    PreviewDance(Dance),
    UseDance(Dance),
    RemoveDance(String),
    MoveDance { id: String, folder: Option<String> },
    PreviewTune(Tune),
    UseTune(Tune),
    RemoveTune(String),
    MoveTune { id: String, folder: Option<String> },
}

/// A folder operation requested by the folder panel.
pub(super) enum FolderCommand {
    Create(String),
    Rename { old: String, new: String },
    Delete(String),
}

/// The app-side result of a successful folder operation.
pub(super) enum FolderOutcome {
    Created(String),
    Renamed(String),
    Deleted(String),
}

/// Mutable view of the fields the shared folder panel renders.
pub(super) struct FolderPanel<'a> {
    all_label: &'a str,
    folders: &'a [String],
    counts: &'a [usize],
    all_count: usize,
    unfiled_count: usize,
    filter: &'a mut FolderFilter,
    new_folder: &'a mut String,
    renaming: &'a mut Option<String>,
    rename_text: &'a mut String,
}

/// Prompt the caller with the move they must call next while the current
/// figure is being danced.
pub(super) fn upcoming_move_card(
    ui: &mut egui::Ui,
    next: &contra_core::ScheduledCall,
    beat: f64,
    show_original: bool,
    call_size: f32,
) {
    ui.label(
        egui::RichText::new("UPCOMING MOVE")
            .small()
            .color(theme::muted(ui)),
    );
    ui.label(
        egui::RichText::new(&next.call.text)
            .size(call_size)
            .strong()
            .color(theme::on_card(ui)),
    );
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "call in {:.1} beats",
                (next.start_beat - beat).max(0.0)
            ))
            .color(theme::muted(ui)),
        );
        if show_original {
            ui.label(
                egui::RichText::new(format!("· {}", next.call.original_text))
                    .small()
                    .color(theme::muted(ui)),
            );
        }
    });
}

/// What the session controls changed this frame.
#[derive(Default)]
pub(super) struct SessionControlChanges {
    timeline_dirty: bool,
    music_dirty: bool,
    save: bool,
    action: Option<&'static str>,
}

impl ContraApp {
    pub(super) fn pagination_ui(&mut self, ui: &mut egui::Ui, dance: bool) {
        let (page, page_size, total) = if dance {
            let results = self.picker.dance_results.as_ref();
            (
                self.picker.dance_query.page.max(1),
                self.picker.dance_query.page_size.max(1),
                results.map(|page| page.total).unwrap_or(0),
            )
        } else {
            let results = self.picker.tune_results.as_ref();
            (
                self.picker.tune_query.page.max(1),
                self.picker.tune_query.page_size.max(1),
                results.map(|page| page.total).unwrap_or(0),
            )
        };
        let pages = total.div_ceil(page_size);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(page > 1, egui::Button::new("Previous"))
                .clicked()
            {
                if dance {
                    self.picker.dance_query.page = page - 1;
                    self.source.search_dances(self.picker.dance_query.clone());
                } else {
                    self.picker.tune_query.page = page - 1;
                    self.source.search_tunes(self.picker.tune_query.clone());
                }
                self.picker.search_loading = true;
            }
            ui.label(format!("Page {page} of {}", pages.max(1)));
            if ui
                .add_enabled(page < pages, egui::Button::new("Next"))
                .clicked()
            {
                if dance {
                    self.picker.dance_query.page = page + 1;
                    self.source.search_dances(self.picker.dance_query.clone());
                } else {
                    self.picker.tune_query.page = page + 1;
                    self.source.search_tunes(self.picker.tune_query.clone());
                }
                self.picker.search_loading = true;
            }
        });
    }
}
