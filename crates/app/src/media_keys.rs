//! Hardware play/pause keys from the operating system's media session.
//!
//! A keyboard event never reaches the app while its window is in the
//! background, so the transport cannot listen for the media keys itself. The
//! app instead registers with the platform media session — System Media
//! Transport Controls on Windows, MPRIS on Linux/BSD, and `MediaSession` on
//! Android — and the operating system routes the key to whichever player is
//! active. The session also shows the current dance and tune in the OS media
//! controls.
//!
//! The browser build keeps the same small API but never creates a session:
//! pages have their own Media Session API, which this build does not use yet.

/// A transport command requested by a hardware media key.
///
/// The browser build never creates a media session, so it never constructs a
/// command, but the app matches on these variants everywhere.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaCommand {
    /// One button that reports its own state, as on many keyboards.
    Toggle,
    Play,
    Pause,
}

impl MediaCommand {
    /// The play state this command asks for, given the current state.
    ///
    /// Keeping the mapping pure makes it testable without an audio device; the
    /// app ignores a request that matches the transport it already has.
    pub fn desired(self, playing: bool) -> bool {
        match self {
            Self::Toggle => !playing,
            Self::Play => true,
            Self::Pause => false,
        }
    }
}

/// The Android side of the media session: command codes and the command queue.
///
/// Java owns the `MediaSession` and its notification, so a media button arrives
/// as a JNI call on a Java thread instead of a channel. The activity passes one
/// of the `COMMAND_*` codes below; the native method turns it into a
/// [`MediaCommand`] and queues it for the next frame.
///
/// The module is compiled on Android and under `cfg(test)` so the mapping and
/// the queue can be tested on a desktop host; neither needs a Java VM.
#[cfg(any(target_os = "android", test))]
mod android_bridge {
    use std::sync::Mutex;

    use super::MediaCommand;

    /// Codes must match the `COMMAND_*` constants in `MainActivity.java`.
    pub(super) const PLAY: i32 = 1;
    pub(super) const PAUSE: i32 = 2;
    pub(super) const TOGGLE: i32 = 3;

    /// The command a Java code asks for, or `None` for a code this build does
    /// not know (for example an activity newer than the library).
    pub(super) fn command_from_java(code: i32) -> Option<MediaCommand> {
        match code {
            PLAY => Some(MediaCommand::Play),
            PAUSE => Some(MediaCommand::Pause),
            TOGGLE => Some(MediaCommand::Toggle),
            _ => None,
        }
    }

    /// Commands the activity has delivered but the app has not applied yet.
    ///
    /// A plain `Mutex<Vec<_>>` rather than a channel: the Java callback cannot
    /// own the receiving end, and draining must never block the frame.
    pub(super) type CommandQueue = Mutex<Vec<MediaCommand>>;

    /// Add a command for the next frame to pick up.
    ///
    /// A poisoned lock still hands back the vector: losing a media key because
    /// an unrelated frame panicked would be worse than reusing the queue.
    pub(super) fn enqueue(queue: &CommandQueue, command: MediaCommand) {
        let mut commands = match queue.lock() {
            Ok(commands) => commands,
            Err(poisoned) => poisoned.into_inner(),
        };
        commands.push(command);
    }

    /// Take every command received since the previous call.
    pub(super) fn drain(queue: &CommandQueue) -> Vec<MediaCommand> {
        let mut commands = match queue.lock() {
            Ok(commands) => commands,
            Err(poisoned) => poisoned.into_inner(),
        };
        std::mem::take(&mut commands)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn maps_the_activity_command_codes() {
            assert_eq!(command_from_java(PLAY), Some(MediaCommand::Play));
            assert_eq!(command_from_java(PAUSE), Some(MediaCommand::Pause));
            assert_eq!(command_from_java(TOGGLE), Some(MediaCommand::Toggle));
            assert_eq!(command_from_java(0), None);
            assert_eq!(command_from_java(99), None);
        }

        #[test]
        fn queues_commands_in_order_and_drains_once() {
            let queue = CommandQueue::new(Vec::new());
            enqueue(&queue, MediaCommand::Play);
            enqueue(&queue, MediaCommand::Toggle);

            assert_eq!(
                drain(&queue),
                vec![MediaCommand::Play, MediaCommand::Toggle]
            );
            assert!(drain(&queue).is_empty());
        }

        #[test]
        fn a_poisoned_queue_still_delivers() {
            let queue = CommandQueue::new(Vec::new());
            // The panic message is expected output; the test only checks that
            // the queue recovers from the poisoned lock.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _guard = queue.lock().expect("fresh queue");
                panic!("a frame panicked while draining");
            }));

            enqueue(&queue, MediaCommand::Pause);
            assert_eq!(drain(&queue), vec![MediaCommand::Pause]);
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
mod platform {
    use std::sync::mpsc::Receiver;

    use souvlaki::{
        MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig,
    };

    use super::MediaCommand;

    /// The app's OS media session, plus the commands it has received.
    pub struct MediaKeys {
        controls: MediaControls,
        commands: Receiver<MediaCommand>,
        /// The state last published, so an idle frame does not repeat it.
        playback: Option<bool>,
        /// The dance and tune last published, for the same reason.
        track: Option<(String, String)>,
    }

    impl MediaKeys {
        /// Register the app as a media player.
        ///
        /// Returns `None` when the platform has no media session available or
        /// when the session cannot be created; the transport still works from
        /// the app window.
        pub fn new(cc: &eframe::CreationContext<'_>) -> Option<Self> {
            if !session_bus_available() {
                return None;
            }
            #[cfg(target_os = "windows")]
            let hwnd = Some(window_handle(cc)?);
            #[cfg(not(target_os = "windows"))]
            let hwnd: Option<*mut std::ffi::c_void> = None;
            let mut controls = MediaControls::new(PlatformConfig {
                dbus_name: "org.contrakaraoke.ContraKaraoke",
                display_name: "Contra Karaoke",
                hwnd,
            })
            .ok()?;

            // The callback runs on the platform's media-session thread (or on
            // the main thread through the Windows message loop). A repaint is
            // what lets an unfocused, idle app notice the key promptly.
            let (tx, commands) = std::sync::mpsc::channel();
            let ctx = cc.egui_ctx.clone();
            controls
                .attach(move |event| {
                    if let Some(command) = command_for(event)
                        && tx.send(command).is_ok()
                    {
                        ctx.request_repaint();
                    }
                })
                .ok()?;

            Some(Self {
                controls,
                commands,
                playback: None,
                track: None,
            })
        }

        /// Take the commands received since the previous frame.
        pub fn poll(&mut self) -> Vec<MediaCommand> {
            self.commands.try_iter().collect()
        }

        /// Publish whether the transport is running to the OS media controls.
        pub fn set_playback(&mut self, playing: bool) {
            if self.playback == Some(playing) {
                return;
            }
            let playback = if playing {
                MediaPlayback::Playing { progress: None }
            } else {
                MediaPlayback::Paused { progress: None }
            };
            // A failed publish is not retried every frame: the session is
            // either gone or cannot accept this update, and hammering it would
            // not change that.
            let _ = self.controls.set_playback(playback);
            self.playback = Some(playing);
        }

        /// Publish the loaded dance and tune as the current media item.
        pub fn set_track(&mut self, dance: &str, tune: &str) {
            if self
                .track
                .as_ref()
                .is_some_and(|(saved_dance, saved_tune)| saved_dance == dance && saved_tune == tune)
            {
                return;
            }
            let metadata = MediaMetadata {
                title: Some(dance),
                artist: Some(tune),
                album: Some("Contra Karaoke"),
                ..Default::default()
            };
            let _ = self.controls.set_metadata(metadata);
            self.track = Some((dance.to_owned(), tune.to_owned()));
        }
    }

    fn command_for(event: MediaControlEvent) -> Option<MediaCommand> {
        match event {
            MediaControlEvent::Toggle => Some(MediaCommand::Toggle),
            MediaControlEvent::Play => Some(MediaCommand::Play),
            MediaControlEvent::Pause | MediaControlEvent::Stop => Some(MediaCommand::Pause),
            _ => None,
        }
    }

    /// Whether an MPRIS session is usable.
    ///
    /// MPRIS needs a D-Bus session bus. The zbus backend reports a missing bus
    /// by panicking on its service thread, so a bare TTY (or a test runner)
    /// skips the media session instead.
    #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
    fn session_bus_available() -> bool {
        if std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() {
            return true;
        }
        std::env::var_os("XDG_RUNTIME_DIR")
            .is_some_and(|dir| std::path::Path::new(&dir).join("bus").exists())
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "ios"))]
    fn session_bus_available() -> bool {
        true
    }

    /// The native window handle the Windows media session must be attached to.
    #[cfg(target_os = "windows")]
    fn window_handle(cc: &eframe::CreationContext<'_>) -> Option<*mut std::ffi::c_void> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};

        match cc.window_handle().ok()?.as_raw() {
            RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as *mut std::ffi::c_void),
            _ => None,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn maps_transport_buttons_and_ignores_track_buttons() {
            use MediaControlEvent::{Next, Pause, Play, Previous, Stop, Toggle};

            assert_eq!(command_for(Toggle), Some(MediaCommand::Toggle));
            assert_eq!(command_for(Play), Some(MediaCommand::Play));
            assert_eq!(command_for(Pause), Some(MediaCommand::Pause));
            assert_eq!(command_for(Stop), Some(MediaCommand::Pause));
            assert_eq!(command_for(Next), None);
            assert_eq!(command_for(Previous), None);
        }
    }
}

#[cfg(target_os = "android")]
mod platform {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};

    use jni::JNIEnv;
    use jni::JavaVM;
    use jni::objects::{JClass, JValue};

    use super::MediaCommand;
    use super::android_bridge::{self, CommandQueue};

    /// The activity class as a raw global reference, published at startup.
    ///
    /// `find_class` only sees app classes on a thread the app class loader
    /// belongs to. The native event-loop thread is attached to the VM without
    /// that loader, so the lookup there raises `ClassNotFoundException`, and
    /// the pending exception aborts the process when the thread detaches. The
    /// activity therefore hands its class over from `onCreate` (see
    /// [`attach_activity`]), and every later call re-wraps this handle.
    static ACTIVITY_CLASS: AtomicUsize = AtomicUsize::new(0);

    /// The UI context, so a media key that arrives while the app is idle still
    /// produces a frame promptly.
    static CONTEXT: OnceLock<eframe::egui::Context> = OnceLock::new();

    /// Commands delivered by the activity's media-session callback.
    static COMMANDS: OnceLock<CommandQueue> = OnceLock::new();

    fn commands() -> &'static CommandQueue {
        COMMANDS.get_or_init(|| Mutex::new(Vec::new()))
    }

    /// Queue a transport command for the next frame.
    pub(super) fn push_command(command: MediaCommand) {
        android_bridge::enqueue(commands(), command);
        if let Some(ctx) = CONTEXT.get() {
            ctx.request_repaint();
        }
    }

    /// Keep the activity class for later calls from native threads.
    ///
    /// The activity calls this from `onCreate`, on a Java thread whose class
    /// loader is the app's, so `NewGlobalRef` here pins a handle that stays
    /// valid on every thread until the process exits. A failure leaves the
    /// app without a media session rather than raising on the caller.
    pub(super) fn attach_activity(env: &mut JNIEnv<'_>, class: JClass<'_>) {
        if ACTIVITY_CLASS.load(Ordering::Relaxed) != 0 {
            return;
        }
        let Ok(global) = env.new_global_ref(&class) else {
            let _ = env.exception_clear();
            return;
        };
        let raw = global.as_obj().as_raw() as usize;
        // Deliberate leak: the class must stay pinned for the life of the
        // process, and there is nothing to release at shutdown.
        std::mem::forget(global);
        let _ = ACTIVITY_CLASS.compare_exchange(0, raw, Ordering::Relaxed, Ordering::Relaxed);
    }

    /// Run `call` against the activity class, ignoring any failure.
    ///
    /// The media session is optional: a missing VM or a Java exception must not
    /// stop the in-app transport. Returns `false` while the activity has not
    /// published its class yet, so the caller can retry on the next frame.
    fn with_activity(
        call: impl FnOnce(&mut JNIEnv<'_>, &JClass<'_>) -> jni::errors::Result<()>,
    ) -> bool {
        let raw = ACTIVITY_CLASS.load(Ordering::Relaxed);
        if raw == 0 {
            return false;
        }
        let context = ndk_context::android_context();
        // SAFETY: ndk-context holds the JavaVM the activity runs on; the
        // pointer is valid for as long as the process lives.
        let Ok(vm) = (unsafe { JavaVM::from_raw(context.vm().cast()) }) else {
            return false;
        };
        let Ok(mut env) = vm.attach_current_thread() else {
            return false;
        };
        // SAFETY: the handle is a global reference the activity created, so it
        // stays valid on this thread until the process exits.
        let class = unsafe { JClass::from_raw(raw as jni::sys::jclass) };
        let result = call(&mut env, &class);
        if result.is_err() {
            // Leave no pending exception behind for the next JNI call.
            let _ = env.exception_clear();
        }
        result.is_ok()
    }

    /// Rust's end of the Android media session.
    ///
    /// The session itself lives in `MainActivity`: only Java can register a
    /// `MediaSession` and post the MediaStyle notification. The activity sends
    /// play, pause, and toggle through [`Java_org_contrakaraoke_app_MainActivity_nativeMediaCommand`],
    /// so the transport sees the same [`MediaCommand`] values as on desktop.
    pub struct MediaKeys {
        /// The state last published, so an idle frame does not repeat it.
        playback: Option<bool>,
        /// The dance and tune last published, for the same reason.
        track: Option<(String, String)>,
    }

    impl MediaKeys {
        /// Connect to the media session the activity created in `onCreate`.
        ///
        /// The activity owns the session, so there is nothing to register here;
        /// the app only needs its context for repaints. Failing to reach the
        /// Java side later is not fatal, which is why this always succeeds.
        pub fn new(cc: &eframe::CreationContext<'_>) -> Option<Self> {
            let _ = CONTEXT.set(cc.egui_ctx.clone());
            Some(Self {
                playback: None,
                track: None,
            })
        }

        /// Take the commands received since the previous frame.
        pub fn poll(&mut self) -> Vec<MediaCommand> {
            android_bridge::drain(commands())
        }

        /// Publish whether the transport is running to the media session.
        ///
        /// The state is only remembered once Java accepted it, so a frame that
        /// runs before the activity published its class retries instead of
        /// leaving the session stuck on the default state.
        pub fn set_playback(&mut self, playing: bool) {
            if self.playback == Some(playing) {
                return;
            }
            let published = with_activity(|env, class| {
                env.call_static_method(
                    class,
                    "setMediaPlaybackState",
                    "(Z)V",
                    &[JValue::Bool(u8::from(playing))],
                )?;
                Ok(())
            });
            if published {
                self.playback = Some(playing);
            }
        }

        /// Publish the loaded dance and tune as the current media item.
        pub fn set_track(&mut self, dance: &str, tune: &str) {
            if self
                .track
                .as_ref()
                .is_some_and(|(saved_dance, saved_tune)| saved_dance == dance && saved_tune == tune)
            {
                return;
            }
            let published = with_activity(|env, class| {
                let title = env.new_string(dance)?;
                let artist = env.new_string(tune)?;
                let album = env.new_string("Contra Karaoke")?;
                env.call_static_method(
                    class,
                    "setMediaMetadata",
                    "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
                    &[
                        JValue::Object(&title),
                        JValue::Object(&artist),
                        JValue::Object(&album),
                    ],
                )?;
                Ok(())
            });
            if published {
                self.track = Some((dance.to_owned(), tune.to_owned()));
            }
        }
    }
}

/// Called by `MainActivity.onCreate` to publish its class to the media bridge.
///
/// The activity is still on a Java thread here, so its class loader sees the
/// app's classes; the native event-loop thread cannot look the class up itself
/// (see [`platform::attach_activity`]).
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_contrakaraoke_app_MainActivity_nativeAttachActivity(
    mut env: jni::JNIEnv<'_>,
    class: jni::objects::JClass<'_>,
) {
    platform::attach_activity(&mut env, class);
}

/// Called by `MainActivity` when the media session receives a command.
///
/// The activity calls this from its media-session callback thread, so the
/// command is queued for the next frame instead of touching the transport from
/// a Java thread. Unknown codes are ignored, which lets the APK and the library
/// disagree about a future command without crashing the session.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_contrakaraoke_app_MainActivity_nativeMediaCommand(
    _env: *mut core::ffi::c_void,
    _class: *mut core::ffi::c_void,
    command: i32,
) {
    if let Some(command) = android_bridge::command_from_java(command) {
        platform::push_command(command);
    }
}

#[cfg(target_arch = "wasm32")]
mod platform {
    use super::MediaCommand;

    /// No-op media session for the browser build.
    pub struct MediaKeys;

    impl MediaKeys {
        pub fn new(_cc: &eframe::CreationContext<'_>) -> Option<Self> {
            None
        }

        pub fn poll(&mut self) -> Vec<MediaCommand> {
            Vec::new()
        }

        pub fn set_playback(&mut self, _playing: bool) {}

        pub fn set_track(&mut self, _dance: &str, _tune: &str) {}
    }
}

pub use platform::MediaKeys;

#[cfg(test)]
mod tests {
    use super::MediaCommand;

    #[test]
    fn toggle_flips_the_transport() {
        assert!(!MediaCommand::Toggle.desired(true));
        assert!(MediaCommand::Toggle.desired(false));
    }

    #[test]
    fn play_and_pause_are_explicit() {
        assert!(MediaCommand::Play.desired(false));
        assert!(MediaCommand::Play.desired(true));
        assert!(!MediaCommand::Pause.desired(true));
        assert!(!MediaCommand::Pause.desired(false));
    }
}
