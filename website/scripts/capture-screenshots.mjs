#!/usr/bin/env node
// Generate the documentation screenshots with the headless renderer.
//
// The app draws the yellow focus rings itself: each shot names the callout
// targets it wants, the widgets register their own rectangles while the frame
// draws, and the saved PNG already carries the rings. There is no second
// annotation pass and no per-shot pixel coordinates to keep in step with the
// layout.

import {spawnSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const root = path.resolve(import.meta.dirname, '..', '..');
const out = path.join(root, 'website', 'static', 'img', 'screens');
const manifestPath = path.join(root, 'website', 'screenshots', 'manifest.json');

// `cargo` is not always on the PATH inherited by npm (for example when the
// toolchain is installed only for the current user).
const home = process.env.USERPROFILE ?? process.env.HOME ?? '';
const localCargo = path.join(
  home,
  '.cargo',
  'bin',
  process.platform === 'win32' ? 'cargo.exe' : 'cargo',
);
const cargo = process.env.CARGO ?? (fs.existsSync(localCargo) ? localCargo : 'cargo');

const result = spawnSync(
  cargo,
  [
    'run',
    '-p',
    'caller-app',
    '--features',
    'headless',
    '--bin',
    'caller-headless',
    '--',
    out,
  ],
  {
    cwd: root,
    stdio: 'inherit',
    env: {
      ...process.env,
      WGPU_BACKEND: process.env.WGPU_BACKEND ?? 'vulkan',
      // The headless binary links a large wgpu stack. On MSVC its debug PDB can
      // exceed the linker limit, and captures do not need debug information.
      CARGO_PROFILE_DEV_DEBUG: process.env.CARGO_PROFILE_DEV_DEBUG ?? '0',
    },
  },
);

if (result.error) {
  console.error(result.error);
  process.exit(1);
}
if (result.status !== 0) process.exit(result.status ?? 1);

const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
let ready = 0;
for (const shot of manifest.shots) {
  const image = path.join(out, `${shot.id}.png`);
  if (!fs.existsSync(image)) continue;
  const bytes = fs.readFileSync(image);
  shot.status = 'ready';
  shot.width = bytes.readUInt32BE(16);
  shot.height = bytes.readUInt32BE(20);
  ready += 1;
}
fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`Screenshots ready: ${ready} of ${manifest.shots.length}.`);
