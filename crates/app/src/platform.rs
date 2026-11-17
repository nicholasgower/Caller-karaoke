//! The platform this build targets, for runtime feature gating.

/// The platform this build was compiled for, as a lowercase name.
///
/// Returns `"web"` for the `wasm32-unknown-unknown` browser build. Every other
/// target reports its target OS: `"windows"`, `"linux"`, `"android"`,
/// `"macos"`, and so on.
///
/// This is for gating features that are built on every target but should not
/// be offered everywhere, for example:
///
/// ```ignore
/// if !matches!(caller_app::platform(), "web" | "android") {
///     show_soundfont_file_picker(ui);
/// }
/// ```
///
/// It cannot replace `#[cfg]`: code that would not compile on a target (a
/// desktop-only crate, `std::fs`, the TTS bridge) must still be removed at
/// compile time. Because the result is a compile-time constant, a runtime
/// `match` costs nothing.
pub const fn platform() -> &'static str {
    if cfg!(all(target_arch = "wasm32", target_os = "unknown")) {
        "web"
    } else {
        std::env::consts::OS
    }
}

#[cfg(test)]
mod tests {
    use super::platform;

    #[test]
    fn platform_names_the_build_target() {
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        assert_eq!(platform(), "web");
        #[cfg(target_os = "windows")]
        assert_eq!(platform(), "windows");
        #[cfg(target_os = "linux")]
        assert_eq!(platform(), "linux");
        #[cfg(target_os = "android")]
        assert_eq!(platform(), "android");
        #[cfg(target_os = "macos")]
        assert_eq!(platform(), "macos");
    }

    #[test]
    fn platform_is_a_lowercase_name() {
        let name = platform();
        assert!(!name.is_empty(), "every platform has a name");
        assert!(
            name.chars().all(|c| c.is_ascii_lowercase()),
            "{name:?} should be a lowercase name"
        );
    }

    #[test]
    fn desktop_builds_are_not_web_or_android() {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            assert_ne!(platform(), "web");
            assert_ne!(platform(), "android");
        }
    }
}
