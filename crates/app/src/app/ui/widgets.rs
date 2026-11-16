use crate::theme;
use eframe::egui;

/// The narrowest a desktop action button may get before the row stacks.
///
/// Below this a two- or three-button row would squeeze labels such as
/// "Remove from saved dances" into an unreadable column, so the callers give
/// every button the full row width instead.
const MIN_ACTION_WIDTH: f32 = 132.0;

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

/// A row action button that fills `width` and is always touch sized.
///
/// Every button that shares a row in a saved-item list must go through this
/// helper (or [`row_menu_button`]), otherwise egui mixes sizes and the row has
/// two different heights.
pub(in crate::app) fn row_button(ui: &mut egui::Ui, width: f32, label: &str) -> egui::Response {
    ui.add_sized(
        [width, theme::TOUCH_TARGET],
        egui::Button::new(label).wrap_mode(egui::TextWrapMode::Truncate),
    )
}

/// A menu button that matches [`row_button`] so the actions in one row line up.
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

/// Whether a text field lost focus because Enter was pressed.
pub(in crate::app) fn submitted(ui: &egui::Ui, response: &egui::Response) -> bool {
    response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter))
}

/// A single-line search field that reports Enter as a submit action.
pub(in crate::app) fn submit_text_edit(ui: &mut egui::Ui, text: &mut String) -> bool {
    let response = ui.text_edit_singleline(text);
    submitted(ui, &response)
}

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
) {
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
    combo.selected_text(selected).show_ui(ui, |ui| {
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
    });
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
}
