//! The in-app dance glossary.
//!
//! Calls and dance cards are ordinary text until this module turns a recognised
//! term into a link. The reference page itself is a scrolling list so it fits
//! the same phone layout as the alias editor, and the filter is the only input
//! besides Back and the source link.

use super::*;
use crate::app::*;
use crate::theme;
use contra_core::glossary::{self, GlossaryEntry};
use eframe::egui;

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
    style: impl Fn(&str) -> egui::RichText,
) -> Option<&'static GlossaryEntry> {
    let mut clicked = None;
    let matches = glossary::scan(text);
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
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(format!("Glossary: {}", found.entry.term));
            if response.clicked() {
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
    /// Open the glossary, optionally highlighting one entry.
    ///
    /// `return_to` is where Back and Escape go, so a term selected on the
    /// session returns to the session while the About link returns to
    /// Preferences.
    pub(in crate::app) fn open_glossary(&mut self, term: Option<&str>, return_to: Screen) {
        self.glossary_query.clear();
        self.glossary_focus = term.map(str::to_owned);
        self.glossary_return = if return_to == Screen::Glossary {
            Screen::Session
        } else {
            return_to
        };
        self.glossary_epoch = self.glossary_epoch.wrapping_add(1);
        self.screen = Screen::Glossary;
    }

    /// Leave the glossary for the screen that opened it.
    pub(in crate::app) fn leave_glossary(&mut self) {
        self.screen = self.glossary_return;
    }

    pub(in crate::app) fn glossary_ui(&mut self, ui: &mut egui::Ui) {
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
        let back_label = if compact {
            "< Back"
        } else {
            match self.glossary_return {
                Screen::Preferences => "< Back to preferences",
                Screen::DancePicker => "< Back to saved dances",
                Screen::DanceDownload => "< Back to download dance",
                Screen::TunePicker => "< Back to saved tunes",
                Screen::TuneDownload => "< Back to download tune",
                _ => "< Back to session",
            }
        };
        ui.horizontal_wrapped(|ui| {
            if ui.button(back_label).on_hover_text("Back (Esc)").clicked() {
                self.leave_glossary();
            }
            ui.heading("Dance glossary");
        });
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
                if !self.glossary_query.is_empty() && ui.small_button("Clear").clicked() {
                    self.glossary_query.clear();
                }
            });
        }

        let focus = self.glossary_focus.clone();
        // Scroll to the selected entry once; the stored focus is also used for
        // the first frame's highlight so a caller sees which link they followed.
        let scroll_to = self.glossary_focus.take();
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

        let body = |ui: &mut egui::Ui| {
            let mut focus_response = None;
            for entry in entries {
                let selected = focus.as_deref() == Some(entry.term);
                let frame = if selected {
                    egui::Frame::group(ui.style()).fill(theme::call_active_bg(ui))
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
    }
}
