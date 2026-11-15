#!/usr/bin/env node
// Build the WASM app and stage it under website/static/app so the docs site
// can serve a hosted copy at /app/. Run from a checkout with Rust and Trunk
// installed; CI does this before `npm run check`.
//
// The public URL must match Docusaurus `baseUrl` plus `app/`, otherwise the
// hashed JS/WASM assets resolve outside the Pages project path.

import {spawnSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import {fileURLToPath} from 'node:url';

const websiteRoot = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const repoRoot = path.resolve(websiteRoot, '..');
const webCrate = path.join(repoRoot, 'crates', 'web');
const trunkOutput = path.join(repoRoot, 'dist', 'web');
const staged = path.join(websiteRoot, 'static', 'app');
const publicUrl = process.env.CONTRA_WEB_PUBLIC_URL ?? '/Contra-karaoke/app/';

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {stdio: 'inherit', ...options});
  if (result.error?.code === 'ENOENT') {
    console.error(`\n${command} was not found on PATH.`);
    if (command === 'trunk') {
      console.error('Install Trunk with: cargo install trunk --locked');
      console.error('Add the WebAssembly target with: rustup target add wasm32-unknown-unknown');
    }
    process.exit(1);
  }
  if (result.status !== 0) {
    console.error(`\n${command} ${args.join(' ')} exited with code ${result.status}.`);
    process.exit(result.status ?? 1);
  }
}

console.log(`Building the hosted web app with public URL ${publicUrl}`);
run('trunk', ['build', '--release', '--public-url', publicUrl], {cwd: webCrate});

const index = path.join(trunkOutput, 'index.html');
if (!fs.existsSync(index)) {
  console.error(`Trunk did not write ${path.relative(repoRoot, index)}.`);
  process.exit(1);
}

fs.rmSync(staged, {recursive: true, force: true});
fs.mkdirSync(path.dirname(staged), {recursive: true});
fs.cpSync(trunkOutput, staged, {recursive: true});
console.log(`Staged ${path.relative(repoRoot, staged)} for the documentation site.`);
