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
    action_width, block_button, block_selectable, combo_option, empty_label, formation_label,
    row_button, row_menu_button, submit_text_edit, submitted,
};

use crate::theme;
use contra_core::{Dance, FolderFilter, Tune};
use eframe::egui;

use super::*;

/// Whether this screen should use the phone layout.
///
/// Every screen shares the session tab's breakpoint so the tab row, the
/// controls, and the forms never switch layouts at different widths. The app
/// window's minimum width is 360 points, so each phone layout must also fit a
/// 360-point-wide viewport.
pub(super) fn compact(ui: &egui::Ui) -> bool {
    compact_layout(ui.available_width())
}

/// Phone screens use 44-point touch rows and full-width text fields.
pub(super) fn apply_phone_spacing(ui: &mut egui::Ui, compact: bool) {
    if compact {
        ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
        ui.spacing_mut().button_padding.x = 10.0;
        ui.spacing_mut().text_edit_width = ui.available_width();
    }
}

/// One row of a label/value form.
///
/// In a desktop grid the label and control occupy two columns. On a phone the
/// row becomes one column with a small label beside a full-width control, which
/// keeps a long search form from pushing the results below the fold.
pub(super) fn form_row<R>(
    ui: &mut egui::Ui,
    compact: bool,
    label: &str,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    if compact {
        ui.horizontal(|ui| {
            ui.spacing_mut().text_edit_width = ui.available_width();
            ui.label(egui::RichText::new(label).small().color(theme::muted(ui)));
            add(ui)
        })
        .inner
    } else {
        ui.label(label);
        add(ui)
    }
}

/// A label beside a control on desktop and a small label beside a full-width
/// control on a phone.
pub(super) fn inline_field<R>(
    ui: &mut egui::Ui,
    compact: bool,
    label: &str,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    if compact {
        ui.horizontal(|ui| {
            ui.spacing_mut().text_edit_width = ui.available_width();
            ui.label(egui::RichText::new(label).small().color(theme::muted(ui)));
            add(ui)
        })
        .inner
    } else {
        ui.horizontal(|ui| {
            ui.label(label);
            add(ui)
        })
        .inner
    }
}

/// A slider with a label beside it on desktop and above it on a phone.
///
/// On a phone the rail takes the whole row; leaving room for the value box
/// keeps the value readable while the thumb is still easy to hit.
pub(super) fn slider_row(
    ui: &mut egui::Ui,
    compact: bool,
    label: &str,
    add: impl FnOnce(&mut egui::Ui) -> egui::Response,
) -> egui::Response {
    if compact {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(label).small().color(theme::muted(ui)));
            // Leave room for the value box and a suffix such as "syllables/s";
            // without this the row is wider than a 360-point window and every
            // label in the page stops wrapping at the viewport edge.
            ui.spacing_mut().slider_width = (ui.available_width() - 160.0).max(96.0);
            add(ui)
        })
        .inner
    } else {
        ui.horizontal(|ui| {
            ui.label(label);
            add(ui)
        })
        .inner
    }
}

/// Size a combo box to the available width on a phone.
pub(super) fn responsive_combo<'a>(
    combo: egui::ComboBox<'a>,
    ui: &egui::Ui,
    compact: bool,
) -> egui::ComboBox<'a> {
    if compact {
        combo.width((ui.available_width() - 12.0).max(96.0))
    } else {
        combo
    }
}

/// Space the picker panes must leave below them for the pagination row and
/// the gap above it.
const PAGINATION_ROW_HEIGHT: f32 = 40.0;

/// The smallest height a split desktop picker gives its list and preview.
///
/// Below this a forced box would show only a row or two, so the panes keep a
/// usable floor and the window clips instead of collapsing the list.
const MIN_PANE_HEIGHT: f32 = 120.0;

/// Height left for a list or preview pane below the rows already drawn.
///
/// The panes take every point that is left, so on a tall window they end at
/// the bottom edge instead of at an arbitrary fixed cap. A short window keeps
/// a usable floor rather than shrinking the pane to nothing.
fn remaining_pane_height(available_height: f32, reserved_below: f32) -> f32 {
    (available_height - reserved_below).max(MIN_PANE_HEIGHT)
}

/// Height for a picker's list and preview panes, or `None` when the page
/// itself scrolls.
///
/// A phone page already scrolls, so the stacked list and preview keep their
/// natural content height and must not reserve a fixed empty box; that is what
/// keeps a 360-point window usable. A desktop has no page scroll, so the panes
/// consume the remaining viewport height and both columns end at the same
/// bottom edge.
pub(super) fn picker_pane_height(
    compact: bool,
    available_height: f32,
    reserved_below: f32,
) -> Option<f32> {
    (!compact).then(|| remaining_pane_height(available_height, reserved_below))
}

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
    ToggleRotation(Tune),
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
    rotation_dirty: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_panes_take_the_height_left_after_the_pagination_row() {
        assert_eq!(
            picker_pane_height(false, 700.0, PAGINATION_ROW_HEIGHT),
            Some(660.0)
        );
        // Without a row below them (the saved-library panes), the panes reach
        // the bottom of the viewport.
        assert_eq!(picker_pane_height(false, 700.0, 0.0), Some(700.0));
    }

    #[test]
    fn picker_panes_keep_a_usable_floor_on_short_windows() {
        assert_eq!(
            picker_pane_height(false, 90.0, PAGINATION_ROW_HEIGHT),
            Some(MIN_PANE_HEIGHT)
        );
        assert_eq!(
            picker_pane_height(false, 0.0, PAGINATION_ROW_HEIGHT),
            Some(MIN_PANE_HEIGHT)
        );
        assert_eq!(
            remaining_pane_height(20.0, PAGINATION_ROW_HEIGHT),
            MIN_PANE_HEIGHT
        );
    }

    #[test]
    fn a_360_point_window_uses_natural_stacked_heights() {
        // The phone page scrolls, so the picker must not force a fixed-height
        // list or preview at the minimum window width.
        assert_eq!(
            picker_pane_height(compact_layout(360.0), 800.0, PAGINATION_ROW_HEIGHT),
            None
        );
        assert_eq!(
            picker_pane_height(compact_layout(390.0), 844.0, PAGINATION_ROW_HEIGHT),
            None
        );
        // The narrowest desktop width still receives a derived pane height.
        assert_eq!(
            picker_pane_height(compact_layout(640.0), 800.0, PAGINATION_ROW_HEIGHT),
            Some(760.0)
        );
    }
}
