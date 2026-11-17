//! Resolve a tune rotation's set from the saved-tune library.
//!
//! A rotation either plays an explicit list or every tune in one folder. The
//! folder form is resolved on every use, so a tune downloaded into the folder
//! joins the set without another edit, and a folder that has been deleted falls
//! back to the explicit list instead of stalling playback.
//!
//! This is pure app-level logic: it reads the library and the preference but
//! never saves, renders, or plays anything.

use caller_core::{Library, Tune, TuneRotation, TuneSetSource};

/// The tunes a rotation resolves to, plus a fallback marker for the status line.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResolvedTuneSet {
    /// Resolvable saved tunes, in play order.
    pub tunes: Vec<Tune>,
    /// The folder named by the rotation when it no longer exists. The tunes
    /// are then the explicit list, and the UI shows a non-blocking notice.
    pub missing_folder: Option<String>,
}

/// Resolve a rotation against the saved-tune library.
///
/// The explicit-list form keeps its order and skips references that are no
/// longer saved, so a deleted tune cannot stall the rotation. The folder form
/// uses the library's entry order, which is the order the library stores.
pub(crate) fn resolve_tune_set(
    library: &Library<Tune>,
    rotation: &TuneRotation,
) -> ResolvedTuneSet {
    match &rotation.source {
        TuneSetSource::Folder(name) if library.has_folder(name) => ResolvedTuneSet {
            tunes: library
                .entries
                .iter()
                .filter(|entry| entry.folder.as_deref() == Some(name.as_str()))
                .map(|entry| entry.item.clone())
                .collect(),
            missing_folder: None,
        },
        TuneSetSource::Folder(name) => ResolvedTuneSet {
            tunes: explicit_tunes(library, rotation),
            missing_folder: Some(name.clone()),
        },
        TuneSetSource::List => ResolvedTuneSet {
            tunes: explicit_tunes(library, rotation),
            missing_folder: None,
        },
    }
}

fn explicit_tunes(library: &Library<Tune>, rotation: &TuneRotation) -> Vec<Tune> {
    rotation
        .tunes
        .iter()
        .filter_map(|source| {
            library
                .get(&source.provider, &source.id)
                .map(|entry| entry.item.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use caller_core::SourceRef;

    fn tune(id: &str, title: &str) -> Tune {
        Tune {
            source: SourceRef::new(
                "abcsessiontunes",
                id,
                format!("https://thesession.org/tunes/{id}"),
            ),
            title: title.into(),
            composer: String::new(),
            key: "D".into(),
            meter: "4/4".into(),
            rhythm: "reel".into(),
            abc: "X:1\nK:D\n".into(),
            original_abc: None,
            credits: String::new(),
        }
    }

    fn library_with_folder(folder: &str, titles: &[(&str, &str)]) -> Library<Tune> {
        let mut library = Library::default();
        library.create_folder(folder).expect("create");
        for (id, title) in titles {
            library.insert(tune(id, title), Some(folder.into()), 0);
        }
        library
    }

    #[test]
    fn a_folder_set_is_resolved_live_from_the_library() {
        let mut library = library_with_folder("Reels", &[("1", "First"), ("2", "Second")]);
        let mut rotation = TuneRotation {
            source: TuneSetSource::Folder("Reels".into()),
            ..TuneRotation::default()
        };

        let resolved = resolve_tune_set(&library, &rotation);
        assert_eq!(
            resolved
                .tunes
                .iter()
                .map(|t| t.title.as_str())
                .collect::<Vec<_>>(),
            ["First", "Second"]
        );
        assert_eq!(resolved.missing_folder, None);

        // A tune added to the folder after the set was chosen joins it.
        library.insert(tune("3", "Third"), Some("Reels".into()), 0);
        let resolved = resolve_tune_set(&library, &rotation);
        assert_eq!(resolved.tunes.len(), 3);

        // A tune in another folder (or unfiled) does not join it.
        library.create_folder("Jigs").expect("create");
        library.insert(tune("4", "Jig"), Some("Jigs".into()), 0);
        let resolved = resolve_tune_set(&library, &rotation);
        assert_eq!(resolved.tunes.len(), 3);

        // Switching back to the explicit list ignores folder membership.
        rotation.source = TuneSetSource::List;
        rotation.add(
            library
                .get("abcsessiontunes", "1")
                .expect("saved")
                .item
                .source
                .clone(),
        );
        let resolved = resolve_tune_set(&library, &rotation);
        assert_eq!(resolved.tunes.len(), 1);
        assert_eq!(resolved.tunes[0].title, "First");
    }

    #[test]
    fn a_deleted_folder_falls_back_to_the_explicit_list() {
        let library = library_with_folder("Reels", &[("1", "First")]);
        let mut rotation = TuneRotation {
            source: TuneSetSource::Folder("Gone".into()),
            ..TuneRotation::default()
        };
        rotation.add(
            library
                .get("abcsessiontunes", "1")
                .expect("saved")
                .item
                .source
                .clone(),
        );

        let resolved = resolve_tune_set(&library, &rotation);
        assert_eq!(resolved.missing_folder.as_deref(), Some("Gone"));
        assert_eq!(resolved.tunes.len(), 1);
        assert_eq!(resolved.tunes[0].title, "First");
    }

    #[test]
    fn an_empty_folder_is_an_empty_set_not_a_fallback() {
        let mut library: Library<Tune> = Library::default();
        library.create_folder("Empty").expect("create");
        let rotation = TuneRotation {
            source: TuneSetSource::Folder("Empty".into()),
            ..TuneRotation::default()
        };

        let resolved = resolve_tune_set(&library, &rotation);
        assert!(resolved.tunes.is_empty());
        assert_eq!(resolved.missing_folder, None);
    }

    #[test]
    fn an_explicit_list_skips_tunes_that_are_no_longer_saved() {
        let library = library_with_folder("Reels", &[("1", "First")]);
        let saved = library
            .get("abcsessiontunes", "1")
            .expect("saved")
            .item
            .source
            .clone();
        let stale = SourceRef::new(
            "abcsessiontunes",
            "deleted",
            "https://thesession.org/tunes/deleted",
        );
        let mut rotation = TuneRotation::default();
        rotation.add(saved);
        rotation.add(stale);

        let resolved = resolve_tune_set(&library, &rotation);
        assert_eq!(resolved.tunes.len(), 1);
        assert_eq!(resolved.tunes[0].title, "First");
    }
}
