//! Bounded undo/redo history for user-editable session state.
//!
//! A [`Snapshot`] holds the preferences and the selected dance and tune: the
//! state a preference edit or a picker choice can change. The saved libraries
//! are deliberately left out. They are edited by downloading, deleting, or
//! re-filing a card, which is not a preference edit, and copying them into
//! every step would make a hundred-entry stack expensive.
//!
//! A step records the state *before* the change, never after: undoing the first
//! keystroke in a field has to restore the text as it was before the field was
//! touched. Consecutive changes that belong to one gesture share a
//! [`MergeKey`], so typing a word or dragging a slider is one step instead of
//! one step per frame.

use contra_core::{Dance, Preferences, Tune};
use eframe::egui;

/// How many states the undo stack keeps.
pub(in crate::app) const HISTORY_LIMIT: usize = 100;

/// The user-editable state an undo step restores.
#[derive(Clone, Debug, PartialEq)]
pub(in crate::app) struct Snapshot {
    pub prefs: Preferences,
    pub dance: Dance,
    pub tune: Tune,
    /// Seed for the shuffled alternative call aliases. It travels with the
    /// dance, so restoring a dance also restores the wording it was showing.
    pub alias_seed: u64,
}

/// Which history direction an input asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum HistoryAction {
    Undo,
    Redo,
}

/// Changes that belong to the same user gesture.
///
/// While the key stays the same, the first recorded snapshot is kept and the
/// later ones are dropped: the state before the whole gesture is the only one
/// undo should return to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::app) enum MergeKey {
    /// A focused widget, typically a text field receiving keystrokes.
    Focus(egui::Id),
    /// A pointer held down, typically a slider being dragged.
    Pointer,
}

struct Entry {
    snapshot: Snapshot,
    /// Set while the gesture that produced this step is still running.
    merge: Option<MergeKey>,
}

/// Two stacks of previous states, capped at [`Self::limit`] entries each.
pub(in crate::app) struct History {
    undo: Vec<Entry>,
    redo: Vec<Entry>,
    limit: usize,
}

impl Default for History {
    fn default() -> Self {
        Self::new(HISTORY_LIMIT)
    }
}

impl History {
    pub(in crate::app) fn new(limit: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Record `previous` as the state to return to when `current` turns out to
    /// differ from it. Returns whether the state changed at all.
    ///
    /// A no-op change records nothing, a new change clears the redo stack, and
    /// a change that continues the previous gesture keeps the earlier snapshot.
    pub(in crate::app) fn record(
        &mut self,
        previous: Snapshot,
        current: &Snapshot,
        merge: Option<MergeKey>,
    ) -> bool {
        if previous == *current {
            return false;
        }
        // An edit that turns the last recorded edit into a no-op cancels it:
        // toggling a checkbox off and on again must not leave two undo steps
        // that walk through the value the user rejected.
        if merge.is_none()
            && self
                .undo
                .last()
                .is_some_and(|entry| entry.merge.is_none() && entry.snapshot == *current)
        {
            self.undo.pop();
            return false;
        }
        self.redo.clear();
        if merge.is_some_and(|merge| {
            self.undo
                .last()
                .is_some_and(|entry| entry.merge == Some(merge))
        }) {
            return true;
        }
        self.push_undo(Entry {
            snapshot: previous,
            merge,
        });
        true
    }

    /// Pop the state to restore, moving `current` onto the redo stack.
    pub(in crate::app) fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let entry = self.undo.pop()?;
        self.end_gesture();
        self.push_redo(current);
        Some(entry.snapshot)
    }

    /// Pop the state to redo, moving `current` onto the undo stack.
    pub(in crate::app) fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let entry = self.redo.pop()?;
        self.push_undo(Entry {
            snapshot: current,
            merge: None,
        });
        Some(entry.snapshot)
    }

    pub(in crate::app) fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Number of steps on the undo stack.
    #[cfg(test)]
    pub(in crate::app) fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Number of steps on the redo stack.
    #[cfg(test)]
    pub(in crate::app) fn redo_len(&self) -> usize {
        self.redo.len()
    }

    pub(in crate::app) fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Approximate bytes held by both stacks.
    ///
    /// Snapshots clone the preferences (including the alias table) and the
    /// selected cards, so a full hundred-entry stack is a real, if modest,
    /// consumer worth showing in the memory table.
    pub(in crate::app) fn estimated_bytes(&self) -> u64 {
        let snapshot = |snapshot: &Snapshot| {
            std::mem::size_of::<Snapshot>() as u64
                + serde_json::to_vec(&snapshot.prefs)
                    .map(|json| json.len() as u64)
                    .unwrap_or(0)
                + (snapshot.dance.title.len()
                    + snapshot.dance.choreographer.len()
                    + snapshot.tune.title.len()
                    + snapshot.tune.abc.len()) as u64
        };
        self.undo
            .iter()
            .chain(self.redo.iter())
            .map(|entry| snapshot(&entry.snapshot))
            .sum()
    }

    fn push_undo(&mut self, entry: Entry) {
        if self.undo.len() >= self.limit {
            self.undo.remove(0);
        }
        self.undo.push(entry);
    }

    fn push_redo(&mut self, snapshot: Snapshot) {
        if self.redo.len() >= self.limit {
            self.redo.remove(0);
        }
        self.redo.push(Entry {
            snapshot,
            merge: None,
        });
    }

    /// A gesture cannot continue across an undo or a redo: a later keystroke in
    /// the same field is a new step, not a continuation of the state just
    /// restored.
    fn end_gesture(&mut self) {
        if let Some(entry) = self.undo.last_mut() {
            entry.merge = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use contra_core::MelodyInstrument;

    fn snapshot() -> Snapshot {
        Snapshot {
            prefs: Preferences::default(),
            dance: crate::defaults::default_dance(),
            tune: crate::defaults::default_tune(),
            alias_seed: 0,
        }
    }

    fn with_melody(melody: MelodyInstrument) -> Snapshot {
        let mut snapshot = snapshot();
        snapshot.prefs.melody = melody;
        snapshot
    }

    #[test]
    fn a_no_op_change_records_nothing() {
        let mut history = History::default();
        let state = snapshot();
        assert!(!history.record(state.clone(), &state, None));
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn an_edit_changed_back_cancels_the_previous_step() {
        let mut history = History::default();
        let first = snapshot();
        let second = with_melody(MelodyInstrument::Accordion);
        assert!(history.record(first.clone(), &second, None));
        assert_eq!(
            history.undo(second.clone()),
            Some(first.clone()),
            "the first edit is one step"
        );

        // The same sequence without an undo in between: off and back on is
        // not two steps that walk through the rejected value.
        let mut history = History::default();
        assert!(history.record(first.clone(), &second, None));
        assert!(!history.record(second, &first, None));
        assert!(!history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn undo_and_redo_walk_between_the_recorded_states() {
        let mut history = History::default();
        let first = snapshot();
        let second = with_melody(MelodyInstrument::Accordion);
        assert!(history.record(first.clone(), &second, None));
        assert!(history.can_undo());
        assert!(!history.can_redo());

        let restored = history.undo(second.clone()).expect("a state to undo to");
        assert_eq!(restored, first);
        assert!(history.can_redo());
        assert!(!history.can_undo());

        let redone = history.redo(first.clone()).expect("a state to redo to");
        assert_eq!(redone, second);
        assert!(history.can_undo());
        assert!(!history.can_redo());

        assert_eq!(history.undo(second), Some(first.clone()));
        assert_eq!(history.undo(first), None);
    }

    #[test]
    fn a_new_change_clears_the_redo_stack() {
        let mut history = History::default();
        let first = snapshot();
        let second = with_melody(MelodyInstrument::Accordion);
        history.record(first.clone(), &second, None);
        assert_eq!(history.undo(second.clone()), Some(first.clone()));
        assert!(history.can_redo());

        let mut third = with_melody(MelodyInstrument::Fiddle);
        third.prefs.melody_volume = 0.4;
        history.record(first.clone(), &third, None);
        assert!(
            !history.can_redo(),
            "an edit after undo discards the redo steps"
        );
        assert_eq!(history.undo(third), Some(first));
    }

    #[test]
    fn the_stack_is_capped_and_drops_the_oldest_step() {
        let mut history = History::new(3);
        // Four distinct states: a sequence that returns exactly to an older
        // state would legitimately cancel its pair (see the test above), so
        // each step also moves the volume.
        let mut states = vec![snapshot()];
        for step in 1..=4 {
            let mut state = snapshot();
            state.prefs.melody_volume = 0.1 * step as f32;
            states.push(state);
        }
        for pair in states.windows(2) {
            history.record(pair[0].clone(), &pair[1], None);
        }

        // Four changes were recorded but only the newest three are kept.
        let newest = states[4].clone();
        let third = history.undo(newest).expect("the newest step");
        assert_eq!(third, states[3]);
        let second = history.undo(third).expect("another step");
        assert_eq!(second, states[2]);
        let first = history.undo(second).expect("the last kept step");
        assert_eq!(first, states[1]);
        assert_eq!(history.undo(first), None, "the oldest step was dropped");
    }

    #[test]
    fn a_gesture_is_one_step() {
        let mut history = History::default();
        let focus = MergeKey::Focus(egui::Id::unique("role-field"));
        let first = snapshot();
        let mut second = first.clone();
        second.prefs.role_terms.left_singular = "p".into();
        let mut third = second.clone();
        third.prefs.role_terms.left_singular = "po".into();
        let mut fourth = third.clone();
        fourth.prefs.role_terms.left_singular = "por".into();

        history.record(first.clone(), &second, Some(focus));
        history.record(second, &third, Some(focus));
        history.record(third, &fourth, Some(focus));
        assert_eq!(
            history.undo(fourth),
            Some(first.clone()),
            "typing three letters is one undo step"
        );

        // Releasing focus ends the gesture: the next typed word is its own step.
        let mut fifth = first.clone();
        fifth.prefs.role_terms.left_singular = "ports".into();
        history.record(first, &fifth, None);
        let mut sixth = fifth.clone();
        sixth.prefs.role_terms.left_singular = "ports!".into();
        history.record(fifth.clone(), &sixth, Some(focus));
        assert_eq!(history.undo(sixth), Some(fifth));
    }

    #[test]
    fn a_gesture_does_not_continue_across_an_undo() {
        let mut history = History::default();
        let focus = MergeKey::Focus(egui::Id::unique("role-field"));
        let first = snapshot();
        let mut second = first.clone();
        second.prefs.role_terms.left_singular = "p".into();
        history.record(first.clone(), &second, Some(focus));
        assert_eq!(history.undo(second.clone()), Some(first.clone()));

        // The user types again without leaving the field. That is a new step,
        // so undo returns to the restored text rather than dropping the edit.
        let mut third = first.clone();
        third.prefs.role_terms.left_singular = "po".into();
        history.record(first.clone(), &third, Some(focus));
        assert_eq!(history.undo(third), Some(first));
    }

    #[test]
    fn a_pointer_drag_is_one_step() {
        let mut history = History::default();
        let first = snapshot();
        let mut second = first.clone();
        second.prefs.melody_volume = 0.6;
        let mut third = second.clone();
        third.prefs.melody_volume = 0.5;
        history.record(first.clone(), &second, Some(MergeKey::Pointer));
        history.record(second, &third, Some(MergeKey::Pointer));
        assert_eq!(history.undo(third), Some(first));
    }
}
