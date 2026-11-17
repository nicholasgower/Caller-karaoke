//! Contra Karaoke application library.
//!
//! The desktop binary, Android native activity, and web entry point all share
//! the same `ContraApp` state machine.

mod android_ime;
pub mod app;
mod audio;
mod defaults;
mod files;
mod library;
mod media_keys;
mod memory;
mod platform;
mod sheet;
mod source;
mod theme;
mod tts;
mod tune_set;
mod updates;
#[cfg(target_arch = "wasm32")]
mod web_store;
mod window;

pub use app::ContraApp;
pub use platform::platform;

/// System-bar insets reported by the Android activity.
///
/// The Java `MainActivity` sends these from `onApplyWindowInsets`, so the UI
/// uses the actual status/navigation bar sizes rather than guessing from the
/// window content rectangle (which also includes the gesture area).
#[cfg(target_os = "android")]
mod android_insets {
    use std::sync::atomic::{AtomicI32, Ordering};

    static INSETS: [AtomicI32; 4] = [
        AtomicI32::new(0),
        AtomicI32::new(0),
        AtomicI32::new(0),
        AtomicI32::new(0),
    ];

    pub(crate) fn system_bar_insets() -> [i32; 4] {
        [
            INSETS[0].load(Ordering::Relaxed),
            INSETS[1].load(Ordering::Relaxed),
            INSETS[2].load(Ordering::Relaxed),
            INSETS[3].load(Ordering::Relaxed),
        ]
    }

    /// Called from `MainActivity` with left, top, right, bottom in pixels.
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_org_contrakaraoke_app_MainActivity_nativeSetSystemBarInsets(
        _env: *mut core::ffi::c_void,
        _class: *mut core::ffi::c_void,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) {
        INSETS[0].store(left, Ordering::Relaxed);
        INSETS[1].store(top, Ordering::Relaxed);
        INSETS[2].store(right, Ordering::Relaxed);
        INSETS[3].store(bottom, Ordering::Relaxed);
    }
}

/// Run the native desktop application.
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub fn run_desktop() -> eframe::Result {
    env_logger::init();
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([1120.0, 780.0])
        .with_min_inner_size([360.0, 560.0])
        .with_title("Contra Karaoke")
        // Ask to be the focused window at launch. eframe shows the window
        // only after the first frame, which on its own can open it behind
        // the program that launched it; `window::LaunchFocus` requests the
        // focus again once the window is on screen.
        .with_active(true);
    if let Some(icon) = window::window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Contra Karaoke",
        options,
        Box::new(|cc| Ok(Box::new(ContraApp::new(cc)))),
    )
}

/// Android entry point used by `cargo-apk`.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    // `directories` and `std::env::temp_dir()` do not point at app-private
    // storage on Android, so keep preferences and caches in the app's files
    // directory.
    let data_dir = app.internal_data_path();
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Contra Karaoke",
        options,
        Box::new(move |cc| Ok(Box::new(ContraApp::new_android(cc, data_dir.clone())))),
    );
}

/// Browser entry point. Called from the binary's `#[wasm_bindgen(start)]`.
#[cfg(target_arch = "wasm32")]
pub fn start_web() -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;

    console_error_panic_hook::set_once();
    eframe::WebLogger::init(log::LevelFilter::Info).ok();

    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let canvas = document
        .get_element_by_id("contra-canvas")
        .ok_or("missing #contra-canvas")?;
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into()?;
    let runner = eframe::WebRunner::new();
    wasm_bindgen_futures::spawn_local(async move {
        let _ = runner
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(ContraApp::new(cc)))),
            )
            .await;
    });
    Ok(())
}
