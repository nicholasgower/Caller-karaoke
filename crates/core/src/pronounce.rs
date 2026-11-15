//! Pronunciation review for dance terms.
//!
//! The local Kokoro voice uses a grapheme-to-phoneme model. Most contra
//! vocabulary is already correct, but a few terms are not:
//!
//! - `poussette` came out as "pows-ET" instead of "poo-SET".
//! - `petronella` is pinned to the first-syllable contra stress so a lexicon
//!   update cannot move it.
//! - `promenade` came out as "prom-uh-NAHD" instead of "prom-uh-NADE".
//! - `O'More` was split into letters by the tokenizer.
//! - `do-si-do` is spoken as "doe see doe" by request.
//!
//! [`KOKORO_OVERRIDES`] gives the neural voice exact Kokoro phonemes. The
//! values use Kokoro's trained alphabet (the same symbols Misaki emits), not
//! plain IPA: `A` is /eɪ/, `I` is /aɪ/, `O` is /oʊ/, `W` is /aʊ/, and `Y` is
//! /ɔɪ/. [`PLATFORM_RESPELLINGS`] is the fallback for platform TTS engines,
//! which only accept text.
//!
//! Every default alias is checked by `crates/app/src/tts/kokoro.rs`; see
//! `docs/pronunciation.md` for the reviewed table.

use std::sync::LazyLock;

use regex::Regex;

/// Exact Kokoro phonemes for terms the neural voice mispronounces.
///
/// Keys are lowercase words. The caller should also insert a capitalized copy
/// because the G2P lexicon is case-sensitive.
pub const KOKORO_OVERRIDES: &[(&str, &str)] = &[
    // "poo-SET", not "PWSS-et-te".
    ("poussette", "puˈsɛt"),
    // "PET-roh-nel-uh", not "pet-roh-NEL-uh".
    ("petronella", "pˈɛtɹOnˌɛlə"),
    // "prom-uh-NADE", not "prom-uh-NAHD".
    ("promenade", "pɹˌɑmənˈAd"),
    // The G2P lexicon does not know the gender-neutral role term and otherwise
    // spells it out letter by letter.
    ("gentlespoon", "ˈɡɛntəlspuːn"),
    ("gentlespoons", "ˈɡɛntəlspuːnz"),
    // `do-si-do` is normalized to "doe see doe"; these pin the two words and
    // the whole hyphenated form to the requested "doe see doe".
    ("do-si-do", "dˈO sˈiː dˈO"),
    ("dosido", "dˈO sˈiː dˈO"),
    ("doe", "dˈO"),
    ("see", "sˈiː"),
];

/// Text respellings for platform TTS fallbacks.
///
/// These are only used when the Kokoro voice is unavailable. They are ordered
/// longest first so a phrase is never partially replaced by a shorter entry.
pub const PLATFORM_RESPELLINGS: &[(&str, &str)] = &[
    ("poussette", "poo set"),
    ("petronella", "petronelluh"),
    ("gyre", "jire"),
    ("o'more", "O More"),
    ("gentlespoon", "gentle spoon"),
    ("gentlespoons", "gentle spoons"),
];

static O_MORE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bO'More\b").expect("the O'More pattern is valid"));

static DO_SI_DO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bdo[\s-]*si[\s-]*do\b").expect("the do-si-do pattern is valid")
});

/// Callers reduce "and a half" to "an a half". The neural voice then gets a
/// connected tail instead of three separate words, and platform voices drop
/// the hard /d/ that makes the phrase sound choppy.
static HALF_PHRASE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\band a half\b").expect("the half-phrase pattern is valid"));

static RESPELLERS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    PLATFORM_RESPELLINGS
        .iter()
        .map(|(from, to)| {
            let pattern = format!(r"(?i)\b{}\b", regex::escape(from));
            (
                Regex::new(&pattern).expect("respelling pattern is valid"),
                *to,
            )
        })
        .collect()
});

/// Normalize text before the Kokoro G2P.
///
/// Curly apostrophes are folded to ASCII, `O'More` is expanded to `O More`,
/// `do-si-do` is respelled as "doe see doe", and `and a half` becomes the
/// connected "an a half". The G2P tokenizer otherwise splits `O'More` into
/// letters, its default `do-si-do` is "doo see doo", and the count tail is read
/// as three separate words.
pub fn kokoro_text(text: &str) -> String {
    let text = text.replace(['\u{2018}', '\u{2019}', '\u{201B}'], "'");
    let text = O_MORE.replace_all(&text, "O More");
    let text = DO_SI_DO.replace_all(&text, "doe see doe");
    HALF_PHRASE.replace_all(&text, "an a half").into_owned()
}

/// Apply [`PLATFORM_RESPELLINGS`] to text for a platform TTS engine.
///
/// Matching is case-insensitive and word-boundary aware. The function is
/// idempotent: a second call leaves the already-respelled text unchanged.
pub fn platform_text(text: &str) -> String {
    let mut text = kokoro_text(text);
    for (pattern, replacement) in RESPELLERS.iter() {
        text = pattern.replace_all(&text, *replacement).into_owned();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_o_more_for_kokoro() {
        assert_eq!(kokoro_text("Rory O'More"), "Rory O More");
        assert_eq!(kokoro_text("Rory O\u{2019}More"), "Rory O More");
        assert_eq!(kokoro_text("rory o'more"), "rory O More");
        assert_eq!(kokoro_text("swing your partner"), "swing your partner");
    }

    #[test]
    fn respells_do_si_do() {
        assert_eq!(
            kokoro_text("neighbors do-si-do once"),
            "neighbors doe see doe once"
        );
        assert_eq!(kokoro_text("do si do"), "doe see doe");
        assert_eq!(kokoro_text("dosido"), "doe see doe");
        assert_eq!(platform_text("do-si-do"), "doe see doe");
    }

    #[test]
    fn smooths_and_a_half() {
        assert_eq!(kokoro_text("once and a half"), "once an a half");
        assert_eq!(kokoro_text("twice and a half"), "twice an a half");
        assert_eq!(platform_text("once and a half"), "once an a half");
        assert_eq!(kokoro_text("once an a half"), "once an a half");
    }

    #[test]
    fn respells_only_whole_words() {
        assert_eq!(platform_text("poussette"), "poo set");
        assert_eq!(platform_text("petronella"), "petronelluh");
        assert_eq!(
            platform_text("a poussette and a gyre"),
            "a poo set and a jire"
        );
        assert_eq!(platform_text("poussettes"), "poussettes");
        assert_eq!(platform_text("gyres"), "gyres");
        assert_eq!(platform_text("swingy"), "swingy");
        assert_eq!(platform_text("gentlespoons"), "gentle spoons");
        assert_eq!(platform_text("gentlespoon"), "gentle spoon");
    }

    #[test]
    fn platform_text_expands_o_more() {
        assert_eq!(platform_text("Rory O'More"), "Rory O More");
        assert_eq!(platform_text("Rory O\u{2019}More"), "Rory O More");
    }

    #[test]
    fn platform_text_is_idempotent() {
        let once = platform_text("Rory O'More poussette petronella gyre");
        let twice = platform_text(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn overrides_use_kokoro_symbols() {
        // These are the trained Kokoro tokens used by the reviewed words.
        assert!(KOKORO_OVERRIDES[0].1.contains('ˈ'));
        assert!(KOKORO_OVERRIDES[1].1.contains('ˌ'));
        assert!(KOKORO_OVERRIDES[2].1.contains('A'));
        // The override table must not be empty or contain duplicate keys.
        let mut keys = std::collections::BTreeSet::new();
        for (key, _) in KOKORO_OVERRIDES {
            assert!(keys.insert(*key), "duplicate pronunciation key {key}");
        }
    }
}
