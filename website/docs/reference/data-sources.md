---
title: Data sources
description: Where dances and tunes come from, how the app fetches and caches them, and the attribution rules.
sidebar_position: 3
---

Contra Karaoke separates source parsing from the session engine. The parsers
implement generic provider traits and do not leak a site's HTML or JSON shapes
into `core` or `music`. The download screen reads each provider's `FilterSpec`
list, so it shows the fields a source supports and hides the rest. Tune
downloads still come from ABC Session Tunes.

## Contra move glossary

The Caller's Box publishes a [contra dance move glossary and notation
reference](https://www.ibiblio.org/contradance/thecallersbox/Glossary.htm). It
explains the mechanics behind shorthand such as allemandes, gyres, heys, and
poussettes, and the notation used in a downloaded card, which helps when a
figure name is unfamiliar. The site also has a shorter [notation
glossary](https://www.ibiblio.org/contradance/thecallersbox/Brief-glossary.htm).
The glossary is a reading aid; the app never fetches or bundles it.

## Dance sources

The dance source is chosen at the top of the **Download dance** screen, beside
the search form it controls. (The old **Preferences → Data sources** menu is
gone; that page names the current source and links to it.) The selector shows
the source's address and result limit. Saved dances keep the source they came
from, so switching the menu changes where the next download
comes from without rewriting the library. The preview's **Open on …** link
always opens the card's own source.

### ContraDB

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

### The Caller's Box

- Base: `https://www.ibiblio.org/contradance/thecallersbox/`
- Search: `GET /` with `title`, `author`, `formation`, and figure text
  (`pos_lines` with `pos_mode=all_given`).
- Detail: `GET /dance.php?id={id}&format=JSON`.
- The site declares `windows-1252`; the parser falls back to that encoding when
  a response is not valid UTF-8.

The search form offers **Title**, **Author**, **Formation** (the site's own
formation list, matched exactly), and **Figure contains**. The site has no
user, hook, verified, or sort field, so those controls are hidden while The
Caller's Box is selected.

A search returns at most 50 matches and has no offset parameter. The screen
says so and the **Previous**/**Next** row is hidden; narrow the search with a
title, author, or figure phrase to reach matches beyond the first 50. The
result count still comes from the site's “your query matches N” line, so the
screen can show that more matches exist.

The JSON record provides `Name`, `Authors`, `FormationBase`,
`FormationDetail`, `Progression`, `Direction`, `Permission`, and `phrases`.
Every figure line begins with a beat count in parentheses, such as
`(8) Men allemande left 1 & 1/2`; the parser stores eight beats (or the given
count) and the call text without the prefix. `PhraseStructure` is ignored in
0.1.0, so an unusual phrase length is read from the figure counts.

The record is only usable when `Permission` is `full`, the site's level that
allows the figures to be shown. Any other level, or a record with no figures,
is reported as a parse error instead of producing an empty card. Downloaded
cards keep the canonical `dance.php?id=…` URL for attribution.

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

- only the upstream hosts and the specific API paths are allowlisted;
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
    // Optional capabilities with defaults:
    fn supports_pagination(&self) -> bool;
    fn supports_sort(&self) -> bool;
    fn result_limit(&self) -> Option<usize>;
}
```

`TuneProvider` mirrors it. Parsers are pure functions: bytes in, domain model
out. The intended provider boundary keeps source parsing out of `core` and
`music`. Dance providers advertise optional capabilities (`supports_pagination`,
`supports_sort`, `result_limit`) and the filters they understand, so the
download screen can adapt without naming a site. Tune downloads are still wired
to ABC Session Tunes.

## Attribution and redistribution

Dance transcriptions and ABC transcriptions belong to their contributors, and
The Caller's Box data belongs to its editors and the dance authors who gave
permission. Contra Karaoke stores downloads locally for the user but does not
ship a dance or tune database. The two bundled defaults are included with
source URLs and credit. If you publish screenshots or example cards, keep the
attribution.

The `samples/` directory in the repository contains parser fixtures from
third-party sources. It is not a documentation dataset; do not copy those cards
into the public docs.
