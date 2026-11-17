#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
fn main() -> eframe::Result {
    caller_app::run_desktop()
}

#[cfg(target_os = "android")]
fn main() {}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() -> Result<(), wasm_bindgen::JsValue> {
    caller_app::start_web()
}
