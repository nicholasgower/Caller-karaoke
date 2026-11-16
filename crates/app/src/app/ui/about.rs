use crate::app::*;
use eframe::egui;

/// The About section, the last section of Preferences.
///
/// It is part of the scrolling Preferences page rather than a modal window:
/// reference text should not interrupt the session on every screen or swallow
/// the transport shortcuts while it is open. Preferences already applies the
/// phone spacing, so this only adds the separating rule and the content.
pub(in crate::app) fn about(app: &mut ContraApp, ui: &mut egui::Ui, _compact: bool) {
    let top_left = ui.cursor().min;
    ui.separator();
    ui.heading("About Contra Karaoke");
    ui.label(
        egui::RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
            .color(theme::muted(ui)),
    );
    ui.add_space(8.0);
    ui.label(
        "A contra dance caller trainer. It plays ABC-sourced music in sync with the \
         dance card and shows each call early enough that dancers hear it before they \
         move.",
    );
    ui.add_space(8.0);
    ui.label(egui::RichText::new("By Nicholas Gower").strong());
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Project links").strong());
    ui.hyperlink_to(
        egui::RichText::new("GitHub repository").underline(),
        REPOSITORY_URL,
    );
    ui.hyperlink_to(
        egui::RichText::new("Documentation").underline(),
        DOCUMENTATION_URL,
    );
    if app.prefs.glossary_enabled
        && ui
            .link(egui::RichText::new("Open the dance glossary").underline())
            .on_hover_text("Short definitions of the terms in a dance card and its calls")
            .clicked()
    {
        app.open_glossary(None, Screen::Preferences);
    }
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "MIT licensed. Default instruments are a Fluid R3 GM subset by Frank Wen \
             (MIT). Bundled defaults are attributed in CREDITS.md.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new(
            "The saved-tune ABC editor was inspired by Michael Eskin's abctools \
             (github.com/seisiuneer/abctools). This is an independent, minimal \
             implementation, not a port of that project.",
        )
        .small()
        .color(theme::muted(ui)),
    );
    // Breathing room below the last line so the credits do not sit on the
    // bottom edge of the scrolling page.
    ui.add_space(8.0);

    // Documentation shots ask for this section rather than the whole page; the
    // request is consumed so a person scrolling Preferences is never yanked
    // back to the bottom.
    if app.about_reveal {
        app.about_reveal = false;
        let section = egui::Rect::from_min_max(top_left, ui.min_rect().max);
        ui.scroll_to_rect_animation(
            section,
            Some(egui::Align::BOTTOM),
            egui::style::ScrollAnimation::none(),
        );
    }
}
