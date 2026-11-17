//! Persistence for the locally saved dance and tune libraries.
//!
//! Native and Android builds keep one JSON file per item type under
//! `<config>/library`; the web build uses `localStorage`. The libraries are
//! deliberately separate from the HTTP source cache: clearing the cache must
//! not discard content the user downloaded.

use std::path::Path;

// `std::time::SystemTime` panics on `wasm32-unknown-unknown`; `web-time` reads
// the browser clock there and re-exports `std` everywhere else.
use web_time::{SystemTime, UNIX_EPOCH};

use caller_core::{Dance, Library, Tune};

const DANCES_FILE: &str = "dances.json";
const TUNES_FILE: &str = "tunes.json";

pub fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

pub fn load_dances(dir: Option<&Path>) -> Option<Library<Dance>> {
    read_text(dir, DANCES_FILE).and_then(|json| Library::<Dance>::from_json(&json).ok())
}

pub fn load_tunes(dir: Option<&Path>) -> Option<Library<Tune>> {
    read_text(dir, TUNES_FILE).and_then(|json| Library::<Tune>::from_json(&json).ok())
}

pub fn save_dances(dir: Option<&Path>, library: &Library<Dance>) {
    if let Ok(json) = library.to_json() {
        write_text(dir, DANCES_FILE, &json);
    }
}

pub fn save_tunes(dir: Option<&Path>, library: &Library<Tune>) {
    if let Ok(json) = library.to_json() {
        write_text(dir, TUNES_FILE, &json);
    }
}

fn read_text(dir: Option<&Path>, name: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read_to_string(dir?.join(name)).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dir;
        crate::web_store::read_library(name)
    }
}

fn write_text(dir: Option<&Path>, name: &str, json: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let Some(dir) = dir else {
            return;
        };
        if std::fs::create_dir_all(dir).is_ok() {
            let _ = std::fs::write(dir.join(name), json);
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dir;
        crate::web_store::write_library(name, json);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "caller-library-{name}-{}-{}",
            std::process::id(),
            now_seconds()
        ));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn dances_round_trip_through_the_library_file() {
        let dir = temp_dir("dances");
        let mut library = Library::default();
        library.create_folder("Warmups").expect("create the folder");
        library.insert(crate::defaults::default_dance(), Some("Warmups".into()), 42);
        save_dances(Some(&dir), &library);

        let restored = load_dances(Some(&dir)).expect("load the saved library");
        assert_eq!(restored, library);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tunes_round_trip_through_the_library_file() {
        let dir = temp_dir("tunes");
        let mut library = Library::default();
        library.insert(crate::defaults::default_tune(), None, 42);
        save_tunes(Some(&dir), &library);

        let restored = load_tunes(Some(&dir)).expect("load the saved library");
        assert_eq!(restored, library);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn no_directory_means_no_persistence() {
        save_dances(None, &Library::default());
        save_tunes(None, &Library::default());
        assert!(load_dances(None).is_none());
        assert!(load_tunes(None).is_none());
    }
}
