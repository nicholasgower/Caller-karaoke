//! Locally saved dances and tunes, grouped into user folders.
//!
//! The library is pure data: it knows nothing about where it is stored or how
//! it is rendered. The app loads and saves it; the pickers search and organise
//! it. Entries are keyed by their [`SourceRef`], so downloading a newer copy of
//! the same item replaces the stored one without creating a duplicate.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::dance::{Dance, SourceRef};
use crate::tune::Tune;

/// Schema version for the on-disk library files.
pub const LIBRARY_SCHEMA_VERSION: u32 = 1;

/// Which group of saved items a picker is showing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum FolderFilter {
    /// Every saved item.
    #[default]
    All,
    /// Items that are not in a folder.
    Unfiled,
    /// Items in one named folder.
    Folder(String),
}

impl FolderFilter {
    /// Whether an entry's folder belongs to this filter.
    pub fn accepts(&self, folder: Option<&str>) -> bool {
        match self {
            Self::All => true,
            Self::Unfiled => folder.is_none(),
            Self::Folder(name) => folder == Some(name.as_str()),
        }
    }

    /// A short label for the picker header.
    pub fn label(&self) -> &str {
        match self {
            Self::All => "All",
            Self::Unfiled => "Unfiled",
            Self::Folder(name) => name,
        }
    }
}

/// The fields a library search looks at.
///
/// The implementations include the metadata a caller would remember, not just
/// the title, because searching saved content is the main way to find a card
/// again without another network request.
pub trait LibraryItem {
    fn source(&self) -> &SourceRef;
    fn title(&self) -> &str;
    fn search_haystack(&self) -> String;
}

impl LibraryItem for Dance {
    fn source(&self) -> &SourceRef {
        &self.source
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn search_haystack(&self) -> String {
        let mut text = format!(
            "{} {} {} {} {}",
            self.title, self.choreographer, self.formation, self.preamble, self.notes
        );
        for figure in &self.figures {
            text.push(' ');
            text.push_str(figure.text.as_str());
        }
        text.to_lowercase()
    }
}

impl LibraryItem for Tune {
    fn source(&self) -> &SourceRef {
        &self.source
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn search_haystack(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.title, self.composer, self.key, self.meter, self.rhythm, self.credits
        )
        .to_lowercase()
    }
}

/// One saved item plus its folder and download time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LibraryEntry<T> {
    pub item: T,
    /// `None` means unfiled.
    #[serde(default)]
    pub folder: Option<String>,
    /// Unix timestamp in seconds; refreshed when the item is downloaded again.
    #[serde(default)]
    pub downloaded_at: u64,
}

impl<T: LibraryItem> LibraryEntry<T> {
    pub fn matches(&self, terms: &[String]) -> bool {
        let haystack = self.item.search_haystack();
        terms.iter().all(|term| haystack.contains(term))
    }
}

/// A saved-content library with a flat list of user folders.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: DeserializeOwned"))]
pub struct Library<T> {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub entries: Vec<LibraryEntry<T>>,
    #[serde(default)]
    pub folders: Vec<String>,
}

fn default_schema_version() -> u32 {
    LIBRARY_SCHEMA_VERSION
}

impl<T> Default for Library<T> {
    fn default() -> Self {
        Self {
            schema_version: LIBRARY_SCHEMA_VERSION,
            entries: Vec::new(),
            folders: Vec::new(),
        }
    }
}

/// Why a folder operation was rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FolderError {
    #[error("folder name is empty")]
    Empty,
    #[error("a folder with that name already exists")]
    Duplicate,
    #[error("folder not found")]
    Missing,
}

impl<T: LibraryItem> Library<T> {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, provider: &str, id: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.item.source().provider == provider && entry.item.source().id == id)
    }

    pub fn get(&self, provider: &str, id: &str) -> Option<&LibraryEntry<T>> {
        self.entries
            .iter()
            .find(|entry| entry.item.source().provider == provider && entry.item.source().id == id)
    }

    /// Store a downloaded item, replacing a previous copy of the same source.
    ///
    /// An existing entry keeps its folder so re-downloading a card does not
    /// move it out of the group the user put it in. Returns true when the item
    /// was new.
    pub fn insert(&mut self, item: T, folder: Option<String>, downloaded_at: u64) -> bool {
        let source = item.source().clone();
        let folder = normalize_folder(folder);
        let folder = folder.map(|folder| self.canonical_folder(&folder));
        if let Some(folder) = &folder {
            self.ensure_folder(folder);
        }
        if let Some(entry) = self.entries.iter_mut().find(|entry| {
            entry.item.source().provider == source.provider && entry.item.source().id == source.id
        }) {
            entry.item = item;
            entry.downloaded_at = downloaded_at;
            if folder.is_some() {
                entry.folder = folder;
            }
            return false;
        }
        self.entries.push(LibraryEntry {
            item,
            folder,
            downloaded_at,
        });
        true
    }

    /// Remove one saved item and return it.
    pub fn remove(&mut self, provider: &str, id: &str) -> Option<LibraryEntry<T>> {
        let index = self.entries.iter().position(|entry| {
            entry.item.source().provider == provider && entry.item.source().id == id
        })?;
        Some(self.entries.remove(index))
    }

    /// Put an item in a folder, or unfile it with `None`.
    pub fn set_folder(
        &mut self,
        provider: &str,
        id: &str,
        folder: Option<String>,
    ) -> Result<(), FolderError> {
        let folder = normalize_folder(folder);
        let folder = match folder {
            Some(folder) => {
                if !self.has_folder(&folder) {
                    return Err(FolderError::Missing);
                }
                Some(self.canonical_folder(&folder))
            }
            None => None,
        };
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.item.source().provider == provider && entry.item.source().id == id)
            .ok_or(FolderError::Missing)?;
        entry.folder = folder;
        Ok(())
    }

    /// Number of saved items in a folder filter.
    pub fn count(&self, filter: &FolderFilter) -> usize {
        self.entries
            .iter()
            .filter(|entry| filter.accepts(entry.folder.as_deref()))
            .count()
    }

    /// Add a folder. Names are trimmed and compared case-insensitively, but
    /// the first spelling is kept for display.
    pub fn create_folder(&mut self, name: &str) -> Result<(), FolderError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(FolderError::Empty);
        }
        if self.has_folder(name) {
            return Err(FolderError::Duplicate);
        }
        self.folders.push(name.to_string());
        Ok(())
    }

    /// Rename a folder and move its entries with it.
    pub fn rename_folder(&mut self, old: &str, new: &str) -> Result<(), FolderError> {
        let new = new.trim();
        if new.is_empty() {
            return Err(FolderError::Empty);
        }
        if !self.has_folder(old) {
            return Err(FolderError::Missing);
        }
        if !old.eq_ignore_ascii_case(new) && self.has_folder(new) {
            return Err(FolderError::Duplicate);
        }
        let index = self
            .folders
            .iter()
            .position(|folder| folder == old)
            .ok_or(FolderError::Missing)?;
        self.folders[index] = new.to_string();
        for entry in &mut self.entries {
            if entry.folder.as_deref() == Some(old) {
                entry.folder = Some(new.to_string());
            }
        }
        Ok(())
    }

    /// Delete a folder. Its saved items become unfiled rather than deleted.
    pub fn delete_folder(&mut self, name: &str) -> bool {
        let Some(index) = self.folders.iter().position(|folder| folder == name) else {
            return false;
        };
        self.folders.remove(index);
        for entry in &mut self.entries {
            if entry.folder.as_deref() == Some(name) {
                entry.folder = None;
            }
        }
        true
    }

    /// Move a folder to a new position in the displayed order.
    ///
    /// `to` is clamped to the last index, so dropping a folder past the end
    /// (or onto itself) is harmless. Returns true when the order changed.
    pub fn move_folder(&mut self, from: usize, to: usize) -> bool {
        if from >= self.folders.len() {
            return false;
        }
        let to = to.min(self.folders.len() - 1);
        if from == to {
            return false;
        }
        let folder = self.folders.remove(from);
        self.folders.insert(to, folder);
        true
    }

    /// Entries matching every whitespace-separated search term in the selected
    /// folder, in insertion order.
    pub fn search(&self, query: &str, filter: &FolderFilter) -> Vec<&LibraryEntry<T>> {
        let terms: Vec<String> = query
            .split_whitespace()
            .map(|term| term.to_lowercase())
            .collect();
        self.entries
            .iter()
            .filter(|entry| filter.accepts(entry.folder.as_deref()))
            .filter(|entry| entry.matches(&terms))
            .collect()
    }

    /// True when a folder with this name exists, ignoring case.
    pub fn has_folder(&self, name: &str) -> bool {
        self.folders
            .iter()
            .any(|folder| folder.eq_ignore_ascii_case(name.trim()))
    }

    fn ensure_folder(&mut self, name: &str) {
        if !self.has_folder(name) {
            self.folders.push(name.to_string());
        }
    }

    /// Use the stored spelling of a folder when the caller passes a different
    /// case, so entries and the folder filter always compare exactly.
    fn canonical_folder(&self, name: &str) -> String {
        self.folders
            .iter()
            .find(|folder| folder.eq_ignore_ascii_case(name.trim()))
            .cloned()
            .unwrap_or_else(|| name.trim().to_string())
    }
}

impl<T> Library<T> {
    /// Repair data from an older or hand-edited file.
    pub fn validate(&mut self) {
        if self.schema_version == 0 {
            self.schema_version = LIBRARY_SCHEMA_VERSION;
        }

        let mut folders: Vec<String> = Vec::new();
        for folder in self.folders.drain(..) {
            let folder = folder.trim().to_string();
            if folder.is_empty()
                || folders
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(&folder))
            {
                continue;
            }
            folders.push(folder);
        }

        for entry in &mut self.entries {
            entry.folder = normalize_folder(entry.folder.take());
            if let Some(folder) = entry.folder.clone() {
                if let Some(existing) = folders
                    .iter()
                    .find(|existing| existing.eq_ignore_ascii_case(&folder))
                {
                    entry.folder = Some(existing.clone());
                } else {
                    folders.push(folder);
                }
            }
        }
        self.folders = folders;
    }
}

impl<T: Serialize> Library<T> {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl<T: DeserializeOwned> Library<T> {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut library: Self = serde_json::from_str(json)?;
        library.validate();
        Ok(library)
    }
}

fn normalize_folder(folder: Option<String>) -> Option<String> {
    folder
        .map(|folder| folder.trim().to_string())
        .filter(|folder| !folder.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dance::Figure;

    fn dance(id: &str, title: &str, choreographer: &str, figure: &str) -> Dance {
        Dance {
            source: SourceRef::new("contradb", id, format!("https://contradb.com/dances/{id}")),
            title: title.into(),
            choreographer: choreographer.into(),
            formation: "improper".into(),
            preamble: String::new(),
            notes: String::new(),
            figures: vec![Figure::new(Some("A1".into()), 8.0, figure)],
        }
    }

    fn tune(id: &str, title: &str, composer: &str) -> Tune {
        Tune {
            source: SourceRef::new(
                "abcsessiontunes",
                id,
                format!("https://thesession.org/tunes/{id}"),
            ),
            title: title.into(),
            composer: composer.into(),
            key: "D".into(),
            meter: "4/4".into(),
            rhythm: "reel".into(),
            abc: "X:1\nK:D\n".into(),
            original_abc: None,
            credits: String::new(),
        }
    }

    #[test]
    fn inserting_the_same_source_replaces_without_duplicating() {
        let mut library = Library::default();
        assert!(library.insert(dance("1", "First", "A", "swing"), None, 10));
        assert!(!library.insert(dance("1", "First again", "A", "swing"), None, 20));

        assert_eq!(library.len(), 1);
        let entry = library.get("contradb", "1").expect("stored");
        assert_eq!(entry.item.title, "First again");
        assert_eq!(entry.downloaded_at, 20);
    }

    #[test]
    fn redownloading_keeps_the_existing_folder() {
        let mut library = Library::default();
        library.create_folder("Warmups").expect("create");
        library.insert(
            dance("1", "First", "A", "swing"),
            Some("Warmups".into()),
            10,
        );
        library.insert(dance("1", "First", "A", "swing"), None, 20);

        assert_eq!(
            library
                .get("contradb", "1")
                .expect("stored")
                .folder
                .as_deref(),
            Some("Warmups")
        );
    }

    #[test]
    fn search_matches_title_choreographer_and_figure_text() {
        let mut library = Library::default();
        library.insert(
            dance(
                "1",
                "Another Last Dance",
                "Erik Hoffman",
                "neighbors do si do",
            ),
            None,
            1,
        );
        library.insert(
            dance("2", "Belle of the Ball", "Ralph Sweet", "ladles chain"),
            None,
            2,
        );

        let all = library.search("", &FolderFilter::All);
        assert_eq!(all.len(), 2);

        let by_title = library.search("belle", &FolderFilter::All);
        assert_eq!(by_title.len(), 1);
        assert_eq!(by_title[0].item.title, "Belle of the Ball");

        let by_choreographer = library.search("erik", &FolderFilter::All);
        assert_eq!(by_choreographer.len(), 1);

        let by_figure = library.search("chain", &FolderFilter::All);
        assert_eq!(by_figure.len(), 1);
        assert_eq!(by_figure[0].item.title, "Belle of the Ball");

        // Every term must match.
        assert!(library.search("belle erik", &FolderFilter::All).is_empty());
    }

    #[test]
    fn folders_filter_and_move_entries() {
        let mut library = Library::default();
        library.create_folder("Warmups").expect("create");
        library.insert(dance("1", "First", "A", "swing"), None, 1);
        library.insert(dance("2", "Second", "B", "chain"), None, 2);
        library
            .set_folder("contradb", "1", Some("Warmups".into()))
            .expect("move");

        assert_eq!(library.count(&FolderFilter::All), 2);
        assert_eq!(library.count(&FolderFilter::Unfiled), 1);
        assert_eq!(library.count(&FolderFilter::Folder("Warmups".into())), 1);
        assert_eq!(
            library.search("", &FolderFilter::Folder("Warmups".into()))[0]
                .item
                .title,
            "First"
        );

        // A folder name from a hand-edited file uses the stored spelling.
        library
            .set_folder("contradb", "2", Some("warmups".into()))
            .expect("move with different case");
        assert_eq!(
            library
                .get("contradb", "2")
                .expect("stored")
                .folder
                .as_deref(),
            Some("Warmups")
        );
    }

    #[test]
    fn folders_are_unique_ignoring_case_and_append_missing_ones() {
        let mut library = Library::default();
        library.create_folder("Warmups").expect("create");
        assert_eq!(
            library.create_folder("  warmups "),
            Err(FolderError::Duplicate)
        );
        assert_eq!(library.create_folder("   "), Err(FolderError::Empty));

        library.insert(dance("1", "First", "A", "swing"), Some("Encores".into()), 1);
        assert!(library.has_folder("encores"));
    }

    #[test]
    fn renaming_a_folder_moves_its_entries() {
        let mut library = Library::default();
        library.create_folder("Warmups").expect("create");
        library.insert(dance("1", "First", "A", "swing"), Some("Warmups".into()), 1);
        library.rename_folder("Warmups", "Encores").expect("rename");

        assert!(!library.has_folder("Warmups"));
        assert!(library.has_folder("Encores"));
        assert_eq!(
            library
                .get("contradb", "1")
                .expect("stored")
                .folder
                .as_deref(),
            Some("Encores")
        );
    }

    #[test]
    fn deleting_a_folder_keeps_its_entries_unfiled() {
        let mut library = Library::default();
        library.create_folder("Warmups").expect("create");
        library.insert(dance("1", "First", "A", "swing"), Some("Warmups".into()), 1);

        assert!(library.delete_folder("Warmups"));
        assert!(!library.delete_folder("Warmups"));
        assert_eq!(library.len(), 1);
        assert!(
            library
                .get("contradb", "1")
                .expect("stored")
                .folder
                .is_none()
        );
    }

    #[test]
    fn removing_an_item_returns_it_and_leaves_the_folder() {
        let mut library = Library::default();
        library.create_folder("Warmups").expect("create");
        library.insert(dance("1", "First", "A", "swing"), Some("Warmups".into()), 1);

        let removed = library.remove("contradb", "1").expect("removed");
        assert_eq!(removed.item.title, "First");
        assert!(library.is_empty());
        assert!(library.has_folder("Warmups"));
    }

    #[test]
    fn moving_a_folder_reorders_and_clamps_the_target() {
        let mut library: Library<Dance> = Library::default();
        for name in ["Ball", "Chains", "Warmups"] {
            library.create_folder(name).expect("create");
        }

        // Down: remove then insert at the requested index.
        assert!(library.move_folder(0, 2));
        assert_eq!(library.folders, ["Chains", "Warmups", "Ball"]);

        // Up.
        assert!(library.move_folder(2, 0));
        assert_eq!(library.folders, ["Ball", "Chains", "Warmups"]);

        // Dropping past the end clamps to the last slot.
        assert!(library.move_folder(0, 99));
        assert_eq!(library.folders, ["Chains", "Warmups", "Ball"]);

        // Dropping onto itself changes nothing.
        assert!(!library.move_folder(1, 1));

        // An index outside the list is rejected and leaves the order alone.
        assert!(!library.move_folder(3, 0));
        assert!(!library.move_folder(usize::MAX, 0));
        assert_eq!(library.folders, ["Chains", "Warmups", "Ball"]);

        // Reordering never moves entries between folders.
        library.insert(dance("1", "First", "A", "swing"), Some("Ball".into()), 1);
        assert!(library.move_folder(2, 0));
        assert_eq!(
            library
                .get("contradb", "1")
                .expect("stored")
                .folder
                .as_deref(),
            Some("Ball")
        );
    }

    #[test]
    fn moving_a_folder_in_a_one_folder_library_is_a_no_op() {
        let mut library: Library<Dance> = Library::default();
        library.create_folder("Only").expect("create");
        assert!(!library.move_folder(0, 0));
        assert!(!library.move_folder(0, 3));
        assert_eq!(library.folders, ["Only"]);

        let mut empty: Library<Dance> = Library::default();
        assert!(!empty.move_folder(0, 0));
    }

    #[test]
    fn json_round_trip_preserves_folders_and_entries() {
        let mut library = Library::default();
        library.create_folder("Warmups").expect("create");
        library.insert(tune("75815", "Sandy Boys", ""), Some("Warmups".into()), 42);

        let json = library.to_json().expect("serialize");
        let restored: Library<Tune> = Library::from_json(&json).expect("deserialize");
        assert_eq!(library, restored);
    }

    #[test]
    fn validation_repairs_duplicate_and_unknown_folders() {
        let json = r#"{
            "schema_version": 0,
            "folders": ["Warmups", "warmups", "  "],
            "entries": [{
                "item": {
                    "source": {"provider": "contradb", "id": "1", "url": "https://example.invalid/1"},
                    "title": "First",
                    "choreographer": "A",
                    "formation": "improper",
                    "preamble": "",
                    "notes": "",
                    "figures": []
                },
                "folder": " Encores ",
                "downloaded_at": 1
            }]
        }"#;
        let library: Library<Dance> = Library::from_json(json).expect("deserialize");
        assert_eq!(library.schema_version, LIBRARY_SCHEMA_VERSION);
        assert_eq!(library.folders, vec!["Warmups", "Encores"]);
        assert_eq!(
            library
                .get("contradb", "1")
                .expect("stored")
                .folder
                .as_deref(),
            Some("Encores")
        );
    }
}
