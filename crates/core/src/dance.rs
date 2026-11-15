//! Source-agnostic dance and figure models.
//!
//! The models deliberately avoid any knowledge of ContraDB's HTML or JSON.
//! A provider in `contra-sources` turns a site response into these types.

use serde::{Deserialize, Serialize};

/// A stable reference to an item in an external source.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    /// Provider key, e.g. `contradb` or `abcsessiontunes`.
    pub provider: String,
    /// The source-specific identifier, e.g. `2409`.
    pub id: String,
    /// The canonical page URL for attribution and "open in browser".
    pub url: String,
}

impl SourceRef {
    pub fn new(provider: impl Into<String>, id: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            id: id.into(),
            url: url.into(),
        }
    }
}

/// One row of a dance card.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Figure {
    /// Phrase label such as `A1`, `A2`, `B1`, or `B2`. May be absent on
    /// continuing rows.
    pub phrase: Option<String>,
    /// The number of dance beats assigned to this row.
    pub beats: f64,
    /// The card text, with HTML removed but wording and abbreviations intact.
    pub text: String,
}

impl Figure {
    pub fn new(phrase: Option<String>, beats: f64, text: impl Into<String>) -> Self {
        Self {
            phrase,
            beats: if beats.is_finite() && beats > 0.0 {
                beats
            } else {
                8.0
            },
            text: text.into(),
        }
    }
}

/// A complete contra dance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dance {
    pub source: SourceRef,
    pub title: String,
    pub choreographer: String,
    pub formation: String,
    pub preamble: String,
    pub notes: String,
    pub figures: Vec<Figure>,
}

impl Dance {
    pub fn total_beats(&self) -> f64 {
        self.figures.iter().map(|f| f.beats).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.figures.is_empty()
    }

    /// Basic validation used before building a practice timeline.
    pub fn validate(&self) -> Result<(), DanceError> {
        if self.title.trim().is_empty() {
            return Err(DanceError::MissingTitle);
        }
        if self.figures.is_empty() {
            return Err(DanceError::NoFigures);
        }
        if self.total_beats() <= 0.0 {
            return Err(DanceError::NoBeats);
        }
        for (index, figure) in self.figures.iter().enumerate() {
            if figure.text.trim().is_empty() {
                return Err(DanceError::EmptyFigure { index });
            }
            if !figure.beats.is_finite() || figure.beats <= 0.0 {
                return Err(DanceError::InvalidBeats { index });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DanceError {
    #[error("dance has no title")]
    MissingTitle,
    #[error("dance has no figures")]
    NoFigures,
    #[error("dance has no beats")]
    NoBeats,
    #[error("figure {index} has no text")]
    EmptyFigure { index: usize },
    #[error("figure {index} has invalid beat count")]
    InvalidBeats { index: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Dance {
        Dance {
            source: SourceRef::new("contradb", "2409", "https://contradb.com/dances/2409"),
            title: "Another Last Dance".into(),
            choreographer: "Erik Hoffman".into(),
            formation: "improper".into(),
            preamble: String::new(),
            notes: String::new(),
            figures: vec![
                Figure::new(Some("A1".into()), 8.0, "neighbors do si do once"),
                Figure::new(None, 8.0, "ladles allemande left 1½"),
            ],
        }
    }

    #[test]
    fn total_beats_is_cumulative() {
        assert_eq!(sample().total_beats(), 16.0);
    }

    #[test]
    fn validates_good_dance() {
        assert!(sample().validate().is_ok());
    }

    #[test]
    fn rejects_empty_figure() {
        let mut dance = sample();
        dance.figures[0].text.clear();
        assert_eq!(dance.validate(), Err(DanceError::EmptyFigure { index: 0 }));
    }
}
