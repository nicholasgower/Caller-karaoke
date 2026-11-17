//! The Caller's Box provider: HTML search, JSON dance records.
//!
//! The Caller's Box is a contra dance database maintained by Chris Page and
//! Michael Dyck. Its search form returns an HTML result table, and each result
//! has a JSON record at `dance.php?id=…&format=JSON`. Only records whose
//! `Permission` allows the figures are usable: the site has several permission
//! levels, and a record without figures is a parse error here rather than an
//! empty card.

use std::sync::LazyLock;

use contra_core::{Dance, Figure, SourceRef};
use scraper::{Html, Selector};
use serde::Deserialize;
use url::Url;

use crate::provider::{
    DanceProvider, DanceQuery, DanceSearchPage, DanceSummary, FilterKind, FilterSpec, HttpRequest,
    SourceError,
};

pub struct CallersBox;

pub static CALLERS_BOX: CallersBox = CallersBox;

const BASE: &str = "https://www.ibiblio.org/contradance/thecallersbox/";

/// Identifies this app to the site, as its operators ask of automated clients.
const USER_AGENT: &str = "ContraKaraoke/0.1 (+https://github.com/nicholasgower/Contra-karaoke)";

/// Formation values the site's search form sends. The field is an exact match,
/// so the picker offers the site's own list instead of a free-text value.
const FORMATIONS: &[&str] = &[
    "Duple Minor",
    "Duple Minor - Improper",
    "Duple Minor - Becket",
    "Duple Minor - Proper",
    "Duple Minor - Indecent",
    "Duple Minor - Reverse progression improper",
    "Duple Minor - Progressed improper",
    "Duple Minor - Cross",
    "Duple Minor - Other",
    "Triple Minor",
    "Three Facing Three",
    "Four Facing Four",
    "Solo",
    "Singlet",
    "Doublet",
    "Triplet",
    "Quadruplet",
    "Longways: 5+ couples",
    "Other Longways",
    "Circle Mixer",
    "Circle of Threesomes",
    "Sicilian Circle",
    "Scatter Mixer",
    "Grid Contra",
    "Grid Square",
    "Zia",
    "other",
];

impl DanceProvider for CallersBox {
    fn id(&self) -> &'static str {
        "callersbox"
    }

    fn display_name(&self) -> &'static str {
        "The Caller's Box"
    }

    fn base_url(&self) -> &'static str {
        BASE
    }

    fn filter_specs(&self) -> Vec<FilterSpec> {
        let mut formations = Vec::with_capacity(FORMATIONS.len() + 1);
        formations.push(("", "Any formation"));
        formations.extend(FORMATIONS.iter().map(|name| {
            // Keep the exact site value; shorten only the menu label so the
            // longest option still fits a phone-width combo.
            let label = match *name {
                "Duple Minor - Reverse progression improper" => {
                    "Duple Minor - Reverse prog. improper"
                }
                other => other,
            };
            (*name, label)
        }));
        vec![
            FilterSpec {
                key: "title",
                label: "Title",
                kind: FilterKind::Text,
            },
            FilterSpec {
                // The site calls the dance author an author; the app's query
                // field keeps the source-neutral name.
                key: "choreographer",
                label: "Author",
                kind: FilterKind::Text,
            },
            FilterSpec {
                key: "formation",
                label: "Formation",
                kind: FilterKind::Select(formations),
            },
            FilterSpec {
                key: "figure_text",
                label: "Figure contains",
                kind: FilterKind::Text,
            },
        ]
    }

    /// The site has no offset parameter and shows the first 50 matches, so a
    /// second page cannot be requested. The screen explains this instead of
    /// offering a Next button that would repeat the same results.
    fn supports_pagination(&self) -> bool {
        false
    }

    /// The search form has no sort control.
    fn supports_sort(&self) -> bool {
        false
    }

    /// Without `show_all`, the site shows the first 50 matches.
    fn result_limit(&self) -> Option<usize> {
        Some(50)
    }

    fn search(&self, query: &DanceQuery) -> HttpRequest {
        let query = query.clone().normalized();
        let mut params: Vec<(&str, String)> = Vec::new();
        if !query.title.is_empty() {
            params.push(("title", query.title.clone()));
        }
        if !query.choreographer.is_empty() {
            params.push(("author", query.choreographer.clone()));
        }
        if let Some(formation) = query.formation.as_deref().filter(|value| !value.is_empty()) {
            params.push(("formation", formation.to_string()));
        }
        if !query.figure_text.is_empty() {
            params.push(("pos_lines", query.figure_text.clone()));
            // The form's default matching mode; a second mode would need its
            // own picker field.
            params.push(("pos_mode", "all_given".to_string()));
        }
        // `show_all` is deliberately not sent: without it the site caps the
        // response at 50 rows, which keeps a broad query cheap to fetch.

        let mut url = Url::parse(BASE).expect("the Caller's Box base URL is valid");
        if !params.is_empty() {
            url.query_pairs_mut().extend_pairs(params);
        }
        with_user_agent(HttpRequest::get(url.to_string()))
    }

    fn parse_search(
        &self,
        body: &[u8],
        _query: &DanceQuery,
    ) -> Result<DanceSearchPage, SourceError> {
        let html = decode_body(body);
        let document = Html::parse_document(&html);
        // A real result page always states the match count, even when it is
        // zero. Requiring it keeps an unrelated or error page from being
        // treated (and cached) as a valid empty search.
        let Some(total) = parse_total(&document) else {
            return Err(SourceError::Parse(
                "The Caller's Box response was not a search page".into(),
            ));
        };

        let mut dances = Vec::new();
        for row in document.select(&ROW_SELECTOR) {
            let cells: Vec<_> = row.select(&TD_SELECTOR).collect();
            let Some((index, cell)) = cells
                .iter()
                .enumerate()
                .find(|(_, cell)| cell.select(&DANCE_LINK_SELECTOR).next().is_some())
            else {
                continue;
            };
            let Some(link) = cell.select(&DANCE_LINK_SELECTOR).next() else {
                continue;
            };
            let href = link.value().attr("href").unwrap_or_default();
            let Some(id) = self.parse_url(href) else {
                continue;
            };
            let title = clean_text(&link.text().collect::<Vec<_>>().join(" "));
            if title.is_empty() {
                continue;
            }
            let author = cells
                .get(index + 1)
                .map(|cell| clean_text(&cell.text().collect::<Vec<_>>().join(" ")))
                .unwrap_or_default();
            let formation = cells
                .get(index + 2)
                .map(|cell| clean_text(&cell.text().collect::<Vec<_>>().join(" ")))
                .unwrap_or_default();
            dances.push(DanceSummary {
                id,
                title,
                choreographer: author,
                formation,
                hook: String::new(),
                user: String::new(),
            });
        }

        if total > 0 && dances.is_empty() {
            return Err(SourceError::Parse(
                "The Caller's Box search page had no result rows".into(),
            ));
        }

        Ok(DanceSearchPage {
            total,
            page: 1,
            page_size: dances.len().max(1),
            dances,
        })
    }

    fn detail(&self, id: &str) -> HttpRequest {
        with_user_agent(HttpRequest::get(format!(
            "{BASE}dance.php?id={id}&format=JSON"
        )))
    }

    fn parse_dance(&self, body: &[u8], id: &str) -> Result<Dance, SourceError> {
        let record: DanceRecord = serde_json::from_slice(body)
            .map_err(|error| SourceError::Parse(format!("The Caller's Box record: {error}")))?;

        if !record.permission.eq_ignore_ascii_case("full") {
            return Err(SourceError::Parse(format!(
                "The Caller's Box does not permit showing this dance's figures \
                 (permission: {})",
                empty_label(&record.permission, "not stated")
            )));
        }

        let mut figures = Vec::new();
        for phrase in &record.phrases {
            for line in &phrase.figures {
                let (beats, text) = split_figure(line);
                if text.is_empty() {
                    continue;
                }
                figures.push(Figure::new(
                    phrase.name.as_deref().map(str::to_string),
                    beats,
                    text,
                ));
            }
        }
        if figures.is_empty() {
            return Err(SourceError::Parse(
                "The Caller's Box record had no dance figures".into(),
            ));
        }

        let title = clean_text(&record.name);
        let title = if title.is_empty() {
            "Untitled dance".to_string()
        } else {
            title
        };
        let choreographer = record.authors.join(", ");
        let formation = match (
            clean_text(&record.formation_base),
            clean_text(&record.formation_detail),
        ) {
            (base, detail) if !detail.is_empty() && !base.is_empty() => {
                format!("{base} ({detail})")
            }
            (base, detail) => {
                if detail.is_empty() {
                    base
                } else {
                    detail
                }
            }
        };
        let mut preamble = Vec::new();
        if !record.progression.trim().is_empty() {
            preamble.push(format!("Progression: {}", record.progression.trim()));
        }
        if !record.direction.trim().is_empty() {
            preamble.push(format!("Direction: {}", record.direction.trim()));
        }

        Ok(Dance {
            source: SourceRef::new("callersbox", id, format!("{BASE}dance.php?id={id}")),
            title,
            choreographer,
            formation,
            preamble: preamble.join(" · "),
            notes: record
                .calling_notes
                .iter()
                .map(|note| clean_text(note))
                .filter(|note| !note.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
            figures,
        })
    }

    fn parse_url(&self, url: &str) -> Option<String> {
        let (_, query) = url.split_once("dance.php?")?;
        let query = query.split('#').next().unwrap_or(query);
        for pair in query.split('&') {
            if let Some(value) = pair.strip_prefix("id=") {
                let id = value.trim();
                if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                    return Some(id.to_string());
                }
            }
        }
        None
    }
}

/// Add the descriptive User-Agent the site asks automated clients to send.
fn with_user_agent(mut request: HttpRequest) -> HttpRequest {
    request
        .headers
        .push(("User-Agent".into(), USER_AGENT.into()));
    request
}

static ROW_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("table tr").expect("row selector"));
static TD_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td").expect("cell selector"));
static DANCE_LINK_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("a[href*='dance.php?id=']").expect("dance link selector"));

/// Read the `your query matches N.` line from a result page.
fn parse_total(document: &Html) -> Option<usize> {
    let text = document.root_element().text().collect::<Vec<_>>().join(" ");
    let marker = "your query matches";
    let rest = text.get(text.find(marker)? + marker.len()..)?.trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Split the site's `(8) Men allemande left 1 & 1/2` notation into beats and
/// call text. A line without a parenthesised count keeps the default of eight
/// beats rather than failing the whole record.
fn split_figure(line: &str) -> (f64, String) {
    let text = clean_text(line);
    if let Some(rest) = text.strip_prefix('(')
        && let Some(end) = rest.find(')')
        && let Ok(beats) = rest[..end].trim().parse::<f64>()
        && beats.is_finite()
        && beats > 0.0
    {
        return (beats, rest[end + 1..].trim().to_string());
    }
    (8.0, text)
}

fn clean_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn empty_label<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    if value.trim().is_empty() {
        fallback
    } else {
        value
    }
}

/// Decode a response. The site declares `windows-1252`, so a page with an
/// accented author name is not valid UTF-8; fall back to cp1252 rather than
/// rejecting the whole response.
fn decode_body(body: &[u8]) -> String {
    match std::str::from_utf8(body) {
        Ok(text) => text.to_owned(),
        Err(_) => body.iter().map(|&byte| decode_cp1252(byte)).collect(),
    }
}

fn decode_cp1252(byte: u8) -> char {
    const HIGH: [char; 32] = [
        '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}',
        '\u{2021}', '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}',
        '\u{017D}', '\u{008F}', '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}',
        '\u{2022}', '\u{2013}', '\u{2014}', '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}',
        '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
    ];
    match byte {
        0x80..=0x9F => HIGH[usize::from(byte - 0x80)],
        other => char::from(other),
    }
}

#[derive(Debug, Deserialize)]
struct DanceRecord {
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "Authors", default)]
    authors: Vec<String>,
    #[serde(rename = "FormationBase", default)]
    formation_base: String,
    #[serde(rename = "FormationDetail", default)]
    formation_detail: String,
    #[serde(rename = "Progression", default)]
    progression: String,
    #[serde(rename = "Direction", default)]
    direction: String,
    #[serde(rename = "Permission", default)]
    permission: String,
    #[serde(default)]
    phrases: Vec<Phrase>,
    #[serde(rename = "CallingNotes", default)]
    calling_notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Phrase {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    figures: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_HTML: &str = include_str!("../../../fixtures/callersbox/search.html");
    const DANCE_JSON: &str = include_str!("../../../fixtures/callersbox/dance-13997.json");

    #[test]
    fn parses_a_trimmed_search_page() {
        let page = CALLERS_BOX
            .parse_search(SEARCH_HTML.as_bytes(), &DanceQuery::default())
            .expect("parse the fixture");
        assert_eq!(page.total, 83);
        assert_eq!(page.dances.len(), 2);
        assert_eq!(page.dances[0].id, "13997");
        assert_eq!(page.dances[0].title, "And Another Thing");
        assert_eq!(page.dances[0].choreographer, "Chris Page");
        assert_eq!(page.dances[0].formation, "Duple Minor - Becket");
        assert_eq!(page.dances[1].id, "12631");
        assert_eq!(page.dances[1].title, "Another Thing");
    }

    #[test]
    fn parses_a_dance_record_and_strips_beat_counts() {
        let dance = CALLERS_BOX
            .parse_dance(DANCE_JSON.as_bytes(), "13997")
            .expect("parse the fixture");
        assert_eq!(dance.source.provider, "callersbox");
        assert_eq!(dance.source.id, "13997");
        assert_eq!(
            dance.source.url,
            "https://www.ibiblio.org/contradance/thecallersbox/dance.php?id=13997"
        );
        assert_eq!(dance.title, "And Another Thing");
        assert_eq!(dance.choreographer, "Chris Page");
        assert_eq!(dance.formation, "Duple Minor - Becket");
        assert_eq!(dance.preamble, "Progression: Single · Direction: CW");
        assert_eq!(dance.figures.len(), 8);
        assert_eq!(dance.figures[0].phrase.as_deref(), Some("A1"));
        assert_eq!(dance.figures[0].beats, 8.0);
        assert_eq!(dance.figures[0].text, "Men allemande left 1 & 1/2");
        assert_eq!(dance.figures[6].beats, 6.0);
        assert_eq!(
            dance.figures[6].text,
            "N2 neighbor poussette clockwise 1/2 [around partner]"
        );
        assert_eq!(dance.figures[7].beats, 10.0);
        assert_eq!(dance.figures[7].phrase.as_deref(), Some("B2"));
        assert!(
            dance
                .figures
                .iter()
                .all(|figure| !figure.text.starts_with('('))
        );
        assert_eq!(dance.total_beats(), 64.0);
        assert!(dance.validate().is_ok());
    }

    #[test]
    fn refuses_records_without_permission() {
        let json = br#"{"Name":"Restricted","Permission":"none","phrases":[{"name":"A1","figures":["(8) Neighbor swing"]}]}"#;
        let error = CALLERS_BOX
            .parse_dance(json, "1")
            .expect_err("figures are not permitted");
        let message = error.to_string();
        assert!(message.contains("does not permit"), "{message}");
        assert!(message.contains("none"), "{message}");
    }

    #[test]
    fn a_permitted_record_without_figures_is_a_parse_error() {
        let json = br#"{"Name":"Empty","Permission":"full","phrases":[]}"#;
        let error = CALLERS_BOX
            .parse_dance(json, "2")
            .expect_err("there are no figures");
        assert!(error.to_string().contains("no dance figures"));
    }

    #[test]
    fn builds_an_encoded_search_url() {
        let query = DanceQuery {
            title: "and another thing".into(),
            choreographer: "Chris Page".into(),
            formation: Some("Duple Minor - Becket".into()),
            figure_text: "allemande left".into(),
            ..DanceQuery::default()
        };
        let request = CALLERS_BOX.search(&query);
        assert_eq!(request.method, crate::provider::HttpMethod::Get);
        assert!(
            request
                .url
                .starts_with("https://www.ibiblio.org/contradance/thecallersbox/?")
        );
        assert!(request.url.contains("title=and+another+thing"));
        assert!(request.url.contains("author=Chris+Page"));
        assert!(request.url.contains("formation=Duple+Minor+-+Becket"));
        assert!(request.url.contains("pos_lines=allemande+left"));
        assert!(request.url.contains("pos_mode=all_given"));
        assert!(!request.url.contains("show_all"));
        assert!(
            request
                .headers
                .iter()
                .any(|(name, value)| name == "User-Agent" && value.contains("ContraKaraoke"))
        );
    }

    #[test]
    fn an_empty_search_requests_the_plain_search_page() {
        let request = CALLERS_BOX.search(&DanceQuery::default());
        assert_eq!(request.url, BASE);
    }

    #[test]
    fn omits_empty_filters() {
        let request = CALLERS_BOX.search(&DanceQuery {
            title: "  ".into(),
            ..DanceQuery::default()
        });
        assert_eq!(request.url, BASE);
    }

    #[test]
    fn parses_dance_urls() {
        assert_eq!(
            CALLERS_BOX
                .parse_url("https://www.ibiblio.org/contradance/thecallersbox/dance.php?id=13997"),
            Some("13997".into())
        );
        assert_eq!(
            CALLERS_BOX.parse_url(
                "https://www.ibiblio.org/contradance/thecallersbox/dance.php?id=13997&format=JSON"
            ),
            Some("13997".into())
        );
        assert_eq!(
            CALLERS_BOX.parse_url("dance.php?id=13997"),
            Some("13997".into())
        );
        assert_eq!(
            CALLERS_BOX.parse_url("dance.php?format=JSON&id=42"),
            Some("42".into())
        );
        assert_eq!(CALLERS_BOX.parse_url("dance.php?id=abc"), None);
        assert_eq!(CALLERS_BOX.parse_url("https://example.com/"), None);
    }

    #[test]
    fn a_page_without_a_match_count_is_not_a_search_page() {
        let error = CALLERS_BOX
            .parse_search(
                b"<html><body>Database unavailable</body></html>",
                &DanceQuery::default(),
            )
            .expect_err("this is not a result page");
        assert!(error.to_string().contains("was not a search page"));
    }

    #[test]
    fn falls_back_to_cp1252_for_legacy_bytes() {
        // 0xE9 is `é` in windows-1252 and invalid on its own in UTF-8.
        assert_eq!(decode_body(b"Jos\xE9"), "José");
        assert_eq!(decode_body("José".as_bytes()), "José");
    }

    #[test]
    fn a_line_without_a_beat_count_keeps_the_default() {
        assert_eq!(
            split_figure("Neighbor swing"),
            (8.0, "Neighbor swing".into())
        );
        assert_eq!(split_figure("(x) Odd line"), (8.0, "(x) Odd line".into()));
    }
}
