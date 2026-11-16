use eframe::egui;

/// A row action button: touch sized on a phone, dense on a desktop.
///
/// Every button that shares a row in a saved-item list must go through this
/// helper (or [`row_menu_button`]), otherwise egui mixes a small button with a
/// normal one and the row has two different heights.
pub(in crate::app) fn row_button(ui: &mut egui::Ui, compact: bool, label: &str) -> egui::Response {
    if compact {
        ui.button(label)
    } else {
        ui.small_button(label)
    }
}

/// A menu button that matches [`row_button`] so the actions in one row line up.
pub(in crate::app) fn row_menu_button(
    ui: &mut egui::Ui,
    compact: bool,
    label: impl Into<egui::WidgetText>,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let button = egui::Button::new(label);
    let button = if compact { button } else { button.small() };
    egui::containers::menu::MenuButton::from_button(button)
        .ui(ui, add_contents)
        .0
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

pub(in crate::app) fn formation_label(value: Option<&str>) -> &'static str {
    match value.unwrap_or("") {
        "improper" => "Improper",
        "becket" => "Becket",
        "proper" => "Proper",
        "everything else" => "Everything else",
        _ => "Any formation",
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
