//! Screen implementations for the app shell.
//!
//! Each screen owns one file; shared card helpers stay here so the session
//! screen and the state-machine tests see the same definitions. Preferences
//! keeps its sections in `preferences.rs`, with the About section in
//! `about.rs` as the last of them.

mod about;
mod aliases;
mod dance_picker;
mod gesture;
mod library;
// The shell tests read the section list to assert where About is drawn.
pub(in crate::app) mod preferences;
mod session;
mod tune_picker;
mod widgets;

pub(in crate::app) use widgets::{
    RowPress, action_width, block_button, block_selectable, clamp_pane_width, combo_option,
    empty_label, group_chip, long_press_row, pane_splitter, row_activated, row_menu_button,
    submit_text_edit, submitted,
};

use crate::theme;
use contra_core::{Dance, FolderFilter, Tune};
use eframe::egui;

use super::*;

/// Whether this screen should use the phone layout.
///
/// Every screen shares the session tab's breakpoint so the tab row, the
/// controls, and the forms never switch layouts at different widths. The app
/// window's minimum width is 360 points, so each phone layout must also fit a
/// 360-point-wide viewport.
pub(super) fn compact(ui: &egui::Ui) -> bool {
    compact_layout(ui.available_width())
}

/// Phone screens use 44-point touch rows and full-width text fields.
pub(super) fn apply_phone_spacing(ui: &mut egui::Ui, compact: bool) {
    if compact {
        ui.spacing_mut().interact_size.y = theme::TOUCH_TARGET;
        ui.spacing_mut().button_padding.x = 10.0;
        ui.spacing_mut().text_edit_width = ui.available_width();
    }
}

/// One row of a label/value form.
///
/// In a desktop grid the label and control occupy two columns. On a phone the
/// row becomes one column with a small label beside a full-width control, which
/// keeps a long search form from pushing the results below the fold.
pub(super) fn form_row<R>(
    ui: &mut egui::Ui,
    compact: bool,
    label: &str,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    if compact {
        ui.horizontal(|ui| {
            ui.spacing_mut().text_edit_width = ui.available_width();
            ui.label(egui::RichText::new(label).small().color(theme::muted(ui)));
            add(ui)
        })
        .inner
    } else {
        ui.label(label);
        add(ui)
    }
}

/// A label beside a control on desktop and a small label beside a full-width
/// control on a phone.
pub(super) fn inline_field<R>(
    ui: &mut egui::Ui,
    compact: bool,
    label: &str,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    if compact {
        ui.horizontal(|ui| {
            ui.spacing_mut().text_edit_width = ui.available_width();
            ui.label(egui::RichText::new(label).small().color(theme::muted(ui)));
            add(ui)
        })
        .inner
    } else {
        ui.horizontal(|ui| {
            ui.label(label);
            add(ui)
        })
        .inner
    }
}

/// A slider with a label beside it on desktop and above it on a phone.
///
/// On a phone the rail takes the whole row; leaving room for the value box
/// keeps the value readable while the thumb is still easy to hit. The slider
/// itself goes through [`scroll_safe_slider`] so a swipe that starts on it
/// scrolls the page instead of changing the value.
pub(super) fn slider_row<T>(
    ui: &mut egui::Ui,
    compact: bool,
    label: &str,
    value: &mut T,
    add: impl FnOnce(&mut egui::Ui, &mut T) -> egui::Response,
) -> egui::Response
where
    T: Copy + Send + Sync + 'static,
{
    if compact {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(label).small().color(theme::muted(ui)));
            // Leave room for the value box and a suffix such as "syllables/s";
            // without this the row is wider than a 360-point window and every
            // label in the page stops wrapping at the viewport edge.
            ui.spacing_mut().slider_width = (ui.available_width() - 160.0).max(96.0);
            scroll_safe_slider(ui, value, add)
        })
        .inner
    } else {
        ui.horizontal(|ui| {
            ui.label(label);
            scroll_safe_slider(ui, value, add)
        })
        .inner
    }
}

/// Distance the pointer must travel before a slider gesture's direction is
/// judged. Below this a press is still a potential tap, so the slider holds
/// the value it had before the press instead of committing the press position.
const SLIDER_GESTURE_SLOP: f32 = 4.0;

/// A vertical drag must travel this far before it counts as a scroll.
const SCROLL_GESTURE_MIN: f32 = 8.0;

/// A scroll must be at least this much more vertical than horizontal.
const SCROLL_GESTURE_RATIO: f32 = 1.5;

/// Whether a pointer drag is a vertical scroll rather than a slider drag.
///
/// A deliberate horizontal drag stays a slider drag even if the finger wanders
/// a little: only a clearly vertical gesture (more than eight points and more
/// than 1.5 times as tall as it is wide) takes over.
pub(super) fn is_vertical_scroll_gesture(dx: f32, dy: f32) -> bool {
    dy.abs() > SCROLL_GESTURE_MIN && dy.abs() > dx.abs() * SCROLL_GESTURE_RATIO
}

/// The value a slider had before the pointer gesture that is in progress.
///
/// Kept in egui's temporary data store for the duration of one press so a drag
/// that becomes a scroll can put the value back exactly.
#[derive(Clone, Copy)]
struct SliderGesture<T> {
    base: T,
    suppressed: bool,
}

/// The id under which [`begin_slider_frame`] stores this frame's slider rects.
fn slider_rects_id() -> egui::Id {
    egui::Id::unique("scroll-safe-slider-rects")
}

/// Start a new frame's list of slider rectangles.
///
/// The tab-swipe handler uses the list to tell a swipe on empty content from a
/// drag that started on a slider. Call this once, before drawing a screen.
pub(super) fn begin_slider_frame(ui: &mut egui::Ui) {
    ui.data_mut(|data| {
        data.insert_temp(slider_rects_id(), Vec::<egui::Rect>::new());
    });
}

/// Whether a pointer press at `pos` started on a slider drawn this frame.
pub(super) fn press_started_on_slider(ui: &egui::Ui, pos: egui::Pos2) -> bool {
    ui.data(|data| {
        data.get_temp::<Vec<egui::Rect>>(slider_rects_id())
            .is_some_and(|rects| rects.iter().any(|rect| rect.contains(pos)))
    })
}

fn record_slider_rect(ui: &egui::Ui, rect: egui::Rect) {
    ui.data_mut(|data| {
        data.get_temp_mut_or_default::<Vec<egui::Rect>>(slider_rects_id())
            .push(rect);
    });
}

/// Whether this widget sits inside a scroll area that can take the gesture.
///
/// `Ui::scroll_with_delta` targets whichever scroll area consumes the delta
/// next, which need not be an ancestor. Only inject a scroll when an ancestor
/// is a scroll area, so a vertical drag on a desktop slider cannot scroll an
/// unrelated list drawn later in the frame.
fn has_scroll_ancestor(ui: &egui::Ui) -> bool {
    ui.stack()
        .iter()
        .any(|stack| stack.kind() == Some(egui::UiKind::ScrollArea))
}

/// Add a slider that ignores a vertical swipe.
///
/// Sliders live inside scrolling pages on a phone, and a thumb swipe down the
/// page often starts on a slider rail or its value box. egui's slider sets its
/// value from the pointer as soon as it is pressed, so by the time the gesture
/// is recognisable as a scroll the value has already moved. This helper gives
/// the slider a draft value, watches the drag, and writes the draft back only
/// when the gesture is not a vertical scroll. A scroll restores the pre-press
/// value, suppresses the change, and hands the vertical movement to the
/// enclosing scroll area.
///
/// Keyboard and screen-reader changes never press the pointer, so they pass
/// straight through and arrow keys keep adjusting a focused slider.
pub(super) fn scroll_safe_slider<T>(
    ui: &mut egui::Ui,
    value: &mut T,
    add: impl FnOnce(&mut egui::Ui, &mut T) -> egui::Response,
) -> egui::Response
where
    T: Copy + Send + Sync + 'static,
{
    let base = *value;
    let mut draft = base;
    let mut response = add(ui, &mut draft);
    record_slider_rect(ui, response.rect);

    let gesture_id = response.id.with("scroll-safe-slider");
    let holding =
        response.is_pointer_button_down_on() || response.dragged() || response.drag_stopped();
    if !holding {
        // A click, a keyboard step, or a screen-reader request: accept it.
        *value = draft;
        ui.data_mut(|data| data.remove::<SliderGesture<T>>(gesture_id));
        return response;
    }

    let mut gesture = ui
        .data(|data| data.get_temp::<SliderGesture<T>>(gesture_id))
        .unwrap_or(SliderGesture {
            base,
            suppressed: false,
        });
    let total = ui.input(|input| {
        input
            .pointer
            .press_origin()
            .zip(input.pointer.interact_pos())
            .map(|(origin, pointer)| pointer - origin)
            .unwrap_or(egui::Vec2::ZERO)
    });
    if !gesture.suppressed && total.length() > SLIDER_GESTURE_SLOP {
        gesture.suppressed = is_vertical_scroll_gesture(total.x, total.y);
    }

    if gesture.suppressed {
        // Put the value back, scroll instead, and hide the transient change so
        // the caller does not rebuild the timeline or re-render the music.
        *value = gesture.base;
        let delta_y = ui.input(|input| input.pointer.delta().y);
        if delta_y != 0.0 && has_scroll_ancestor(ui) {
            ui.scroll_with_delta_animation(
                egui::vec2(0.0, delta_y),
                egui::style::ScrollAnimation::none(),
            );
        }
        // egui has no public "clear changed" call; clearing the bit is the
        // only way to keep a scroll from looking like a value change.
        response.flags.remove(egui::response::Flags::CHANGED);
    } else if total.length() <= SLIDER_GESTURE_SLOP && !response.drag_stopped() {
        // Direction is not known yet: hold the old value so a press that turns
        // into a scroll cannot commit the press position.
        *value = gesture.base;
        response.flags.remove(egui::response::Flags::CHANGED);
    } else {
        *value = draft;
    }

    if response.drag_stopped() {
        ui.data_mut(|data| data.remove::<SliderGesture<T>>(gesture_id));
    } else {
        ui.data_mut(|data| data.insert_temp(gesture_id, gesture));
    }
    response
}

/// Size a combo box to the available width on a phone.
pub(super) fn responsive_combo<'a>(
    combo: egui::ComboBox<'a>,
    ui: &egui::Ui,
    compact: bool,
) -> egui::ComboBox<'a> {
    if compact {
        combo.width((ui.available_width() - 12.0).max(96.0))
    } else {
        combo
    }
}

/// Space the picker panes must leave below them for the pagination row and
/// the gap above it.
const PAGINATION_ROW_HEIGHT: f32 = 40.0;

/// The smallest height a split desktop picker gives its list and preview.
///
/// Below this a forced box would show only a row or two, so the panes keep a
/// usable floor and the window clips instead of collapsing the list.
const MIN_PANE_HEIGHT: f32 = 120.0;

/// Height left for a list or preview pane below the rows already drawn.
///
/// The panes take every point that is left, so on a tall window they end at
/// the bottom edge instead of at an arbitrary fixed cap. A short window keeps
/// a usable floor rather than shrinking the pane to nothing.
fn remaining_pane_height(available_height: f32, reserved_below: f32) -> f32 {
    (available_height - reserved_below).max(MIN_PANE_HEIGHT)
}

/// Height for a picker's list and preview panes, or `None` when the page
/// itself scrolls.
///
/// A phone page already scrolls, so the stacked list and preview keep their
/// natural content height and must not reserve a fixed empty box; that is what
/// keeps a 360-point window usable. A desktop has no page scroll, so the panes
/// consume the remaining viewport height and both columns end at the same
/// bottom edge.
pub(super) fn picker_pane_height(
    compact: bool,
    available_height: f32,
    reserved_below: f32,
) -> Option<f32> {
    (!compact).then(|| remaining_pane_height(available_height, reserved_below))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CallPhase {
    Upcoming,
    Calling,
    Executing,
}

pub(super) fn phase_at(call: &contra_core::ScheduledCall, beat: f64) -> CallPhase {
    if beat < call.start_beat {
        CallPhase::Upcoming
    } else if beat < call.execute_beat {
        CallPhase::Calling
    } else {
        CallPhase::Executing
    }
}

/// What the large prompt card is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CardMode {
    /// The call the caller is about to speak or is speaking now.
    CurrentCall,
    /// The figure is already being danced, so the card shows the move the
    /// caller must say next.
    UpcomingMove,
    /// The figure is being danced and there is no following move to call.
    FinalMove,
}

/// Pick the card content from the active call's phase and whether another
/// call follows it.
///
/// A caller looks ahead while the dancers are executing, so the card must stop
/// describing the figure in progress and start prompting the next move.
pub(super) fn card_state(phase: Option<CallPhase>, has_next: bool) -> CardMode {
    match phase {
        Some(CallPhase::Executing) if has_next => CardMode::UpcomingMove,
        Some(CallPhase::Executing) => CardMode::FinalMove,
        _ => CardMode::CurrentCall,
    }
}

/// Background cue for the prompt card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CardTone {
    /// Nothing to speak yet: ready, count-in, dancing the last figure, done.
    Neutral,
    /// The caller has to say the shown call now.
    CallNow,
    /// The dancers are executing, so prompt the move to call next.
    UpcomingMove,
}

/// Green while the caller is speaking, red while the card looks ahead, grey
/// otherwise. The hue is a glanceable cue on top of the text, never the only
/// signal: the labels still say CALL NOW and UPCOMING MOVE.
pub(super) fn card_tone(started: bool, phase: Option<CallPhase>, mode: CardMode) -> CardTone {
    if !started {
        return CardTone::Neutral;
    }
    match (phase, mode) {
        (Some(CallPhase::Calling), _) => CardTone::CallNow,
        (_, CardMode::UpcomingMove) => CardTone::UpcomingMove,
        _ => CardTone::Neutral,
    }
}

/// A change requested by one of the saved-library lists.
pub(super) enum LibraryAction {
    PreviewDance(Dance),
    UseDance(Dance),
    RemoveDance {
        provider: String,
        id: String,
    },
    MoveDance {
        provider: String,
        id: String,
        folder: Option<String>,
    },
    PreviewTune(Tune),
    UseTune(Tune),
    RemoveTune(String),
    MoveTune {
        id: String,
        folder: Option<String>,
    },
    ToggleRotation(Tune),
    /// Switch the tune set from a folder back to the explicit chosen list.
    UseChosenTuneSet,
}

/// The site name to show for a provider, for "Open on …" links and messages.
pub(super) fn source_label(provider: &str) -> &'static str {
    contra_sources::dance_provider(provider)
        .map(|provider| provider.display_name())
        .unwrap_or("the source")
}

/// A folder operation requested by the folder panel.
pub(super) enum FolderCommand {
    Create(String),
    Rename {
        old: String,
        new: String,
    },
    Delete(String),
    /// Move the folder at `from` to the final index `to`.
    Reorder {
        from: usize,
        to: usize,
    },
    /// Use a folder (or the explicit list) as the tune set.
    UseTuneSet(contra_core::TuneSetSource),
    /// Restore every split pane to its default width.
    ResetPanes,
}

/// The app-side result of a successful folder operation.
pub(super) enum FolderOutcome {
    Created(String),
    Renamed(String),
    Deleted(String),
    Reordered(usize),
}

/// Mutable view of the fields the shared folder panel renders.
pub(super) struct FolderPanel<'a> {
    all_label: &'a str,
    folders: &'a [String],
    counts: &'a [usize],
    all_count: usize,
    unfiled_count: usize,
    filter: &'a mut FolderFilter,
    new_folder: &'a mut String,
    renaming: &'a mut Option<String>,
    rename_text: &'a mut String,
    /// Prefix for the drag state stored in egui's temporary data. The dance
    /// and tune panels use different keys so one screen's drag cannot leak
    /// into the other.
    drag_id: &'a str,
    /// Extra controls for the tune library: the current set source and count.
    tune_set: Option<TuneSetPanel<'a>>,
}

/// The tune-set controls the folder panel offers below the selected folder.
pub(super) struct TuneSetPanel<'a> {
    pub source: &'a contra_core::TuneSetSource,
    pub count: usize,
}

/// Prompt the caller with the move they must call next while the current
/// figure is being danced.
pub(super) fn upcoming_move_card(
    ui: &mut egui::Ui,
    next: &contra_core::ScheduledCall,
    beat: f64,
    show_original: bool,
    call_size: f32,
) {
    ui.label(
        egui::RichText::new("UPCOMING MOVE")
            .small()
            .color(theme::muted(ui)),
    );
    ui.label(
        egui::RichText::new(&next.call.text)
            .size(call_size)
            .strong()
            .color(theme::on_card(ui)),
    );
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "call in {:.1} beats",
                (next.start_beat - beat).max(0.0)
            ))
            .color(theme::muted(ui)),
        );
        if show_original {
            ui.label(
                egui::RichText::new(format!("· {}", next.call.original_text))
                    .small()
                    .color(theme::muted(ui)),
            );
        }
    });
}

/// What the session controls changed this frame.
#[derive(Default)]
pub(super) struct SessionControlChanges {
    timeline_dirty: bool,
    music_dirty: bool,
    rotation_dirty: bool,
    save: bool,
    action: Option<&'static str>,
}

impl ContraApp {
    pub(super) fn pagination_ui(&mut self, ui: &mut egui::Ui, dance: bool) {
        // A source with one capped result list explains the cap on the screen
        // instead of offering Previous/Next that would repeat the same rows.
        if dance && !self.dance_source().supports_pagination() {
            return;
        }
        let (page, page_size, total) = if dance {
            let results = self.picker.dance_results.as_ref();
            (
                self.picker.dance_query.page.max(1),
                self.picker.dance_query.page_size.max(1),
                results.map(|page| page.total).unwrap_or(0),
            )
        } else {
            let results = self.picker.tune_results.as_ref();
            (
                self.picker.tune_query.page.max(1),
                self.picker.tune_query.page_size.max(1),
                results.map(|page| page.total).unwrap_or(0),
            )
        };
        let pages = total.div_ceil(page_size);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(page > 1, egui::Button::new("Previous"))
                .clicked()
            {
                if dance {
                    self.picker.dance_query.page = page - 1;
                    self.search_dances();
                } else {
                    self.picker.tune_query.page = page - 1;
                    self.search_tunes();
                }
            }
            ui.label(format!("Page {page} of {}", pages.max(1)));
            if ui
                .add_enabled(page < pages, egui::Button::new("Next"))
                .clicked()
            {
                if dance {
                    self.picker.dance_query.page = page + 1;
                    self.search_dances();
                } else {
                    self.picker.tune_query.page = page + 1;
                    self.search_tunes();
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_dominant_vertical_drag_counts_as_a_scroll() {
        // A straight swipe down or up.
        assert!(is_vertical_scroll_gesture(0.0, 24.0));
        assert!(is_vertical_scroll_gesture(0.0, -24.0));
        // A finger wander of a few points sideways still scrolls.
        assert!(is_vertical_scroll_gesture(9.0, 30.0));
        // The threshold keeps a tap and a short nudge as slider gestures.
        assert!(!is_vertical_scroll_gesture(0.0, 8.0));
        assert!(!is_vertical_scroll_gesture(0.0, 5.0));
        // A deliberate horizontal drag, even a diagonal one, keeps the slider.
        assert!(!is_vertical_scroll_gesture(40.0, 20.0));
        assert!(!is_vertical_scroll_gesture(20.0, 29.0));
        // Exactly 1.5:1 is still a horizontal slider gesture.
        assert!(!is_vertical_scroll_gesture(20.0, 30.0));
    }

    #[test]
    fn a_scroll_takeover_only_happens_inside_a_scroll_area() {
        let ctx = egui::Context::default();
        let (mut outside, mut inside) = (true, false);
        let mut output = ctx.run_ui(raw_input(Vec::new()), |ui| {
            outside = has_scroll_ancestor(ui);
            egui::ScrollArea::vertical().show(ui, |ui| {
                inside = has_scroll_ancestor(ui);
            });
        });
        output.textures_delta.clear();
        assert!(!outside, "the root should not look like a scroll area");
        assert!(inside, "the scroll content must see its scroll ancestor");
    }

    fn raw_input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 300.0),
            )),
            events,
            ..Default::default()
        }
    }

    fn pointer_moved(pos: egui::Pos2) -> egui::Event {
        egui::Event::PointerMoved(pos)
    }

    fn pointer_button(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// Run one frame of the helper and return the slider response.
    fn frame(ctx: &egui::Context, events: Vec<egui::Event>, value: &mut f64) -> egui::Response {
        let mut captured = None;
        let mut output = ctx.run_ui(raw_input(events), |ui| {
            captured = Some(scroll_safe_slider(ui, value, |ui, value| {
                ui.add(egui::Slider::new(value, 0.0..=100.0))
            }));
        });
        output.textures_delta.clear();
        captured.expect("the slider is added")
    }

    #[test]
    fn a_vertical_drag_over_a_slider_keeps_the_value() {
        let ctx = egui::Context::default();
        let mut value = 50.0_f64;
        let rect = frame(&ctx, Vec::new(), &mut value).rect;
        let start = egui::pos2(rect.left() + rect.width() * 0.3, rect.center().y);

        frame(&ctx, vec![pointer_moved(start)], &mut value);
        frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        for dy in [20.0, 45.0, 70.0] {
            let response = frame(
                &ctx,
                vec![pointer_moved(start + egui::vec2(0.0, dy))],
                &mut value,
            );
            // A scroll must not look like a slider change either, or every
            // frame would rebuild the timeline and re-render the music.
            assert!(!response.changed(), "scroll reported a change at dy={dy}");
        }
        let response = frame(
            &ctx,
            vec![pointer_button(start + egui::vec2(0.0, 70.0), false)],
            &mut value,
        );
        assert!(!response.changed());

        // The thumb may follow the finger while the press lasts, but the value
        // the app reads is the one from before the press.
        assert_eq!(value, 50.0);
    }

    #[test]
    fn a_horizontal_drag_still_changes_the_value() {
        let ctx = egui::Context::default();
        let mut value = 50.0_f64;
        let rect = frame(&ctx, Vec::new(), &mut value).rect;
        let start = egui::pos2(rect.left() + rect.width() * 0.3, rect.center().y);

        frame(&ctx, vec![pointer_moved(start)], &mut value);
        frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        frame(
            &ctx,
            vec![pointer_moved(start + egui::vec2(40.0, 0.0))],
            &mut value,
        );
        let response = frame(
            &ctx,
            vec![pointer_moved(start + egui::vec2(80.0, 0.0))],
            &mut value,
        );
        assert!(response.changed(), "the drag did not report a change");
        frame(
            &ctx,
            vec![pointer_button(start + egui::vec2(80.0, 0.0), false)],
            &mut value,
        );

        assert!(value > 50.0, "the horizontal drag was ignored: {value}");
    }

    #[test]
    fn a_tap_on_the_rail_still_jumps_to_that_value() {
        let ctx = egui::Context::default();
        let mut value = 50.0_f64;
        let rect = frame(&ctx, Vec::new(), &mut value).rect;
        // Just inside the left end of the rail, before the value box.
        let start = egui::pos2(rect.left() + 20.0, rect.center().y);

        frame(&ctx, vec![pointer_moved(start)], &mut value);
        frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        frame(&ctx, vec![pointer_button(start, false)], &mut value);

        assert!(value < 35.0, "the tap did not set the value: {value}");
    }

    #[test]
    fn picker_panes_take_the_height_left_after_the_pagination_row() {
        assert_eq!(
            picker_pane_height(false, 700.0, PAGINATION_ROW_HEIGHT),
            Some(660.0)
        );
        // Without a row below them (the saved-library panes), the panes reach
        // the bottom of the viewport.
        assert_eq!(picker_pane_height(false, 700.0, 0.0), Some(700.0));
    }

    #[test]
    fn picker_panes_keep_a_usable_floor_on_short_windows() {
        assert_eq!(
            picker_pane_height(false, 90.0, PAGINATION_ROW_HEIGHT),
            Some(MIN_PANE_HEIGHT)
        );
        assert_eq!(
            picker_pane_height(false, 0.0, PAGINATION_ROW_HEIGHT),
            Some(MIN_PANE_HEIGHT)
        );
        assert_eq!(
            remaining_pane_height(20.0, PAGINATION_ROW_HEIGHT),
            MIN_PANE_HEIGHT
        );
    }

    #[test]
    fn a_360_point_window_uses_natural_stacked_heights() {
        // The phone page scrolls, so the picker must not force a fixed-height
        // list or preview at the minimum window width.
        assert_eq!(
            picker_pane_height(compact_layout(360.0), 800.0, PAGINATION_ROW_HEIGHT),
            None
        );
        assert_eq!(
            picker_pane_height(compact_layout(390.0), 844.0, PAGINATION_ROW_HEIGHT),
            None
        );
        // The narrowest desktop width still receives a derived pane height.
        assert_eq!(
            picker_pane_height(compact_layout(640.0), 800.0, PAGINATION_ROW_HEIGHT),
            Some(760.0)
        );
    }
}
