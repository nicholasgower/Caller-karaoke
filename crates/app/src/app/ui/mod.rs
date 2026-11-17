//! Screen implementations for the app shell.
//!
//! Each screen owns one file; shared card helpers stay here so the session
//! screen and the state-machine tests see the same definitions. Preferences
//! keeps its sections in `preferences.rs`, with the About section in
//! `about.rs` as the last of them.

mod abc_editor;
mod about;
mod aliases;
mod dance_picker;
mod gesture;
mod glossary;
mod library;
// The shell tests read the section list to assert where About is drawn.
pub(in crate::app) mod preferences;
mod session;
mod tune_picker;
mod widgets;

use crate::theme;
pub(in crate::app) use abc_editor::{AbcEditorState, share_prompt};
use caller_core::{Dance, FolderFilter, Tune};
use eframe::egui;
#[cfg(feature = "headless")]
pub(in crate::app) use glossary::show_tooltip_preview;
pub(in crate::app) use glossary::{clear_tooltip, glossary_text, glossary_tooltip};
pub(in crate::app) use widgets::{
    RowPress, action_width, block_button, block_selectable, clamp_pane_width, combo_option,
    empty_label, group_chip, long_press_row, pane_splitter, row_activated, row_menu_button,
    submit_text_edit, submitted,
};

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
/// itself goes through [`scroll_safe_slider`], which guards it on a touch
/// screen: a tap arms it, and only then does a drag change the value.
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

/// Add a numeric slider whose typed value applies when editing finishes.
///
/// Dragging still changes the value continuously. Only a number typed into the
/// value box waits for Enter or for the field to lose focus. Without that,
/// every digit is parsed and clamped while the user is still typing: a partial
/// number such as "1" becomes the minimum, and a setting that normalizes a pair
/// of boxes (the tempo window) can swap the two fields and rewrite the text
/// under the keyboard. On Android each rewrite also restarts the IME's
/// composing text, which is what made numeric entry flicker and lose digits.
pub(super) fn number_slider(ui: &mut egui::Ui, slider: egui::Slider<'_>) -> egui::Response {
    ui.add(slider.update_while_editing(false))
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

/// Which pointer device the user reached for most recently.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PointerDevice {
    /// A mouse, trackpad, pen, or any pointer that is not a finger.
    #[default]
    Mouse,
    /// A finger on a touch screen.
    Touch,
}

/// The id under which the last pointer device is stored.
fn pointer_device_id() -> egui::Id {
    egui::Id::unique("last-pointer-device")
}

/// The device that produced the most recent pointer event.
fn last_pointer_device(ui: &egui::Ui) -> PointerDevice {
    ui.data(|data| data.get_temp::<PointerDevice>(pointer_device_id()))
        .unwrap_or_default()
}

/// Record the device that produced this frame's pointer events.
///
/// A touch backend also synthesizes pointer events from the finger, so a frame
/// carrying any touch event is a touch frame even when it also carries
/// `PointerMoved` or `PointerButton`. A mouse move or click without a touch
/// event switches back, so a desktop touchscreen unguards its sliders as soon
/// as the mouse is used again. Changing device also restores every guard.
fn note_pointer_device(ui: &egui::Ui) {
    let device = ui.input(|input| {
        if input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Touch { .. }))
        {
            Some(PointerDevice::Touch)
        } else if input.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::PointerMoved(_) | egui::Event::PointerButton { .. }
            )
        }) {
            Some(PointerDevice::Mouse)
        } else {
            None
        }
    });
    let Some(device) = device else {
        return;
    };
    if last_pointer_device(ui) == device {
        return;
    }
    ui.data_mut(|data| data.insert_temp(pointer_device_id(), device));
    disarm_sliders(ui);
}

/// Whether a slider must be tapped before a drag may change it.
///
/// A finger that starts scrolling on a rail looks exactly like a slider drag,
/// so a touch-first build guards every slider. Android and iOS are known to be
/// touch-first before the first event arrives. Everywhere else the guard
/// follows the last pointer used: touching the screen arms the guard and
/// moving or clicking with the mouse releases it, so a desktop touchscreen is
/// not left guarded while the user is on the mouse.
fn slider_guard_active(ui: &egui::Ui) -> bool {
    matches!(crate::platform(), "android" | "ios")
        || last_pointer_device(ui) == PointerDevice::Touch
}

/// The id under which the one armed slider is stored.
fn armed_slider_id() -> egui::Id {
    egui::Id::unique("armed-slider")
}

/// The slider that has been tapped and may now be dragged, if any.
fn armed_slider(ui: &egui::Ui) -> Option<egui::Id> {
    ui.data(|data| data.get_temp::<egui::Id>(armed_slider_id()))
}

/// Let one slider take drags, replacing any other armed slider.
fn arm_slider(ui: &egui::Ui, id: egui::Id) {
    ui.data_mut(|data| data.insert_temp(armed_slider_id(), id));
}

/// Put the guard back on every slider.
pub(super) fn disarm_sliders(ui: &egui::Ui) {
    ui.data_mut(|data| data.remove::<egui::Id>(armed_slider_id()));
}

/// The value a slider had before the pointer gesture that is in progress.
///
/// Kept in egui's temporary data store for the duration of one press so a drag
/// that becomes a scroll can put the value back exactly. `locked` is captured
/// when the press starts: a slider that was tapped stays locked for the rest of
/// that gesture, so the arming tap cannot also commit its position. `adjusting`
/// marks a drag on an armed slider, which never becomes a page scroll.
#[derive(Clone, Copy)]
struct SliderGesture<T> {
    base: T,
    suppressed: bool,
    origin: egui::Pos2,
    moved: bool,
    locked: bool,
    adjusting: bool,
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
    note_pointer_device(ui);
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

/// Restore the slider guard after a press anywhere else.
///
/// Call this once, after the screen has drawn, when every slider rect for the
/// frame is known. A press on empty content, another control, or the start of a
/// swipe disarms the slider that was tapped; a press that starts on a slider
/// belongs to that slider and arms itself.
pub(super) fn finish_slider_frame(ui: &egui::Ui) {
    if armed_slider(ui).is_none() {
        return;
    }
    let press_origin = ui.input(|input| {
        input
            .pointer
            .primary_pressed()
            .then(|| input.pointer.press_origin())
            .flatten()
    });
    if press_origin.is_some_and(|pos| !press_started_on_slider(ui, pos)) {
        disarm_sliders(ui);
    }
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

/// Add a slider that is safe to meet while scrolling and while swiping.
///
/// Sliders live inside scrolling pages on a phone, and a thumb swipe down the
/// page often starts on a slider rail or its value box. egui's slider sets its
/// value from the pointer as soon as it is pressed, so by the time the gesture
/// is recognisable as a scroll the value has already moved. This helper gives
/// the slider a draft value, watches the drag, and writes the draft back only
/// when the gesture is not a vertical scroll.
///
/// On a touch screen the slider is also *guarded*: a tap arms it (and never
/// changes the value), the drag that follows adjusts it, and a tap or swipe
/// elsewhere puts the guard back. While an armed slider is being dragged the
/// enclosing page does not scroll. A swipe that starts on a locked slider still
/// scrolls the page as it did before.
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

    let guarded = slider_guard_active(ui);
    let armed = !guarded || armed_slider(ui) == Some(response.id);
    let locked = !armed;
    let gesture_id = response.id.with("scroll-safe-slider");

    let existing = ui.data(|data| data.get_temp::<SliderGesture<T>>(gesture_id));
    let holding =
        response.is_pointer_button_down_on() || response.dragged() || response.drag_stopped();
    let pointer_down = ui.input(|input| input.pointer.primary_down());
    // A stored gesture whose finger is no longer down is on its release frame,
    // even when egui reports neither a button down nor a drag stop there.
    let ending = existing.is_some() && !pointer_down;

    if !holding && !ending {
        let clicked_on_slider = ui.input(|input| {
            input.pointer.primary_clicked()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|pos| response.rect.contains(pos))
        });
        if guarded && locked && clicked_on_slider {
            // A click that starts and ends in one frame is still a tap: it
            // arms the slider and leaves the level where it was.
            *value = base;
            arm_slider(ui, response.id);
        } else {
            // A keyboard step or a screen-reader request: accept it.
            *value = draft;
        }
        ui.data_mut(|data| data.remove::<SliderGesture<T>>(gesture_id));
    } else {
        let pointer = ui.input(|input| input.pointer.interact_pos());
        let new_press = ui.input(|input| input.pointer.primary_pressed())
            && response.is_pointer_button_down_on();
        let mut gesture =
            if new_press { None } else { existing }.unwrap_or_else(|| SliderGesture {
                base,
                suppressed: false,
                origin: pointer.unwrap_or(response.rect.center()),
                moved: false,
                locked,
                // An armed slider on a touch screen is being adjusted:
                // scrolling is disabled for the whole drag.
                adjusting: guarded && !locked,
            });
        if let Some(pos) = pointer {
            let total = pos - gesture.origin;
            if total.length() > SLIDER_GESTURE_SLOP {
                gesture.moved = true;
            }
            if !gesture.suppressed
                && !gesture.adjusting
                && gesture.moved
                && is_vertical_scroll_gesture(total.x, total.y)
            {
                gesture.suppressed = true;
            }
        }

        if gesture.suppressed {
            // Put the value back, scroll instead, and restore the guard: a
            // swipe is not an adjustment, whether the slider was locked or
            // had already been tapped.
            *value = gesture.base;
            disarm_sliders(ui);
            let delta_y = ui.input(|input| input.pointer.delta().y);
            if !ending && delta_y != 0.0 && has_scroll_ancestor(ui) {
                ui.scroll_with_delta_animation(
                    egui::vec2(0.0, delta_y),
                    egui::style::ScrollAnimation::none(),
                );
            }
            // egui has no public "clear changed" call; clearing the bit is the
            // only way to keep a scroll from looking like a value change.
            response.flags.remove(egui::response::Flags::CHANGED);
        } else if gesture.locked {
            // A locked slider holds its value for the whole gesture. Only a
            // tap — a press and release that did not travel — arms it.
            *value = gesture.base;
            if ending && !gesture.moved {
                arm_slider(ui, response.id);
            }
            response.flags.remove(egui::response::Flags::CHANGED);
        } else if !ending {
            if gesture.moved || response.drag_stopped() {
                // An armed slider takes the drag, so the enclosing page does
                // not scroll while the level is being adjusted.
                *value = draft;
            } else {
                // Direction is not known yet: hold the old value so a press
                // that turns into a scroll cannot commit the press position.
                *value = gesture.base;
                response.flags.remove(egui::response::Flags::CHANGED);
            }
        } else {
            // A tap on an unlocked slider still jumps to the pressed position.
            *value = draft;
        }

        if ending {
            ui.data_mut(|data| data.remove::<SliderGesture<T>>(gesture_id));
        } else {
            ui.data_mut(|data| data.insert_temp(gesture_id, gesture));
        }
    }

    if guarded && armed_slider(ui) != Some(response.id) {
        draw_locked_slider(ui, &response);
        response = response.on_hover_text("Tap the slider to adjust it");
    }
    response
}

/// Draw the dimmed rail and padlock that say a slider must be tapped first.
///
/// The value box to the right of the rail stays crisp, so the current level is
/// still readable; only the rail the user cannot yet drag is muted.
fn draw_locked_slider(ui: &egui::Ui, response: &egui::Response) {
    let rect = response.rect;
    let rail_width = ui.spacing().slider_width.clamp(0.0, rect.width());
    // The rail area only: the value box and any unit label sit outside it, and
    // the guard frame must not make them look locked too.
    let rail = egui::Rect::from_min_size(rect.min, egui::vec2(rail_width, rect.height()));
    // The rail is the thin bar centred in the row; the rest of the widget's
    // height belongs to the touch target.
    let bar_height = (rail.height() * 0.8).max(6.0).min(rail.height());
    let bar = egui::Rect::from_center_size(rail.center(), egui::vec2(rail.width(), bar_height));
    let fill = ui.visuals().panel_fill;
    let painter = ui.painter();
    painter.rect_filled(
        bar,
        bar_height * 0.5,
        egui::Color32::from_rgba_unmultiplied(fill.r(), fill.g(), fill.b(), 150),
    );
    paint_padlock(
        painter,
        bar.center(),
        ui.visuals().widgets.noninteractive.fg_stroke.color,
        fill,
    );
    // A rounded frame around the rail makes the locked state visible even
    // where the dimmed rail meets a card of the same colour.
    let outline = ui
        .visuals()
        .widgets
        .noninteractive
        .fg_stroke
        .color
        .gamma_multiply(0.6);
    painter.rect_stroke(
        rail.expand(1.0),
        8,
        egui::Stroke::new(1.0, outline),
        egui::StrokeKind::Inside,
    );
}

/// Paint a small padlock centred at `center`.
fn paint_padlock(
    painter: &egui::Painter,
    center: egui::Pos2,
    color: egui::Color32,
    hole: egui::Color32,
) {
    let body = egui::Rect::from_center_size(center + egui::vec2(0.0, 2.0), egui::vec2(11.0, 8.0));
    painter.rect_filled(body, 1.5, color);
    // The shackle is a half circle sitting on the top edge of the body.
    let radius = 3.8;
    let base_y = body.top();
    let points = (0..=8)
        .map(|step| {
            let angle = std::f32::consts::PI * step as f32 / 8.0;
            egui::pos2(
                center.x - radius * angle.cos(),
                base_y - radius * angle.sin(),
            )
        })
        .collect::<Vec<_>>();
    painter.add(egui::Shape::line(points, egui::Stroke::new(1.4, color)));
    // Keyhole, in the colour of the scrim behind the lock.
    painter.circle_filled(body.center() + egui::vec2(0.0, -0.5), 1.4, hole);
    painter.line_segment(
        [
            body.center() + egui::vec2(0.0, -0.2),
            body.center() + egui::vec2(0.0, 1.8),
        ],
        egui::Stroke::new(1.0, hole),
    );
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

pub(super) fn phase_at(call: &caller_core::ScheduledCall, beat: f64) -> CallPhase {
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
    /// Write this saved tune's ABC text to a file.
    ExportTune(Tune),
    /// Write this saved dance to a versioned JSON file.
    ExportDance(Dance),
    /// Open this saved tune in the ABC editor.
    EditTuneAbc(Tune),
    /// Open a glossary entry from a linked term in the saved-dance preview.
    OpenGlossary(&'static str),
    /// Switch the tune set from a folder back to the explicit chosen list.
    UseChosenTuneSet,
}

/// The site name to show for a provider, for "Open on …" links and messages.
pub(super) fn source_label(provider: &str) -> &'static str {
    caller_sources::dance_provider(provider)
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
    UseTuneSet(caller_core::TuneSetSource),
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
    pub source: &'a caller_core::TuneSetSource,
    pub count: usize,
}

/// Round a beat count to the nearest half beat for display.
///
/// Countdowns move continuously with the audio clock, so showing every tenth
/// of a beat flickers. Calls are timed to whole beats, which makes a half-beat
/// readout the finest step the caller needs.
pub(super) fn round_half_beat(beats: f64) -> f64 {
    (beats * 2.0).round() / 2.0
}

/// Format a beat count to the nearest half beat, always with one decimal.
///
/// Whole beats read as `5.0`, half beats as `5.5`.
pub(super) fn format_half_beats(beats: f64) -> String {
    format!("{:.1}", round_half_beat(beats))
}

/// Prompt the caller with the move they must call next while the current
/// figure is being danced.
pub(super) fn upcoming_move_card(
    ui: &mut egui::Ui,
    next: &caller_core::ScheduledCall,
    beat: f64,
    show_original: bool,
    glossary_enabled: bool,
    call_size: f32,
) -> Option<&'static caller_core::glossary::GlossaryEntry> {
    ui.label(
        egui::RichText::new("UPCOMING MOVE")
            .small()
            .color(theme::muted(ui)),
    );
    let call_color = theme::on_card(ui);
    let mut clicked = glossary_text(ui, &next.call.text, glossary_enabled, |text| {
        egui::RichText::new(text)
            .size(call_size)
            .strong()
            .color(call_color)
    });
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!(
                "call in {} beats",
                format_half_beats((next.start_beat - beat).max(0.0))
            ))
            .color(theme::muted(ui)),
        );
        if show_original {
            ui.label(egui::RichText::new("·").small().color(theme::muted(ui)));
            let original_color = theme::muted(ui);
            if let Some(entry) =
                glossary_text(ui, &next.call.original_text, glossary_enabled, |text| {
                    egui::RichText::new(text).small().color(original_color)
                })
            {
                clicked = Some(entry);
            }
        }
    });
    clicked
}

/// What the session controls changed this frame.
#[derive(Default)]
pub(super) struct SessionControlChanges {
    timeline_dirty: bool,
    music_dirty: bool,
    /// The count-in checkbox changed. Timeline, music, and transport apply
    /// together through `CallerApp::apply_count_in_change`.
    count_in_dirty: bool,
    rotation_dirty: bool,
    save: bool,
    action: Option<&'static str>,
}

impl CallerApp {
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
    fn lead_countdowns_show_the_nearest_half_beat() {
        assert_eq!(format_half_beats(3.74), "3.5");
        assert_eq!(format_half_beats(3.76), "4.0");
        assert_eq!(format_half_beats(4.0), "4.0");
        assert_eq!(format_half_beats(0.24), "0.0");
        // A live countdown never shows more than one decimal, and whole beats
        // keep their tenths place.
        assert_eq!(format_half_beats(2.1), "2.0");
        assert_eq!(format_half_beats(2.3), "2.5");
    }

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

    /// One finger event, which is how egui learns the screen is a touch
    /// screen and which device produced a frame's pointer events.
    fn touch_event(phase: egui::TouchPhase, pos: egui::Pos2) -> egui::Event {
        egui::Event::Touch {
            device_id: egui::TouchDeviceId(1),
            id: egui::TouchId(1),
            phase,
            pos,
            force: None,
        }
    }

    /// Add the finger events a touch backend sends beside each pointer event.
    fn with_touch(events: Vec<egui::Event>) -> Vec<egui::Event> {
        let mut out = Vec::with_capacity(events.len() * 2);
        for event in events {
            match event {
                egui::Event::PointerButton { pos, pressed, .. } => {
                    let phase = if pressed {
                        egui::TouchPhase::Start
                    } else {
                        egui::TouchPhase::End
                    };
                    out.push(touch_event(phase, pos));
                    out.push(event);
                }
                egui::Event::PointerMoved(pos) => {
                    out.push(touch_event(egui::TouchPhase::Move, pos));
                    out.push(event);
                }
                other => out.push(other),
            }
        }
        out
    }

    /// Run one frame of the helper and return the slider response.
    fn frame(ctx: &egui::Context, events: Vec<egui::Event>, value: &mut f64) -> egui::Response {
        let mut captured = None;
        let mut output = ctx.run_ui(raw_input(events), |ui| {
            begin_slider_frame(ui);
            captured = Some(scroll_safe_slider(ui, value, |ui, value| {
                ui.add(egui::Slider::new(value, 0.0..=100.0))
            }));
            finish_slider_frame(ui);
        });
        output.textures_delta.clear();
        captured.expect("the slider is added")
    }

    /// One frame whose pointer events come from a finger on a touch screen.
    fn touch_frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        value: &mut f64,
    ) -> egui::Response {
        frame(ctx, with_touch(events), value)
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
    fn a_guarded_slider_needs_a_tap_before_a_drag_changes_it() {
        let ctx = egui::Context::default();
        let mut value = 50.0_f64;
        let rect = touch_frame(&ctx, Vec::new(), &mut value).rect;
        let start = egui::pos2(rect.left() + rect.width() * 0.3, rect.center().y);

        // A finger drag that starts on the slider must leave it alone.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(
            &ctx,
            vec![pointer_moved(start + egui::vec2(40.0, 0.0))],
            &mut value,
        );
        let response = touch_frame(
            &ctx,
            vec![pointer_moved(start + egui::vec2(80.0, 0.0))],
            &mut value,
        );
        assert!(!response.changed(), "a locked slider reported a change");
        touch_frame(
            &ctx,
            vec![pointer_button(start + egui::vec2(80.0, 0.0), false)],
            &mut value,
        );
        assert_eq!(value, 50.0, "the slider moved before it was tapped");

        // A tap arms it without changing the value.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(&ctx, vec![pointer_button(start, false)], &mut value);
        assert_eq!(value, 50.0, "the arming tap changed the value");

        // The next drag adjusts the level.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(
            &ctx,
            vec![pointer_moved(start + egui::vec2(40.0, 0.0))],
            &mut value,
        );
        let response = touch_frame(
            &ctx,
            vec![pointer_moved(start + egui::vec2(80.0, 0.0))],
            &mut value,
        );
        assert!(response.changed(), "the armed slider ignored the drag");
        touch_frame(
            &ctx,
            vec![pointer_button(start + egui::vec2(80.0, 0.0), false)],
            &mut value,
        );
        assert!(value > 50.0, "the armed slider did not move: {value}");
    }

    #[test]
    fn an_armed_slider_keeps_adjusting_through_a_vertical_drag() {
        let ctx = egui::Context::default();
        let mut value = 50.0_f64;
        let rect = touch_frame(&ctx, Vec::new(), &mut value).rect;
        let start = egui::pos2(rect.left() + rect.width() * 0.3, rect.center().y);

        // Tap to arm.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(&ctx, vec![pointer_button(start, false)], &mut value);

        // A vertical drag on the armed slider is an adjustment, not a page
        // scroll: the guard stays for the rest of the interaction.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        for dy in [20.0, 45.0, 70.0] {
            touch_frame(
                &ctx,
                vec![pointer_moved(start + egui::vec2(0.0, dy))],
                &mut value,
            );
        }
        touch_frame(
            &ctx,
            vec![pointer_button(start + egui::vec2(0.0, 70.0), false)],
            &mut value,
        );
        let after_vertical = value;

        // Still armed, so a horizontal drag changes the level.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(
            &ctx,
            vec![pointer_moved(start + egui::vec2(80.0, 0.0))],
            &mut value,
        );
        touch_frame(
            &ctx,
            vec![pointer_button(start + egui::vec2(80.0, 0.0), false)],
            &mut value,
        );
        assert!(
            value > after_vertical,
            "a vertical drag restored the guard: {after_vertical} -> {value}"
        );
    }

    #[test]
    fn a_tap_elsewhere_puts_the_guard_back() {
        let ctx = egui::Context::default();
        let mut value = 50.0_f64;
        let rect = touch_frame(&ctx, Vec::new(), &mut value).rect;
        let start = egui::pos2(rect.left() + rect.width() * 0.3, rect.center().y);

        // Tap the slider to arm it.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(&ctx, vec![pointer_button(start, false)], &mut value);

        // Tap empty content. The guard comes back, so the next drag is ignored.
        let blank = egui::pos2(rect.right() + 40.0, rect.center().y);
        touch_frame(&ctx, vec![pointer_moved(blank)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(blank), pointer_button(blank, true)],
            &mut value,
        );
        touch_frame(&ctx, vec![pointer_button(blank, false)], &mut value);

        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(
            &ctx,
            vec![pointer_moved(start + egui::vec2(80.0, 0.0))],
            &mut value,
        );
        touch_frame(
            &ctx,
            vec![pointer_button(start + egui::vec2(80.0, 0.0), false)],
            &mut value,
        );
        assert_eq!(value, 50.0, "a tap elsewhere did not restore the guard");
    }

    #[test]
    fn using_the_mouse_releases_the_guard_again() {
        let ctx = egui::Context::default();
        let mut value = 50.0_f64;
        let rect = touch_frame(&ctx, Vec::new(), &mut value).rect;
        let start = egui::pos2(rect.left() + rect.width() * 0.3, rect.center().y);

        // A touch guards the slider and a tap arms it.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(&ctx, vec![pointer_button(start, false)], &mut value);

        // Moving the mouse makes the mouse the current device again; the
        // guard and the armed state go away.
        frame(&ctx, vec![pointer_moved(start)], &mut value);
        frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        frame(
            &ctx,
            vec![pointer_moved(start + egui::vec2(80.0, 0.0))],
            &mut value,
        );
        frame(
            &ctx,
            vec![pointer_button(start + egui::vec2(80.0, 0.0), false)],
            &mut value,
        );

        assert!(value > 50.0, "the mouse drag was still guarded: {value}");
    }

    #[test]
    fn a_swipe_elsewhere_puts_the_guard_back() {
        let ctx = egui::Context::default();
        let mut value = 50.0_f64;
        let rect = touch_frame(&ctx, Vec::new(), &mut value).rect;
        let start = egui::pos2(rect.left() + rect.width() * 0.3, rect.center().y);

        // Tap the slider, then drag it to a new level.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(&ctx, vec![pointer_button(start, false)], &mut value);
        // One idle frame: the tap arms the slider when the press is released.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(
            &ctx,
            vec![pointer_moved(start + egui::vec2(80.0, 0.0))],
            &mut value,
        );
        touch_frame(
            &ctx,
            vec![pointer_button(start + egui::vec2(80.0, 0.0), false)],
            &mut value,
        );
        let armed_value = value;
        assert!(armed_value > 50.0, "the slider did not arm: {value}");

        // A vertical swipe on empty content puts the guard back instead of
        // leaving the level armed.
        let swipe = egui::pos2(rect.right() + 40.0, rect.center().y);
        touch_frame(&ctx, vec![pointer_moved(swipe)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(swipe), pointer_button(swipe, true)],
            &mut value,
        );
        for dy in [20.0, 45.0, 70.0] {
            touch_frame(
                &ctx,
                vec![pointer_moved(swipe + egui::vec2(0.0, dy))],
                &mut value,
            );
        }
        touch_frame(
            &ctx,
            vec![pointer_button(swipe + egui::vec2(0.0, 70.0), false)],
            &mut value,
        );
        assert_eq!(value, armed_value, "the swipe changed the level");

        // The guard is back: a horizontal drag no longer moves it.
        touch_frame(&ctx, vec![pointer_moved(start)], &mut value);
        touch_frame(
            &ctx,
            vec![pointer_moved(start), pointer_button(start, true)],
            &mut value,
        );
        touch_frame(
            &ctx,
            vec![pointer_moved(start - egui::vec2(80.0, 0.0))],
            &mut value,
        );
        touch_frame(
            &ctx,
            vec![pointer_button(start - egui::vec2(80.0, 0.0), false)],
            &mut value,
        );
        assert_eq!(value, armed_value, "a swipe did not restore the guard");
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
