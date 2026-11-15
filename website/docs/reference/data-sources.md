---
title: Data sources
description: Where dances and tunes come from, how the app fetches and caches them, and the attribution rules.
sidebar_position: 2
---

Contra Karaoke separates source parsing from the session engine. The parsers
implement generic provider traits and do not leak ContraDB's HTML or ABC
Session Tunes' JSON shapes into `core` or `music`. The current picker UI is
still hand-written for these two providers rather than generated from
`FilterSpec`.

## ContraDB

- Base: `https://contradb.com`
- Search: `POST /api/v1/dances` with count/offset/sort/filter JSON.
- Detail: `GET /dances/{id}` with an HTML card.
- The app parses the title, choreographer, formation, preamble, figures, and
  notes, then normalises entities, fractions, and emphasis.

The search UI exposes title, choreographer, user, hook, formation, figure type,
figure text, verified, and sort. **Figure type** sends a move-type filter;
**Figure contains** sends a free-text figure clause. If both are set, the
move-type filter wins and the free-text clause is omitted. The search UI lives
under **Select dance → Download dance**; **Select dance** itself lists saved
dances.

The preview includes an **Open on ContraDB** link to the source card.

The default dance is **Another Last Dance** (ContraDB 2409), choreographed by
Erik Hoffman.

## ABC Session Tunes

- Base: `https://backend.abcsessiontunes.com`
- Search: `GET /api/tunes` with title/composer/key/meter/rhythm/page/limit.
- Detail: `GET /api/tunes/{id}` returning metadata plus `abc_content`,
  `source_url`, notes, and transcription credit.

The default tune is **Sandy Boys** (tune 75815). The app displays its credits and
an **Open on ABC Session Tunes** link in the tune preview.

## CORS and the web proxy

The desktop and Android builds fetch directly. The browser build cannot use the
upstream sites directly because of CORS, so the repository includes a small
Cloudflare Worker proxy in `proxy/`:

- only the two upstream hosts and the specific API paths are allowlisted;
- the worker may require a shared token;
- the app stores the proxy URL and token in preferences.

A web build without a proxy still works with the bundled defaults and anything
already in the saved library.

## Saved library

Downloads are separate from the source cache:

- **Select dance** and **Select tune** read a local library of complete cards and
  tunes, with user folders and a local search.
- Native and Android builds keep `dances.json` and `tunes.json` in the app
  config directory; the browser uses `localStorage`.
- The bundled defaults are added on first launch.
- Downloading the same source again replaces its stored copy and keeps its
  folder.
- Clearing the source cache does not remove saved items.

## Caching

- Search responses: 7 days.
- Dance and tune details: 30 days.
- Cache keys include the provider ID and a hash of the query/detail key.
- Raw responses are cached; parsing runs again when a cached entry is read, so
  a failed request can fall back to stale bytes.
- Native and Android builds use files in the app cache directory; the browser
  uses `localStorage`.
- Error pages are not cached as valid content.

Cache clearing is manual. There is no cache-size display or refresh button in
0.1.0.

## Adding another source

The provider traits in `crates/sources` are:

```rust
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
}
```

`TuneProvider` mirrors it. Parsers are pure functions: bytes in, domain model
out. The intended provider boundary keeps source parsing out of `core` and
`music`. The current UI is still hard-wired to one dance provider and one tune
provider, so adding a source also means updating the provider selection in
`crates/app/src/source.rs` until the registry is wired through the UI.

## Attribution and redistribution

Dance transcriptions and ABC transcriptions belong to their contributors.
Contra Karaoke stores downloads locally for the user but does not ship a dance
or tune database. The two bundled defaults are included with source URLs and
credit. If you publish screenshots or example cards, keep the attribution.

The `samples/` directory in the repository contains parser fixtures from
third-party sources. It is not a documentation dataset; do not copy those cards
into the public docs.
