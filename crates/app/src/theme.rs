//! Grey theme for Contra Karaoke.

use contra_core::Theme;
use eframe::egui;

pub const CALL_ACTIVE_BG: egui::Color32 = egui::Color32::from_rgb(58, 58, 58);
pub const CALL_UPCOMING_BG: egui::Color32 = egui::Color32::from_rgb(38, 38, 38);
pub const MUTED: egui::Color32 = egui::Color32::from_rgb(154, 154, 154);

pub fn apply(ctx: &egui::Context, theme: Theme) {
    let dark = visuals_for(true);
    let light = visuals_for(false);
    ctx.set_theme(match theme {
        Theme::Dark => egui::ThemePreference::Dark,
        Theme::Light => egui::ThemePreference::Light,
    });
    ctx.set_visuals_of(egui::Theme::Dark, dark);
    ctx.set_visuals_of(egui::Theme::Light, light);
    for egui_theme in [egui::Theme::Dark, egui::Theme::Light] {
        ctx.style_mut_of(egui_theme, |style| {
            style.spacing.item_spacing = egui::vec2(8.0, 8.0);
            style.spacing.button_padding = egui::vec2(12.0, 8.0);
            style.spacing.interact_size.y = 40.0;
            style
                .text_styles
                .insert(egui::TextStyle::Heading, egui::FontId::proportional(26.0));
            style
                .text_styles
                .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
            style
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
            style
                .text_styles
                .insert(egui::TextStyle::Small, egui::FontId::proportional(13.0));
        });
    }
}

fn visuals_for(dark_mode: bool) -> egui::Visuals {
    let mut visuals = if dark_mode {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    let (bg, panel, button, hover, active, text, muted, border) = if dark_mode {
        (
            egui::Color32::from_rgb(20, 20, 20),
            egui::Color32::from_rgb(35, 35, 35),
            egui::Color32::from_rgb(46, 46, 46),
            egui::Color32::from_rgb(58, 58, 58),
            egui::Color32::from_rgb(74, 74, 74),
            egui::Color32::from_rgb(230, 230, 230),
            egui::Color32::from_rgb(154, 154, 154),
            egui::Color32::from_rgb(58, 58, 58),
        )
    } else {
        (
            egui::Color32::from_rgb(230, 230, 230),
            egui::Color32::from_rgb(242, 242, 242),
            egui::Color32::from_rgb(214, 214, 214),
            egui::Color32::from_rgb(200, 200, 200),
            egui::Color32::from_rgb(184, 184, 184),
            egui::Color32::from_rgb(28, 28, 28),
            egui::Color32::from_rgb(92, 92, 92),
            egui::Color32::from_rgb(188, 188, 188),
        )
    };

    visuals.override_text_color = Some(text);
    visuals.panel_fill = panel;
    visuals.window_fill = panel;
    visuals.extreme_bg_color = bg;
    visuals.faint_bg_color = if dark_mode {
        egui::Color32::from_rgb(28, 28, 28)
    } else {
        egui::Color32::from_rgb(224, 224, 224)
    };
    visuals.code_bg_color = bg;
    visuals.hyperlink_color = text;
    visuals.selection.bg_fill = active;
    visuals.selection.stroke = egui::Stroke::new(1.0, text);
    visuals.widgets.noninteractive.bg_fill = panel;
    visuals.widgets.noninteractive.weak_bg_fill = panel;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, border);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, muted);
    visuals.widgets.inactive.bg_fill = button;
    visuals.widgets.inactive.weak_bg_fill = button;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, border);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text);
    visuals.widgets.hovered.bg_fill = hover;
    visuals.widgets.hovered.weak_bg_fill = hover;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, muted);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text);
    visuals.widgets.active.bg_fill = active;
    visuals.widgets.active.weak_bg_fill = active;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, text);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, text);
    visuals.window_corner_radius = egui::CornerRadius::same(8);
    visuals.menu_corner_radius = egui::CornerRadius::same(6);
    visuals
}
