//! Async source requests and cache integration.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use contra_core::{Dance, Tune};
use contra_sources::{
    Cache, DanceProvider, DanceQuery, DanceSearchPage, HttpMethod, HttpRequest, TuneProvider,
    TuneQuery, TuneSearchPage, dance_provider, tune_provider,
};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

const SEARCH_TTL: u64 = 7 * 24 * 60 * 60;
const DETAIL_TTL: u64 = 30 * 24 * 60 * 60;

pub enum SourceEvent {
    DanceSearch {
        /// Provider that answered, so a response for a source the user has
        /// since switched away from can be ignored.
        provider: String,
        query: DanceQuery,
        result: Result<DanceSearchPage, String>,
    },
    Dance {
        provider: String,
        result: Result<Dance, String>,
    },
    TuneSearch {
        query: TuneQuery,
        result: Result<TuneSearchPage, String>,
    },
    Tune {
        result: Result<Tune, String>,
    },
}

pub struct SourceClient {
    tx: std::sync::mpsc::Sender<SourceEvent>,
    cache: Arc<Mutex<Cache>>,
    proxy: Option<String>,
    proxy_token: Option<String>,
}

impl SourceClient {
    pub fn new(
        tx: std::sync::mpsc::Sender<SourceEvent>,
        cache: Cache,
        proxy: Option<String>,
        proxy_token: Option<String>,
    ) -> Self {
        Self {
            tx,
            cache: Arc::new(Mutex::new(cache)),
            proxy,
            proxy_token,
        }
    }

    pub fn search_dances(&self, provider_id: &str, query: DanceQuery) {
        let Some(provider) = dance_provider(provider_id) else {
            let _ = self.tx.send(SourceEvent::DanceSearch {
                provider: provider_id.to_string(),
                query,
                result: Err(format!("{provider_id} is not an available dance source")),
            });
            return;
        };
        let provider_id = provider.id();
        let request = provider.search(&query);
        let key = format!("dance_search/{provider_id}/{query:?}");
        if let Some(bytes) = self.cache_get(&key, SEARCH_TTL)
            && let Ok(page) = provider.parse_search(&bytes, &query)
        {
            let _ = self.tx.send(SourceEvent::DanceSearch {
                provider: provider_id.to_string(),
                query,
                result: Ok(page),
            });
            return;
        }
        let provider: &'static dyn DanceProvider = provider;
        let parse_query = query.clone();
        let event_provider = provider_id.to_string();
        self.fetch(
            key,
            request,
            SEARCH_TTL,
            move |body| {
                provider
                    .parse_search(body, &parse_query)
                    .map_err(|error| error.to_string())
            },
            move |result| SourceEvent::DanceSearch {
                provider: event_provider,
                query,
                result,
            },
        );
    }

    pub fn load_dance(&self, provider_id: &str, id: String) {
        let Some(provider) = dance_provider(provider_id) else {
            let _ = self.tx.send(SourceEvent::Dance {
                provider: provider_id.to_string(),
                result: Err(format!("{provider_id} is not an available dance source")),
            });
            return;
        };
        let provider_id = provider.id();
        let request = provider.detail(&id);
        let key = format!("dance/{provider_id}/{id}");
        if let Some(bytes) = self.cache_get(&key, DETAIL_TTL)
            && let Ok(dance) = provider.parse_dance(&bytes, &id)
        {
            let _ = self.tx.send(SourceEvent::Dance {
                provider: provider_id.to_string(),
                result: Ok(dance),
            });
            return;
        }
        let provider: &'static dyn DanceProvider = provider;
        let id_for_parse = id.clone();
        let event_provider = provider_id.to_string();
        self.fetch(
            key,
            request,
            DETAIL_TTL,
            move |body| {
                provider
                    .parse_dance(body, &id_for_parse)
                    .map_err(|error| error.to_string())
            },
            move |result| SourceEvent::Dance {
                provider: event_provider,
                result,
            },
        );
    }

    pub fn search_tunes(&self, query: TuneQuery) {
        let Some(provider) = tune_provider("abcsessiontunes") else {
            let _ = self.tx.send(SourceEvent::TuneSearch {
                query,
                result: Err("ABC Session Tunes provider is unavailable".into()),
            });
            return;
        };
        let request = provider.search(&query);
        let key = format!("tune_search/abcsessiontunes/{query:?}");
        if let Some(bytes) = self.cache_get(&key, SEARCH_TTL)
            && let Ok(page) = provider.parse_search(&bytes, &query)
        {
            let _ = self.tx.send(SourceEvent::TuneSearch {
                query,
                result: Ok(page),
            });
            return;
        }
        let provider: &'static dyn TuneProvider = provider;
        let parse_query = query.clone();
        self.fetch(
            key,
            request,
            SEARCH_TTL,
            move |body| {
                provider
                    .parse_search(body, &parse_query)
                    .map_err(|error| error.to_string())
            },
            move |result| SourceEvent::TuneSearch { query, result },
        );
    }

    pub fn load_tune(&self, id: String) {
        let Some(provider) = tune_provider("abcsessiontunes") else {
            let _ = self.tx.send(SourceEvent::Tune {
                result: Err("ABC Session Tunes provider is unavailable".into()),
            });
            return;
        };
        let request = provider.detail(&id);
        let key = format!("tune/abcsessiontunes/{id}");
        if let Some(bytes) = self.cache_get(&key, DETAIL_TTL)
            && let Ok(tune) = provider.parse_tune(&bytes, &id)
        {
            let _ = self.tx.send(SourceEvent::Tune { result: Ok(tune) });
            return;
        }
        let provider: &'static dyn TuneProvider = provider;
        let id_for_parse = id.clone();
        self.fetch(
            key,
            request,
            DETAIL_TTL,
            move |body| {
                provider
                    .parse_tune(body, &id_for_parse)
                    .map_err(|error| error.to_string())
            },
            move |result| SourceEvent::Tune { result },
        );
    }

    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.clear();
        }
    }

    pub fn set_proxy(&mut self, proxy: Option<String>, token: Option<String>) {
        self.proxy = proxy;
        self.proxy_token = token;
    }

    fn cache_get(&self, key: &str, ttl: u64) -> Option<Vec<u8>> {
        self.cache.lock().ok()?.get(key, ttl)
    }

    fn fetch<T, P, W>(&self, key: String, request: HttpRequest, _ttl: u64, parse: P, wrap: W)
    where
        T: Send + 'static,
        P: FnOnce(&[u8]) -> Result<T, String> + Send + 'static,
        W: FnOnce(Result<T, String>) -> SourceEvent + Send + 'static,
    {
        let tx = self.tx.clone();
        let cache = Arc::clone(&self.cache);
        let request = self.to_ehttp(request);
        ehttp::fetch(request, move |response| {
            let result = match response {
                Ok(response) if response.ok => {
                    // Parse before caching: an HTTP 200 error page must not
                    // become a cached "valid" response.
                    let parsed = parse(&response.bytes);
                    if parsed.is_ok()
                        && let Ok(mut cache) = cache.lock()
                    {
                        let _ = cache.put(&key, &response.bytes);
                    }
                    parsed
                }
                Ok(response) => {
                    let stale = cache.lock().ok().and_then(|cache| cache.get_stale(&key));
                    if let Some(bytes) = stale {
                        parse(&bytes)
                    } else {
                        Err(format!("HTTP {} {}", response.status, response.status_text))
                    }
                }
                Err(error) => {
                    let stale = cache.lock().ok().and_then(|cache| cache.get_stale(&key));
                    if let Some(bytes) = stale {
                        parse(&bytes)
                    } else {
                        Err(error.to_string())
                    }
                }
            };
            let _ = tx.send(wrap(result));
        });
    }

    fn to_ehttp(&self, request: HttpRequest) -> ehttp::Request {
        let url = if let Some(proxy) = &self.proxy {
            proxy_url(proxy, &request.url)
        } else {
            request.url.clone()
        };
        let mut ehttp_request = match request.method {
            HttpMethod::Get => ehttp::Request::get(url),
            HttpMethod::Post => ehttp::Request::post(url, request.body.unwrap_or_default()),
        };
        ehttp_request.headers = ehttp::Headers {
            headers: request.headers,
        };
        if let Some(token) = &self.proxy_token {
            ehttp_request.headers.insert("X-Proxy-Token", token.clone());
        }
        ehttp_request.timeout = Some(Duration::from_secs(25));
        ehttp_request
    }
}

fn proxy_url(proxy: &str, upstream: &str) -> String {
    let encoded = utf8_percent_encode(upstream, NON_ALPHANUMERIC).to_string();
    if proxy.contains("{url}") {
        proxy.replace("{url}", &encoded)
    } else {
        let separator = if proxy.contains('?') { '&' } else { '?' };
        format!("{proxy}{separator}url={encoded}")
    }
}
