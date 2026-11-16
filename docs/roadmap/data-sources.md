## 7. Data sources

### 7.1 ContraDB

Base: `https://contradb.com`

Search:

```
POST /api/v1/dances
Content-Type: application/json
{
  "count": 10,
  "offset": 0,
  "sort_by": "titleA",
  "filter": ["and", ["figure", "*"], ["formation", "improper"]]
}
```

Observed response:

```json
{
  "numberSearched": 2474,
  "numberMatching": 1234,
  "dances": [
    {
      "id": 3582,
      "title": "Let's Play Already",
      "choreographer_id": 465,
      "choreographer_name": "Michael Karcher",
      "formation": "improper",
      "hook": "",
      "user_id": 96,
      "user_name": "Sarah Trop",
      "created_at": "...",
      "updated_at": "...",
      "publish": "everywhere",
      "matching_figures_html": "whole dance"
    }
  ]
}
```

Filter expressions observed:

| Purpose | Expression |
| --- | --- |
| All figures | `["figure", "*"]` |
| Specific move | `["figure", "swing"]` |
| Figure keyword search | `["figure", "custom", "ladles chain"]` |
| Title contains | `["title", "last"]` |
| Choreographer contains | `["choreographer", "hoffman"]` |
| Formation | `["formation", "improper"]` |
| Public/shared | `["publish", "all"]` |
| Verified | `["tag", "verified"]` |
| Boolean combination | `["and", ...]`, `["or", ...]` |

Boolean operands are variadic and flat: `["and", clause, clause, ...]`. Nesting
them as `["and", [clause, clause]]` makes the endpoint answer HTTP 500.

Sort values use `<column><A|D>`, e.g. `titleA`, `titleD`.

Formation text is case-sensitive and validated server-side. The accepted values
are `improper`, `proper`, `everything else`, and `Becket *` (the wildcard covers
stored variants such as `Becket (ccw)`). An unrecognised value answers HTTP 500
rather than an empty page. Figure names are validated the same way (for example
`slide along set`, not `slide`).

Detail:

```
GET /dances/{id}
```

Parse the HTML:

- `h1.dance-show-title`
- `p.dance-show-choreographer`
- `p.dance-show-formation`
- `div.dance-show-preamble`
- `table tr` with `td.dance-show-beats` and `div.show-figure`
- `div.dance-show-notes`

Normalize entities, Unicode fractions, HTML emphasis, arrows, and progression
markers. Preserve the source URL for attribution.

Default dance: `https://contradb.com/dances/2409` — *Another Last Dance*.

### 7.2 The Caller's Box

Base: `https://www.ibiblio.org/contradance/thecallersbox/`

Search (GET form, one page of at most 50 rows):

```
GET /?title=another+thing
    &author=Chris+Page
    &formation=Duple+Minor+-+Becket
    &pos_lines=allemande+left
    &pos_mode=all_given
```

The form also has `progression`, negative figure text (`neg_lines`,
`neg_mode`), per-phrase fields (`phr1_…` to `phr4_…`), and `show_all`. The app
sends only `title`, `author`, `formation`, `pos_lines`, and `pos_mode`, and it
deliberately omits `show_all` so a broad query stays bounded. The page declares
`windows-1252`; decode it as such when the bytes are not valid UTF-8.

The results page contains:

```
Of 16874 dances in the db, your query matches 83.
<table>
<tr>
<td>…permission icon…</td><td>…source icon…</td><td>…video icon…</td>
<td><a href='dance.php?id=13997'>And Another Thing</a></td>
<td>Chris Page</td>
<td>Duple Minor - Becket</td>
</tr>
</table>
```

The dance link may be the fourth cell; find the cell that holds the link rather
than assuming a fixed column count, then read the next two cells as author and
formation. The site has no offset parameter: without `show_all` the first 50
matches are the whole response, so the UI hides paging and explains the cap.

Detail:

```
GET /dance.php?id=13997&format=JSON
```

Fields used:

| Field | Use |
| --- | --- |
| `Name` | Dance title |
| `Authors` | Choreographer, joined with commas |
| `FormationBase`, `FormationDetail` | Formation, detail in parentheses |
| `Progression`, `Direction` | Preamble summary |
| `Permission` | `full` is the only level that allows figures |
| `phrases[].name` | Figure phrase label (`A1`, `A2`, …) |
| `phrases[].figures` | `(8) Men allemande left 1 & 1/2` lines |
| `CallingNotes` | Notes, one per line |

Every figure line starts with a beat count in parentheses; strip it and store
the count. A record without `Permission: full`, or one with no figures, is a
parse error rather than an empty card. `PhraseStructure` is ignored in 0.1.0.

Canonical card URL for attribution: `https://www.ibiblio.org/contradance/thecallersbox/dance.php?id={id}`.

The site's [Glossary.htm](https://www.ibiblio.org/contradance/thecallersbox/Glossary.htm)
documents the move notation used in these cards and is the reference to link
when explaining a figure. It is fetched by the user, never bundled.

The dance source is a preference (`Preferences::dance_source`, default
`contradb`). Saved dances store their provider in `SourceRef`, so the library
and **Open on …** link follow the card's own source.

### 7.3 ABC Session Tunes

Base: `https://backend.abcsessiontunes.com`

Search:

```
GET /api/tunes
  ?search=sandy
  &composer=
  &key_signature=Amix
  &meter=4/4
  &rhythm=reel
  &page=1
  &limit=25
  &sort=title
  &dir=asc
  &fields=summary
  &envelope=1
```

Response:

```json
{
  "tunes": [
    {
      "id": 75815,
      "title": "Sandy Boys",
      "composer": "",
      "key_signature": "A mix",
      "meter": "4/4",
      "rhythm": "",
      "origin": null,
      "createdAt": "2025-06-25T06:29:13.000Z"
    }
  ],
  "pagination": { "page": 1, "limit": 2, "total": 6, "pages": 3 }
}
```

Detail:

```
GET /api/tunes/{id}
```

Returns metadata plus `abc_content` (the ABC text), `source_url`, `notes`, and
transcription credit. Unknown metadata is sent as JSON `null` (for example
`composer`, `key_signature`, `meter`, `rhythm`), so parsers must treat `null`
like an empty field.

Filter option lists:

```
GET /api/filters/keys
GET /api/filters/meters
GET /api/filters/rhythms
```

These lists are large and noisy. The UI should present curated common values
plus a searchable selector; the API accepts arbitrary strings.

Default tune: `https://abcsessiontunes.com/tunes/75815` — *Sandy Boys*.

### 7.4 CORS and the web proxy

Verified in November 2026:

- `contradb.com` sends no `Access-Control-Allow-Origin`.
- `backend.abcsessiontunes.com` allows only `https://abcsessiontunes.com`.
- `www.ibiblio.org` sends no `Access-Control-Allow-Origin` for The Caller's Box.

Desktop and Android fetch directly. The web build needs a proxy:

- `proxy/` contains a Cloudflare Worker (MIT) with fixed upstream paths, CORS
  headers, an optional shared token, and short GET caching.
- The worker must not be an open proxy; only allow the upstream hosts above and
  the specific paths used by the app.
- Preferences: **Web data proxy URL** and **token**.
- The web build may bake in a default proxy URL from an environment variable.
- Without a proxy, the web build still works with bundled defaults and cached
  content.

### 7.5 Cache

Cache raw and parsed responses:

- `dance/{provider}/{id}.json` and `.html`
- `tune/{provider}/{id}.json` and `.abc`
- `search/{provider}/{query-hash}.json`
- `prefs.json`

Rules:

- Read from cache first; revalidate in the background when online.
- Fall back to cached content on network failure.
- Detail TTL: 30 days; search TTL: 7 days.
- Manual "Refresh" bypasses TTL.
- LRU eviction with a size budget (native/Android 50 MB; web limited by
  localStorage/IndexedDB quota).
- Never cache an HTML/JSON error page as a valid result.
- Cache keys include the provider ID so sources cannot collide.
- Bundled defaults are separate from the cache and always available.

### 7.6 Adding another source

1. Implement `DanceProvider` or `TuneProvider` in `sources`.
2. Add pure parsing functions and fixture tests under `fixtures/`.
3. Describe search fields with `FilterSpec`; the dance screen draws only those
   fields, and `supports_pagination`, `supports_sort`, and `result_limit`
   describe the rest of its search behaviour.
4. Register the provider in `DANCE_PROVIDERS` or `TUNE_PROVIDERS`.
5. Implement URL parsing for direct links.
6. Add attribution and any source-specific rate-limit rules to `CREDITS.md`.
7. Keep site-specific URLs, JSON shapes, and selectors out of `core`, `music`,
   and the UI. The dance source preference resolves a provider by its id, so a
   new dance provider needs no new preference field.
