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

const manifestIds = new Set(manifest.shots.map((shot) => shot.id));
const rendered = manifest.shots.filter((shot) =>
  fs.existsSync(path.join(rawDir, `${shot.id}.png`)),
);
const renderedIds = new Set(rendered.map((shot) => shot.id));
const configured = new Set(Object.keys(emphasis.shots ?? {}));

for (const id of renderedIds) {
  if (!configured.has(id)) fail(`emphasis.json has no region for rendered shot "${id}"`);
}
for (const id of configured) {
  if (!manifestIds.has(id)) fail(`emphasis.json names "${id}", which is not in the manifest`);
}

const style = emphasis.style ?? {};
const color = style.color ?? '#ffc857';
const width = style.width ?? 3;
const radius = style.radius ?? 8;
const shadow = style.shadow ?? {color: '#000000', opacity: 55, distance: 0, size: 6};
// A numbered callout is a filled badge beside its focus ring. The digit is
// point text; Arial Bold's digits are tabular, so nudging the origin by a fixed
// fraction of the size keeps the glyph centred inside the circle.
const badgeStyle = style.badge ?? {};
const badgeFill = badgeStyle.fill ?? color;
const badgeInk = badgeStyle.ink ?? '#141414';
const badgeRadius = badgeStyle.radius ?? 13;
const badgeGap = badgeStyle.gap ?? 8;
const badgeSize = badgeStyle.size ?? 19;
const badgeFont = badgeStyle.font ?? 'Arial';

fs.mkdirSync(outDir, {recursive: true});

let annotated = 0;
for (const shot of rendered) {
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
  const labels = entry.labels ?? [];
  const positions = entry.labelPositions ?? [];
  if (!Array.isArray(labels) || (labels.length !== 0 && labels.length !== regions.length)) {
    fail(`${shot.id}: "labels" needs one entry per region`);
  }
  if (
    !Array.isArray(positions) ||
    (positions.length !== 0 && positions.length !== regions.length)
  ) {
    fail(`${shot.id}: "labelPositions" needs one [x, y] per region`);
  }
  const target = path.join(outDir, `${shot.id}.png`);
  const temporary = path.join(outDir, `.${shot.id}.annotated.png`);
  fs.rmSync(temporary, {force: true});

  const args = ['run', source, '--metadata', 'none'];
  for (const [index, region] of regions.entries()) {
    const [x, y, w, h] = region;
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

    const label = labels[index];
    if (label === undefined) continue;
    if (typeof label !== 'string' || !/^[1-9]$/.test(label)) {
      fail(`${shot.id}: callout ${index + 1} has an invalid label ${JSON.stringify(label)}`);
    }
    // Sit the badge to the left of the ring, or to its right when the ring
    // touches the edge of the image; a shot can pin its own centres when the
    // neighbours leave no free space.
    let centerX = x - badgeGap - badgeRadius;
    let centerY = y + h / 2;
    if (centerX - badgeRadius < 4) centerX = x + w + badgeGap + badgeRadius;
    const pinned = positions[index];
    if (pinned !== undefined) {
      if (
        !Array.isArray(pinned) ||
        pinned.length !== 2 ||
        !pinned.every((value) => Number.isInteger(value)) ||
        pinned[0] < badgeRadius ||
        pinned[1] < badgeRadius ||
        pinned[0] + badgeRadius > sourceWidth ||
        pinned[1] + badgeRadius > sourceHeight
      ) {
        fail(`${shot.id}: callout ${index + 1} has an invalid position ${JSON.stringify(pinned)}`);
      }
      [centerX, centerY] = pinned;
    }
    args.push('--cmd', 'shape.create');
    args.push(
      '--params',
      JSON.stringify({
        kind: 'ellipse',
        rect: [centerX - badgeRadius, centerY - badgeRadius, badgeRadius * 2, badgeRadius * 2],
        fill: badgeFill,
        stroke: null,
        name: `Callout ${label}`,
      }),
    );
    args.push('--cmd', 'type.create');
    args.push(
      '--params',
      JSON.stringify({
        x: Math.round(centerX - badgeSize * 0.32),
        y: Math.round(centerY + badgeSize * 0.37),
        text: label,
        font: badgeFont,
        fontStyle: 'Bold',
        size: badgeSize,
        color: badgeInk,
      }),
    );
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
