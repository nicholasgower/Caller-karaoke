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
    #[cfg(target_os = "android")]
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

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

    /// The last keyboard state sent to the activity.
    #[cfg(target_os = "android")]
    static ACTIVE: AtomicBool = AtomicBool::new(false);

    /// The egui widget that owned the last keyboard state.
    #[cfg(target_os = "android")]
    static FOCUS: AtomicU64 = AtomicU64::new(0);

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

    /// Show or hide the keyboard to match the focused egui text field.
    ///
    /// The activity is only told when the state actually changed, when the
    /// focused field changed, or when the user taps while a field is still
    /// focused (the system Back button can dismiss the keyboard without egui
    /// losing focus, and a second tap should bring it back).
    #[cfg(target_os = "android")]
    pub(crate) fn sync_text_input(ctx: &egui::Context) {
        let active = ctx.text_edit_focused();
        let focus = ctx
            .memory(|memory| memory.focused())
            .map_or(0, |id| id.value());
        let pointer_pressed = ctx.input(|input| input.pointer.any_pressed());
        let was_active = ACTIVE.load(Ordering::Relaxed);
        let was_focus = FOCUS.load(Ordering::Relaxed);
        let show = active && (pointer_pressed || focus != was_focus);
        if active == was_active && !show {
            return;
        }
        if crate::media_keys::set_text_input_active(active) {
            ACTIVE.store(active, Ordering::Relaxed);
            FOCUS.store(focus, Ordering::Relaxed);
        }
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
pub(crate) use bridge::{drain_events, sync_text_input};

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_contrakaraoke_app_MainActivity_nativeImeText(
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
pub extern "system" fn Java_org_contrakaraoke_app_MainActivity_nativeImeKey(
    _env: *mut core::ffi::c_void,
    _class: *mut core::ffi::c_void,
    key: i32,
) {
    bridge::push_key(key);
}
