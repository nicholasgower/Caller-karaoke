//! Source-agnostic tune model.

use serde::{Deserialize, Serialize};

use crate::dance::SourceRef;

/// A tune with its ABC source text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tune {
    pub source: SourceRef,
    pub title: String,
    pub composer: String,
    pub key: String,
    pub meter: String,
    pub rhythm: String,
    pub abc: String,
    /// Transcription credit or source note from the provider.
    pub credits: String,
}

impl Tune {
    pub fn display_key(&self) -> &str {
        if self.key.trim().is_empty() {
            "unknown key"
        } else {
            &self.key
        }
    }

    pub fn display_meter(&self) -> &str {
        if self.meter.trim().is_empty() {
            "4/4"
        } else {
            &self.meter
        }
    }

    pub fn display_rhythm(&self) -> &str {
        if self.rhythm.trim().is_empty() {
            "tune"
        } else {
            &self.rhythm
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_labels_are_friendly() {
        let tune = Tune {
            source: SourceRef::new("abcsessiontunes", "1", "https://example.invalid"),
            title: "Test".into(),
            composer: String::new(),
            key: String::new(),
            meter: String::new(),
            rhythm: String::new(),
            abc: "X:1\nK:D\n".into(),
            credits: String::new(),
        };
        assert_eq!(tune.display_key(), "unknown key");
        assert_eq!(tune.display_meter(), "4/4");
        assert_eq!(tune.display_rhythm(), "tune");
    }
}
