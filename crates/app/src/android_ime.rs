//! The Android soft keyboard bridge.
//!
//! A native activity has no editable view, so Android's input method refuses to
//! show a keyboard for its content view. `MainActivity` therefore owns a
//! one-pixel invisible `EditText`; when egui focuses a text field this module
//! asks the activity to focus it and show the IME. The activity forwards
//! committed text and editing keys back through JNI, and this module turns
//! them into the same [`egui::Event`] values a desktop keyboard produces.
//!
//! The event queue and key mapping are compiled on the test host as well, so
//! the mapping and draining can be tested without a device or a Java VM.

#[cfg(any(target_os = "android", test))]
mod bridge {
    use std::sync::Mutex;
    #[cfg(target_os = "android")]
    use std::sync::OnceLock;

    use eframe::egui;

    /// Editing key codes; they must match the `KEY_*` constants in
    /// `MainActivity.java`.
    pub(super) const KEY_BACKSPACE: i32 = 1;
    pub(super) const KEY_DELETE: i32 = 2;
    pub(super) const KEY_ENTER: i32 = 3;
    pub(super) const KEY_LEFT: i32 = 4;
    pub(super) const KEY_RIGHT: i32 = 5;
    pub(super) const KEY_UP: i32 = 6;
    pub(super) const KEY_DOWN: i32 = 7;

    /// Text and keys the activity delivered but the frame has not applied yet.
    ///
    /// A plain `Mutex<Vec<_>>` like the media-key queue: the Java callback
    /// cannot own a receiver, and draining must never block a frame.
    pub(super) type EventQueue = Mutex<Vec<egui::Event>>;

    #[cfg(target_os = "android")]
    static EVENTS: OnceLock<EventQueue> = OnceLock::new();

    /// The keyboard state the activity has accepted.
    ///
    /// Only the UI thread touches this, but it lives behind a mutex like the
    /// event queue so a panic in one frame cannot lose the keyboard.
    #[cfg(target_os = "android")]
    static KEYBOARD: Mutex<KeyboardState> = Mutex::new(KeyboardState::new());

    /// Frames a focused widget may stop being a text edit before the keyboard
    /// is hidden.
    ///
    /// A number box is an egui `DragValue`/slider value that only renders its
    /// text field while it is being edited. It can drop out of text-edit mode
    /// for a frame when its value is clamped or the layout is rebuilt, and
    /// hiding the IME for that frame is what makes the Android keyboard flash.
    /// Keep the IME for one frame so a transient state change is invisible; a
    /// real end of editing still dismisses it on the next frame.
    pub(super) const IDLE_FRAMES_BEFORE_HIDE: u8 = 2;

    /// What the activity should be told about the keyboard this frame.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(super) enum KeyboardRequest {
        /// Leave the keyboard alone; the state did not change.
        Nothing,
        /// Focus the invisible field and show the IME, or bring it back after
        /// the system Back gesture dismissed it.
        Show,
        /// Hide the IME and let the invisible field go.
        Hide,
    }

    /// The keyboard state that survives between frames.
    ///
    /// `focus` is the egui widget that owns the keyboard, not necessarily a
    /// text edit: a number box keeps focus while it is between edit and button
    /// mode.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(super) struct KeyboardState {
        visible: bool,
        focus: Option<egui::Id>,
        idle_frames: u8,
        /// A tap happened last frame. Numeric boxes only become text edits on
        /// the frame after the tap, so the request has to survive one frame.
        pending_tap: bool,
    }

    impl KeyboardState {
        pub(super) const fn new() -> Self {
            Self {
                visible: false,
                focus: None,
                idle_frames: 0,
                pending_tap: false,
            }
        }
    }

    /// Decide the keyboard state for one frame.
    ///
    /// The rules keep the IME tied to the *widget* rather than to one frame of
    /// its text-edit state:
    /// - a focused text edit keeps it up;
    /// - a tap gives the keyboard one extra frame, because a number box only
    ///   becomes a text edit on the frame after the tap and a press can clear
    ///   the old field's focus before the new one claims it;
    /// - a focused widget that is not (or is no longer) a text edit gets
    ///   [`IDLE_FRAMES_BEFORE_HIDE`] frames to become one, so a number box can
    ///   dip out of edit mode without dismissing the keyboard;
    /// - losing focus with no tap hides it, and a tap on the same text edit
    ///   asks for it again, which recovers from the system Back gesture hiding
    ///   it without egui losing focus.
    pub(super) fn advance(
        mut state: KeyboardState,
        focused: Option<egui::Id>,
        text_edit: bool,
        tapped: bool,
    ) -> (KeyboardState, KeyboardRequest) {
        // A tap spans two frames (press and release) and a number box starts
        // editing one frame after it, so remember the tap for that frame.
        let resummon = tapped || state.pending_tap;
        state.pending_tap = tapped;

        let request = if !state.visible {
            if text_edit && focused.is_some() {
                KeyboardRequest::Show
            } else {
                KeyboardRequest::Nothing
            }
        } else if focused.is_none() && !resummon {
            KeyboardRequest::Hide
        } else if text_edit {
            if resummon {
                KeyboardRequest::Show
            } else {
                KeyboardRequest::Nothing
            }
        } else if resummon {
            // The tap may be the one that turns a number box back into a text
            // edit, or the press that clears one field before another claims
            // focus. Give the next frame a chance to say so instead of hiding
            // (and immediately re-showing) the keyboard under the finger.
            KeyboardRequest::Nothing
        } else if state.idle_frames + 1 < IDLE_FRAMES_BEFORE_HIDE {
            KeyboardRequest::Nothing
        } else {
            KeyboardRequest::Hide
        };

        match request {
            KeyboardRequest::Show => {
                state.visible = true;
                state.focus = focused;
                state.idle_frames = 0;
                state.pending_tap = false;
            }
            KeyboardRequest::Hide => {
                state.visible = false;
                state.focus = None;
                state.idle_frames = 0;
                state.pending_tap = false;
            }
            KeyboardRequest::Nothing => {
                if state.visible {
                    if state.focus != focused {
                        // A newly focused widget gets the full grace period to
                        // render its text edit.
                        state.idle_frames = 0;
                    }
                    state.focus = focused;
                    if text_edit {
                        state.idle_frames = 0;
                    } else {
                        state.idle_frames = state.idle_frames.saturating_add(1);
                    }
                }
            }
        }

        (state, request)
    }

    #[cfg(target_os = "android")]
    fn queue() -> &'static EventQueue {
        EVENTS.get_or_init(|| Mutex::new(Vec::new()))
    }

    /// Add an event for the next frame to pick up.
    ///
    /// A poisoned lock still hands back the vector: losing a typed character
    /// because an unrelated frame panicked would be worse than reusing the
    /// queue.
    pub(super) fn enqueue(queue: &EventQueue, event: egui::Event) {
        let mut events = match queue.lock() {
            Ok(events) => events,
            Err(poisoned) => poisoned.into_inner(),
        };
        events.push(event);
        // The IME runs on a Java thread. Ask for a frame so the character
        // appears even when the session tab is otherwise idle.
        crate::media_keys::request_repaint();
    }

    /// Take every event received since the previous call.
    pub(super) fn drain(queue: &EventQueue) -> Vec<egui::Event> {
        let mut events = match queue.lock() {
            Ok(events) => events,
            Err(poisoned) => poisoned.into_inner(),
        };
        std::mem::take(&mut *events)
    }

    /// Queue text committed by the IME.
    #[cfg(target_os = "android")]
    pub(super) fn push_text(text: String) {
        if !text.is_empty() {
            enqueue(queue(), egui::Event::Text(text));
        }
    }

    /// Queue an editing key from the IME.
    #[cfg(target_os = "android")]
    pub(super) fn push_key(code: i32) {
        if let Some(event) = key_event(code) {
            enqueue(queue(), event);
        }
    }

    /// Take every event received since the previous frame.
    #[cfg(target_os = "android")]
    pub(crate) fn drain_events() -> Vec<egui::Event> {
        drain(queue())
    }

    /// The egui key event for one Java key code, or `None` for a code this
    /// build does not know (for example an activity newer than the library).
    pub(super) fn key_event(code: i32) -> Option<egui::Event> {
        let key = match code {
            KEY_BACKSPACE => egui::Key::Backspace,
            KEY_DELETE => egui::Key::Delete,
            KEY_ENTER => egui::Key::Enter,
            KEY_LEFT => egui::Key::ArrowLeft,
            KEY_RIGHT => egui::Key::ArrowRight,
            KEY_UP => egui::Key::ArrowUp,
            KEY_DOWN => egui::Key::ArrowDown,
            _ => return None,
        };
        Some(egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        })
    }

    /// Show or hide the keyboard to match the focused egui widget.
    ///
    /// The activity is only told when the state actually changed (or when a
    /// tap asks for the keyboard again), never once per frame. See
    /// [`advance`] for the rules.
    #[cfg(target_os = "android")]
    pub(crate) fn sync_text_input(ctx: &egui::Context) {
        let focused = ctx.memory(|memory| memory.focused());
        let text_edit = ctx.text_edit_focused();
        // A tap spans a press frame and a release frame; a number box starts
        // editing on the frame after the release, so count both.
        let tapped = ctx.input(|input| input.pointer.any_pressed() || input.pointer.any_click());

        let mut keyboard = match KEYBOARD.lock() {
            Ok(keyboard) => keyboard,
            Err(poisoned) => poisoned.into_inner(),
        };
        let (next, request) = advance(*keyboard, focused, text_edit, tapped);
        let visible = match request {
            KeyboardRequest::Nothing => None,
            KeyboardRequest::Show => Some(true),
            KeyboardRequest::Hide => Some(false),
        };
        match visible {
            // Remember the new state only once the activity accepted it, so a
            // frame that runs before the activity is attached retries instead
            // of leaving the keyboard stuck.
            Some(visible) => {
                if crate::media_keys::set_text_input_active(visible) {
                    *keyboard = next;
                }
            }
            None => *keyboard = next,
        }
    }

    /// Take sole ownership of the Android IME.
    ///
    /// egui also publishes an IME request in its platform output, and on
    /// Android `winit` turns that into `AndroidApp::show_soft_input` /
    /// `hide_soft_input` for the decor view. That is a second controller for
    /// the same keyboard: it calls hide and show *in the same frame* whenever
    /// egui interrupts composition (which happens when a number box is tapped
    /// to start editing), and it hides the IME when a number box briefly
    /// leaves text-edit mode. Either path makes the keyboard flash.
    ///
    /// The activity's invisible `EditText` is the only thing that can deliver
    /// text to a native activity, so the bridge is the single owner and the
    /// backend is told nothing. The cursor rectangle this drops is already a
    /// no-op on Android (`winit`'s `set_ime_cursor_area` is empty there).
    #[cfg(target_os = "android")]
    pub(crate) fn suppress_backend_ime(ctx: &egui::Context) {
        ctx.output_mut(|output| output.ime = None);
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn maps_the_activity_key_codes() {
            assert_eq!(key_event(KEY_BACKSPACE), Some(key(egui::Key::Backspace)));
            assert_eq!(key_event(KEY_DELETE), Some(key(egui::Key::Delete)));
            assert_eq!(key_event(KEY_ENTER), Some(key(egui::Key::Enter)));
            assert_eq!(key_event(KEY_LEFT), Some(key(egui::Key::ArrowLeft)));
            assert_eq!(key_event(KEY_RIGHT), Some(key(egui::Key::ArrowRight)));
            assert_eq!(key_event(KEY_UP), Some(key(egui::Key::ArrowUp)));
            assert_eq!(key_event(KEY_DOWN), Some(key(egui::Key::ArrowDown)));
            assert_eq!(key_event(0), None);
            assert_eq!(key_event(99), None);
        }

        #[test]
        fn drains_text_and_keys_in_order_and_once() {
            let queue = EventQueue::new(Vec::new());
            enqueue(&queue, egui::Event::Text("sandy".into()));
            enqueue(&queue, key_event(KEY_BACKSPACE).expect("known key"));
            enqueue(&queue, egui::Event::Text(" boys".into()));

            assert_eq!(
                drain(&queue),
                vec![
                    egui::Event::Text("sandy".into()),
                    key(egui::Key::Backspace),
                    egui::Event::Text(" boys".into()),
                ]
            );
            assert!(drain(&queue).is_empty());
        }

        #[test]
        fn empty_text_and_unknown_keys_are_ignored() {
            assert!(key_event(0).is_none());
            assert!(key_event(99).is_none());
        }

        #[test]
        fn a_poisoned_queue_still_delivers() {
            let queue = EventQueue::new(Vec::new());
            // The panic message is expected output; the test only checks that
            // the queue recovers from the poisoned lock.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _guard = queue.lock().expect("fresh queue");
                panic!("a frame panicked while draining");
            }));
            enqueue(&queue, egui::Event::Text("ok".into()));
            assert_eq!(drain(&queue), vec![egui::Event::Text("ok".into())]);
        }

        #[test]
        fn a_text_field_asks_for_the_keyboard_once_and_keeps_it() {
            let field = egui::Id::unique("sandy");
            let (mut state, request) = advance(KeyboardState::new(), Some(field), true, false);
            assert_eq!(request, KeyboardRequest::Show);
            assert!(state.visible);

            for _ in 0..8 {
                let (next, request) = advance(state, Some(field), true, false);
                assert_eq!(
                    request,
                    KeyboardRequest::Nothing,
                    "typing must not re-show the keyboard on every frame"
                );
                assert!(next.visible);
                state = next;
            }
        }

        /// A number box is a text field only while it is being edited. A frame
        /// between edit and button mode must not dismiss the keyboard.
        #[test]
        fn a_number_box_that_leaves_text_edit_mode_for_a_frame_keeps_the_keyboard() {
            let box_id = egui::Id::unique("tempo");
            let (state, _) = advance(KeyboardState::new(), Some(box_id), true, false);
            assert!(state.visible);

            let (state, request) = advance(state, Some(box_id), false, false);
            assert_eq!(request, KeyboardRequest::Nothing);
            assert!(state.visible, "one idle frame kept the keyboard");

            let (state, request) = advance(state, Some(box_id), true, false);
            assert_eq!(request, KeyboardRequest::Nothing);
            assert!(state.visible);
            assert_eq!(state.idle_frames, 0, "editing again clears the grace");
        }

        #[test]
        fn a_focused_widget_that_stays_out_of_text_editing_hides_the_keyboard() {
            let box_id = egui::Id::unique("tempo");
            let (state, _) = advance(KeyboardState::new(), Some(box_id), true, false);
            let (state, request) = advance(state, Some(box_id), false, false);
            assert_eq!(request, KeyboardRequest::Nothing);
            assert!(state.visible);

            let (state, request) = advance(state, Some(box_id), false, false);
            assert_eq!(request, KeyboardRequest::Hide);
            assert!(!state.visible);
            assert_eq!(state.focus, None);
        }

        #[test]
        fn losing_focus_hides_the_keyboard_at_once() {
            let field = egui::Id::unique("alias");
            let (state, _) = advance(KeyboardState::new(), Some(field), true, false);
            let (state, request) = advance(state, None, false, false);
            assert_eq!(request, KeyboardRequest::Hide);
            assert!(!state.visible);
        }

        #[test]
        fn a_tap_on_the_focused_text_field_asks_for_the_keyboard_again() {
            let field = egui::Id::unique("alias");
            let (state, _) = advance(KeyboardState::new(), Some(field), true, false);
            let (state, request) = advance(state, Some(field), true, true);
            assert_eq!(
                request,
                KeyboardRequest::Show,
                "the system Back gesture can hide the IME without clearing focus"
            );
            assert!(state.visible);
        }

        #[test]
        fn moving_between_text_fields_keeps_the_keyboard_up() {
            let alias = egui::Id::unique("alias");
            let search = egui::Id::unique("search");
            let (state, _) = advance(KeyboardState::new(), Some(alias), true, false);
            let (state, request) = advance(state, Some(search), true, false);
            assert_eq!(request, KeyboardRequest::Nothing);
            assert!(state.visible);
            assert_eq!(state.focus, Some(search), "focus followed the new field");
        }

        #[test]
        fn a_tap_on_a_number_box_that_is_not_editing_yet_does_not_hide_then_show() {
            let box_id = egui::Id::unique("tempo");
            let (state, _) = advance(KeyboardState::new(), Some(box_id), true, false);
            let (state, _) = advance(state, Some(box_id), false, false);
            assert_eq!(state.idle_frames, 1, "the grace is already used");

            let (state, request) = advance(state, Some(box_id), false, true);
            assert_eq!(
                request,
                KeyboardRequest::Nothing,
                "the tap may be starting an edit; do not flash the keyboard"
            );
            assert!(state.visible);

            // The box enters text-edit mode on the next frame; the tap must
            // still bring the IME back if the system Back gesture hid it.
            let (state, request) = advance(state, Some(box_id), true, false);
            assert_eq!(request, KeyboardRequest::Show);
            assert!(state.visible);
            assert!(!state.pending_tap);
        }

        #[test]
        fn tapping_between_fields_keeps_the_keyboard_up() {
            let alias = egui::Id::unique("alias");
            let search = egui::Id::unique("search");
            let (state, _) = advance(KeyboardState::new(), Some(alias), true, false);

            // Pressing the second field clears the first one's focus before the
            // second one claims it.
            let (state, request) = advance(state, None, false, true);
            assert_eq!(request, KeyboardRequest::Nothing);
            assert!(state.visible);

            // Release: the new field has focus, but its text edit appears on the
            // next frame.
            let (state, request) = advance(state, Some(search), false, true);
            assert_eq!(request, KeyboardRequest::Nothing);
            assert!(state.visible);

            let (state, request) = advance(state, Some(search), true, false);
            assert_eq!(request, KeyboardRequest::Show);
            assert!(state.visible);
        }

        #[test]
        fn a_tap_on_empty_space_hides_the_keyboard_after_the_tap() {
            let field = egui::Id::unique("alias");
            let (state, _) = advance(KeyboardState::new(), Some(field), true, false);
            let (state, request) = advance(state, None, false, true);
            assert_eq!(request, KeyboardRequest::Nothing);
            let (state, request) = advance(state, None, false, true);
            assert_eq!(request, KeyboardRequest::Nothing);
            let (state, request) = advance(state, None, false, false);
            assert_eq!(request, KeyboardRequest::Nothing);
            let (state, request) = advance(state, None, false, false);
            assert_eq!(request, KeyboardRequest::Hide);
            assert!(!state.visible);
        }

        #[test]
        fn nothing_is_requested_without_a_focused_text_edit() {
            let (state, request) = advance(KeyboardState::new(), None, false, false);
            assert_eq!(request, KeyboardRequest::Nothing);
            assert!(!state.visible);

            let button = egui::Id::unique("play");
            let (state, request) = advance(state, Some(button), false, true);
            assert_eq!(request, KeyboardRequest::Nothing);
            assert!(!state.visible);
        }

        fn key(key: egui::Key) -> egui::Event {
            egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }
        }
    }
}

#[cfg(target_os = "android")]
pub(crate) use bridge::{drain_events, suppress_backend_ime, sync_text_input};

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_callerkaraoke_app_MainActivity_nativeImeText(
    mut env: jni::JNIEnv<'_>,
    _class: jni::objects::JClass<'_>,
    text: jni::objects::JString<'_>,
) {
    let Ok(text) = env.get_string(&text) else {
        let _ = env.exception_clear();
        return;
    };
    bridge::push_text(text.into());
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_callerkaraoke_app_MainActivity_nativeImeKey(
    _env: *mut core::ffi::c_void,
    _class: *mut core::ffi::c_void,
    key: i32,
) {
    bridge::push_key(key);
}
