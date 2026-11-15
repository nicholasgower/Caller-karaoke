use crate::app::*;
use eframe::egui;

impl ContraApp {
    /// The About window: what the app is, who made it, and the project links.
    ///
    /// It is a modal over every screen so it can be opened from anywhere and
    /// closes on its own button, a click outside, or Escape.
    pub(in crate::app) fn about_ui(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }
        let mut close = false;
        // Keep the window inside a phone-width viewport: the modal area is not
        // constrained to the screen by default.
        let max_width = (ctx.content_rect().width() - 48.0).clamp(240.0, 440.0);
        let response = egui::Modal::new(egui::Id::unique("about-window")).show(ctx, |ui| {
            ui.set_max_width(max_width);
            ui.heading("Contra Karaoke");
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
            if ui.button("Close").clicked() {
                close = true;
            }
        });
        if close || response.should_close() {
            self.about_open = false;
        }
    }
}
