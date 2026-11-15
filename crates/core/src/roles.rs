//! Contra dance role terms.
//!
//! ContraDB's canonical card text uses "ladles" and "gentlespoons". Callers
//! use many different role vocabularies, so card text and rendered calls are
//! passed through a configurable role replacement.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleSide {
    Left,
    Right,
}

/// A pair of role terms in singular and plural form.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleTerms {
    pub left_singular: String,
    pub left_plural: String,
    pub right_singular: String,
    pub right_plural: String,
}

impl Default for RoleTerms {
    fn default() -> Self {
        Self::preset(RolePreset::LarksRobins)
    }
}

impl RoleTerms {
    pub fn preset(preset: RolePreset) -> Self {
        match preset {
            RolePreset::LarksRobins => Self {
                left_singular: "lark".into(),
                left_plural: "larks".into(),
                right_singular: "robin".into(),
                right_plural: "robins".into(),
            },
            RolePreset::LadiesGentlemen => Self {
                left_singular: "lady".into(),
                left_plural: "ladies".into(),
                right_singular: "gentleman".into(),
                right_plural: "gentlemen".into(),
            },
            RolePreset::WomenMen => Self {
                left_singular: "woman".into(),
                left_plural: "women".into(),
                right_singular: "man".into(),
                right_plural: "men".into(),
            },
            RolePreset::LeadsFollows => Self {
                left_singular: "lead".into(),
                left_plural: "leads".into(),
                right_singular: "follow".into(),
                right_plural: "follows".into(),
            },
            RolePreset::LeftsRights => Self {
                left_singular: "left".into(),
                left_plural: "lefts".into(),
                right_singular: "right".into(),
                right_plural: "rights".into(),
            },
            RolePreset::Custom => Self {
                left_singular: "lark".into(),
                left_plural: "larks".into(),
                right_singular: "robin".into(),
                right_plural: "robins".into(),
            },
        }
    }

    pub fn singular(&self, side: RoleSide) -> &str {
        match side {
            RoleSide::Left => &self.left_singular,
            RoleSide::Right => &self.right_singular,
        }
    }

    pub fn plural(&self, side: RoleSide) -> &str {
        match side {
            RoleSide::Left => &self.left_plural,
            RoleSide::Right => &self.right_plural,
        }
    }

    pub fn term(&self, side: RoleSide, plural: bool) -> &str {
        if plural {
            self.plural(side)
        } else {
            self.singular(side)
        }
    }

    /// Replace known role words in `text` with the configured terms.
    ///
    /// Direction words such as "left" and "right" are deliberately not
    /// treated as role words unless they are explicitly plural ("lefts",
    /// "rights"), so "circle left" never becomes "circle rights".
    pub fn apply(&self, text: &str) -> String {
        ROLE_RE
            .replace_all(text, |caps: &regex::Captures<'_>| {
                let matched = &caps[0];
                let Some(info) = role_word(matched) else {
                    return matched.to_string();
                };
                let replacement = self.term(info.side, info.plural);
                preserve_case(matched, replacement)
            })
            .into_owned()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RolePreset {
    LarksRobins,
    LadiesGentlemen,
    WomenMen,
    LeadsFollows,
    LeftsRights,
    Custom,
}

impl RolePreset {
    pub const ALL: [RolePreset; 6] = [
        RolePreset::LarksRobins,
        RolePreset::LadiesGentlemen,
        RolePreset::WomenMen,
        RolePreset::LeadsFollows,
        RolePreset::LeftsRights,
        RolePreset::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RolePreset::LarksRobins => "Larks / Robins",
            RolePreset::LadiesGentlemen => "Ladies / Gentlemen",
            RolePreset::WomenMen => "Women / Men",
            RolePreset::LeadsFollows => "Leads / Follows",
            RolePreset::LeftsRights => "Lefts / Rights",
            RolePreset::Custom => "Custom",
        }
    }

    /// Infer the preset that matches a pair of terms, if any.
    pub fn detect(terms: &RoleTerms) -> Self {
        RolePreset::ALL
            .into_iter()
            .find(|preset| *preset != RolePreset::Custom && &RoleTerms::preset(*preset) == terms)
            .unwrap_or(RolePreset::Custom)
    }
}

struct RoleWord {
    side: RoleSide,
    plural: bool,
}

/// Role words as they are likely to appear on a dance card.
///
/// Longest forms come first so that "gentlespoons" is not partially matched.
static ROLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(gentlespoons|gentlespoon|gentlemen|gentleman|gentles|gents|gent|ladles|ladle|ladies|lady|larks|lark|leads|lead|lefts|women|woman|robins|robin|follows|follow|rights|men|man|gals|gal|guys|guy|belles|belle|beaus|beau)\b",
    )
    .expect("role regex is valid")
});

fn role_word(word: &str) -> Option<RoleWord> {
    let lower = word.to_ascii_lowercase();
    let (side, plural) = match lower.as_str() {
        "gentlespoons" | "gentlespoon" | "gentlemen" | "gentleman" | "gents" | "gent" => {
            (RoleSide::Right, lower.ends_with('s'))
        }
        "ladles" | "ladle" | "ladies" | "lady" => (RoleSide::Left, lower.ends_with('s')),
        "larks" | "lark" => (RoleSide::Left, lower.ends_with('s')),
        "leads" | "lead" | "lefts" => (RoleSide::Left, lower.ends_with('s')),
        "women" | "woman" => (RoleSide::Left, lower == "women"),
        "robins" | "robin" => (RoleSide::Right, lower.ends_with('s')),
        "follows" | "follow" | "rights" => (RoleSide::Right, lower.ends_with('s')),
        "men" | "man" => (RoleSide::Right, lower == "men"),
        "gals" | "gal" => (RoleSide::Left, lower.ends_with('s')),
        "guys" | "guy" => (RoleSide::Right, lower.ends_with('s')),
        "belles" | "belle" => (RoleSide::Left, lower.ends_with('s')),
        "beaus" | "beau" => (RoleSide::Right, lower.ends_with('s')),
        _ => return None,
    };
    Some(RoleWord { side, plural })
}

fn preserve_case(original: &str, replacement: &str) -> String {
    let mut chars = original.chars();
    let Some(first) = chars.next() else {
        return replacement.to_string();
    };
    if first.is_uppercase() {
        let mut out = String::with_capacity(replacement.len());
        let mut replacement_chars = replacement.chars();
        if let Some(c) = replacement_chars.next() {
            out.extend(c.to_uppercase());
        }
        out.extend(replacement_chars);
        out
    } else {
        replacement.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_larks_and_robins() {
        let terms = RoleTerms::default();
        assert_eq!(terms.left_plural, "larks");
        assert_eq!(terms.right_plural, "robins");
    }

    #[test]
    fn replaces_contradb_terms() {
        let terms = RoleTerms::default();
        assert_eq!(
            terms.apply("ladles allemande left 1½"),
            "larks allemande left 1½"
        );
        assert_eq!(terms.apply("gentlespoons chain"), "robins chain");
    }

    #[test]
    fn leaves_directions_alone() {
        let terms = RoleTerms::preset(RolePreset::LeftsRights);
        assert_eq!(terms.apply("ladles allemande left"), "lefts allemande left");
        assert_eq!(terms.apply("circle left 3 places"), "circle left 3 places");
    }

    #[test]
    fn preserves_capitalisation() {
        let terms = RoleTerms::default();
        assert_eq!(terms.apply("Ladles chain"), "Larks chain");
        assert_eq!(terms.apply("Gentlespoons swing"), "Robins swing");
    }

    #[test]
    fn presets_round_trip() {
        for preset in RolePreset::ALL {
            if preset == RolePreset::Custom {
                continue;
            }
            let terms = RoleTerms::preset(preset);
            assert_eq!(RolePreset::detect(&terms), preset);
        }
    }

    #[test]
    fn custom_terms_are_detected_as_custom() {
        let mut terms = RoleTerms::default();
        terms.left_plural = "jades".into();
        assert_eq!(RolePreset::detect(&terms), RolePreset::Custom);
    }
}
