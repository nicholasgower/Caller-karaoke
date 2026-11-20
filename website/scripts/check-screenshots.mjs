#!/usr/bin/env node
// Validate the authored screenshot manifest and the shot IDs used in the docs.
// Deliberately dependency-free so it can run before the Docusaurus install.

import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import crypto from 'node:crypto';

const root = path.resolve(import.meta.dirname, '..');
const docsRoot = path.join(root, 'docs');
const screensRoot = path.join(root, 'static', 'img', 'screens');
const manifestPath = path.join(root, 'screenshots', 'manifest.json');

const errors = [];
const fail = (message) => errors.push(message);

if (!fs.existsSync(manifestPath)) {
  fail(`Missing ${path.relative(root, manifestPath)}`);
  report();
  process.exit(1);
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
if (manifest.schemaVersion !== 1) fail('manifest.json: schemaVersion must be 1');
if (!Array.isArray(manifest.shots)) fail('manifest.json: shots must be an array');

const shots = new Map();
for (const shot of manifest.shots ?? []) {
  if (!shot.id) {
    fail('manifest.json: a shot has no id');
    continue;
  }
  if (shots.has(shot.id)) fail(`manifest.json: duplicate shot id "${shot.id}"`);
  shots.set(shot.id, shot);
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(shot.id)) {
    fail(`manifest.json: invalid id "${shot.id}"`);
  }
  if (!shot.alt || shot.alt.length < 20) {
    fail(`manifest.json: "${shot.id}" needs alt text of at least 20 characters`);
  }
  if (shot.alt && /^screenshot of/i.test(shot.alt)) {
    fail(`manifest.json: "${shot.id}" alt text should describe the screen, not say "screenshot of"`);
  }
  if (shot.caption && shot.caption.length > 160) {
    fail(`manifest.json: "${shot.id}" caption is longer than 160 characters`);
  }
  if (!['pending', 'ready'].includes(shot.status)) {
    fail(`manifest.json: "${shot.id}" status must be pending or ready`);
  }
}

function walk(dir) {
  const files = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) files.push(...walk(full));
    else if (entry.name.endsWith('.md') || entry.name.endsWith('.mdx')) files.push(full);
  }
  return files;
}

const used = new Set();
for (const file of walk(docsRoot)) {
  let text = fs.readFileSync(file, 'utf8');
  // Ignore examples inside fenced and inline code.
  text = text.replace(/```[\s\S]*?```/g, '').replace(/`[^`]*`/g, '');
  const rel = path.relative(root, file);
  for (const match of text.matchAll(/<Step\s+shot="([^"]+)"/g)) {
    used.add(match[1]);
    if (!shots.has(match[1])) fail(`${rel}: unknown screenshot "${match[1]}"`);
  }
  for (const match of text.matchAll(/<Step\b[^>]*>/g)) {
    if (!/\bshot="/.test(match[0])) {
      fail(`${rel}: a <Step> is missing its shot attribute`);
    }
  }
  for (const match of text.matchAll(/<Shot\s+id="([^"]+)"/g)) {
    used.add(match[1]);
    if (!shots.has(match[1])) fail(`${rel}: unknown screenshot "${match[1]}"`);
  }
}

for (const id of shots.keys()) {
  if (!used.has(id)) fail(`manifest.json: shot "${id}" is not used by any docs page`);
}

for (const [id, shot] of shots) {
  if (shot.status !== 'ready') continue;
  const image = path.join(screensRoot, `${id}.png`);
  if (!fs.existsSync(image)) {
    fail(`screenshot "${id}" is marked ready but ${path.relative(root, image)} is missing`);
    continue;
  }
  const stat = fs.statSync(image);
  if (stat.size > 300 * 1024) {
    fail(`screenshot "${id}" is ${(stat.size / 1024).toFixed(0)} KB; budget is 300 KB`);
  }
  const header = fs.readFileSync(image).subarray(0, 24);
  const png = header[0] === 0x89 && header.toString('ascii', 1, 4) === 'PNG';
  if (!png) {
    fail(`screenshot "${id}" is not a PNG`);
    continue;
  }
  const width = header.readUInt32BE(16);
  const height = header.readUInt32BE(20);
  if (shot.width && shot.width !== width) {
    fail(`screenshot "${id}" width ${width} does not match manifest ${shot.width}`);
  }
  if (shot.height && shot.height !== height) {
    fail(`screenshot "${id}" height ${height} does not match manifest ${shot.height}`);
  }
}

// Every PNG in the screens folder is published, so compare all of them, not
// just the ones the manifest currently marks ready. Byte-identical images add
// no documentation value and usually mean a scenario did not change state.
const hashToId = new Map();
for (const name of fs.readdirSync(screensRoot).sort()) {
  if (!name.toLowerCase().endsWith('.png')) continue;
  const id = name.slice(0, -'.png'.length);
  const shot = shots.get(id);
  if (!shot || shot.status !== 'ready') {
    fail(`screenshot "${id}" is saved but has no ready manifest entry`);
  }
  const hash = crypto
    .createHash('sha256')
    .update(fs.readFileSync(path.join(screensRoot, name)))
    .digest('hex');
  if (hashToId.has(hash)) {
    fail(
      `screenshot "${id}" is identical to "${hashToId.get(hash)}"; every screenshot must be distinct`,
    );
  } else {
    hashToId.set(hash, id);
  }
}

function report() {
  if (errors.length === 0) {
    const ready = [...shots.values()].filter((shot) => shot.status === 'ready').length;
    console.log(
      `Screenshots OK: ${shots.size} planned, ${ready} ready, ${shots.size - ready} pending.`,
    );
    return;
  }
  for (const error of errors) console.error(`- ${error}`);
  console.error(`\n${errors.length} screenshot problem(s).`);
}

report();
process.exit(errors.length === 0 ? 0 : 1);
