//! Contra Karaoke application library.
//!
//! The desktop binary, Android native activity, and web entry point all share
//! the same `ContraApp` state machine.

pub mod app;
mod audio;
mod defaults;
mod library;
mod sheet;
mod source;
mod theme;
mod tts;
#[cfg(target_arch = "wasm32")]
mod web_store;

pub use app::ContraApp;

/// Run the native desktop application.
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub fn run_desktop() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 780.0])
            .with_min_inner_size([360.0, 560.0])
            .with_title("Contra Karaoke"),
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
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "Contra Karaoke",
        options,
        Box::new(|cc| Ok(Box::new(ContraApp::new(cc)))),
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
