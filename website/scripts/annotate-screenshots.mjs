#!/usr/bin/env node
// Add the documentation focus ring to the raw headless screenshots with
// PhotoCraft. The raw PNGs are left untouched so the pass can be repeated.

import {spawnSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const root = path.resolve(import.meta.dirname, '..', '..');
const rawDir = path.join(root, 'website', 'screenshots', 'raw');
const outDir = path.join(root, 'website', 'static', 'img', 'screens');
const manifestPath = path.join(root, 'website', 'screenshots', 'manifest.json');
const emphasisPath = path.join(root, 'website', 'screenshots', 'emphasis.json');

function findPhotoCraft() {
  if (process.env.PHOTOCRAFT_CLI) return process.env.PHOTOCRAFT_CLI;
  const candidates = [];
  if (process.platform === 'win32') {
    candidates.push('C:\\Program Files\\PhotoCraft\\photocraft-cli.exe');
    candidates.push(
      path.join(process.env.LOCALAPPDATA ?? '', 'Programs', 'PhotoCraft', 'photocraft-cli.exe'),
    );
  } else if (process.platform === 'darwin') {
    candidates.push('/Applications/PhotoCraft.app/Contents/MacOS/photocraft-cli');
  }
  candidates.push('photocraft-cli');
  for (const candidate of candidates) {
    if (!candidate.includes(path.sep) && candidate === 'photocraft-cli') return candidate;
    if (fs.existsSync(candidate)) return candidate;
  }
  return null;
}

function fail(message) {
  console.error(`annotate-screenshots: ${message}`);
  process.exit(1);
}

const cli = findPhotoCraft();
if (!cli) {
  fail(
    'PhotoCraft CLI not found. Set PHOTOCRAFT_CLI or install PhotoCraft (photocraft-cli).',
  );
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
const emphasis = JSON.parse(fs.readFileSync(emphasisPath, 'utf8'));
if (emphasis.schemaVersion !== 1) fail('emphasis.json: schemaVersion must be 1');

const ready = manifest.shots.filter((shot) => shot.status === 'ready');
const readyIds = new Set(ready.map((shot) => shot.id));
const configured = new Set(Object.keys(emphasis.shots ?? {}));

for (const id of readyIds) {
  if (!configured.has(id)) fail(`emphasis.json has no region for ready shot "${id}"`);
}
for (const id of configured) {
  if (!readyIds.has(id)) fail(`emphasis.json names "${id}", which is not a ready shot`);
}

const style = emphasis.style ?? {};
const color = style.color ?? '#ffc857';
const width = style.width ?? 3;
const radius = style.radius ?? 8;
const shadow = style.shadow ?? {color: '#000000', opacity: 55, distance: 0, size: 6};

fs.mkdirSync(outDir, {recursive: true});

let annotated = 0;
for (const shot of ready) {
  const entry = emphasis.shots[shot.id];
  const source = path.join(rawDir, `${shot.id}.png`);
  if (!fs.existsSync(source)) {
    fail(`missing raw screenshot ${path.relative(root, source)}; run the headless capture first`);
  }

  const bytes = fs.readFileSync(source);
  if (bytes.readUInt32BE(0) !== 0x89504e47) fail(`${shot.id}: raw screenshot is not a PNG`);
  const sourceWidth = bytes.readUInt32BE(16);
  const sourceHeight = bytes.readUInt32BE(20);
  if (entry.size && (entry.size[0] !== sourceWidth || entry.size[1] !== sourceHeight)) {
    fail(
      `${shot.id}: emphasis regions are for ${entry.size[0]}x${entry.size[1]}, ` +
        `but the raw screenshot is ${sourceWidth}x${sourceHeight}; update emphasis.json`,
    );
  }

  const regions = entry.regions ?? [];
  const target = path.join(outDir, `${shot.id}.png`);
  const temporary = path.join(outDir, `.${shot.id}.annotated.png`);
  fs.rmSync(temporary, {force: true});

  const args = ['run', source, '--metadata', 'none'];
  for (const [x, y, w, h] of regions) {
    if (![x, y, w, h].every((value) => Number.isInteger(value)) || w <= 0 || h <= 0) {
      fail(`${shot.id}: invalid emphasis region ${JSON.stringify([x, y, w, h])}`);
    }
    if (x < 0 || y < 0 || x + w > sourceWidth || y + h > sourceHeight) {
      fail(`${shot.id}: emphasis region ${JSON.stringify([x, y, w, h])} is outside the image`);
    }
    args.push('--cmd', 'shape.create');
    args.push(
      '--params',
      JSON.stringify({
        kind: 'roundedRect',
        rect: [x, y, w, h],
        radii: radius,
        fill: null,
        stroke: {width, color, align: 'outside'},
        name: 'Documentation focus',
      }),
    );
    args.push('--cmd', 'layer.layerStyle.dropShadow');
    args.push('--params', JSON.stringify(shadow));
  }
  if (regions.length === 0) {
    fs.copyFileSync(source, temporary);
  } else {
    args.push('--out', temporary);
    let lastDetail = '';
    let ok = false;
    for (let attempt = 1; attempt <= 3 && !ok; attempt += 1) {
      fs.rmSync(temporary, {force: true});
      const result = spawnSync(cli, args, {cwd: root, encoding: 'utf8'});
      if (result.error) fail(`${shot.id}: ${result.error.message}`);
      if (result.status === 0 && fs.existsSync(temporary)) {
        ok = true;
      } else {
        lastDetail = (result.stderr || result.stdout || '').trim();
        // PhotoCraft occasionally exits 1 when a previous process is still
        // releasing its session; a short retry is enough.
        Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 250);
      }
    }
    if (!ok) {
      fail(`${shot.id}: PhotoCraft failed after 3 attempts${lastDetail ? `\n${lastDetail}` : ''}`);
    }
  }

  fs.rmSync(target, {force: true});
  fs.renameSync(temporary, target);
  annotated += 1;
}

console.log(
  `Annotated ${annotated} screenshots with PhotoCraft (${path.relative(root, outDir)}).`,
);
