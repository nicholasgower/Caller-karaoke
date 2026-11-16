//! Themes for Contra Karaoke: dark grey (the default) and a warm light cream.

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
    /// Fill behind the tab row and the wordmark.
    ///
    /// The header is a panel outside the central panel, so it only shows what
    /// it paints itself; without a fill it exposed the window's clear colour
    /// (near-black), which left the dark wordmark unreadable on a black strip.
    pub header_bg: egui::Color32,
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
    // Matches the window clear colour the transparent header used to expose, so
    // the dark header keeps the look it has always had.
    header_bg: egui::Color32::from_rgb(12, 12, 12),
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

/// The light theme is the warm cream of the ContraDB site (`#faf7e0` page,
/// `#f0f0ea` navigation) rather than white or a cold grey: stage lights make
/// pure white glare, and a warm paper tone keeps the black text calm. The page,
/// cards, and fields are a shade deeper than ContraDB's own page so the tab row
/// and the call cards still separate from the background.
const LIGHT: Palette = Palette {
    // The header shares the page fill; buttons and the separator carry the
    // structure. A darker header band made the inactive buttons look disabled.
    header_bg: egui::Color32::from_rgb(236, 231, 214),
    call_active_bg: egui::Color32::from_rgb(211, 204, 180),
    call_upcoming_bg: egui::Color32::from_rgb(246, 243, 230),
    call_now_bg: egui::Color32::from_rgb(198, 228, 203),
    upcoming_move_bg: egui::Color32::from_rgb(239, 211, 201),
    muted: egui::Color32::from_rgb(86, 81, 68),
    on_card: egui::Color32::from_rgb(33, 31, 24),
    executing: egui::Color32::from_rgb(88, 81, 66),
    subtle: egui::Color32::from_rgb(105, 98, 81),
    warning: egui::Color32::from_rgb(122, 90, 32),
    error: egui::Color32::from_rgb(159, 44, 44),
};

/// The palette that matches `dark_mode`.
pub fn palette(dark_mode: bool) -> &'static Palette {
    if dark_mode { &DARK } else { &LIGHT }
}

fn palette_for(ui: &egui::Ui) -> &'static Palette {
    palette(ui.visuals().dark_mode)
}

/// Fill behind the tab row, including the wordmark and the Preferences gear.
pub fn header_bg(ui: &egui::Ui) -> egui::Color32 {
    palette_for(ui).header_bg
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
            // Fields sit a shade lighter than the page, like the white tables
            // on ContraDB's cream background.
            egui::Color32::from_rgb(248, 244, 231),
            egui::Color32::from_rgb(236, 231, 214),
            egui::Color32::from_rgb(217, 211, 189),
            egui::Color32::from_rgb(205, 198, 175),
            egui::Color32::from_rgb(190, 183, 157),
            egui::Color32::from_rgb(38, 36, 28),
            egui::Color32::from_rgb(86, 81, 68),
            egui::Color32::from_rgb(174, 165, 139),
        )
    };

    visuals.override_text_color = Some(text);
    visuals.panel_fill = panel;
    visuals.window_fill = panel;
    visuals.extreme_bg_color = bg;
    visuals.faint_bg_color = if dark_mode {
        egui::Color32::from_rgb(28, 28, 28)
    } else {
        egui::Color32::from_rgb(228, 223, 205)
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

    /// The light page follows ContraDB's warm cream, not white: a pure white
    /// page glares under stage lighting and leaves cards and fields nowhere to
    /// sit. A warm tone also keeps the page from looking like a blank sheet on
    /// a projector.
    #[test]
    fn light_surfaces_are_warm_cream_not_white() {
        let visuals = visuals_for(false);
        for (name, surface) in [
            ("panel", visuals.panel_fill),
            ("field", visuals.extreme_bg_color),
            ("faint", visuals.faint_bg_color),
        ] {
            assert_ne!(surface, egui::Color32::WHITE, "{name} is pure white");
            assert!(
                surface.r() >= surface.g() && surface.g() >= surface.b(),
                "{name}={surface:?} is not a warm neutral"
            );
            let lum = luminance(surface);
            assert!(
                (0.5..0.97).contains(&lum),
                "{name}={surface:?} luminance={lum} is outside the light cream range"
            );
        }
        // Fields read as paper on the page, mirroring the dark theme where the
        // extreme background is darker than the panel.
        assert!(
            luminance(visuals.extreme_bg_color) > luminance(visuals.panel_fill),
            "light fields should be lighter than the panel"
        );
        assert!(
            luminance(visuals.panel_fill) < luminance(egui::Color32::from_gray(242)),
            "the light page should be calmer than the old near-white panel"
        );
    }

    /// Every surface that carries text has to reach 4.5:1 for body text; muted
    /// text is allowed to fall to 3:1 only on pressed controls, where it is a
    /// border and not a label.
    #[test]
    fn light_text_contrasts_with_every_light_surface() {
        let visuals = visuals_for(false);
        let palette = palette(false);
        let text = visuals
            .override_text_color
            .expect("the theme sets an explicit text colour");
        let strong = visuals.strong_text_color();
        let surfaces = [
            ("page", visuals.panel_fill, 4.5),
            ("field", visuals.extreme_bg_color, 4.5),
            ("header", palette.header_bg, 4.5),
            ("call card", palette.call_active_bg, 4.5),
            ("upcoming card", palette.call_upcoming_bg, 4.5),
            ("button", visuals.widgets.inactive.bg_fill, 3.0),
            ("button hover", visuals.widgets.hovered.bg_fill, 3.0),
            ("button active", visuals.widgets.active.bg_fill, 3.0),
        ];
        for (name, background, muted_min) in surfaces {
            for (role, foreground, min) in [
                ("text", text, 4.5),
                // The wordmark is a heading, which egui draws in `strong`.
                ("strong text", strong, 4.5),
                ("muted", palette.muted, muted_min),
            ] {
                let ratio = contrast(background, foreground);
                assert!(
                    ratio >= min,
                    "{name} {role} {foreground:?} on {background:?} ratio={ratio}, want {min}"
                );
            }
        }
    }

    /// The tab row once painted nothing, so the light theme showed the window's
    /// near-black clear colour behind the wordmark. The header fill must be a
    /// light surface in the light theme and keep the dark header as it was.
    #[test]
    fn header_fill_carries_the_wordmark_in_both_themes() {
        let light = palette(false).header_bg;
        let dark = palette(true).header_bg;
        assert!(
            luminance(light) > 0.5,
            "light header={light:?} would swallow the dark wordmark"
        );
        assert!(
            luminance(dark) < 0.1,
            "dark header={dark:?} changed the dark theme's header"
        );
        for dark_mode in [true, false] {
            let visuals = visuals_for(dark_mode);
            let header = palette(dark_mode).header_bg;
            let ratio = contrast(header, visuals.strong_text_color());
            assert!(
                ratio >= 4.5,
                "dark_mode={dark_mode} header text ratio={ratio}"
            );
        }
    }
}
