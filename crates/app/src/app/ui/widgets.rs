use eframe::egui;

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
    egui::ComboBox::from_id_salt(id)
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
        });
}
