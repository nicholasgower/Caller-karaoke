//! Approximate spoken length of a call.
//!
//! Callers do not read the card at a constant rate, so this is deliberately a
//! guide rather than a precise speech model. The lead settings expose the
//! rate, word gap, and pause so a caller can tune the feel.

/// Count words using letters, digits, apostrophes, and the fraction glyph as
/// word characters.
pub fn word_count(text: &str) -> usize {
    tokens(text).count()
}

/// Estimate syllables in a call.
///
/// The estimator counts vowel groups, subtracts a silent trailing `e`, and
/// treats each number token as one syllable. It is intentionally simple and
/// is covered by tests for the default alias phrases.
pub fn count_syllables(text: &str) -> usize {
    let mut total = 0;
    for token in tokens(text) {
        total += token_syllables(token);
    }
    total.max(1)
}

fn tokens(text: &str) -> impl Iterator<Item = &str> + '_ {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\'' || c == '-' || c == '½'))
        .filter(|token| !token.is_empty())
}

fn token_syllables(token: &str) -> usize {
    if token.chars().all(|c| c.is_ascii_digit()) {
        return 1;
    }

    if token.contains('½') {
        // "1½" is normally rewritten by the alias engine to "once and a half".
        return 1 + token.chars().filter(|c| c.is_ascii_digit()).count();
    }

    let lower = token.to_ascii_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut groups = 0usize;
    let mut in_vowel = false;
    for c in &chars {
        let vowel = matches!(c, 'a' | 'e' | 'i' | 'o' | 'u');
        if vowel && !in_vowel {
            groups += 1;
        }
        in_vowel = vowel;
    }

    if groups == 0 {
        // "my", "rhythm": y is the only vowel-like letter.
        groups = 1;
    }

    if groups > 1
        && lower.ends_with('e')
        && !lower.ends_with("le")
        && !lower.ends_with("ee")
        && !lower.ends_with("ie")
    {
        groups -= 1;
    }

    groups.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_common_call_phrases() {
        assert_eq!(count_syllables("swing"), 1);
        assert_eq!(count_syllables("circle left"), 3);
        assert_eq!(count_syllables("do-si-do"), 3);
        assert_eq!(count_syllables("neighbors do-si-do once"), 6);
        assert_eq!(count_syllables("allemande"), 3);
        assert_eq!(count_syllables("balance"), 2);
        assert_eq!(count_syllables("ladies chain"), 3);
        assert_eq!(count_syllables("long lines forward and back"), 7);
    }

    #[test]
    fn counts_words() {
        assert_eq!(word_count("swing"), 1);
        assert_eq!(word_count("circle left three"), 3);
        assert_eq!(word_count("do-si-do"), 1);
        assert_eq!(word_count("long lines forward and back"), 5);
    }

    #[test]
    fn handles_numbers_and_fractions() {
        assert_eq!(count_syllables("circle left 3 places"), 6);
        assert!(count_syllables("1½") >= 1);
    }

    #[test]
    fn empty_text_is_one_syllable() {
        assert_eq!(count_syllables(""), 1);
    }
}
