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
            RolePreset::LarksRavens => Self {
                left_singular: "lark".into(),
                left_plural: "larks".into(),
                right_singular: "raven".into(),
                right_plural: "ravens".into(),
            },
            // A gent finishes a swing on the left (the lark side); a lady
            // finishes on the right (the robin side). Preset fields are in
            // left-then-right order, so the gent terms come first here.
            RolePreset::LadiesGentlemen => Self {
                left_singular: "gentleman".into(),
                left_plural: "gentlemen".into(),
                right_singular: "lady".into(),
                right_plural: "ladies".into(),
            },
            RolePreset::WomenMen => Self {
                left_singular: "man".into(),
                left_plural: "men".into(),
                right_singular: "woman".into(),
                right_plural: "women".into(),
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
    LarksRavens,
    LadiesGentlemen,
    WomenMen,
    LeadsFollows,
    LeftsRights,
    Custom,
}

impl RolePreset {
    pub const ALL: [RolePreset; 7] = [
        RolePreset::LarksRobins,
        RolePreset::LarksRavens,
        RolePreset::LadiesGentlemen,
        RolePreset::WomenMen,
        RolePreset::LeadsFollows,
        RolePreset::LeftsRights,
        RolePreset::Custom,
    ];

    /// The menu label, always in left-then-right order.
    ///
    /// The model is [`RoleSide::Left`] and [`RoleSide::Right`], and every
    /// label names the left-side term first ("Larks / Robins", "Gentlemen /
    /// Ladies"). A dance card may write its pair in either order, but the
    /// preset menu must not mix the two; a new preset follows the same rule.
    pub fn label(self) -> &'static str {
        match self {
            RolePreset::LarksRobins => "Larks / Robins",
            RolePreset::LarksRavens => "Larks / Ravens",
            RolePreset::LadiesGentlemen => "Gentlemen / Ladies",
            RolePreset::WomenMen => "Men / Women",
            RolePreset::LeadsFollows => "Leads / Follows",
            RolePreset::LeftsRights => "Lefts / Rights",
            RolePreset::Custom => "Custom",
        }
    }

    /// Infer the preset that matches a pair of terms, if any.
    ///
    /// This is only an inference, used to upgrade older preference files: the
    /// UI reads the stored selection, because custom words can equal a named
    /// preset and must not snap the menu back.
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
        r"(?i)\b(gentlespoons|gentlespoon|gentlemen|gentleman|gentles|gents|gent|ladles|ladle|ladies|lady|larks|lark|leads|lead|lefts|women|woman|ravens|raven|robins|robin|follows|follow|rights|men|man|gals|gal|guys|guy|belles|belle|beaus|beau)\b",
    )
    .expect("role regex is valid")
});

fn role_word(word: &str) -> Option<RoleWord> {
    let lower = word.to_ascii_lowercase();
    // A gent (and its synonyms) is the lark/left-side role; a lady is the
    // robin/right-side role. This matches where each dancer finishes a swing.
    let (side, plural) = match lower.as_str() {
        "gentlespoons" | "gentlespoon" | "gentleman" | "gentles" | "gents" | "gent" => {
            (RoleSide::Left, lower.ends_with('s'))
        }
        // "gentlemen" is an irregular plural, like "men".
        "gentlemen" => (RoleSide::Left, true),
        "ladles" | "ladle" | "ladies" | "lady" => (RoleSide::Right, lower.ends_with('s')),
        "larks" | "lark" => (RoleSide::Left, lower.ends_with('s')),
        "leads" | "lead" | "lefts" => (RoleSide::Left, lower.ends_with('s')),
        "women" | "woman" => (RoleSide::Right, lower == "women"),
        "robins" | "robin" | "ravens" | "raven" => (RoleSide::Right, lower.ends_with('s')),
        "follows" | "follow" | "rights" => (RoleSide::Right, lower.ends_with('s')),
        "men" | "man" => (RoleSide::Left, lower == "men"),
        "gals" | "gal" => (RoleSide::Right, lower.ends_with('s')),
        "guys" | "guy" => (RoleSide::Left, lower.ends_with('s')),
        "belles" | "belle" => (RoleSide::Right, lower.ends_with('s')),
        "beaus" | "beau" => (RoleSide::Left, lower.ends_with('s')),
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
        // A lady is a robin (right side); a gent is a lark (left side).
        assert_eq!(
            terms.apply("ladles allemande left 1½"),
            "robins allemande left 1½"
        );
        assert_eq!(terms.apply("gentlespoons chain"), "larks chain");
    }

    #[test]
    fn leaves_directions_alone() {
        let terms = RoleTerms::preset(RolePreset::LeftsRights);
        assert_eq!(
            terms.apply("gentlespoons allemande left"),
            "lefts allemande left"
        );
        assert_eq!(terms.apply("circle left 3 places"), "circle left 3 places");
    }

    #[test]
    fn preserves_capitalisation() {
        let terms = RoleTerms::default();
        assert_eq!(terms.apply("Ladles chain"), "Robins chain");
        assert_eq!(terms.apply("Gentlespoons swing"), "Larks swing");
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
    fn every_preset_label_reads_left_then_right() {
        for preset in RolePreset::ALL {
            if preset == RolePreset::Custom {
                assert_eq!(preset.label(), "Custom");
                continue;
            }
            let terms = RoleTerms::preset(preset);
            let left = capitalise(&terms.left_plural);
            let right = capitalise(&terms.right_plural);
            assert_eq!(
                preset.label(),
                format!("{left} / {right}"),
                "{} must name the left side first",
                preset.label()
            );
        }
    }

    fn capitalise(word: &str) -> String {
        let mut chars = word.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    }

    #[test]
    fn larks_and_ravens_map_left_and_right() {
        let terms = RoleTerms::preset(RolePreset::LarksRavens);
        assert_eq!(terms.left_plural, "larks");
        assert_eq!(terms.right_plural, "ravens");
        assert_eq!(
            terms.apply("ladles allemande left 1½"),
            "ravens allemande left 1½"
        );
        assert_eq!(terms.apply("gentlespoons chain"), "larks chain");
        assert_eq!(RolePreset::detect(&terms), RolePreset::LarksRavens);
    }

    #[test]
    fn ravens_are_a_right_side_role_word() {
        assert_eq!(RoleTerms::default().apply("ravens chain"), "robins chain");
        assert_eq!(RoleTerms::default().apply("Raven swing"), "Robin swing");
    }

    #[test]
    fn custom_terms_apply_to_card_and_call_phrases() {
        let terms = RoleTerms {
            left_singular: "port".into(),
            left_plural: "ports".into(),
            right_singular: "starboard".into(),
            right_plural: "starboards".into(),
        };
        // A dance-card phrase and a spoken call phrase.
        assert_eq!(
            terms.apply("ladles allemande left 1½"),
            "starboards allemande left 1½"
        );
        assert_eq!(terms.apply("gentlespoons chain"), "ports chain");
        assert_eq!(terms.apply("swing your ladle"), "swing your starboard");
        assert_eq!(terms.apply("gentlespoon do si do"), "port do si do");
    }

    #[test]
    fn lady_is_robin_and_gent_is_lark() {
        // A gent finishes a swing on the left (lark); a lady on the right
        // (robin). This must hold for every preset, not just the default.
        for preset in [
            RolePreset::LarksRobins,
            RolePreset::LarksRavens,
            RolePreset::LadiesGentlemen,
            RolePreset::WomenMen,
            RolePreset::LeadsFollows,
            RolePreset::LeftsRights,
        ] {
            let terms = RoleTerms::preset(preset);
            assert_eq!(
                terms.apply("ladies chain"),
                format!("{} chain", terms.right_plural),
                "ladies must be the robin side at {preset:?}"
            );
            assert_eq!(
                terms.apply("gentlemen chain"),
                format!("{} chain", terms.left_plural),
                "gentlemen must be the lark side at {preset:?}"
            );
            assert_eq!(
                terms.apply("lady swing"),
                format!("{} swing", terms.right_singular),
                "lady must be the robin side at {preset:?}"
            );
            assert_eq!(
                terms.apply("gentleman swing"),
                format!("{} swing", terms.left_singular),
                "gentleman must be the lark side at {preset:?}"
            );
        }
    }

    #[test]
    fn leaves_shadow_neighbor_partner_and_directions_alone() {
        for preset in [
            RolePreset::LarksRobins,
            RolePreset::LarksRavens,
            RolePreset::LadiesGentlemen,
            RolePreset::LeftsRights,
        ] {
            let terms = RoleTerms::preset(preset);
            for text in [
                "shadow",
                "neighbor",
                "partners balance & swing",
                "circle left 3 places",
                "right hand",
            ] {
                assert_eq!(
                    terms.apply(text),
                    text,
                    "{} at {preset:?}",
                    terms.apply(text)
                );
            }
        }
    }

    #[test]
    fn custom_terms_are_detected_as_custom() {
        let terms = RoleTerms {
            left_plural: "jades".into(),
            ..RoleTerms::default()
        };
        assert_eq!(RolePreset::detect(&terms), RolePreset::Custom);
    }
}
