# Sample dances and tunes

Real content from the two sources the app supports, saved for development and
manual testing. They let the source parsers, the call timeline and the music
pipeline be exercised without a network connection and without hand-writing
fixtures for every awkward case.

**These samples are development fixtures, not app content.** Per `AGENTS.md`
§2.8, they must not be bundled into any release artifact and are not the
attributed defaults under `assets/defaults/`. The two defaults remain
*Another Last Dance* and *Sandy Boys*.

## Refreshing

```sh
node scripts/fetch-samples.mjs            # both kinds
node scripts/fetch-samples.mjs --dances   # dances only
node scripts/fetch-samples.mjs --tunes    # tunes only
```

The script uses Node 18+ (built-in `fetch`), identifies itself with a
descriptive `User-Agent`, serializes requests with a delay, and retries with
backoff. It rewrites `manifest.json` and the matching `.json`/`.abc`/`.html`
files. Re-running it is how to pick up upstream edits; do not copy whole
databases or crawl beyond the curated list.

## Layout

```
samples/
  manifest.json                 machine-readable index + provenance + what each item tests
  dances/
    <id>-<slug>.json            normalized dance (contra_core::Dance shape)
    <id>-<slug>.html            raw ContraDB page, byte-for-byte
  tunes/
    <id>-<slug>.json            normalized tune (contra_core::Tune shape)
    <id>-<slug>.abc             ABC text on its own
    <id>-<slug>.api.json        raw ABC Session Tunes detail response
```

The normalized JSON maps directly onto the `contra-core` domain models:

```json
{
  "source": { "provider": "contradb", "id": "2409", "url": "https://contradb.com/dances/2409" },
  "title": "Another Last Dance",
  "choreographer": "Erik Hoffman",
  "formation": "improper",
  "preamble": "",
  "notes": "",
  "figures": [
    { "phrase": "A1", "beats": 8.0, "text": "neighbors do si do once" }
  ]
}
```

Phrase labels are carried through on the first figure of each phrase; following
figures in the same phrase have `phrase: null`, which is how ContraDB's table
presents them.

### Normalization applied to dances

- HTML is stripped; entities and Unicode characters are decoded.
- The trailing `⁋` progression marker is removed from card text (the raw HTML
  keeps it).
- ContraDB zero-beat rows — setup cues such as "join hands in your ring of 8"
  and "meanwhile" moves — are merged into the adjacent figure, because the
  domain model requires a positive beat count. A zero-beat row prefixes the
  next real figure, or appends to the last one when it is the final row. The
  raw HTML keeps the original row split.
- Footnotes such as `[1]`, `+`, `++`, and other editorial marks are left in the
  text: they are part of the card and a good test of alias handling.

### Normalization applied to tunes

- `abc_content` is stored verbatim (line endings normalized to LF).
- `credits` combines the transcription credit (`transcribed_by`, else the
  `Z:` line) with the ABC source URL.
- The raw API response keeps every other field for reference.

## Dances (13)

| ID | Title | Choreographer | Formation | Beats |
| ---: | --- | --- | --- | ---: |
| 2409 | Another Last Dance | Erik Hoffman | improper | 64 |
| 1246 | Badlands Contra | Don Flaherty | Becket | 64 |
| 434 | A New Idea | Michael Fuerst | Becket ccw | 64 |
| 901 | 3 Places Left | Arthur Kingsland | proper | 64 |
| 2380 | Allison's Favorite | Colin Hume | four face four | 64 |
| 794 | Adam's Trip to the Corner | Adam Carlson | triplet | 64 |
| 664 | Accretion Reel | Chris Page | scatter mixer | 64 |
| 3061 | Arkansas Traveler | Sue Hulsether | circle mixer | 64 |
| 1692 | 24th of October variation | Don Lennartson | singlet | 64 |
| 218 | Across Nottingham A | Cary Ravitz | cross contra | 64 |
| 3110 | Adventures in C# | Bob Isaacs | Improper | 64 |
| 553 | Attleboro Reel | Linda Leslie | Sicilian circle | 60 |
| 74 | 40 Years of Penguin Pam | Seth Tepfer | Becket | 96 |

The set covers the four common formations plus triplets and mixers, 4- and
16-beat figures, odd 2/3/4/6/10-beat splits, zero-beat setup rows, notes and
preambles, Unicode fractions, and two cards that do not total 64 beats (60 and
96) so the "card does not fill / overruns the music loop" paths can be tested.

## Tunes (15)

| ID | Title | Key | Meter | Rhythm | Notes |
| ---: | --- | --- | --- | --- | --- |
| 75815 | Sandy Boys | A mix | 4/4 | (none) | default tune, quoted chords |
| 9248 | Red Haired Boy | AMix | C\| | reel | quoted chords, pickup |
| 55939 | Moneymusk | A | 4/4 | Reel | no chords, pickup, triplets |
| 75412 | Saint Anne's Reel | D | C\| | reel | inline "variations" text, >32 bars |
| 2 | Drowsy Maggie | Edor | 4/4 | reel | four parts with endings |
| 8847 | Cold Frosty Morning | Ador | 4/4 | reel | no repeat barlines |
| 7404 | Banish Misfortune | D | 6/8 | (none) | jig, sparse chords |
| 7268 | Haste to the Wedding | D | 6/8 | jig | dense chords, pickup |
| 13476 | the Butterfly | Em | 9/8 | slip jig | three parts |
| 3017 | Fisher's Hornpipe | D | C\| | hornpipe | quoted chords, pickup |
| 16494 | Ashokan Farewell | D | 3/4 | waltz | long `N:` notes, D7/C |
| 38052 | Angeline The Baker | Gmaj | 2/4 | polka | no chords, blank line before `X:` |
| 63383 | Old Joe Clark | A mix | 4/4 | (none) | `L:1/4`, three-note pickup |
| 38058 | Angeline the Baker | D | 4/4 | (none) | no `L:`/`R:`, no repeat barlines |
| 74283 | Kid on the Mountain, The | Em | 9/8 | slip jig | medley of three tunes in one file |

The set covers reels, jigs, a slip jig, a hornpipe, a waltz and a polka; 4/4,
2/4, 6/8, 9/8 and 3/4; quoted chords and none; pickups, endings, four parts, a
linear body, and one medley file so the parser's error/edge handling is
exercised.

## Provenance and licensing

- **Dances** come from <https://contradb.com>. Cards are transcribed by
  ContraDB contributors; each normalized file and manifest entry keeps the
  canonical URL, and the raw page keeps the contributor/user credit. The
  choreography belongs to its choreographers.
- **Tunes** come from <https://abcsessiontunes.com> and the ABC transcriptions
  belong to their respective transcribers. Each file keeps the transcription
  credit and the upstream ABC source URL (often
  <https://abcnotation.com> or a tunebook), stored alongside the ABC.
- Nothing here is relicensed. The application code is MIT; this sample content
  is third-party material fetched for local testing only.
- The curated set is small on purpose. It is not a dance or tune database and
  must not be presented or redistributed as one.

See `manifest.json` for the per-item URL, file paths, and the specific parser or
timeline behaviour each item is meant to exercise.
