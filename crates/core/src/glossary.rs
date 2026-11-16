//! Short in-app reference for the contra dance terms shown in a session.
//!
//! The definitions are original summaries of the language used in The Caller's
//! Box glossary, not copies of its entries. The full glossary explains the
//! notation and the regional variants behind each name, while this table exists
//! only so a caller can select a term in a call or on a dance card and read a
//! short reminder without leaving the app.
//!
//! The table and the matcher are pure domain data: the UI decides how a link is
//! drawn, and a provider or timing change never has to know about glossary
//! entries.

/// A single term and the short definition shown in the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlossaryEntry {
    /// Canonical name shown as the entry heading.
    pub term: &'static str,
    /// Case-insensitive spellings that should link to this entry. The canonical
    /// term is repeated here so a caller can check one list for every match.
    pub aliases: &'static [&'static str],
    /// A short original definition, not a quotation from the source.
    pub definition: &'static str,
}

/// Where the definitions came from.
pub const SOURCE_NAME: &str = "The Caller's Box";
/// The full external glossary.
pub const SOURCE_URL: &str = "https://www.ibiblio.org/contradance/thecallersbox/Glossary.htm";
/// How the in-app text relates to the source.
pub const SOURCE_NOTE: &str = "Short original summaries based on The Caller's Box \
     Glossary of contra dance figures. Follow the link for the full definitions, \
     notation, and variants.";

/// Every term in the app glossary, in alphabetical order for the reference page.
///
/// The list deliberately concentrates on terms that can appear in a dance card
/// or a rendered call. It is not an exhaustive dictionary of contra dance.
pub const ENTRIES: &[GlossaryEntry] = &[
    GlossaryEntry {
        term: "Allemande",
        aliases: &["allemande", "arm turn"],
        definition: "A hand turn with a neighbor or partner using a forearm grip, \
            usually for a stated fraction of a circle.",
    },
    GlossaryEntry {
        term: "Allemande orbit",
        aliases: &["allemande orbit"],
        definition: "An allemande that keeps going so the two dancers travel around \
            the set together.",
    },
    GlossaryEntry {
        term: "Arch and dive",
        aliases: &["arch and dive", "arch", "dive"],
        definition: "One couple raises joined hands into an arch while the other \
            couple ducks through it and the couples trade places.",
    },
    GlossaryEntry {
        term: "Balance",
        aliases: &["balance"],
        definition: "An in-place four-beat movement, usually toward and away from \
            another dancer, before the next figure.",
    },
    GlossaryEntry {
        term: "Balance and swing",
        aliases: &["balance and swing", "balance & swing", "balance then swing"],
        definition: "Balance first, then swing the same partner or neighbor.",
    },
    GlossaryEntry {
        term: "Balance the ring",
        aliases: &["balance the ring", "balance in the ring", "balance ring"],
        definition: "Join hands in a ring and balance in and out together.",
    },
    GlossaryEntry {
        term: "Becket",
        aliases: &["becket"],
        definition: "A formation in which partners begin side by side on the same \
            side of the set, facing another couple across.",
    },
    GlossaryEntry {
        term: "Box the gnat",
        aliases: &["box the gnat"],
        definition: "Two dancers take right hands, raise an arch, and trade places \
            under the joined hands.",
    },
    GlossaryEntry {
        term: "Butterfly whirl",
        aliases: &["butterfly whirl"],
        definition: "A swinging turn in which partners spin while one or both pass \
            under joined hands.",
    },
    GlossaryEntry {
        term: "California twirl",
        aliases: &["california twirl"],
        definition: "Partners join hands, raise an arch, and turn so they trade \
            places under it.",
    },
    GlossaryEntry {
        term: "Chain",
        aliases: &[
            "chain",
            "ladies chain",
            "ladies' chain",
            "gents chain",
            "gentlemen's chain",
        ],
        definition: "The named dancers cross to the opposite couple while the \
            others turn them into place.",
    },
    GlossaryEntry {
        term: "Circle",
        aliases: &["circle"],
        definition: "Take hands in a ring and walk in the named direction for the \
            stated count.",
    },
    GlossaryEntry {
        term: "Contra corners",
        aliases: &["contra corners", "contra corner"],
        definition: "A figure that turns first one diagonal corner and then the \
            other before returning to a partner swing.",
    },
    GlossaryEntry {
        term: "Corner",
        aliases: &["corner", "corners"],
        definition: "A named dancer relative to you, often the person beside or \
            diagonally across in the set.",
    },
    GlossaryEntry {
        term: "Count-in",
        aliases: &["count-in", "count in"],
        definition: "Four beats of accompaniment before the dance begins, used to \
            set the tempo and the first step.",
    },
    GlossaryEntry {
        term: "Cross trails",
        aliases: &["cross trails", "cross the trails"],
        definition: "Two dancers cross the set on different paths, one in front and \
            one behind.",
    },
    GlossaryEntry {
        term: "Do-si-do",
        aliases: &["do-si-do", "do si do", "dosido", "back to back"],
        definition: "Two dancers pass back to back around each other without \
            turning and return to place.",
    },
    GlossaryEntry {
        term: "Dolphin hey",
        aliases: &["dolphin hey"],
        definition: "A hey in which the middle pass is a tight loop around a pair \
            of dancers.",
    },
    GlossaryEntry {
        term: "Down the hall",
        aliases: &["down the hall", "head down the hall"],
        definition: "Travel toward the far end of the hall, away from the band.",
    },
    GlossaryEntry {
        term: "Duple minor",
        aliases: &["duple minor"],
        definition: "A contra formation with two couples in each minor set, the \
            usual four-person working group.",
    },
    GlossaryEntry {
        term: "Figure eight",
        aliases: &["figure eight", "figure 8"],
        definition: "A path that loops in a figure-eight shape around two people or \
            positions.",
    },
    GlossaryEntry {
        term: "Form an ocean wave",
        aliases: &[
            "ocean wave",
            "form an ocean wave",
            "form a wave",
            "make a wave",
        ],
        definition: "Join hands in a short or long wave with alternating right and \
            left hands.",
    },
    GlossaryEntry {
        term: "Gate",
        aliases: &["gate"],
        definition: "One dancer turns another around in a circle while their joined \
            hands stay together.",
    },
    GlossaryEntry {
        term: "Give and take",
        aliases: &["give and take"],
        definition: "One dancer gives weight while the other takes it, pulling or \
            pushing into the next position.",
    },
    GlossaryEntry {
        term: "Gyre / gypsy",
        aliases: &[
            "gyre",
            "gypsy",
            "right shoulder round",
            "left shoulder round",
            "mirror shoulder round",
        ],
        definition: "Two dancers walk around each other while facing, without \
            taking hands; also called a shoulder round.",
    },
    GlossaryEntry {
        term: "Hey",
        aliases: &["hey", "hey for four"],
        definition: "A weaving figure in which dancers pass alternately by the \
            right and left shoulders around a ring.",
    },
    GlossaryEntry {
        term: "Improper",
        aliases: &["improper"],
        definition: "The common contra formation in which each couple has swapped \
            sides, so the roles are opposite the proper arrangement.",
    },
    GlossaryEntry {
        term: "Lark / robin",
        aliases: &["lark", "larks", "robin", "robins", "raven", "ravens"],
        definition: "The default role names: larks dance the left side and robins \
            the right side when facing across the set.",
    },
    GlossaryEntry {
        term: "Lady / gentleman",
        aliases: &[
            "lady",
            "ladies",
            "gentleman",
            "gentlemen",
            "gent",
            "gents",
            "woman",
            "women",
            "man",
            "men",
        ],
        definition: "Traditional role names that follow the same left/right role \
            model as larks and robins.",
    },
    GlossaryEntry {
        term: "Leads / follows",
        aliases: &["leads", "follows"],
        definition: "Role names that follow the same left/right role model as larks \
            and robins.",
    },
    GlossaryEntry {
        term: "Long lines",
        aliases: &["long lines", "lines forward and back"],
        definition: "The two sides of the set face in, join hands in long lines, \
            and move forward and back.",
    },
    GlossaryEntry {
        term: "Long waves",
        aliases: &[
            "long wave",
            "long waves",
            "form long waves",
            "make long waves",
        ],
        definition: "Join hands in long zigzag lines with dancers facing in and out \
            of the center.",
    },
    GlossaryEntry {
        term: "Mad robin",
        aliases: &["mad robin"],
        definition: "Two dancers circle around each other, one walking forward while \
            the other backs up.",
    },
    GlossaryEntry {
        term: "Major set",
        aliases: &["major set"],
        definition: "The entire line of dancers, made up of all the minor sets.",
    },
    GlossaryEntry {
        term: "Meltdown swing",
        aliases: &["meltdown swing"],
        definition: "A swing that flows straight into the next figure without a \
            separate stop or reset.",
    },
    GlossaryEntry {
        term: "Minor set",
        aliases: &["minor set"],
        definition: "The group of dancers who work together for one pass of the \
            dance before progressing.",
    },
    GlossaryEntry {
        term: "Neighbor",
        aliases: &["neighbor", "neighbors", "neighbour", "neighbours"],
        definition: "The person beside you in the minor set, who changes when the \
            dance progresses.",
    },
    GlossaryEntry {
        term: "Partner",
        aliases: &["partner", "partners"],
        definition: "The person you dance the sequence with; a partner stays with \
            you as the minor set progresses.",
    },
    GlossaryEntry {
        term: "Pass by",
        aliases: &["pass by", "pass on by"],
        definition: "Pass a dancer without taking hands or turning.",
    },
    GlossaryEntry {
        term: "Pass through",
        aliases: &["pass through", "pass on through"],
        definition: "Pass the facing dancer by the right shoulder and keep going.",
    },
    GlossaryEntry {
        term: "Petronella",
        aliases: &["petronella", "petronella turn"],
        definition: "Dancers move one position around the set, each turning into \
            the place ahead.",
    },
    GlossaryEntry {
        term: "Poussette",
        aliases: &["poussette"],
        definition: "Two couples hold hands and travel as a four, often trading \
            places.",
    },
    GlossaryEntry {
        term: "Promenade",
        aliases: &["promenade", "promenade around"],
        definition: "Walk as a couple, side by side, around the set in the named \
            direction.",
    },
    GlossaryEntry {
        term: "Proper",
        aliases: &["proper"],
        definition: "A formation with all larks on one side of the set and all \
            robins on the other.",
    },
    GlossaryEntry {
        term: "Revolving door",
        aliases: &["revolving door"],
        definition: "A turn in which two dancers pass around each other and \
            continue in the same direction.",
    },
    GlossaryEntry {
        term: "Right and left through",
        aliases: &["right and left through", "right left through"],
        definition: "Two facing couples cross through each other with a courtesy \
            turn and end on opposite sides.",
    },
    GlossaryEntry {
        term: "Roll away",
        aliases: &["roll away"],
        definition: "One dancer rolls the other across the set while their joined \
            hands guide the turn.",
    },
    GlossaryEntry {
        term: "Rory O'More",
        aliases: &["rory o'more", "rory o more"],
        definition: "A slide with a partner followed by a turn past the next \
            neighbor.",
    },
    GlossaryEntry {
        term: "Shadow",
        aliases: &["shadow", "shadows"],
        definition: "A dancer in your hands-four who is not your partner but takes \
            the partner role in a figure.",
    },
    GlossaryEntry {
        term: "Slide",
        aliases: &["slide"],
        definition: "Move sideways along the set while staying with your partner.",
    },
    GlossaryEntry {
        term: "Square through",
        aliases: &["square through"],
        definition: "A sequence of right- and left-hand pull-bys around a square.",
    },
    GlossaryEntry {
        term: "Star",
        aliases: &["star"],
        definition: "Four or more dancers join hands in the center and walk around \
            in the named direction.",
    },
    GlossaryEntry {
        term: "Star promenade",
        aliases: &["star promenade"],
        definition: "Form a star, then take a neighbor in promenade while the star \
            continues around.",
    },
    GlossaryEntry {
        term: "Swat the flea",
        aliases: &["swat the flea"],
        definition: "Two dancers take left hands, raise an arch, and trade places \
            under the joined hands.",
    },
    GlossaryEntry {
        term: "Swing",
        aliases: &["swing"],
        definition: "A partner turn in a close hold, usually with a buzz step; the \
            high point of many contras.",
    },
    GlossaryEntry {
        term: "Turn alone",
        aliases: &["turn alone"],
        definition: "Turn around in place without taking a partner.",
    },
    GlossaryEntry {
        term: "Up the hall",
        aliases: &["up the hall", "head up the hall"],
        definition: "Travel toward the top of the hall, where the band usually \
            plays.",
    },
];

/// A glossary term found in a longer string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlossaryMatch {
    /// Byte offset of the first character of the term.
    pub start: usize,
    /// Byte offset one past the last character of the term.
    pub end: usize,
    /// The entry the span belongs to.
    pub entry: &'static GlossaryEntry,
}

/// Look up an entry by its canonical term or any alias.
pub fn find(term: &str) -> Option<&'static GlossaryEntry> {
    ENTRIES.iter().find(|entry| {
        entry.term.eq_ignore_ascii_case(term)
            || entry
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(term))
    })
}

/// Find every glossary term in `text`, preferring the longest match.
///
/// Matches begin and end on a word boundary, are case-insensitive, and never
/// overlap. A phrase such as "balance and swing" is therefore one link rather
/// than two, and a word such as "swinging" is not a swing. The input is ordinary
/// card or call text, so the scan is deliberately small and allocation-free
/// apart from the returned vector.
pub fn scan(text: &str) -> Vec<GlossaryMatch> {
    let mut matches = Vec::new();
    let mut index = 0;
    let mut previous: Option<char> = None;

    while index < text.len() {
        let Some(ch) = text[index..].chars().next() else {
            break;
        };
        if !previous.is_some_and(is_word_char)
            && let Some((entry, length)) = longest_match_at(text, index)
        {
            matches.push(GlossaryMatch {
                start: index,
                end: index + length,
                entry,
            });
            previous = text[index..index + length].chars().next_back();
            index += length;
            continue;
        }
        previous = Some(ch);
        index += ch.len_utf8();
    }

    matches
}

/// The longest alias that starts at `start` and ends on a word boundary.
fn longest_match_at(text: &str, start: usize) -> Option<(&'static GlossaryEntry, usize)> {
    let mut best: Option<(&'static GlossaryEntry, usize)> = None;
    for entry in ENTRIES {
        for alias in entry.aliases {
            let Some(end) = start.checked_add(alias.len()) else {
                continue;
            };
            let Some(candidate) = text.get(start..end) else {
                continue;
            };
            if !candidate.eq_ignore_ascii_case(alias) {
                continue;
            }
            if text[end..].chars().next().is_some_and(is_word_char) {
                continue;
            }
            if best.is_none_or(|(_, length)| alias.len() > length) {
                best = Some((entry, alias.len()));
            }
        }
    }
    best
}

/// A character that continues a word for boundary purposes.
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(text: &str) -> Vec<&'static str> {
        scan(text)
            .into_iter()
            .map(|found| found.entry.term)
            .collect()
    }

    #[test]
    fn finds_the_longest_phrase_at_a_position() {
        assert_eq!(terms("balance and swing"), ["Balance and swing"]);
        assert_eq!(terms("star promenade"), ["Star promenade"]);
        assert_eq!(terms("right and left through"), ["Right and left through"]);
    }

    #[test]
    fn matches_case_insensitively_and_allows_punctuation_before_a_term() {
        assert_eq!(
            terms("Larks Allemande left 1½"),
            ["Lark / robin", "Allemande"]
        );
        assert_eq!(terms("1½ SWING"), ["Swing"]);
    }

    #[test]
    fn does_not_match_inside_a_longer_word() {
        assert!(terms("swinging").is_empty());
        assert_eq!(terms("women"), ["Lady / gentleman"]);
    }

    #[test]
    fn matches_aliases_to_the_canonical_entry() {
        assert_eq!(terms("dosido"), ["Do-si-do"]);
        assert_eq!(terms("right shoulder round"), ["Gyre / gypsy"]);
        assert_eq!(terms("back to back"), ["Do-si-do"]);
    }

    #[test]
    fn spans_cover_the_matched_text() {
        let text = "neighbors swing";
        let found = scan(text);
        assert_eq!(found.len(), 2);
        assert_eq!(&text[found[0].start..found[0].end], "neighbors");
        assert_eq!(&text[found[1].start..found[1].end], "swing");
    }

    #[test]
    fn every_entry_is_findable_by_its_canonical_term() {
        for entry in ENTRIES {
            assert_eq!(find(entry.term).map(|found| found.term), Some(entry.term));
        }
    }

    #[test]
    fn every_alias_is_ascii_so_byte_lengths_match_the_scan() {
        for entry in ENTRIES {
            assert!(entry.term.is_ascii(), "{} is not ASCII", entry.term);
            for alias in entry.aliases {
                assert!(alias.is_ascii(), "{alias} is not ASCII");
                assert!(!alias.is_empty(), "{} has an empty alias", entry.term);
            }
        }
    }
}
