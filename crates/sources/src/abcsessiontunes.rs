//! ABC Session Tunes provider: JSON search and tune detail.

use contra_core::{SourceRef, Tune};
use serde::{Deserialize, Deserializer};
use url::Url;

use crate::provider::{
    FilterKind, FilterSpec, HttpRequest, SourceError, TuneProvider, TuneQuery, TuneSearchPage,
    TuneSummary,
};

pub struct AbcSessionTunes;

pub static ABC_SESSION_TUNES: AbcSessionTunes = AbcSessionTunes;

const BASE: &str = "https://backend.abcsessiontunes.com";
const SITE: &str = "https://abcsessiontunes.com";

impl TuneProvider for AbcSessionTunes {
    fn id(&self) -> &'static str {
        "abcsessiontunes"
    }

    fn display_name(&self) -> &'static str {
        "ABC Session Tunes"
    }

    fn base_url(&self) -> &'static str {
        SITE
    }

    fn filter_specs(&self) -> Vec<FilterSpec> {
        vec![
            FilterSpec {
                key: "search",
                label: "Search",
                kind: FilterKind::Text,
            },
            FilterSpec {
                key: "composer",
                label: "Composer",
                kind: FilterKind::Text,
            },
            FilterSpec {
                key: "key",
                label: "Key",
                kind: FilterKind::Select(vec![
                    ("", "Any key"),
                    ("D", "D major"),
                    ("G", "G major"),
                    ("A", "A major"),
                    ("C", "C major"),
                    ("F", "F major"),
                    ("Bb", "B♭ major"),
                    ("Eb", "E♭ major"),
                    ("Am", "A minor"),
                    ("Em", "E minor"),
                    ("Bm", "B minor"),
                    ("Dm", "D minor"),
                    ("Gm", "G minor"),
                    ("Ador", "A dorian"),
                    ("Amix", "A mixolydian"),
                    ("Edor", "E dorian"),
                    ("Emix", "E mixolydian"),
                ]),
            },
            FilterSpec {
                key: "meter",
                label: "Meter",
                kind: FilterKind::Select(vec![
                    ("", "Any meter"),
                    ("4/4", "4/4"),
                    ("2/2", "2/2"),
                    ("2/4", "2/4"),
                    ("6/8", "6/8"),
                    ("9/8", "9/8"),
                    ("12/8", "12/8"),
                    ("3/4", "3/4"),
                ]),
            },
            FilterSpec {
                key: "rhythm",
                label: "Genre / rhythm",
                kind: FilterKind::Select(vec![
                    ("", "Any genre"),
                    ("reel", "Reel"),
                    ("jig", "Jig"),
                    ("slip jig", "Slip jig"),
                    ("hornpipe", "Hornpipe"),
                    ("polka", "Polka"),
                    ("slide", "Slide"),
                    ("strathspey", "Strathspey"),
                    ("waltz", "Waltz"),
                ]),
            },
        ]
    }

    fn search(&self, query: &TuneQuery) -> HttpRequest {
        let query = query.clone().normalized();
        let mut url = Url::parse(&format!("{BASE}/api/tunes")).expect("base URL is valid");
        {
            let mut pairs = url.query_pairs_mut();
            if !query.search.is_empty() {
                pairs.append_pair("search", &query.search);
            }
            if !query.composer.is_empty() {
                pairs.append_pair("composer", &query.composer);
            }
            if let Some(key) = query.key.as_deref().filter(|value| !value.is_empty()) {
                pairs.append_pair("key_signature", key);
            }
            if let Some(meter) = query.meter.as_deref().filter(|value| !value.is_empty()) {
                pairs.append_pair("meter", meter);
            }
            if let Some(rhythm) = query.rhythm.as_deref().filter(|value| !value.is_empty()) {
                pairs.append_pair("rhythm", rhythm);
            }
            pairs.append_pair("page", &query.page.max(1).to_string());
            pairs.append_pair("limit", &query.page_size.max(1).to_string());
            pairs.append_pair("sort", "title");
            pairs.append_pair("dir", "asc");
            pairs.append_pair("fields", "summary");
            pairs.append_pair("envelope", "1");
        }
        HttpRequest::get(url.to_string())
    }

    fn parse_search(&self, body: &[u8], _query: &TuneQuery) -> Result<TuneSearchPage, SourceError> {
        let response: SearchResponse = serde_json::from_slice(body)
            .map_err(|error| SourceError::Parse(format!("ABC Session Tunes search: {error}")))?;
        let tunes = response
            .tunes
            .into_iter()
            .map(|tune| TuneSummary {
                id: tune.id.to_string(),
                title: tune.title,
                composer: tune.composer,
                key: tune.key_signature,
                meter: tune.meter,
                rhythm: tune.rhythm,
            })
            .collect();
        Ok(TuneSearchPage {
            total: response.pagination.total,
            page: response.pagination.page.max(1),
            page_size: response.pagination.limit.max(1),
            tunes,
        })
    }

    fn detail(&self, id: &str) -> HttpRequest {
        HttpRequest::get(format!("{BASE}/api/tunes/{id}"))
    }

    fn parse_tune(&self, body: &[u8], id: &str) -> Result<Tune, SourceError> {
        let response: TuneResponse = serde_json::from_slice(body)
            .map_err(|error| SourceError::Parse(format!("ABC Session Tunes detail: {error}")))?;
        if response.abc_content.trim().is_empty() {
            return Err(SourceError::Parse("tune has no ABC content".into()));
        }
        let source_url = response
            .source_url
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("{SITE}/tunes/{id}"));
        let mut credits = Vec::new();
        if let Some(transcriber) = response
            .transcribed_by
            .filter(|value| !value.trim().is_empty())
        {
            credits.push(format!("transcribed by {transcriber}"));
        }
        if let Some(notes) = response.notes.filter(|value| !value.trim().is_empty()) {
            credits.push(notes);
        }
        Ok(Tune {
            source: SourceRef::new("abcsessiontunes", id, source_url),
            title: response.title,
            composer: response.composer,
            key: response.key_signature,
            meter: response.meter,
            rhythm: response.rhythm,
            original_abc: Some(response.abc_content.clone()),
            abc: response.abc_content,
            credits: credits.join(" · "),
        })
    }

    fn parse_url(&self, url: &str) -> Option<String> {
        let rest = url.split("/tunes/").nth(1)?;
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

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    tunes: Vec<SearchTune>,
    #[serde(default)]
    pagination: Pagination,
}

#[derive(Debug, Deserialize, Default)]
struct Pagination {
    #[serde(default = "default_page")]
    page: usize,
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    total: usize,
}

fn default_page() -> usize {
    1
}
fn default_limit() -> usize {
    25
}

#[derive(Debug, Deserialize)]
struct SearchTune {
    id: u64,
    #[serde(default, deserialize_with = "null_as_default")]
    title: String,
    #[serde(default, deserialize_with = "null_as_default")]
    composer: String,
    #[serde(default, deserialize_with = "null_as_default")]
    key_signature: String,
    #[serde(default, deserialize_with = "null_as_default")]
    meter: String,
    #[serde(default, deserialize_with = "null_as_default")]
    rhythm: String,
}

#[derive(Debug, Deserialize)]
struct TuneResponse {
    #[serde(default, deserialize_with = "null_as_default")]
    title: String,
    #[serde(default, deserialize_with = "null_as_default")]
    composer: String,
    #[serde(default, deserialize_with = "null_as_default")]
    key_signature: String,
    #[serde(default, deserialize_with = "null_as_default")]
    meter: String,
    #[serde(default, deserialize_with = "null_as_default")]
    rhythm: String,
    #[serde(default, deserialize_with = "null_as_default")]
    abc_content: String,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    transcribed_by: Option<String>,
}

/// Treats an explicit `null` like a missing field.
///
/// The API stores unknown metadata as JSON `null`; `#[serde(default)]` alone
/// only covers absent keys, so one tune without a composer would otherwise fail
/// the whole search page or detail response.
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_response() {
        let body = br#"{
          "tunes": [{
            "id": 75815,
            "title": "Sandy Boys",
            "composer": "",
            "key_signature": "A mix",
            "meter": "4/4",
            "rhythm": "reel"
          }],
          "pagination": {"page": 1, "limit": 25, "total": 1, "pages": 1}
        }"#;
        let page = ABC_SESSION_TUNES
            .parse_search(body, &TuneQuery::default())
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.tunes[0].id, "75815");
        assert_eq!(page.tunes[0].key, "A mix");
    }

    #[test]
    fn parses_tune_detail() {
        let body = br#"{
          "id": 75815,
          "title": "Sandy Boys",
          "composer": "",
          "key_signature": "A mix",
          "meter": "4/4",
          "rhythm": "reel",
          "abc_content": "X:1\nT:Sandy Boys\nM:4/4\nL:1/8\nK:A mix\neg|\n",
          "source_url": "https://example.invalid/sandy",
          "notes": "A note",
          "transcribed_by": "A transcriber"
        }"#;
        let tune = ABC_SESSION_TUNES.parse_tune(body, "75815").unwrap();
        assert_eq!(tune.title, "Sandy Boys");
        assert!(tune.abc.contains("K:A mix"));
        assert!(tune.credits.contains("A transcriber"));
        assert_eq!(tune.source.url, "https://example.invalid/sandy");
    }

    #[test]
    fn parses_search_response_with_null_metadata() {
        // The live API returns JSON null for unknown composer/key/meter/rhythm.
        let body = br#"{
          "tunes": [{
            "id": 29920,
            "title": "10-Na",
            "composer": null,
            "key_signature": null,
            "meter": "9/8",
            "rhythm": null
          }],
          "pagination": {"page": 1, "limit": 25, "total": 1, "pages": 1}
        }"#;
        let page = ABC_SESSION_TUNES
            .parse_search(body, &TuneQuery::default())
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.tunes[0].title, "10-Na");
        assert_eq!(page.tunes[0].composer, "");
        assert_eq!(page.tunes[0].key, "");
        assert_eq!(page.tunes[0].meter, "9/8");
        assert_eq!(page.tunes[0].rhythm, "");
    }

    #[test]
    fn parses_tune_detail_with_null_metadata() {
        let body = br#"{
          "id": 29920,
          "title": "10-Na",
          "composer": null,
          "key_signature": null,
          "meter": null,
          "rhythm": null,
          "abc_content": "X:1\nT:10-Na\nK:Ador\n",
          "source_url": null,
          "notes": null,
          "transcribed_by": null
        }"#;
        let tune = ABC_SESSION_TUNES.parse_tune(body, "29920").unwrap();
        assert_eq!(tune.title, "10-Na");
        assert_eq!(tune.composer, "");
        assert_eq!(tune.key, "");
        assert_eq!(tune.meter, "");
        assert!(tune.abc.contains("K:Ador"));
        assert_eq!(tune.source.url, "https://abcsessiontunes.com/tunes/29920");
    }

    #[test]
    fn rejects_detail_without_abc_content() {
        let body = br#"{"id": 1, "title": "Empty", "abc_content": null}"#;
        let error = ABC_SESSION_TUNES.parse_tune(body, "1").unwrap_err();
        assert!(error.to_string().contains("no ABC content"));
    }

    #[test]
    fn builds_search_url() {
        let query = TuneQuery {
            search: "sandy boys".into(),
            key: Some("Amix".into()),
            meter: Some("4/4".into()),
            page: 2,
            page_size: 10,
            ..TuneQuery::default()
        };
        let request = ABC_SESSION_TUNES.search(&query);
        assert!(request.url.contains("search=sandy+boys"));
        assert!(request.url.contains("key_signature=Amix"));
        assert!(request.url.contains("page=2"));
        assert!(request.url.contains("envelope=1"));
    }

    #[test]
    fn parses_urls() {
        assert_eq!(
            ABC_SESSION_TUNES.parse_url("https://abcsessiontunes.com/tunes/75815"),
            Some("75815".into())
        );
        assert_eq!(
            ABC_SESSION_TUNES.parse_url("https://abcsessiontunes.com/tunes/75815?x=1"),
            Some("75815".into())
        );
        assert_eq!(
            ABC_SESSION_TUNES.parse_url("https://example.com/tunes/nope"),
            None
        );
    }
}
