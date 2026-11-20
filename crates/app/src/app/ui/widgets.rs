use crate::theme;
use eframe::egui;

use super::gesture::{LongPress, PressEnd, PressPhase};

/// The narrowest a desktop action button may get before the row stacks.
///
/// Below this a two- or three-button row would squeeze labels such as
/// "Delete dance" or "Use this folder as the tune set" into an unreadable
/// column, so the callers give every button the full row width instead.
const MIN_ACTION_WIDTH: f32 = 132.0;

/// Width of a draggable splitter between two panes.
pub(in crate::app) const SPLITTER_WIDTH: f32 = 10.0;

/// The narrowest a resizable pane may become, in points.
pub(in crate::app) const MIN_PANE_WIDTH: f32 = 120.0;

/// Width for one button of a full-width action row.
///
/// On a phone (or in a content column too narrow to share), the row stacks and
/// each button takes the full width. On a desktop the gaps between the buttons
/// come out of the available width first, so the row ends exactly at the
/// content edge. The result is never negative, which keeps a 360-point row
/// finite.
pub(in crate::app) fn action_width(
    available: f32,
    spacing: f32,
    count: usize,
    compact: bool,
) -> f32 {
    if !available.is_finite() {
        return 0.0;
    }
    let count = count.max(1) as f32;
    if compact || available < count * MIN_ACTION_WIDTH + spacing * (count - 1.0) {
        return available.max(0.0);
    }
    ((available - spacing * (count - 1.0)) / count).max(0.0)
}

/// Width for one button of a row that fills the available width exactly.
///
/// In the mobile layout a set of buttons stays on one line only when each
/// button can take an equal share and the last one ends at the content edge; a
/// row that stops short leaves an empty strip on the right that reads as a
/// broken layout. [`filled_button_row`] applies this width. Rows that may stack
/// instead use [`action_width`].
pub(in crate::app) fn filled_row_width(available: f32, spacing: f32, count: usize) -> f32 {
    if !available.is_finite() || !spacing.is_finite() {
        return 0.0;
    }
    let count = count.max(1) as f32;
    ((available - spacing * (count - 1.0)) / count).max(0.0)
}

/// Lay a set of buttons side by side so the row ends at the content edge.
///
/// `add` receives the width every button must use (`add_sized` or
/// `Button::min_size`), and is called once. A horizontal button set must not
/// leave empty space on the right in the mobile layout, so the widths come out
/// of the available width before the buttons are drawn.
pub(in crate::app) fn filled_button_row<R>(
    ui: &mut egui::Ui,
    count: usize,
    add: impl FnOnce(&mut egui::Ui, f32) -> R,
) -> R {
    let width = filled_row_width(ui.available_width(), ui.spacing().item_spacing.x, count);
    ui.horizontal(|ui| add(ui, width)).inner
}

/// A full-width button: touch sized and spanning the available width.
///
/// The folder panel and the stacked phone action rows use this so every
/// clickable row is a 44-point target instead of a small inline button.
pub(in crate::app) fn block_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add_sized(
        [ui.available_width(), theme::TOUCH_TARGET],
        egui::Button::new(label),
    )
}

/// A full-width selectable row, used by the folder list.
pub(in crate::app) fn block_selectable(
    ui: &mut egui::Ui,
    selected: bool,
    label: impl Into<egui::WidgetText>,
) -> egui::Response {
    ui.add_sized(
        [ui.available_width(), theme::TOUCH_TARGET],
        egui::Button::selectable(selected, label),
    )
}

/// A menu button with a touch-sized slot, so it lines up with the other
/// preview actions.
pub(in crate::app) fn row_menu_button(
    ui: &mut egui::Ui,
    width: f32,
    label: impl Into<egui::WidgetText>,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let button = egui::Button::new(label)
        .min_size(egui::vec2(width, theme::TOUCH_TARGET))
        .wrap_mode(egui::TextWrapMode::Truncate);
    // A menu button is its own widget, so constrain the child UI to the slot
    // width as well; otherwise a long folder name would widen the row and
    // push the next button out of the column.
    ui.allocate_ui_with_layout(
        egui::vec2(width, theme::TOUCH_TARGET),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            egui::containers::menu::MenuButton::from_button(button)
                .ui(ui, add_contents)
                .0
        },
    )
    .inner
}

/// A small rounded label such as "All dances · 12".
///
/// The list header uses it for the current grouping instead of repeating the
/// folder name in the count line. It is painted with the theme's neutral
/// surfaces, never an accent colour, and it is not a button.
pub(in crate::app) fn group_chip(ui: &mut egui::Ui, text: &str) -> egui::Response {
    egui::Frame::new()
        .fill(ui.visuals().widgets.noninteractive.bg_fill)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(egui::CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).small().color(theme::muted(ui)))
        })
        .response
}

/// Clamp a resizable pane to a usable width inside `available`.
///
/// Both panes keep at least [`MIN_PANE_WIDTH`] and the splitter keeps its own
/// width. A tiny or non-finite viewport still returns a finite minimum so the
/// first frame cannot place a pane at NaN.
pub(in crate::app) fn clamp_pane_width(width: f32, available: f32) -> f32 {
    if !width.is_finite() || !available.is_finite() || available <= 0.0 {
        return MIN_PANE_WIDTH;
    }
    let max = (available - MIN_PANE_WIDTH - SPLITTER_WIDTH).max(MIN_PANE_WIDTH);
    width.clamp(MIN_PANE_WIDTH, max)
}

/// A vertical drag handle between two panes.
///
/// The returned response is `dragged()` while the user moves it; callers add
/// `drag_delta().x` to their stored width. The handle is wider than its visible
/// line so it is easy to hit with a mouse, and it is a splitter in the
/// accessibility tree.
pub(in crate::app) fn pane_splitter(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    height: f32,
    label: &str,
) -> egui::Response {
    ui.push_id(id_salt, |ui| {
        let height = if height.is_finite() {
            height.max(theme::TOUCH_TARGET)
        } else {
            theme::TOUCH_TARGET
        };
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(SPLITTER_WIDTH, height), egui::Sense::hover());
        let response = ui
            .interact(rect, ui.scope_id().with("splitter"), egui::Sense::drag())
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
            .on_hover_text("Drag to resize the panes");
        let active = response.dragged() || response.hovered();
        let stroke = if active {
            egui::Stroke::new(2.0, ui.visuals().widgets.hovered.fg_stroke.color)
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke
        };
        ui.painter().vline(rect.center().x, rect.y_range(), stroke);
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Splitter, true, label));
        response
    })
    .inner
}

/// Whether a custom row was activated.
///
/// Custom rows are `Ui::interact` regions rather than `Button`s, so egui does
/// not turn Enter into a click for them. This restores keyboard activation
/// while keeping the pointer, accesskit action, and Enter paths identical.
pub(in crate::app) fn row_activated(ui: &egui::Ui, response: &egui::Response) -> bool {
    response.clicked()
        || (response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)))
}

/// What a press on one row means this frame.
#[derive(Clone, Copy, Debug, Default)]
pub(in crate::app) struct RowPress {
    /// A long press completed on this frame.
    pub fired: bool,
    /// A long press completed earlier and the finger is still down.
    pub held: bool,
    /// The release belongs to a long press, so it must not preview the item.
    pub suppress_click: bool,
    /// How far the hold has progressed, 0 to 1, for the progress line.
    pub progress: f32,
}

/// Track a press on one row and report when it becomes a long press.
///
/// A mouse press never becomes a long press: on a desktop the context menu
/// opens from a secondary click, and dragging starts immediately. A touch is
/// gated until it has stayed within [`super::gesture::LONG_PRESS_SLOP`] for
/// [`super::gesture::LONG_PRESS_SECONDS`]. The state lives in egui's temporary
/// data store, keyed by the row's response id, so it survives across frames
/// and disappears when the row is not drawn.
pub(in crate::app) fn long_press_row(ui: &egui::Ui, response: &egui::Response) -> RowPress {
    let id = response.id.with("row-long-press");
    let now = ui.input(|input| input.time);
    let pointer = ui.input(|input| input.pointer.interact_pos());
    let down = response.is_pointer_button_down_on();
    let touch = ui.input(|input| input.any_touches());
    let mut press = ui.data(|data| data.get_temp::<LongPress>(id));
    let mut result = RowPress::default();

    if down && (touch || press.is_some()) {
        let mut press = press.take().unwrap_or_default();
        if press.phase() == PressPhase::Idle {
            if let Some(pos) = pointer {
                press.begin(now, pos.x, pos.y);
            }
        } else if let Some(pos) = pointer {
            let before = press.phase();
            let after = press.update(now, pos.x, pos.y);
            if before != PressPhase::Fired && after == PressPhase::Fired {
                result.fired = true;
            }
        }
        result.held = press.phase() == PressPhase::Fired;
        result.progress = press.progress(now);
        if press.phase() == PressPhase::Holding {
            // Keep the progress line animating while the finger is still.
            ui.ctx().request_repaint();
        }
        ui.data_mut(|data| data.insert_temp(id, press));
    } else if let Some(mut press) = press {
        // Keep the state while the finger is still down but has moved off the
        // row: a drag must not end the press. The release decides whether this
        // was a long press on the row or a drag away from it.
        if ui.input(|input| input.pointer.any_down()) {
            result.held = press.phase() == PressPhase::Fired;
            ui.data_mut(|data| data.insert_temp(id, press));
        } else {
            let end = press.end();
            let released_on_row =
                pointer.is_some_and(|pos| response.rect.expand(12.0).contains(pos));
            result.suppress_click = end == PressEnd::LongPress && released_on_row;
            result.held = result.suppress_click;
            ui.data_mut(|data| data.remove::<LongPress>(id));
        }
    }

    result
}

/// Whether a text field lost focus because Enter was pressed.
pub(in crate::app) fn submitted(ui: &egui::Ui, response: &egui::Response) -> bool {
    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))
}

/// A single-line search field that reports Enter as a submit action.
pub(in crate::app) fn empty_label<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

pub(in crate::app) fn combo_option(
    ui: &mut egui::Ui,
    id: &str,
    value: &mut Option<String>,
    options: &[(&str, &str)],
    compact: bool,
) -> egui::Rect {
    let selected = value
        .as_deref()
        .and_then(|current| {
            options
                .iter()
                .find(|(option, _)| *option == current)
                .map(|(_, label)| *label)
        })
        .unwrap_or_else(|| options.first().map(|(_, label)| *label).unwrap_or(""));
    let combo = egui::ComboBox::from_id_salt(id);
    let combo = if compact {
        combo.width((ui.available_width() - 12.0).max(96.0))
    } else {
        combo
    };
    combo
        .selected_text(selected)
        .show_ui(ui, |ui| {
            for (option, label) in options {
                let is_selected = value.as_deref().unwrap_or("") == *option;
                if ui.selectable_label(is_selected, *label).clicked() {
                    *value = if option.is_empty() {
                        None
                    } else {
                        Some((*option).to_string())
                    };
                }
            }
        })
        .response
        .rect
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_action_row_divides_its_width_without_exceeding_it() {
        let width = action_width(600.0, 8.0, 3, false);
        assert_eq!(width, (600.0 - 16.0) / 3.0);
        assert!(width * 3.0 + 8.0 * 2.0 <= 600.0);
        // One button still fills the whole row.
        assert_eq!(action_width(600.0, 8.0, 1, false), 600.0);
    }

    #[test]
    fn a_filled_button_row_ends_at_the_content_edge() {
        // 360 points is the minimum window width; three equal buttons plus the
        // two gaps must add up to the row exactly.
        let width = filled_row_width(344.0, 8.0, 3);
        assert!((width * 3.0 + 8.0 * 2.0 - 344.0).abs() < 0.01);
        assert!((width - 109.333).abs() < 0.01);
        // One button still fills the whole row.
        assert_eq!(filled_row_width(344.0, 8.0, 1), 344.0);
        // Degenerate rows stay finite instead of painting a negative width.
        assert_eq!(filled_row_width(f32::NAN, 8.0, 3), 0.0);
        assert_eq!(filled_row_width(10.0, 8.0, 3), 0.0);
        assert_eq!(filled_row_width(344.0, f32::INFINITY, 3), 0.0);
    }

    #[test]
    fn a_phone_stacks_every_action_at_full_width() {
        // 360 points is the minimum window width; the stacked buttons must
        // span it exactly and keep the touch target, not share it.
        assert_eq!(action_width(360.0, 8.0, 3, true), 360.0);
        // A desktop column too narrow for readable side-by-side labels stacks
        // too, so a long label never clips.
        assert_eq!(action_width(300.0, 8.0, 3, false), 300.0);
    }

    #[test]
    fn degenerate_action_widths_stay_finite() {
        assert_eq!(action_width(f32::NAN, 8.0, 3, false), 0.0);
        assert_eq!(action_width(f32::INFINITY, 8.0, 3, false), 0.0);
        assert_eq!(action_width(-24.0, 8.0, 3, false), 0.0);
        assert_eq!(action_width(600.0, 8.0, 0, false), 600.0);
    }

    #[test]
    fn a_resizable_pane_keeps_a_120_point_floor() {
        assert_eq!(clamp_pane_width(40.0, 900.0), MIN_PANE_WIDTH);
        assert_eq!(clamp_pane_width(300.0, 900.0), 300.0);
        // The other pane keeps its floor plus the splitter.
        assert_eq!(
            clamp_pane_width(900.0, 600.0),
            600.0 - MIN_PANE_WIDTH - SPLITTER_WIDTH
        );
        // A viewport that cannot fit two panes still returns the floor, never
        // a negative or NaN width.
        assert_eq!(clamp_pane_width(600.0, 200.0), MIN_PANE_WIDTH);
        assert_eq!(clamp_pane_width(f32::NAN, 900.0), MIN_PANE_WIDTH);
        assert_eq!(clamp_pane_width(300.0, f32::NAN), MIN_PANE_WIDTH);
        assert_eq!(clamp_pane_width(300.0, 0.0), MIN_PANE_WIDTH);
    }
}
