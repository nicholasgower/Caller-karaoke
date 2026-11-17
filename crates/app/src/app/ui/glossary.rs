//! The in-app dance glossary.
//!
//! Calls and dance cards are ordinary text until this module turns a recognised
//! term into a link. The reference page is the fourth top-row tab, to the right
//! of **Select tune**, and is also opened at a specific entry when a caller
//! follows a term. The page itself is a scrolling list so it fits the same phone
//! layout as the alias editor, and the filter is the only input besides the
//! source link.

use super::*;
use crate::app::*;
use crate::theme;
use contra_core::glossary::{self, GlossaryEntry};
use eframe::egui;
use std::time::Duration;

/// How long a mobile definition card lingers after the finger lifts.
const TIP_LINGER_SECONDS: f64 = 4.0;
/// Widest the definition card may become, in points.
const TIP_MAX_WIDTH: f32 = 420.0;
/// Side margin kept free around the definition card on a narrow screen.
const TIP_SCREEN_MARGIN: f32 = 12.0;

/// The definition currently shown above the session on a touch screen.
#[derive(Clone, Copy, Debug)]
struct GlossaryTip {
    entry: &'static GlossaryEntry,
    shown_at: f64,
}

/// The definition card's content, shared by the hover tooltip and the phone
/// card so both always show the same words.
fn definition_body(ui: &mut egui::Ui, entry: &'static GlossaryEntry) {
    ui.label(egui::RichText::new(entry.term).strong());
    ui.label(entry.definition);
}

fn tip_id() -> egui::Id {
    egui::Id::unique("glossary-definition-tip")
}

/// Show a definition at the top of the screen.
///
/// The card is drawn after the active screen so a scroll area cannot clip it.
/// It keeps the last definition visible briefly after a release, which gives a
/// caller time to read it without keeping a finger on the card.
pub(in crate::app) fn glossary_tooltip(ctx: &egui::Context, top: f32) {
    let Some(tip) = ctx.data(|data| data.get_temp::<GlossaryTip>(tip_id())) else {
        return;
    };
    let now = ctx.input(|input| input.time);
    let pointer_down = ctx.input(|input| input.pointer.any_down());
    if tip_expired(now, tip.shown_at, pointer_down) {
        clear_tooltip(ctx);
        return;
    }
    if !pointer_down {
        ctx.request_repaint_after(Duration::from_secs_f64(
            (TIP_LINGER_SECONDS - (now - tip.shown_at)).max(0.0),
        ));
    }

    let width = tooltip_width(ctx.content_rect().width());
    egui::Area::new(egui::Id::unique("glossary-definition-tip-area"))
        .order(egui::Order::Tooltip)
        .interactable(false)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, top))
        .show(ctx, |ui| {
            ui.set_max_width(width);
            egui::Frame::popup(ui.style())
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_max_width(width);
                    definition_body(ui, tip.entry);
                });
        });
}

/// Hide the top card immediately, for example when the screen changes.
pub(in crate::app) fn clear_tooltip(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove::<GlossaryTip>(tip_id()));
}

/// Put a definition card on screen for a documentation shot.
///
/// The headless renderer has no finger to rest on a term, so it sets the same
/// state the long-press path writes. This is not exposed in a release build.
#[cfg(any(feature = "headless", test))]
pub(in crate::app) fn show_tooltip_preview(ctx: &egui::Context, term: &str) {
    let Some(entry) = glossary::find(term) else {
        return;
    };
    let now = ctx.input(|input| input.time);
    ctx.data_mut(|data| {
        data.insert_temp(
            tip_id(),
            GlossaryTip {
                entry,
                shown_at: now,
            },
        );
    });
}

/// Whether a released definition card has stayed up long enough to hide.
fn tip_expired(now: f64, shown_at: f64, pointer_down: bool) -> bool {
    !pointer_down && now - shown_at >= TIP_LINGER_SECONDS
}

/// The card width for a viewport, leaving a margin on phones.
fn tooltip_width(screen_width: f32) -> f32 {
    (screen_width - 2.0 * TIP_SCREEN_MARGIN).clamp(180.0, TIP_MAX_WIDTH)
}

/// Draw `text`, underlining every glossary term and returning the entry the
/// caller selected.
///
/// `style` builds the same text style for both ordinary words and links, so a
/// call keeps its size, weight, and colour on every background. The caller
/// supplies the style rather than this helper because the prompt card changes
/// those attributes with the call phase.
pub(in crate::app) fn glossary_text(
    ui: &mut egui::Ui,
    text: &str,
    enabled: bool,
    style: impl Fn(&str) -> egui::RichText,
) -> Option<&'static GlossaryEntry> {
    let mut clicked = None;
    let matches = if enabled {
        glossary::scan(text)
    } else {
        Vec::new()
    };
    let touch = ui.input(|input| input.any_touches());
    ui.horizontal_wrapped(|ui| {
        // Words carry their own trailing spaces, so a wrapped line reads as one
        // run of text instead of a row of separately spaced labels.
        ui.spacing_mut().item_spacing.x = 0.0;
        let mut cursor = 0;
        for found in &matches {
            push_plain_text(ui, &text[cursor..found.start], &style);
            let link = style(&text[found.start..found.end]).underline();
            let response = ui
                .add(egui::Label::new(link).sense(egui::Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let press = long_press_row(ui, &response);
            if press.held {
                let now = ui.input(|input| input.time);
                ui.data_mut(|data| {
                    data.insert_temp(
                        tip_id(),
                        GlossaryTip {
                            entry: found.entry,
                            shown_at: now,
                        },
                    );
                });
            }
            let response = if touch {
                response
            } else {
                // A desktop tooltip belongs at the pointer, like any other
                // hover tooltip, not in the phone card at the top of the screen.
                response.on_hover_ui_at_pointer(|ui| {
                    ui.set_max_width(300.0);
                    definition_body(ui, found.entry);
                })
            };
            if response.clicked() && !press.suppress_click {
                clicked = Some(found.entry);
            }
            cursor = found.end;
        }
        push_plain_text(ui, &text[cursor..], &style);
    });
    clicked
}

/// Add a plain run as labels split at whitespace, so long call text wraps.
fn push_plain_text(ui: &mut egui::Ui, text: &str, style: &impl Fn(&str) -> egui::RichText) {
    for word in text.split_inclusive(char::is_whitespace) {
        if !word.is_empty() {
            ui.label(style(word));
        }
    }
}

impl ContraApp {
    /// Open the glossary tab, optionally highlighting one entry.
    ///
    /// A term selected in a call, on the dance card, or in a dance preview
    /// brings the caller here with that entry boxed and scrolled into view. The
    /// tab itself opens the same page and keeps the last selection, so a caller
    /// can leave and return without losing their place.
    pub(in crate::app) fn open_glossary(&mut self, term: Option<&str>) {
        if !self.prefs.glossary_enabled {
            return;
        }
        self.glossary_query.clear();
        self.glossary_focus = term.map(str::to_owned);
        self.glossary_focus_scrolled = false;
        self.glossary_epoch = self.glossary_epoch.wrapping_add(1);
        self.screen = Screen::Glossary;
    }

    pub(in crate::app) fn glossary_ui(&mut self, ui: &mut egui::Ui) {
        if !self.prefs.glossary_enabled {
            // The reference is optional. If it is switched off while its tab is
            // in front, fall back to the session rather than leaving a blank
            // page the tab row no longer offers.
            self.screen = Screen::Session;
            return;
        }
        let compact = compact(ui);
        apply_phone_spacing(ui, compact);
        if compact {
            egui::ScrollArea::vertical()
                .id_salt(("glossary-page", self.glossary_epoch))
                .auto_shrink([false, false])
                .show(ui, |ui| self.glossary_body(ui, true));
        } else {
            self.glossary_body(ui, false);
        }
    }

    fn glossary_body(&mut self, ui: &mut egui::Ui, compact: bool) {
        ui.heading("Dance glossary");
        ui.label(
            egui::RichText::new(
                "Select an underlined term in a call or on the dance card to open \
                 its entry here. Use the filter to browse the whole list.",
            )
            .color(theme::muted(ui)),
        );
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("Source:")
                    .small()
                    .color(theme::muted(ui)),
            );
            ui.hyperlink_to(
                egui::RichText::new(format!(
                    "{} glossary of contra dance figures",
                    glossary::SOURCE_NAME
                ))
                .small()
                .underline(),
                glossary::SOURCE_URL,
            );
        });
        ui.label(
            egui::RichText::new(glossary::SOURCE_NOTE)
                .small()
                .color(theme::muted(ui)),
        );
        ui.add_space(8.0);

        if compact {
            ui.label(
                egui::RichText::new("Filter terms")
                    .small()
                    .color(theme::muted(ui)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.glossary_query)
                    .hint_text("Filter terms")
                    .desired_width(f32::INFINITY),
            );
        } else {
            ui.horizontal(|ui| {
                ui.label("Filter terms");
                ui.add(
                    egui::TextEdit::singleline(&mut self.glossary_query)
                        .hint_text("Filter by name or meaning")
                        .desired_width(260.0),
                );
                if !self.glossary_query.is_empty() && ui.button("Clear").clicked() {
                    self.glossary_query.clear();
                }
            });
        }

        let focus = self.glossary_focus.clone();
        // Scroll to the selected entry once. The same focus keeps the emphasis
        // around its box for as long as the entry stays visible.
        let scroll_to = if self.glossary_focus_scrolled {
            None
        } else {
            focus.clone()
        };
        let filter = self.glossary_query.trim().to_lowercase();
        let entries: Vec<&GlossaryEntry> = glossary::ENTRIES
            .iter()
            .filter(|entry| {
                filter.is_empty()
                    || entry.term.to_lowercase().contains(&filter)
                    || entry
                        .aliases
                        .iter()
                        .any(|alias| alias.to_lowercase().contains(&filter))
                    || entry.definition.to_lowercase().contains(&filter)
            })
            .collect();

        let count = entries.len();
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(if count == 1 {
                "1 term".to_string()
            } else {
                format!("{count} terms")
            })
            .small()
            .color(theme::muted(ui)),
        );

        let mut scrolled = false;
        let body = |ui: &mut egui::Ui| {
            let mut focus_response = None;
            for entry in entries {
                let selected = focus.as_deref() == Some(entry.term);
                let frame = if selected {
                    // Keep the entry the caller followed visibly boxed, using
                    // the theme's neutral selection stroke rather than a colour
                    // that would compete with the card cues on the session.
                    egui::Frame::group(ui.style())
                        .fill(theme::call_active_bg(ui))
                        .stroke(egui::Stroke::new(2.0, ui.visuals().selection.stroke.color))
                } else {
                    egui::Frame::group(ui.style())
                };
                let response = frame
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new(entry.term).strong());
                        ui.label(entry.definition);
                    })
                    .response;
                if scroll_to.as_deref() == Some(entry.term) {
                    focus_response = Some(response);
                }
                ui.add_space(2.0);
            }
            if count == 0 {
                ui.label(
                    egui::RichText::new("No glossary term matches that filter.")
                        .color(theme::muted(ui)),
                );
            }
            if let Some(response) = focus_response {
                response.scroll_to_me(Some(egui::Align::Center));
                scrolled = true;
            }
        };

        if compact {
            body(ui);
        } else {
            let height = ui.available_height().max(200.0);
            egui::ScrollArea::vertical()
                .id_salt(("glossary-list", self.glossary_epoch))
                .max_height(height)
                .auto_shrink([false, false])
                .show(ui, body);
        }
        if scrolled {
            self.glossary_focus_scrolled = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_definition_tip_fits_inside_a_phone_screen() {
        assert!(tooltip_width(360.0) <= 360.0 - 2.0 * TIP_SCREEN_MARGIN);
        assert_eq!(tooltip_width(390.0), 390.0 - 2.0 * TIP_SCREEN_MARGIN);
        assert_eq!(tooltip_width(1280.0), TIP_MAX_WIDTH);
        // A degenerate viewport still gets a finite, readable card.
        assert_eq!(tooltip_width(0.0), 180.0);
    }

    #[test]
    fn a_released_tip_lingers_and_then_expires() {
        assert!(!tip_expired(3.9, 0.0, false));
        assert!(tip_expired(4.0, 0.0, false));
        // While the finger is still down the card stays up.
        assert!(!tip_expired(30.0, 0.0, true));
    }

    #[test]
    fn a_preview_tip_can_be_set_and_cleared() {
        let ctx = egui::Context::default();
        show_tooltip_preview(&ctx, "Swing");
        assert!(ctx.data(|data| data.get_temp::<GlossaryTip>(tip_id()).is_some()));
        clear_tooltip(&ctx);
        assert!(ctx.data(|data| data.get_temp::<GlossaryTip>(tip_id()).is_none()));
    }
}
