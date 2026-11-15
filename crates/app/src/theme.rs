//! Grey theme for Contra Karaoke.

use contra_core::Theme;
use eframe::egui;

/// Minimum height of a control in the phone session layout, in points.
///
/// ROADMAP §8.2 sets 44 px for touch targets. The compact session tab applies
/// this to its sliders, transport buttons, toggles, and tab row; the desktop
/// layout keeps egui's 40 pt rows so it stays dense.
pub const TOUCH_TARGET: f32 = 44.0;

/// Colours for the parts of the UI that paint their own backgrounds and text
/// instead of relying on egui's widget visuals.
///
/// Each theme carries its own palette: the dark card greys read as solid black
/// blocks on a light background, and dark-theme text would disappear on them.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Background of the now-playing call card and selected results.
    pub call_active_bg: egui::Color32,
    /// Background of an upcoming figure card.
    pub call_upcoming_bg: egui::Color32,
    /// Background of the prompt card while the caller must speak now.
    pub call_now_bg: egui::Color32,
    /// Background of the prompt card while it looks ahead to the next move.
    pub upcoming_move_bg: egui::Color32,
    /// Secondary and hint text.
    pub muted: egui::Color32,
    /// Strong text drawn on a call or result card.
    pub on_card: egui::Color32,
    /// A call whose move has already started.
    pub executing: egui::Color32,
    /// Small, low-emphasis text such as the original card wording.
    pub subtle: egui::Color32,
    /// Non-blocking warning text.
    pub warning: egui::Color32,
    /// Error text and banners.
    pub error: egui::Color32,
}

const DARK: Palette = Palette {
    call_active_bg: egui::Color32::from_rgb(58, 58, 58),
    call_upcoming_bg: egui::Color32::from_rgb(38, 38, 38),
    call_now_bg: egui::Color32::from_rgb(30, 74, 46),
    upcoming_move_bg: egui::Color32::from_rgb(96, 36, 36),
    muted: egui::Color32::from_rgb(154, 154, 154),
    on_card: egui::Color32::WHITE,
    executing: egui::Color32::from_gray(180),
    subtle: egui::Color32::from_gray(120),
    warning: egui::Color32::from_rgb(205, 180, 135),
    error: egui::Color32::from_rgb(220, 120, 120),
};

const LIGHT: Palette = Palette {
    call_active_bg: egui::Color32::from_rgb(214, 214, 214),
    call_upcoming_bg: egui::Color32::from_rgb(230, 230, 230),
    call_now_bg: egui::Color32::from_rgb(203, 232, 210),
    upcoming_move_bg: egui::Color32::from_rgb(244, 208, 208),
    muted: egui::Color32::from_rgb(92, 92, 92),
    on_card: egui::Color32::from_rgb(24, 24, 24),
    executing: egui::Color32::from_gray(96),
    subtle: egui::Color32::from_gray(104),
    warning: egui::Color32::from_rgb(120, 94, 46),
    error: egui::Color32::from_rgb(160, 40, 40),
};

/// The palette that matches `dark_mode`.
pub fn palette(dark_mode: bool) -> &'static Palette {
    if dark_mode { &DARK } else { &LIGHT }
}

fn palette_for(ui: &egui::Ui) -> &'static Palette {
    palette(ui.visuals().dark_mode)
}

pub fn call_active_bg(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).call_active_bg
}

pub fn call_upcoming_bg(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).call_upcoming_bg
}

/// Card background cue for "speak this now".
pub fn call_now_bg(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).call_now_bg
}

/// Card background cue for "the move after this one".
pub fn upcoming_move_bg(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).upcoming_move_bg
}

pub fn muted(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).muted
}

pub fn on_card(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).on_card
}

pub fn executing(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).executing
}

pub fn subtle(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).subtle
}

pub fn warning(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).warning
}

pub fn error(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).error
}

pub fn apply(ctx: &egui::Context, theme: Theme, animations: bool) {
    let dark = visuals_for(true);
    let light = visuals_for(false);
    ctx.set_theme(match theme {
        Theme::Dark => egui::ThemePreference::Dark,
        Theme::Light => egui::ThemePreference::Light,
    });
    ctx.set_visuals_of(egui::Theme::Dark, dark);
    ctx.set_visuals_of(egui::Theme::Light, light);
    let animation_time = if animations { 0.2 } else { 0.0 };
    for egui_theme in [egui::Theme::Dark, egui::Theme::Light] {
        ctx.style_mut_of(egui_theme, |style| {
            style.animation_time = animation_time;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Relative luminance of an sRGB channel (WCAG 2.x).
    fn channel(value: u8) -> f32 {
        let value = f32::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    fn luminance(color: egui::Color32) -> f32 {
        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    fn contrast(a: egui::Color32, b: egui::Color32) -> f32 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    #[test]
    fn light_cards_are_light_and_dark_cards_are_dark() {
        let dark = palette(true);
        assert!(luminance(dark.call_active_bg) < 0.1);
        assert!(luminance(dark.call_upcoming_bg) < 0.1);
        let light = palette(false);
        assert!(luminance(light.call_active_bg) > 0.5);
        assert!(luminance(light.call_upcoming_bg) > 0.5);
    }

    #[test]
    fn card_text_contrasts_with_every_card_background() {
        for dark_mode in [true, false] {
            let palette = palette(dark_mode);
            for background in [palette.call_active_bg, palette.call_upcoming_bg] {
                for (foreground, min) in [
                    (palette.on_card, 4.5),
                    (palette.muted, 3.0),
                    (palette.executing, 3.0),
                    // `subtle` is the smallest, lowest-emphasis hint text and
                    // intentionally sits below normal body-text contrast.
                    (palette.subtle, 2.5),
                ] {
                    let ratio = contrast(background, foreground);
                    assert!(
                        ratio >= min,
                        "dark_mode={dark_mode} background={background:?} foreground={foreground:?} ratio={ratio}"
                    );
                }
            }
        }
    }

    /// The prompt card only ever draws `on_card` and `muted` text (the label,
    /// the call wording and the beat hint), and in green/red it must stay as
    /// legible as the grey card it replaces.
    #[test]
    fn prompt_card_text_contrasts_with_the_green_and_red_fills() {
        for dark_mode in [true, false] {
            let palette = palette(dark_mode);
            for background in [palette.call_now_bg, palette.upcoming_move_bg] {
                for (foreground, min) in [(palette.on_card, 4.5), (palette.muted, 3.0)] {
                    let ratio = contrast(background, foreground);
                    assert!(
                        ratio >= min,
                        "dark_mode={dark_mode} background={background:?} foreground={foreground:?} ratio={ratio}"
                    );
                }
            }
        }
    }

    /// Green means "speak now" and red means "look ahead"; the hue has to
    /// survive as a cue, so each fill must be recognisably its own colour.
    #[test]
    fn prompt_card_fills_are_green_for_call_now_and_red_for_upcoming() {
        for dark_mode in [true, false] {
            let palette = palette(dark_mode);
            let now = palette.call_now_bg;
            let upcoming = palette.upcoming_move_bg;
            assert!(
                now.g() > now.r() && now.g() > now.b(),
                "dark_mode={dark_mode} call_now_bg={now:?} is not green"
            );
            assert!(
                upcoming.r() > upcoming.g() && upcoming.r() > upcoming.b(),
                "dark_mode={dark_mode} upcoming_move_bg={upcoming:?} is not red"
            );
        }
    }

    #[test]
    fn warning_and_error_read_on_the_panel() {
        for dark_mode in [true, false] {
            let panel = visuals_for(dark_mode).panel_fill;
            let palette = palette(dark_mode);
            for foreground in [palette.warning, palette.error, palette.muted] {
                let ratio = contrast(panel, foreground);
                assert!(
                    ratio >= 3.0,
                    "dark_mode={dark_mode} foreground={foreground:?} ratio={ratio}"
                );
            }
        }
    }
}
