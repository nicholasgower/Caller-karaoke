//! Browser persistence adapters.

#[cfg(target_arch = "wasm32")]
use web_sys::window;

const PREFS_KEY: &str = "caller-karaoke.prefs";
const CACHE_PREFIX: &str = "caller-karaoke.cache.";
const LIBRARY_PREFIX: &str = "caller-karaoke.library.";
// Keys written before the rename. They are still read so a returning visitor
// keeps their preferences and saved dances; the next write uses the new keys.
const LEGACY_PREFS_KEY: &str = "contra-karaoke.prefs";
const LEGACY_LIBRARY_PREFIX: &str = "contra-karaoke.library.";

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    window()?.local_storage().ok().flatten()
}

#[cfg(target_arch = "wasm32")]
pub fn read_prefs() -> Option<String> {
    let storage = storage()?;
    storage
        .get_item(PREFS_KEY)
        .ok()
        .flatten()
        .or_else(|| storage.get_item(LEGACY_PREFS_KEY).ok().flatten())
}

#[cfg(target_arch = "wasm32")]
pub fn write_prefs(json: &str) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(PREFS_KEY, json);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn read_library(name: &str) -> Option<String> {
    let storage = storage()?;
    storage
        .get_item(&format!("{LIBRARY_PREFIX}{name}"))
        .ok()
        .flatten()
        .or_else(|| {
            storage
                .get_item(&format!("{LEGACY_LIBRARY_PREFIX}{name}"))
                .ok()
                .flatten()
        })
}
#[cfg(target_arch = "wasm32")]
pub fn write_library(name: &str, json: &str) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(&format!("{LIBRARY_PREFIX}{name}"), json);
    }
}

/// The `set` share argument in the page URL, if the page was opened with one.
///
/// Both the query and the fragment are read, so a `#set=` link and a hand-edited
/// `?set=` link open the same session.
#[cfg(target_arch = "wasm32")]
pub fn share_code_from_location() -> Option<String> {
    let location = window()?.location();
    let search = location.search().ok().unwrap_or_default();
    let hash = location.hash().ok().unwrap_or_default();
    crate::share::code_from_url(&format!("{search}{hash}"))
}

/// The current page address.
///
/// A share link is built from this rather than from an origin the app guesses,
/// so a bundle hosted under a subpath produces a working link. `share::link_for`
/// replaces any existing query and fragment with the share argument.
#[cfg(target_arch = "wasm32")]
pub fn page_url() -> Option<String> {
    window()?.location().href().ok()
}

/// Copy text with the browser clipboard.
///
/// The temporary-textarea path runs inside the button's click gesture and also
/// works on plain HTTP, where the asynchronous Clipboard API is unavailable.
/// The Clipboard API is the fallback; the link stays selectable in the panel
/// either way.
#[cfg(target_arch = "wasm32")]
pub fn copy_text(text: &str) -> Result<(), String> {
    let window = window().ok_or("no browser window")?;
    let document = window.document().ok_or("no browser document")?;
    if copy_with_textarea(&document, text) {
        return Ok(());
    }
    // The promise is not awaited: the link stays selectable in the panel.
    let _ = window.navigator().clipboard().write_text(text);
    Ok(())
}

/// Select `text` in a temporary off-screen textarea and copy it.
///
/// Returns false when the browser does not allow the synchronous copy; the
/// caller then tries the Clipboard API.
#[cfg(target_arch = "wasm32")]
fn copy_with_textarea(document: &web_sys::Document, text: &str) -> bool {
    use wasm_bindgen::JsCast;

    let Some(body) = document.body() else {
        return false;
    };
    // The copy must not leave the app's canvas unfocused, or keyboard
    // shortcuts stop working until the user clicks the page again.
    let active = document.active_element();
    let Ok(element) = document.create_element("textarea") else {
        return false;
    };
    let Ok(area) = element.dyn_into::<web_sys::HtmlTextAreaElement>() else {
        return false;
    };
    area.set_value(text);
    let _ = area.set_attribute("readonly", "");
    let _ = area.set_attribute(
        "style",
        "position:fixed;left:-9999px;top:0;width:1px;height:1px",
    );
    if body.append_child(&area).is_err() {
        return false;
    }
    area.select();
    let copied = document
        .clone()
        .dyn_into::<web_sys::HtmlDocument>()
        .ok()
        .and_then(|document| document.exec_command("copy").ok())
        .unwrap_or(false);
    area.remove();
    if let Some(active) = active.and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = active.focus();
    }
    copied
}

#[cfg(target_arch = "wasm32")]
pub struct LocalStorageCache;

#[cfg(target_arch = "wasm32")]
impl caller_sources::CacheStore for LocalStorageCache {
    fn read(&self, key: &str) -> Option<Vec<u8>> {
        let storage = storage()?;
        storage
            .get_item(&format!("{CACHE_PREFIX}{key}"))
            .ok()
            .flatten()
            .map(String::into_bytes)
    }

    fn write(&mut self, key: &str, value: &[u8]) -> Result<(), String> {
        let storage = storage().ok_or_else(|| "localStorage is unavailable".to_string())?;
        let text = String::from_utf8_lossy(value).into_owned();
        storage
            .set_item(&format!("{CACHE_PREFIX}{key}"), &text)
            .map_err(|_| "localStorage write failed".to_string())
    }

    fn remove(&mut self, key: &str) {
        if let Some(storage) = storage() {
            let _ = storage.remove_item(&format!("{CACHE_PREFIX}{key}"));
        }
    }

    fn clear(&mut self) {
        let Some(storage) = storage() else {
            return;
        };
        let Ok(length) = storage.length() else {
            return;
        };
        let mut keys = Vec::new();
        for index in 0..length {
            if let Ok(Some(key)) = storage.key(index)
                && key.starts_with(CACHE_PREFIX)
            {
                keys.push(key);
            }
        }
        for key in keys {
            let _ = storage.remove_item(&key);
        }
    }
}
