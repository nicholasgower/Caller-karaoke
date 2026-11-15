//! Hardware play/pause keys from the operating system's media session.
//!
//! A keyboard event never reaches the app while its window is in the
//! background, so the transport cannot listen for the media keys itself. The
//! app instead registers with the platform media session — System Media
//! Transport Controls on Windows and MPRIS on Linux/BSD — and the desktop routes
//! the key to whichever player is active. The session also shows the current
//! dance and tune in the OS media controls.
//!
//! The browser and Android builds keep the same small API but never create a
//! session: browsers have their own Media Session API, and Android delivers
//! hardware keys to the focused activity.

/// A transport command requested by a hardware media key.
///
/// The browser and Android builds never create a media session, so they never
/// construct a command, but the app matches on these variants everywhere.
#[cfg_attr(any(target_arch = "wasm32", target_os = "android"), allow(dead_code))]
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

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
mod platform {
    use super::MediaCommand;

    /// No-op media session for the browser and Android builds.
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
