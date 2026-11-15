//! ContraDB provider: JSON search, HTML dance-card parsing.

use std::sync::LazyLock;

use contra_core::{Dance, Figure, SourceRef};
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::json;

use crate::provider::{
    DanceProvider, DanceQuery, DanceSearchPage, DanceSort, DanceSummary, FilterKind, FilterSpec,
    HttpRequest, SourceError,
};

pub struct ContraDb;

pub static CONTRADB: ContraDb = ContraDb;

const BASE: &str = "https://contradb.com";

impl DanceProvider for ContraDb {
    fn id(&self) -> &'static str {
        "contradb"
    }

    fn display_name(&self) -> &'static str {
        "ContraDB"
    }

    fn base_url(&self) -> &'static str {
        BASE
    }

    fn filter_specs(&self) -> Vec<FilterSpec> {
        vec![
            FilterSpec {
                key: "title",
                label: "Title",
                kind: FilterKind::Text,
            },
            FilterSpec {
                key: "choreographer",
                label: "Choreographer",
                kind: FilterKind::Text,
            },
            FilterSpec {
                key: "user",
                label: "User",
                kind: FilterKind::Text,
            },
            FilterSpec {
                key: "hook",
                label: "Hook",
                kind: FilterKind::Text,
            },
            FilterSpec {
                key: "formation",
                label: "Formation",
                kind: FilterKind::Select(vec![
                    ("", "Any formation"),
                    ("improper", "Improper"),
                    ("becket", "Becket"),
                    ("proper", "Proper"),
                    ("everything else", "Everything else"),
                ]),
            },
            FilterSpec {
                key: "verified",
                label: "Verified only",
                kind: FilterKind::Checkbox,
            },
            FilterSpec {
                key: "figure_text",
                label: "Figure contains",
                kind: FilterKind::Text,
            },
            FilterSpec {
                key: "figure_move",
                label: "Figure",
                kind: FilterKind::Select(vec![
                    ("", "Any figure"),
                    ("swing", "Swing"),
                    ("allemande", "Allemande"),
                    ("do si do", "Do-si-do"),
                    ("balance", "Balance"),
                    ("chain", "Chain"),
                    ("circle", "Circle"),
                    ("star", "Star"),
                    ("hey", "Hey"),
                    ("gyre", "Gyre"),
                    ("promenade", "Promenade"),
                    ("long lines", "Long lines"),
                    ("right left through", "Right and left through"),
                    ("pass through", "Pass through"),
                    ("petronella", "Petronella"),
                    ("mad robin", "Mad robin"),
                    ("box the gnat", "Box the gnat"),
                    ("swat the flea", "Swat the flea"),
                    ("roll away", "Roll away"),
                    ("slide", "Slide"),
                ]),
            },
        ]
    }

    fn search(&self, query: &DanceQuery) -> HttpRequest {
        let query = query.clone().normalized();
        let page_size = query.page_size.max(1);
        let offset = (query.page.max(1) - 1) * page_size;

        let mut clauses = Vec::new();
        if !query.title.is_empty() {
            clauses.push(json!(["title", query.title]));
        }
        if !query.choreographer.is_empty() {
            clauses.push(json!(["choreographer", query.choreographer]));
        }
        if !query.user.is_empty() {
            clauses.push(json!(["user", query.user]));
        }
        if !query.hook.is_empty() {
            clauses.push(json!(["hook", query.hook]));
        }
        if let Some(formation) = query.formation.as_deref().filter(|value| !value.is_empty()) {
            clauses.push(json!(["formation", formation]));
        }
        if query.verified {
            clauses.push(json!([
                "compare",
                ["constant", 0],
                "<",
                ["tag", "verified"]
            ]));
        }
        let has_figure = if let Some(figure_move) = query
            .figure_move
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            clauses.push(json!(["figure", figure_move]));
            true
        } else if !query.figure_text.is_empty() {
            clauses.push(json!(["figure", "custom", query.figure_text]));
            true
        } else {
            false
        };
        if !has_figure {
            clauses.push(json!(["figure", "*"]));
        }

        let filter = if clauses.len() == 1 {
            clauses.remove(0)
        } else {
            json!(["and", clauses])
        };

        let sort_by = match query.sort {
            DanceSort::Relevancy => "",
            DanceSort::TitleAsc => "titleA",
            DanceSort::TitleDesc => "titleD",
            DanceSort::ChoreographerAsc => "choreographer_nameA",
            DanceSort::Newest => "created_atD",
        };

        let body = json!({
            "count": page_size,
            "offset": offset,
            "sort_by": sort_by,
            "filter": filter,
        });
        HttpRequest::post_json(
            format!("{BASE}/api/v1/dances"),
            serde_json::to_vec(&body).expect("search body is serializable"),
        )
    }

    fn parse_search(
        &self,
        body: &[u8],
        query: &DanceQuery,
    ) -> Result<DanceSearchPage, SourceError> {
        let response: SearchResponse = serde_json::from_slice(body)
            .map_err(|error| SourceError::Parse(format!("ContraDB search: {error}")))?;
        let dances = response
            .dances
            .into_iter()
            .map(|dance| DanceSummary {
                id: dance.id.to_string(),
                title: dance.title,
                choreographer: dance.choreographer_name,
                formation: dance.formation,
                hook: dance.hook,
                user: dance.user_name,
            })
            .collect();
        Ok(DanceSearchPage {
            total: response.number_matching,
            page: query.page.max(1),
            page_size: query.page_size.max(1),
            dances,
        })
    }

    fn detail(&self, id: &str) -> HttpRequest {
        HttpRequest::get(format!("{BASE}/dances/{id}"))
    }

    fn parse_dance(&self, body: &[u8], id: &str) -> Result<Dance, SourceError> {
        let html = std::str::from_utf8(body)
            .map_err(|error| SourceError::Parse(format!("ContraDB HTML is not UTF-8: {error}")))?;
        let document = Html::parse_document(html);

        let title = text_of(&document, &TITLE_SELECTOR)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Untitled dance".into());
        let choreographer = text_of(&document, &CHOREOGRAPHER_SELECTOR)
            .map(|value| strip_prefix_case_insensitive(&value, "by:"))
            .unwrap_or_default();
        let formation = text_of(&document, &FORMATION_SELECTOR)
            .map(|value| strip_prefix_case_insensitive(&value, "formation:"))
            .unwrap_or_default();
        let preamble = text_of(&document, &PREAMBLE_SELECTOR).unwrap_or_default();
        let notes = text_of(&document, &NOTES_SELECTOR).unwrap_or_default();

        let mut figures = Vec::new();
        for row in document.select(&ROW_SELECTOR) {
            let Some(figure_element) = row.select(&FIGURE_SELECTOR).next() else {
                continue;
            };
            let figure_text = clean_text(&figure_element.text().collect::<Vec<_>>().join(" "));
            if figure_text.is_empty() {
                continue;
            }
            let beats = row
                .select(&BEATS_SELECTOR)
                .next()
                .and_then(|element| {
                    clean_text(&element.text().collect::<Vec<_>>().join(" "))
                        .parse::<f64>()
                        .ok()
                })
                .filter(|beats| *beats > 0.0)
                .unwrap_or(8.0);
            let phrase = row
                .select(&TD_SELECTOR)
                .next()
                .map(|element| clean_text(&element.text().collect::<Vec<_>>().join(" ")))
                .filter(|value| !value.is_empty());
            figures.push(Figure::new(phrase, beats, figure_text));
        }

        if figures.is_empty() {
            return Err(SourceError::Parse(
                "ContraDB page had no dance figures".into(),
            ));
        }

        Ok(Dance {
            source: SourceRef::new("contradb", id, format!("{BASE}/dances/{id}")),
            title,
            choreographer,
            formation,
            preamble,
            notes,
            figures,
        })
    }

    fn parse_url(&self, url: &str) -> Option<String> {
        let rest = url.split("/dances/").nth(1)?;
        let id = rest
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .trim();
        if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
            Some(id.to_string())
        } else {
            None
        }
    }
}

static TITLE_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("h1.dance-show-title").expect("title selector"));
static CHOREOGRAPHER_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("p.dance-show-choreographer").expect("choreographer selector")
});
static FORMATION_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("p.dance-show-formation").expect("formation selector"));
static PREAMBLE_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.dance-show-preamble").expect("preamble selector"));
static NOTES_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.dance-show-notes").expect("notes selector"));
static ROW_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("table tr").expect("row selector"));
static BEATS_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td.dance-show-beats").expect("beats selector"));
static FIGURE_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.show-figure").expect("figure selector"));
static TD_SELECTOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("td").expect("td selector"));

fn text_of(document: &Html, selector: &Selector) -> Option<String> {
    document
        .select(selector)
        .next()
        .map(|element| clean_text(&element.text().collect::<Vec<_>>().join(" ")))
}

fn clean_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_prefix_case_insensitive(text: &str, prefix: &str) -> String {
    let trimmed = text.trim();
    if trimmed
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
    {
        trimmed.get(prefix.len()..).unwrap_or("").trim().to_string()
    } else {
        trimmed.to_string()
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(rename = "numberMatching", default)]
    number_matching: usize,
    #[serde(default)]
    dances: Vec<SearchDance>,
}

#[derive(Debug, Deserialize)]
struct SearchDance {
    id: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    choreographer_name: String,
    #[serde(default)]
    formation: String,
    #[serde(default)]
    hook: String,
    #[serde(default)]
    user_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const DANCE_HTML: &str = r#"
        <h1 class="dance-show-title">Another Last Dance</h1>
        <p class="dance-show-choreographer">by: <a>Erik Hoffman</a></p>
        <p class="dance-show-formation">formation: improper</p>
        <div class="dance-show-preamble"></div>
        <table>
          <tr><td>A1</td><td class="dance-show-beats">8</td><td><div class="show-figure">neighbors do si do once</div></td></tr>
          <tr><td></td><td class="dance-show-beats">8</td><td><div class="show-figure">ladles allemande left 1½</div></td></tr>
        </table>
        <div class="dance-show-notes">A note</div>
    "#;

    #[test]
    fn parses_dance_page() {
        let dance = CONTRADB.parse_dance(DANCE_HTML.as_bytes(), "2409").unwrap();
        assert_eq!(dance.title, "Another Last Dance");
        assert_eq!(dance.choreographer, "Erik Hoffman");
        assert_eq!(dance.formation, "improper");
        assert_eq!(dance.figures.len(), 2);
        assert_eq!(dance.figures[0].beats, 8.0);
        assert_eq!(dance.figures[0].phrase.as_deref(), Some("A1"));
        assert!(dance.figures[1].text.contains("allemande"));
        assert_eq!(dance.notes, "A note");
    }

    #[test]
    fn parses_search_response() {
        let json = br#"{
          "numberSearched": 2474,
          "numberMatching": 1,
          "dances": [{
            "id": 2409,
            "title": "Another Last Dance",
            "choreographer_name": "Erik Hoffman",
            "formation": "improper",
            "hook": "",
            "user_name": "Koren"
          }]
        }"#;
        let query = DanceQuery::default();
        let page = CONTRADB.parse_search(json, &query).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.dances[0].id, "2409");
        assert_eq!(page.dances[0].choreographer, "Erik Hoffman");
    }

    #[test]
    fn builds_search_filter() {
        let query = DanceQuery {
            title: "last".into(),
            formation: Some("improper".into()),
            figure_text: "swing".into(),
            ..DanceQuery::default()
        };
        let request = CONTRADB.search(&query);
        assert_eq!(request.method, crate::provider::HttpMethod::Post);
        let body: Value = serde_json::from_slice(request.body.as_ref().unwrap()).unwrap();
        let filter = &body["filter"];
        let text = filter.to_string();
        assert!(text.contains("title"));
        assert!(text.contains("improper"));
        assert!(text.contains("custom"));
    }

    #[test]
    fn parses_urls() {
        assert_eq!(
            CONTRADB.parse_url("https://contradb.com/dances/2409"),
            Some("2409".into())
        );
        assert_eq!(
            CONTRADB.parse_url("https://contradb.com/dances/2409?foo=bar"),
            Some("2409".into())
        );
        assert_eq!(
            CONTRADB.parse_url("https://example.com/dances/2409"),
            Some("2409".into())
        );
        assert_eq!(
            CONTRADB.parse_url("https://contradb.com/dances/not-a-number"),
            None
        );
    }
}
