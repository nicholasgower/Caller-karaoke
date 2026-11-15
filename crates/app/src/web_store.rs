//! Browser persistence adapters.

#[cfg(target_arch = "wasm32")]
use web_sys::window;

const PREFS_KEY: &str = "contra-karaoke.prefs";
const CACHE_PREFIX: &str = "contra-karaoke.cache.";
const LIBRARY_PREFIX: &str = "contra-karaoke.library.";

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    window()?.local_storage().ok().flatten()
}

#[cfg(target_arch = "wasm32")]
pub fn read_prefs() -> Option<String> {
    storage()?.get_item(PREFS_KEY).ok().flatten()
}

#[cfg(target_arch = "wasm32")]
pub fn write_prefs(json: &str) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(PREFS_KEY, json);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn read_library(name: &str) -> Option<String> {
    storage()?
        .get_item(&format!("{LIBRARY_PREFIX}{name}"))
        .ok()
        .flatten()
}

#[cfg(target_arch = "wasm32")]
pub fn write_library(name: &str, json: &str) {
    if let Some(storage) = storage() {
        let _ = storage.set_item(&format!("{LIBRARY_PREFIX}{name}"), json);
    }
}

#[cfg(target_arch = "wasm32")]
pub struct LocalStorageCache;

#[cfg(target_arch = "wasm32")]
impl contra_sources::CacheStore for LocalStorageCache {
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
            if let Ok(Some(key)) = storage.key(index) {
                if key.starts_with(CACHE_PREFIX) {
                    keys.push(key);
                }
            }
        }
        for key in keys {
            let _ = storage.remove_item(&key);
        }
    }
}
