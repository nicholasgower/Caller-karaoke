//! Provider traits and source-neutral search types.

use contra_core::{Dance, Tune};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
}

impl HttpRequest {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            method: HttpMethod::Get,
            url: url.into(),
            headers: vec![("Accept".into(), "*/*".into())],
            body: None,
        }
    }

    pub fn post_json(url: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            method: HttpMethod::Post,
            url: url.into(),
            headers: vec![("Content-Type".into(), "application/json".into())],
            body: Some(body.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FilterSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FilterKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FilterKind {
    Text,
    Checkbox,
    Select(Vec<(&'static str, &'static str)>),
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DanceQuery {
    pub title: String,
    pub choreographer: String,
    pub user: String,
    pub hook: String,
    pub formation: Option<String>,
    pub verified: bool,
    pub figure_text: String,
    pub figure_move: Option<String>,
    pub sort: DanceSort,
    pub page: usize,
    pub page_size: usize,
}

impl DanceQuery {
    pub fn normalized(mut self) -> Self {
        self.page = self.page.max(1);
        self.page_size = self.page_size.clamp(1, 100);
        self.title = self.title.trim().to_string();
        self.choreographer = self.choreographer.trim().to_string();
        self.user = self.user.trim().to_string();
        self.hook = self.hook.trim().to_string();
        self.figure_text = self.figure_text.trim().to_string();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DanceSort {
    #[default]
    Relevancy,
    TitleAsc,
    TitleDesc,
    ChoreographerAsc,
    Newest,
}

impl DanceSort {
    pub fn label(self) -> &'static str {
        match self {
            Self::Relevancy => "Relevance",
            Self::TitleAsc => "Title A–Z",
            Self::TitleDesc => "Title Z–A",
            Self::ChoreographerAsc => "Choreographer A–Z",
            Self::Newest => "Newest",
        }
    }

    pub const ALL: [DanceSort; 5] = [
        DanceSort::Relevancy,
        DanceSort::TitleAsc,
        DanceSort::TitleDesc,
        DanceSort::ChoreographerAsc,
        DanceSort::Newest,
    ];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DanceSummary {
    pub id: String,
    pub title: String,
    pub choreographer: String,
    pub formation: String,
    pub hook: String,
    pub user: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DanceSearchPage {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub dances: Vec<DanceSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TuneQuery {
    pub search: String,
    pub composer: String,
    pub key: Option<String>,
    pub meter: Option<String>,
    pub rhythm: Option<String>,
    pub page: usize,
    pub page_size: usize,
}

impl TuneQuery {
    pub fn normalized(mut self) -> Self {
        self.page = self.page.max(1);
        self.page_size = self.page_size.clamp(1, 100);
        self.search = self.search.trim().to_string();
        self.composer = self.composer.trim().to_string();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TuneSummary {
    pub id: String,
    pub title: String,
    pub composer: String,
    pub key: String,
    pub meter: String,
    pub rhythm: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TuneSearchPage {
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub tunes: Vec<TuneSummary>,
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("could not parse source response: {0}")]
    Parse(String),
    #[error("unsupported source URL")]
    UnsupportedUrl,
    #[error("source returned no results")]
    Empty,
}

pub trait DanceProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn base_url(&self) -> &'static str;
    fn filter_specs(&self) -> Vec<FilterSpec>;
    fn search(&self, query: &DanceQuery) -> HttpRequest;
    fn parse_search(&self, body: &[u8], query: &DanceQuery)
    -> Result<DanceSearchPage, SourceError>;
    fn detail(&self, id: &str) -> HttpRequest;
    fn parse_dance(&self, body: &[u8], id: &str) -> Result<Dance, SourceError>;
    fn parse_url(&self, url: &str) -> Option<String>;

    /// Whether a second page of results can be requested.
    ///
    /// A source whose search returns one capped list (no offset parameter)
    /// answers `false`; the picker then hides **Previous**/**Next** and
    /// explains the cap instead of repeating the first page.
    fn supports_pagination(&self) -> bool {
        true
    }

    /// Whether the search has a sort control.
    fn supports_sort(&self) -> bool {
        true
    }

    /// Most rows one search can return when it cannot be paged.
    ///
    /// `None` means the source pages normally. A source with a capped list
    /// returns the cap so the screen can explain why some matches are missing.
    fn result_limit(&self) -> Option<usize> {
        None
    }
}

pub trait TuneProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn base_url(&self) -> &'static str;
    fn filter_specs(&self) -> Vec<FilterSpec>;
    fn search(&self, query: &TuneQuery) -> HttpRequest;
    fn parse_search(&self, body: &[u8], query: &TuneQuery) -> Result<TuneSearchPage, SourceError>;
    fn detail(&self, id: &str) -> HttpRequest;
    fn parse_tune(&self, body: &[u8], id: &str) -> Result<Tune, SourceError>;
    fn parse_url(&self, url: &str) -> Option<String>;
}

static DANCE_PROVIDERS: [&dyn DanceProvider; 2] =
    [&crate::contradb::CONTRADB, &crate::callersbox::CALLERS_BOX];
static TUNE_PROVIDERS: [&dyn TuneProvider; 1] = [&crate::abcsessiontunes::ABC_SESSION_TUNES];

/// The providers compiled into this build.
pub fn dance_providers() -> &'static [&'static dyn DanceProvider] {
    &DANCE_PROVIDERS
}

pub fn tune_providers() -> &'static [&'static dyn TuneProvider] {
    &TUNE_PROVIDERS
}

pub fn dance_provider(id: &str) -> Option<&'static dyn DanceProvider> {
    dance_providers()
        .iter()
        .copied()
        .find(|provider| provider.id() == id)
}

pub fn tune_provider(id: &str) -> Option<&'static dyn TuneProvider> {
    tune_providers()
        .iter()
        .copied()
        .find(|provider| provider.id() == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_both_dance_sources() {
        let ids: Vec<&str> = dance_providers()
            .iter()
            .map(|provider| provider.id())
            .collect();
        assert_eq!(ids, ["contradb", "callersbox"]);

        let callers_box = dance_provider("callersbox").expect("registered");
        assert_eq!(callers_box.display_name(), "The Caller's Box");
        assert!(
            callers_box
                .base_url()
                .starts_with("https://www.ibiblio.org/")
        );
        assert!(!callers_box.supports_pagination());
        assert!(!callers_box.supports_sort());

        let contradb = dance_provider("contradb").expect("registered");
        assert!(contradb.supports_pagination());
        assert!(contradb.supports_sort());
    }

    #[test]
    fn unknown_providers_are_not_registered() {
        assert!(dance_provider("").is_none());
        assert!(dance_provider("example").is_none());
    }
}
