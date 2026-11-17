//! Web entry point for Caller Karaoke.

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() -> Result<(), wasm_bindgen::JsValue> {
    caller_app::start_web()
}

/// Native builds of this package are only used for workspace checks.
#[cfg(not(target_arch = "wasm32"))]
pub fn start() {}
