#!/usr/bin/env node
// Build the WASM app and stage it under website/static/app so the docs site
// can serve a hosted copy at /app/. `prestart` runs this before
// `docusaurus start`, and CI runs it before `npm run check`, so a plain
// `npm start` has the browser app ready. Use `npm run start:docs` for
// docs-only work that does not need Rust or Trunk.
//
// The public URL must match Docusaurus `baseUrl` plus `app/`, otherwise the
// hashed JS/WASM assets resolve outside the site path.

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
const publicUrl = process.env.CALLER_WEB_PUBLIC_URL ?? '/app/';

function setupHelp({missingTrunk, missingTarget}) {
  console.error('\nCannot build the hosted browser app:');
  if (missingTrunk) console.error('  - Trunk is not on PATH.');
  if (missingTarget) {
    console.error('  - the wasm32-unknown-unknown target is not installed.');
  }
  console.error('\nInstall what is missing:');
  if (missingTarget) {
    console.error('  rustup target add wasm32-unknown-unknown');
  }
  if (missingTrunk) {
    console.error('  cargo install trunk --locked');
  }
  console.error('\nFor docs-only work that does not need the browser app, run:');
  console.error('  npm run start:docs');
  console.error('\nThe /play page then shows a placeholder explaining how to build the app.');
}

function missingPrerequisites() {
  const trunk = spawnSync('trunk', ['--version'], {stdio: 'ignore'});
  const missingTrunk = Boolean(trunk.error) || trunk.status !== 0;

  // Only inspect the installed targets when rustup is available. A Trunk
  // installed outside rustup may still have the target through another
  // toolchain, and the build itself reports that case.
  const rustup = spawnSync('rustup', ['target', 'list', '--installed'], {
    encoding: 'utf8',
  });
  const targetList =
    !rustup.error && rustup.status === 0
      ? (rustup.stdout ?? '')
          .split(/\r?\n/)
          .map((line) => line.trim())
          .filter(Boolean)
      : null;
  const missingTarget = Boolean(
    targetList && !targetList.includes('wasm32-unknown-unknown'),
  );

  return {missingTrunk, missingTarget};
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {stdio: 'inherit', ...options});
  if (result.error?.code === 'ENOENT') {
    setupHelp({missingTrunk: command === 'trunk', missingTarget: false});
    process.exit(1);
  }
  if (result.status !== 0) {
    console.error(`\n${command} ${args.join(' ')} exited with code ${result.status}.`);
    process.exit(result.status ?? 1);
  }
}

const missing = missingPrerequisites();
if (missing.missingTrunk || missing.missingTarget) {
  setupHelp(missing);
  process.exit(1);
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
