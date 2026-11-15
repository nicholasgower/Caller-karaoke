#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    contra_app::run_desktop()
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() -> Result<(), wasm_bindgen::JsValue> {
    contra_app::start_web()
}
