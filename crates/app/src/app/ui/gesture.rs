//! Pure press and reorder helpers for the saved-item and folder lists.
//!
//! The widgets pass pointer positions and times in; this module decides when a
//! touch becomes a long press, how much movement cancels a hold, and where a
//! dragged row lands. Keeping the arithmetic here lets the phone gate and the
//! drop index be tested without a touchscreen or an egui context.

/// How long a touch must stay still before it becomes a long press.
///
/// iOS icon-moving uses about half a second; 0.5 s also leaves a comfortable
/// margin over a tap while staying below egui's 0.8 s long-touch threshold.
pub(super) const LONG_PRESS_SECONDS: f64 = 0.5;

/// How far a pointer may drift while holding before the gesture is a drag.
///
/// This is deliberately larger than a finger's jitter and smaller than the
/// height of a 44-point row, so a small wobble still holds and a deliberate
/// scroll or reorder cancels the hold.
pub(super) const LONG_PRESS_SLOP: f32 = 8.0;

/// Where a tracked press is in its lifetime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum PressPhase {
    /// No press is tracked.
    #[default]
    Idle,
    /// The pointer is down and the hold timer is running.
    Holding,
    /// The hold completed; a release is a long press, not a tap.
    Fired,
    /// The pointer moved too far, so this is a drag or a scroll.
    Cancelled,
}

/// What releasing a tracked press means.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum PressEnd {
    /// There was no press to release.
    #[default]
    None,
    /// A short press: select the item.
    Tap,
    /// A completed hold: open the menu or start moving the row.
    LongPress,
    /// The pointer moved during the press: the row is being dragged.
    Drag,
}

/// A press-and-hold timer with a movement threshold.
///
/// The same state machine drives the folder drag gate and the item context
/// menu, so a hold either lifts the row or opens its menu, never both.
#[derive(Clone, Copy, Debug)]
pub(super) struct LongPress {
    phase: PressPhase,
    started: f64,
    origin: (f32, f32),
    position: (f32, f32),
}

impl Default for LongPress {
    fn default() -> Self {
        Self {
            phase: PressPhase::Idle,
            started: 0.0,
            origin: (0.0, 0.0),
            position: (0.0, 0.0),
        }
    }
}

impl LongPress {
    /// Start tracking a press at `now`. Replaces any previous press.
    pub(super) fn begin(&mut self, now: f64, x: f32, y: f32) {
        self.phase = PressPhase::Holding;
        self.started = now;
        self.origin = (x, y);
        self.position = (x, y);
    }

    /// Advance a held press. Returns the resulting phase.
    pub(super) fn update(&mut self, now: f64, x: f32, y: f32) -> PressPhase {
        self.position = (x, y);
        if self.phase == PressPhase::Holding {
            let dx = x - self.origin.0;
            let dy = y - self.origin.1;
            if (dx * dx + dy * dy).sqrt() > LONG_PRESS_SLOP {
                self.phase = PressPhase::Cancelled;
            } else if now - self.started >= LONG_PRESS_SECONDS {
                self.phase = PressPhase::Fired;
            }
        }
        self.phase
    }

    /// How far the hold has progressed, from 0 to 1.
    ///
    /// Used for the row's press-and-hold progress line. A cancelled press
    /// reports zero again so the line disappears immediately.
    pub(super) fn progress(&self, now: f64) -> f32 {
        match self.phase {
            PressPhase::Holding => {
                (((now - self.started) / LONG_PRESS_SECONDS) as f32).clamp(0.0, 1.0)
            }
            PressPhase::Fired => 1.0,
            _ => 0.0,
        }
    }

    pub(super) fn phase(&self) -> PressPhase {
        self.phase
    }

    /// Stop tracking, reporting what the release means and resetting to idle.
    pub(super) fn end(&mut self) -> PressEnd {
        let end = match self.phase {
            PressPhase::Idle => PressEnd::None,
            PressPhase::Holding => PressEnd::Tap,
            PressPhase::Fired => PressEnd::LongPress,
            PressPhase::Cancelled => PressEnd::Drag,
        };
        self.phase = PressPhase::Idle;
        end
    }

    /// Abandon the press without reporting a release.
    #[cfg(test)]
    pub(super) fn cancel(&mut self) {
        self.phase = PressPhase::Idle;
    }
}

/// The insertion slot for a pointer at `y` among row centres.
///
/// The result is an index in `0..=centers.len()`: the gap before the row whose
/// centre is first below the pointer, or the gap after the last row. Comparing
/// centres (rather than edges) makes the row swap at the point where the
/// pointer passes the middle of its neighbour, which is what a drag feels like.
pub(super) fn insertion_slot(centers: &[f32], y: f32) -> usize {
    centers
        .iter()
        .position(|center| y < *center)
        .unwrap_or(centers.len())
}

/// Map a drag from `from` to an insertion slot onto the final list index.
///
/// Dropping into the gap just above or below the dragged row is a no-op, and a
/// dragged index outside the list is ignored. `None` means "do not reorder".
pub(super) fn drop_target(from: usize, slot: usize, len: usize) -> Option<usize> {
    if from >= len {
        return None;
    }
    let to = if slot > from { slot - 1 } else { slot };
    let to = to.min(len - 1);
    (to != from).then_some(to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quick_press_is_a_tap() {
        let mut press = LongPress::default();
        press.begin(10.0, 20.0, 20.0);
        press.update(10.2, 21.0, 20.0);
        assert_eq!(press.phase(), PressPhase::Holding);
        assert_eq!(press.end(), PressEnd::Tap);
        assert_eq!(press.phase(), PressPhase::Idle);
    }

    #[test]
    fn a_still_hold_fires_after_half_a_second() {
        let mut press = LongPress::default();
        press.begin(0.0, 10.0, 10.0);

        assert_eq!(press.update(0.49, 10.0, 10.0), PressPhase::Holding);
        assert!(press.progress(0.49) > 0.9);

        assert_eq!(press.update(0.5, 10.0, 10.0), PressPhase::Fired);
        assert_eq!(press.progress(0.5), 1.0);
        assert_eq!(press.end(), PressEnd::LongPress);
    }

    #[test]
    fn moving_past_the_slop_cancels_the_hold() {
        let mut press = LongPress::default();
        press.begin(0.0, 10.0, 10.0);

        // Small finger wander still holds.
        assert_eq!(press.update(0.2, 14.0, 13.0), PressPhase::Holding);
        // A deliberate drag cancels and stays cancelled even when the pointer
        // returns to the start.
        assert_eq!(press.update(0.3, 10.0, 40.0), PressPhase::Cancelled);
        assert_eq!(press.update(0.6, 10.0, 10.0), PressPhase::Cancelled);
        assert_eq!(press.progress(0.6), 0.0);
        assert_eq!(press.end(), PressEnd::Drag);
    }

    #[test]
    fn a_hold_that_fires_stays_fired_when_the_pointer_moves() {
        let mut press = LongPress::default();
        press.begin(0.0, 10.0, 10.0);
        press.update(0.6, 10.0, 10.0);
        assert_eq!(press.update(0.7, 10.0, 200.0), PressPhase::Fired);
        assert_eq!(press.end(), PressEnd::LongPress);
    }

    #[test]
    fn progress_clamps_and_a_cancelled_press_reports_zero() {
        let mut press = LongPress::default();
        press.begin(5.0, 0.0, 0.0);
        assert_eq!(press.progress(4.0), 0.0);
        assert_eq!(press.progress(5.25), 0.5);
        assert_eq!(press.progress(9.0), 1.0);
        press.cancel();
        assert_eq!(press.progress(9.0), 0.0);
        assert_eq!(press.end(), PressEnd::None);
    }

    #[test]
    fn the_insertion_slot_sits_between_row_centres() {
        let centers = [20.0, 60.0, 100.0];
        assert_eq!(insertion_slot(&centers, 0.0), 0);
        assert_eq!(insertion_slot(&centers, 19.0), 0);
        assert_eq!(insertion_slot(&centers, 20.0), 1);
        assert_eq!(insertion_slot(&centers, 59.0), 1);
        assert_eq!(insertion_slot(&centers, 60.0), 2);
        assert_eq!(insertion_slot(&centers, 140.0), 3);
        assert_eq!(insertion_slot(&[], 50.0), 0);
    }

    #[test]
    fn dropping_a_row_maps_the_insertion_slot_to_a_list_index() {
        // Dropping before the first slot or just after the row is a no-op.
        assert_eq!(drop_target(0, 0, 3), None);
        assert_eq!(drop_target(0, 1, 3), None);
        // Moving down one slot swaps with the next row.
        assert_eq!(drop_target(0, 2, 3), Some(1));
        // Moving to the end.
        assert_eq!(drop_target(0, 3, 3), Some(2));
        // Moving up.
        assert_eq!(drop_target(2, 0, 3), Some(0));
        assert_eq!(drop_target(2, 1, 3), Some(1));
        // Dropping at the end of the list is a no-op for the last row.
        assert_eq!(drop_target(2, 3, 3), None);
        // Invalid source or an empty list.
        assert_eq!(drop_target(3, 0, 3), None);
        assert_eq!(drop_target(0, 0, 0), None);
        // Clamping keeps an over-long slot inside the list.
        assert_eq!(drop_target(1, 99, 3), Some(2));
    }
}
