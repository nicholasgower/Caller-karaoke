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

### 7.2 ABC Session Tunes

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

### 7.3 CORS and the web proxy

Verified in November 2026:

- `contradb.com` sends no `Access-Control-Allow-Origin`.
- `backend.abcsessiontunes.com` allows only `https://abcsessiontunes.com`.

Desktop and Android fetch directly. The web build needs a proxy:

- `proxy/` contains a Cloudflare Worker (MIT) with fixed upstream paths, CORS
  headers, an optional shared token, and short GET caching.
- The worker must not be an open proxy; only allow the two upstream hosts and
  the specific API paths used by the app.
- Preferences: **Web data proxy URL** and **token**.
- The web build may bake in a default proxy URL from an environment variable.
- Without a proxy, the web build still works with bundled defaults and cached
  content.

### 7.4 Cache

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

### 7.5 Adding another source

1. Implement `DanceProvider` or `TuneProvider` in `sources`.
2. Add pure parsing functions and fixture tests.
3. Describe search fields with `FilterSpec`.
4. Register the provider.
5. Implement URL parsing for direct links.
6. Add attribution and any source-specific rate-limit rules to `CREDITS.md`.
7. Do not change `core`, `music`, or the UI to add a source.
