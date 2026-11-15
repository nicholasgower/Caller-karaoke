#!/usr/bin/env node
// Fetch the sample dances and tunes under samples/.
//
// The samples are for local development and manual testing only. They are
// fetched from the two public sources the app supports and keep their
// attribution (source URL + transcription credit) in each file and in the
// generated manifest. They are not bundled into release artifacts.
//
// Usage:
//   node scripts/fetch-samples.mjs            # fetch or refresh everything
//   node scripts/fetch-samples.mjs --dances   # dances only
//   node scripts/fetch-samples.mjs --tunes    # tunes only
//
// Requests are serialized with a delay and retried with backoff so a refresh
// stays within the sources' rate limits. Node 18 or newer is required (uses
// the built-in fetch).

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const SAMPLES = path.join(ROOT, "samples");
const USER_AGENT = "ContraKaraoke/0.1.0 (sample fetch; +https://github.com/storytold/Contra-karaoke)";
const DELAY_MS = 1200;
const RETRIES = 4;

// Curated so that the set exercises the parser, the timeline and the music
// pipeline rather than acting as a mirror of the sites.
const DANCES = [
  {
    id: 2409,
    tests: "Default dance. 64 beats, 7 rows, continued phrase, Unicode fraction; raw page ends with a pilcrow progression marker.",
  },
  {
    id: 1246,
    tests: "Becket. 64 beats, 8 rows, 4-beat sub-moves, notes, 'new neighbor' wording.",
  },
  {
    id: 434,
    tests: "Becket ccw. 64 beats, 2-beat sub-move, '+' and '++' footnote markers in card text.",
  },
  {
    id: 901,
    tests: "Proper. 64 beats, zero-beat 'meanwhile' row, 4+4+0 split of a phrase.",
  },
  {
    id: 2380,
    tests: "Four face four. 64 beats, many zero-beat setup rows, a 16-beat figure mid-dance.",
  },
  {
    id: 794,
    tests: "Triplet. 64 beats, long figure text, 16-beat figures.",
  },
  {
    id: 664,
    tests: "Scatter mixer. 64 beats, 4-beat rows, bracketed footnote markers [1]-[6].",
  },
  {
    id: 3061,
    tests: "Circle mixer. 64 beats, 16-beat figures, misspelled source text.",
  },
  {
    id: 1692,
    tests: "Singlet. 64 beats, repeated 'box circulate' phrase with role variants.",
  },
  {
    id: 218,
    tests: "Cross contra. 64 beats, odd 3/10/3-beat split of a phrase.",
  },
  {
    id: 3110,
    tests: "Capitalized 'Improper'. 64 beats, 6+2 split, notes.",
  },
  {
    id: 553,
    tests: "Sicilian circle. 60 beats, so the card does not fill the 64-beat music loop.",
  },
  {
    id: 74,
    tests: "Becket. 96 beats, so the card runs past one 64-beat music loop.",
  },
];

const TUNES = [
  {
    id: 75815,
    tests: "Default tune. Reel, A mix, 4/4, quoted chords, 32 bars.",
  },
  {
    id: 9248,
    tests: "Reel, A mix, C| meter, quoted chords, pickup note.",
  },
  {
    id: 55939,
    tests: "Reel, A, no quoted chords (chord inference), pickup, triplets.",
  },
  {
    id: 75412,
    tests: "Reel, D, C| meter, inline 'variations' text, more than 32 bars.",
  },
  {
    id: 2,
    tests: "Reel, E dor, four parts with first/second endings, no chords.",
  },
  {
    id: 8847,
    tests: "Reel, A dor, no repeat barlines (linear body).",
  },
  {
    id: 7404,
    tests: "Jig, D, 6/8, one chord symbol, pickup note.",
  },
  {
    id: 7268,
    tests: "Jig, D, 6/8, dense quoted chords, pickup, quoted chord with parentheses.",
  },
  {
    id: 13476,
    tests: "Slip jig, E minor, 9/8, three parts, quoted chords.",
  },
  {
    id: 3017,
    tests: "Hornpipe, D, C| meter, quoted chords, pickup.",
  },
  {
    id: 16494,
    tests: "Waltz, D, 3/4, quoted chords, long N: notes and a D7/C chord.",
  },
  {
    id: 38052,
    tests: "Polka, G, 2/4, no chords, blank line before the X: header.",
  },
  {
    id: 63383,
    tests: "Reel, A mix, L:1/4, three-note pickup, quoted chords.",
  },
  {
    id: 38058,
    tests: "Reel, D, 4/4, no L: or R: header and no repeat barlines.",
  },
  {
    id: 74283,
    tests: "Medley file: three tunes in one ABC body (edge case for the parser).",
  },
];

const args = new Set(process.argv.slice(2));
const doDances = args.size === 0 || args.has("--dances");
const doTunes = args.size === 0 || args.has("--tunes");

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function fetchText(url, init = {}) {
  let lastError;
  for (let attempt = 0; attempt < RETRIES; attempt++) {
    try {
      const res = await fetch(url, {
        ...init,
        headers: { "User-Agent": USER_AGENT, ...(init.headers ?? {}) },
      });
      const text = await res.text();
      if (res.ok) return { text, url: res.url || url };
      lastError = new Error(`${res.status} ${res.statusText} for ${url}`);
      // 4xx responses are not worth retrying except for 429.
      if (res.status !== 429 && res.status < 500) break;
    } catch (error) {
      lastError = error;
    }
    await sleep(DELAY_MS * (attempt + 2));
  }
  throw lastError ?? new Error(`request failed: ${url}`);
}

const NAMED_ENTITIES = {
  amp: "&",
  lt: "<",
  gt: ">",
  quot: '"',
  apos: "'",
  nbsp: " ",
  ndash: "\u2013",
  mdash: "\u2014",
  hellip: "\u2026",
  frac12: "\u00bd",
  frac14: "\u00bc",
  frac34: "\u00be",
  half: "\u00bd",
  ldquo: "\u201c",
  rdquo: "\u201d",
  lsquo: "\u2018",
  rsquo: "\u2019",
  deg: "\u00b0",
  times: "\u00d7",
};

function decodeEntities(text) {
  return text.replace(/&(#x?[0-9a-fA-F]+|[a-zA-Z][a-zA-Z0-9]*);/g, (match, body) => {
    if (body.startsWith("#x") || body.startsWith("#X")) {
      const code = Number.parseInt(body.slice(2), 16);
      return Number.isFinite(code) ? String.fromCodePoint(code) : match;
    }
    if (body.startsWith("#")) {
      const code = Number.parseInt(body.slice(1), 10);
      return Number.isFinite(code) ? String.fromCodePoint(code) : match;
    }
    return NAMED_ENTITIES[body] ?? match;
  });
}

function htmlToText(html) {
  const withBreaks = html
    .replace(/<\s*br\s*\/?\s*>/gi, "\n")
    .replace(/<\/\s*(p|div|li|tr|h[1-6])\s*>/gi, "\n");
  return decodeEntities(withBreaks.replace(/<[^>]*>/g, ""))
    .replace(/\r\n?/g, "\n")
    .split("\n")
    .map((line) => line.replace(/[ \t\u00a0]+/g, " ").trim())
    .filter((line, index, lines) => line !== "" || lines[index - 1] !== "")
    .join("\n")
    .trim();
}

/** Extract the inner HTML of the first element whose opening tag matches. */
function extractBlock(html, openingTagPattern) {
  const match = openingTagPattern.exec(html);
  if (!match) return null;
  const contentStart = match.index + match[0].length;
  let depth = 1;
  const tag = /<\s*(\/?)div\b[^>]*>/gi;
  tag.lastIndex = contentStart;
  for (;;) {
    const next = tag.exec(html);
    if (!next) return html.slice(contentStart);
    if (next[0].startsWith("</")) depth--;
    else if (!next[0].endsWith("/>")) depth++;
    if (depth === 0) return html.slice(contentStart, next.index);
  }
}

function slugify(title) {
  return title
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/['\u2019]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 60);
}

function beatNumber(value) {
  return Number.isFinite(value) ? Math.round(value * 1000) / 1000 : 0;
}

/** Pretty-print JSON but keep beat counts visibly floating point. */
function writeJson(file, value) {
  const text = JSON.stringify(value, null, 2).replace(
    /("beats": )(-?\d+)([,}\]])/g,
    "$1$2.0$3",
  );
  fs.writeFileSync(file, text + "\n", "utf8");
}

function cleanFigureText(text) {
  // ContraDB appends U+204B PILCROW SIGN to the last figure before the
  // progression. It is layout, not move wording, so it is dropped here; the
  // raw HTML keeps the original bytes.
  return text.replace(/\s*\u204b\s*$/u, "").trim();
}

function parseDancePage(html, id) {
  const title = htmlToText((html.match(/<h1 class="dance-show-title">([\s\S]*?)<\/h1>/) || [])[1] ?? "");
  const choreographerRaw = (html.match(/<p class="dance-show-choreographer">([\s\S]*?)<\/p>/) || [])[1] ?? "";
  const choreographer = htmlToText(choreographerRaw).replace(/^by:\s*/i, "").trim();
  const formationRaw = (html.match(/<p class="dance-show-formation">([\s\S]*?)<\/p>/) || [])[1] ?? "";
  const formation = htmlToText(formationRaw).replace(/^formation:\s*/i, "").trim();
  const preamble = htmlToText(extractBlock(html, /<div class="dance-show-preamble">/) ?? "");
  const notes = htmlToText(extractBlock(html, /<div class="dance-show-notes">/) ?? "");

  const rows = [];
  for (const row of html.matchAll(/<tr class="[^"]*">([\s\S]*?)<\/tr>/g)) {
    const cells = [...row[1].matchAll(/<td[^>]*>([\s\S]*?)<\/td>/g)].map((cell) => cell[1]);
    if (cells.length < 3) continue;
    const phrase = htmlToText(cells[0]) || null;
    const beats = Number.parseFloat(htmlToText(cells[1]));
    const text = cleanFigureText(htmlToText(cells[2]));
    rows.push({ phrase, beats: Number.isFinite(beats) ? beats : 0, text });
  }

  // ContraDB uses zero-beat rows for setup cues ("join hands in your ring of
  // 8") and "meanwhile" moves. The domain model wants positive beats, so a
  // zero-beat row prefixes the next real figure, or appends to the last one.
  const figures = [];
  let pending = null;
  let pendingPhrase = null;
  for (const row of rows) {
    if (row.beats <= 0) {
      pending = pending ? `${pending}; ${row.text}` : row.text;
      pendingPhrase ??= row.phrase;
      continue;
    }
    figures.push({
      phrase: row.phrase ?? pendingPhrase,
      beats: beatNumber(row.beats),
      text: pending ? `${pending}; ${row.text}` : row.text,
    });
    pending = null;
    pendingPhrase = null;
  }
  if (pending) {
    if (figures.length === 0) {
      figures.push({ phrase: null, beats: 8.0, text: pending });
    } else {
      figures[figures.length - 1].text += `; ${pending}`;
    }
  }

  return {
    source: {
      provider: "contradb",
      id: String(id),
      url: `https://contradb.com/dances/${id}`,
    },
    title,
    choreographer,
    formation,
    preamble,
    notes,
    figures,
  };
}

function parseTune(tune, id) {
  const abc = tune.abc_content ?? "";
  const zLine = (abc.match(/^Z:(.*)$/m) || [])[1]?.trim() ?? "";
  const transcription = (tune.transcribed_by || zLine || "").trim();
  const sourceUrl = tune.source_url || "";
  const credits =
    [transcription, sourceUrl].filter(Boolean).join(" \u2014 ") ||
    `ABC Session Tunes tune ${id}`;
  return {
    source: {
      provider: "abcsessiontunes",
      id: String(id),
      url: `https://abcsessiontunes.com/tunes/${id}`,
    },
    title: tune.title ?? "",
    composer: tune.composer ?? "",
    key: tune.key_signature ?? "",
    meter: tune.meter ?? "",
    rhythm: tune.rhythm ?? "",
    abc,
    credits,
  };
}

function writeManifest(manifest) {
  const file = path.join(SAMPLES, "manifest.json");
  const ordered = {
    generated: manifest.generated,
    sources: manifest.sources,
    dances: [...manifest.dances].sort((a, b) => a.id.localeCompare(b.id, "en", { numeric: true })),
    tunes: [...manifest.tunes].sort((a, b) => a.id.localeCompare(b.id, "en", { numeric: true })),
  };
  writeJson(file, ordered);
}

async function fetchDances(manifest) {
  const dir = path.join(SAMPLES, "dances");
  fs.mkdirSync(dir, { recursive: true });
  for (const spec of DANCES) {
    const url = `https://contradb.com/dances/${spec.id}`;
    process.stdout.write(`dance ${spec.id} ... `);
    const { text: html } = await fetchText(url, { headers: { Accept: "text/html" } });
    const dance = parseDancePage(html, spec.id);
    if (dance.figures.length === 0) throw new Error(`dance ${spec.id} parsed no figures`);

    const slug = slugify(dance.title) || "dance";
    const base = `${spec.id}-${slug}`;
    fs.writeFileSync(path.join(dir, `${base}.html`), html, "utf8");
    writeJson(path.join(dir, `${base}.json`), dance);

    manifest.dances.push({
      id: String(spec.id),
      title: dance.title,
      choreographer: dance.choreographer,
      formation: dance.formation,
      beats: beatNumber(dance.figures.reduce((sum, figure) => sum + figure.beats, 0)),
      figures: dance.figures.length,
      url,
      files: {
        normalized: `dances/${base}.json`,
        raw: `dances/${base}.html`,
      },
      tests: spec.tests,
    });
    console.log(`${dance.title} (${dance.figures.length} figures)`);
    await sleep(DELAY_MS);
  }
}

async function fetchTunes(manifest) {
  const dir = path.join(SAMPLES, "tunes");
  fs.mkdirSync(dir, { recursive: true });
  for (const spec of TUNES) {
    const apiUrl = `https://backend.abcsessiontunes.com/api/tunes/${spec.id}`;
    process.stdout.write(`tune ${spec.id} ... `);
    const { text } = await fetchText(apiUrl, { headers: { Accept: "application/json" } });
    const raw = JSON.parse(text);
    const tune = parseTune(raw, spec.id);
    if (!tune.abc.trim()) throw new Error(`tune ${spec.id} has no ABC content`);

    const slug = slugify(tune.title) || "tune";
    const base = `${spec.id}-${slug}`;
    fs.writeFileSync(path.join(dir, `${base}.abc`), tune.abc.replace(/\r\n?/g, "\n") + "\n", "utf8");
    writeJson(path.join(dir, `${base}.json`), tune);
    writeJson(path.join(dir, `${base}.api.json`), raw);

    manifest.tunes.push({
      id: String(spec.id),
      title: tune.title,
      composer: tune.composer,
      key: tune.key,
      meter: tune.meter,
      rhythm: tune.rhythm,
      url: tune.source.url,
      abc_source: raw.source_url || "",
      files: {
        normalized: `tunes/${base}.json`,
        abc: `tunes/${base}.abc`,
        api: `tunes/${base}.api.json`,
      },
      tests: spec.tests,
    });
    console.log(`${tune.title} (${tune.key || "?"} ${tune.meter || "?"})`);
    await sleep(DELAY_MS);
  }
}

async function main() {
  fs.mkdirSync(SAMPLES, { recursive: true });
  const manifestFile = path.join(SAMPLES, "manifest.json");
  const manifest = fs.existsSync(manifestFile)
    ? JSON.parse(fs.readFileSync(manifestFile, "utf8"))
    : {
        generated: "",
        sources: [
          {
            provider: "contradb",
            name: "ContraDB",
            url: "https://contradb.com",
            note: "Dance cards transcribed by ContraDB contributors. Each card keeps its source URL and choreographer credit.",
          },
          {
            provider: "abcsessiontunes",
            name: "ABC Session Tunes",
            url: "https://abcsessiontunes.com",
            note: "ABC transcriptions by their respective contributors. Each tune keeps its source URL and transcription credit.",
          },
        ],
        dances: [],
        tunes: [],
      };
  manifest.generated = new Date().toISOString().slice(0, 10);
  if (doDances) manifest.dances = [];
  if (doTunes) manifest.tunes = [];

  if (doDances) await fetchDances(manifest);
  if (doTunes) await fetchTunes(manifest);

  writeManifest(manifest);
  console.log(`Wrote ${manifest.dances.length} dances and ${manifest.tunes.length} tunes to ${SAMPLES}`);
  console.log("Samples are development fixtures only; keep them out of release bundles.");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
