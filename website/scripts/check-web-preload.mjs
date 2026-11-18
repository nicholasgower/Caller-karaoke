#!/usr/bin/env node
// Guards the hover preload of the hosted browser app. The part that turns
// Trunk's generated index.html into asset URLs is pure, so it runs here without
// a browser or a built app: `npm test` calls this script.
//
// The module is transpiled in memory instead of adding a TypeScript test runner
// for one file. It has no imports, which keeps the data: URL import resolvable.

import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import {fileURLToPath} from 'node:url';

import ts from 'typescript';

const here = path.dirname(fileURLToPath(import.meta.url));
const sourceFile = path.join(here, '..', 'src', 'webAppAssets.ts');
const {outputText} = ts.transpileModule(
  fs.readFileSync(sourceFile, 'utf8'),
  {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
    },
  },
);
const moduleUrl = `data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`;
const {webAppAssetUrls} = await import(moduleUrl);

const origin = 'https://callerkaraoke.example';
const appUrl = `${origin}/app/`;

// The shape Trunk writes: a module preload for the JS and a fetch preload for
// the WASM, both hashed.
const built = `<!doctype html><html><head>
<link rel="modulepreload" href="/app/contra-web-abc123.js" crossorigin="anonymous">
<link rel="preload" href="/app/contra-web-def456_bg.wasm" as="fetch" type="application/wasm" crossorigin="anonymous">
</head><body><canvas></canvas>
<script type="module">
import init from '/app/contra-web-abc123.js';
await init({module_or_path: '/app/contra-web-def456_bg.wasm'});
</script></body></html>`;

assert.deepEqual(
  webAppAssetUrls(built, appUrl, origin).sort(),
  [
    `${origin}/app/contra-web-abc123.js`,
    `${origin}/app/contra-web-def456_bg.wasm`,
  ],
  'the hashed JavaScript and WASM are fetched once each',
);

// The tracked placeholder explains how to build the app and loads no assets;
// a hover before the build stages the app must fetch nothing.
const placeholder = `<!doctype html><html><head>
<link rel="icon" type="image/svg+xml" href="caller-karaoke.svg">
</head><body><p>The browser app has not been built</p></body></html>`;

assert.deepEqual(
  webAppAssetUrls(placeholder, appUrl, origin),
  [],
  'the placeholder index.html has no assets to warm',
);

// Relative paths resolve against the app directory, and nothing outside the
// site or of another file type is pulled in.
const mixed = [
  '<script src="./relative.js"></script>',
  '<script src="https://cdn.example/elsewhere.js"></script>',
  '<link rel="stylesheet" href="/app/app.css">',
  '<script src="/app/relative.js"></script>',
].join('\n');

assert.deepEqual(
  webAppAssetUrls(mixed, appUrl, origin),
  [`${origin}/app/relative.js`],
  'only same-origin scripts and WASM are warmed, without duplicates',
);

console.log('web app preload: ok');
