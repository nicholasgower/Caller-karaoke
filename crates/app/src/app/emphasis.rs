//! Documentation callouts drawn by the app itself.
//!
//! The documentation shots mark a control or a group of controls with a yellow
//! focus ring. Those rings used to be painted onto the PNGs by PhotoCraft from
//! hand-measured pixel coordinates, so every layout change left them pointing at
//! the wrong place. Instead, a screen registers the rectangle of a callout
//! target by name while it draws, and this module outlines the registered
//! rectangles after the frame is complete. The headless renderer only names the
//! targets for a shot; the coordinates always come from the live layout.

use crate::theme;
use eframe::egui::{self, Color32, CornerRadius, FontId, Rect, Stroke, StrokeKind};

/// The focus ring's colour, stroke width, corner radius, and distance from the
/// widget it surrounds. The values match the rings the documentation shipped
/// with, so a regenerated set looks like the previous one.
const RING_WIDTH: f32 = 3.0;
const RING_RADIUS: u8 = 8;
const RING_PADDING: f32 = 3.0;

/// A numbered callout badge beside its ring.
const BADGE_RADIUS: f32 = 13.0;
const BADGE_GAP: f32 = 8.0;
const BADGE_FONT_SIZE: f32 = 19.0;
/// The badge ink; the fill is [`theme::EMPHASIS_OUTLINE`].
const BADGE_INK: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);

/// One callout in a documentation shot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) struct Callout {
    /// The name the drawing screen registers for this rectangle.
    pub target: &'static str,
    /// A callout number, for shots whose text refers to "1", "2", and so on.
    pub number: Option<u8>,
}

impl Callout {
    /// A ring without a number.
    #[cfg(any(feature = "headless", test))]
    pub const fn new(target: &'static str) -> Self {
        Self {
            target,
            number: None,
        }
    }

    /// A ring with a numbered badge beside it.
    #[cfg(any(feature = "headless", test))]
    pub const fn numbered(target: &'static str, number: u8) -> Self {
        Self {
            target,
            number: Some(number),
        }
    }
}

/// The callouts a shot asked for, and the rectangles registered this frame.
///
/// The set is empty in a normal session, which costs one length check per
/// registered target and nothing else: no rings are painted and the screens do
/// not grow a second code path.
#[derive(Default)]
pub(in crate::app) struct Emphasis {
    callouts: Vec<Callout>,
    rects: Vec<(&'static str, Rect)>,
}

impl Emphasis {
    /// Ask for these callouts on the next frames; a shot calls this once.
    #[cfg(any(feature = "headless", test))]
    pub fn set(&mut self, callouts: &[Callout]) {
        self.callouts.clear();
        self.callouts.extend_from_slice(callouts);
        self.rects.clear();
    }

    /// Drop last frame's rectangles; called once before a screen draws.
    pub fn begin_frame(&mut self) {
        self.rects.clear();
    }

    /// Record the rectangle of `target` if a callout asks for it.
    ///
    /// A target that is drawn several times (a repeated list row, say) keeps
    /// the first rectangle, which is the one a reader reaches first.
    pub fn note(&mut self, target: &'static str, rect: Rect) {
        if !self.callouts.iter().any(|callout| callout.target == target) {
            return;
        }
        if self.rects.iter().any(|(seen, _)| *seen == target) {
            return;
        }
        self.rects.push((target, rect));
    }

    /// The targets a shot asked for that no screen registered this frame.
    ///
    /// The headless renderer treats this as an error: a mistyped or removed
    /// target would otherwise silently lose its ring.
    #[cfg(any(feature = "headless", test))]
    pub fn missing(&self) -> Vec<&'static str> {
        self.callouts
            .iter()
            .filter(|callout| {
                !self
                    .rects
                    .iter()
                    .any(|(target, _)| *target == callout.target)
            })
            .map(|callout| callout.target)
            .collect()
    }

    /// Paint the rings and badges for the targets registered this frame.
    pub fn paint(&self, ctx: &egui::Context) {
        if self.callouts.is_empty() {
            return;
        }
        let painter = ctx.layer_painter(egui::LayerId::new(
            // Above tooltips: a callout that outlines a definition card must
            // stay visible on top of the card it surrounds.
            egui::Order::Tooltip,
            egui::Id::unique("documentation-callouts"),
        ));
        let screen = ctx.content_rect();
        for callout in &self.callouts {
            let Some((_, rect)) = self
                .rects
                .iter()
                .find(|(target, _)| *target == callout.target)
            else {
                continue;
            };
            let ring = rect.expand(RING_PADDING);
            let shadow = egui::epaint::Shadow {
                offset: [0, 0],
                blur: 6,
                spread: 0,
                color: Color32::from_black_alpha(55),
            };
            painter.add(shadow.as_shape(ring, CornerRadius::same(RING_RADIUS)));
            painter.rect_stroke(
                ring,
                CornerRadius::same(RING_RADIUS),
                Stroke::new(RING_WIDTH, theme::EMPHASIS_OUTLINE),
                StrokeKind::Outside,
            );
            if let Some(number) = callout.number {
                let (center, radius) = badge_center(ring, screen, number >= 10);
                painter.circle_filled(center, radius, theme::EMPHASIS_OUTLINE);
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    number.to_string(),
                    FontId::proportional(BADGE_FONT_SIZE),
                    BADGE_INK,
                );
            }
        }
    }
}

/// Where a numbered badge sits beside its ring, and how big it is.
///
/// The badge prefers the left of the ring. It moves to the right when the left
/// edge would leave the window, and it is nudged down/up so a ring at the top
/// or bottom edge keeps the whole circle visible.
fn badge_center(ring: Rect, screen: Rect, two_digits: bool) -> (egui::Pos2, f32) {
    let radius = if two_digits {
        BADGE_RADIUS + 3.0
    } else {
        BADGE_RADIUS
    };
    let mut x = ring.left() - BADGE_GAP - radius;
    if x - radius < 4.0 {
        x = ring.right() + BADGE_GAP + radius;
    }
    let y = ring
        .center()
        .y
        .clamp(screen.top() + radius + 4.0, screen.bottom() - radius - 4.0);
    (egui::pos2(x, y), radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
    }

    #[test]
    fn a_numbered_callout_keeps_its_digit() {
        let callout = Callout::numbered("search", 2);
        assert_eq!(callout.target, "search");
        assert_eq!(callout.number, Some(2));
        assert_eq!(Callout::new("plain").number, None);
    }

    #[test]
    fn only_requested_targets_are_recorded() {
        let mut emphasis = Emphasis::default();
        emphasis.note("ignored", rect(0.0, 0.0, 10.0, 10.0));
        assert!(emphasis.missing().is_empty());

        emphasis.set(&[Callout::new("kept")]);
        emphasis.note("ignored", rect(0.0, 0.0, 10.0, 10.0));
        emphasis.note("kept", rect(4.0, 5.0, 6.0, 7.0));
        assert!(emphasis.missing().is_empty());
        assert_eq!(emphasis.rects.len(), 1);
    }

    #[test]
    fn a_target_that_never_drew_is_reported() {
        let mut emphasis = Emphasis::default();
        emphasis.set(&[Callout::new("present"), Callout::new("absent")]);
        emphasis.note("present", rect(0.0, 0.0, 10.0, 10.0));
        assert_eq!(emphasis.missing(), vec!["absent"]);
    }

    #[test]
    fn a_repeated_target_keeps_its_first_rectangle() {
        let mut emphasis = Emphasis::default();
        emphasis.set(&[Callout::new("row")]);
        emphasis.note("row", rect(0.0, 0.0, 10.0, 10.0));
        emphasis.note("row", rect(0.0, 30.0, 10.0, 10.0));
        assert_eq!(emphasis.rects, vec![("row", rect(0.0, 0.0, 10.0, 10.0))]);
    }

    #[test]
    fn beginning_a_frame_forgets_the_previous_rectangles() {
        let mut emphasis = Emphasis::default();
        emphasis.set(&[Callout::new("row")]);
        emphasis.note("row", rect(0.0, 0.0, 10.0, 10.0));
        emphasis.begin_frame();
        assert_eq!(emphasis.missing(), vec!["row"]);
    }

    #[test]
    fn a_badge_moves_right_when_the_left_edge_is_too_close() {
        let screen = rect(0.0, 0.0, 1280.0, 800.0);
        let (center, radius) = badge_center(rect(1.0, 100.0, 200.0, 40.0), screen, false);
        assert_eq!(radius, BADGE_RADIUS);
        assert!(center.x > 200.0, "badge should sit right of the ring");
    }

    #[test]
    fn a_badge_stays_inside_the_window_vertically() {
        let screen = rect(0.0, 0.0, 1280.0, 800.0);
        let (top, radius) = badge_center(rect(300.0, 0.0, 200.0, 20.0), screen, false);
        assert!(top.y - radius >= 0.0);
        let (bottom, _) = badge_center(rect(300.0, 780.0, 200.0, 20.0), screen, false);
        assert!(bottom.y + radius <= 800.0);
    }
}
