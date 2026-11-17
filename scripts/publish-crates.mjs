#!/usr/bin/env node
/**
 * Publish the library crates to crates.io.
 *
 * Publishes `caller-core` and then `caller-sources`, in that order: the sources
 * manifest requires the same version of `caller-core` from the registry, and
 * cargo strips the `path` on upload.
 *
 * Usage:
 *   node scripts/publish-crates.mjs                       # dry run both crates
 *   node scripts/publish-crates.mjs --yes                 # upload both crates
 *   node scripts/publish-crates.mjs --env-file path/.env  # read the token here
 *   node scripts/publish-crates.mjs --allow-branch        # publish off `main`
 *   node scripts/publish-crates.mjs --help
 *
 * The script refuses a dirty working tree, a branch other than `main`, and a
 * missing `v<version>` tag, because a crates.io version can only be yanked, not
 * replaced. A version that is already on crates.io is skipped, so a rerun after
 * a partial failure is safe.
 *
 * The token comes from `CARGO_REGISTRY_TOKEN` (or `CRATES_API_KEY`) in the
 * environment, or from a line with either name in `--env-file` (default `.env`
 * in the repository root). It is passed to cargo only and never printed. Real
 * uploads only: a dry run needs no token.
 */
import {spawnSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const CARGO_TOML = path.join(ROOT, 'Cargo.toml');
const USER_AGENT =
  'caller-karaoke-publish (https://github.com/nicholasgower/Caller-karaoke)';
/**
 * Publish order, not a set: `caller-sources` depends on the same version of
 * `caller-core` from the registry, so core has to be visible first.
 */
const CRATES = ['caller-core', 'caller-sources'];
/** `CRATES_API_KEY` is the name the owner's local file uses. */
const TOKEN_KEYS = ['CARGO_REGISTRY_TOKEN', 'CRATES_API_KEY'];

function fail(message) {
  console.error(`publish: ${message}`);
  process.exit(1);
}

function note(message) {
  console.log(`publish: ${message}`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function usage() {
  console.log(
    [
      'Usage: node scripts/publish-crates.mjs [--yes] [--env-file PATH] [--allow-branch]',
      '',
      '  --yes            upload to crates.io (default: cargo publish --dry-run)',
      '  --env-file PATH  read CARGO_REGISTRY_TOKEN from PATH (default: .env)',
      '  --allow-branch   skip the main-branch check',
      '  --help           print this message',
    ].join('\n'),
  );
}

function parseArgs(argv) {
  const options = {
    yes: false,
    allowBranch: false,
    envFile: path.join(ROOT, '.env'),
    help: false,
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--yes') options.yes = true;
    else if (arg === '--dry-run') options.yes = false;
    else if (arg === '--allow-branch') options.allowBranch = true;
    else if (arg === '--env-file') {
      const value = argv[i + 1];
      if (!value) fail('--env-file needs a path');
      options.envFile = path.resolve(value);
      i += 1;
    } else if (arg === '--help' || arg === '-h') options.help = true;
    else fail(`unknown argument "${arg}"`);
  }
  return options;
}

function run(cmd, args, {capture = false, env = process.env} = {}) {
  const result = spawnSync(cmd, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: capture ? 'pipe' : 'inherit',
    env,
    // A packaged build prints dependency compilation, which passes the 1 MiB
    // default capture buffer on a cold cache.
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) fail(`cannot run ${cmd}: ${result.error.message}`);
  return {
    status: result.status,
    stdout: (result.stdout ?? '').trim(),
    stderr: (result.stderr ?? '').trim(),
  };
}

function git(args) {
  const result = run('git', args, {capture: true});
  return {status: result.status, stdout: result.stdout};
}

function readVersion() {
  const lines = fs.readFileSync(CARGO_TOML, 'utf8').split('\n');
  const start = lines.findIndex((line) => line.trim() === '[workspace.package]');
  if (start < 0) fail('Cargo.toml has no [workspace.package] section');
  for (let i = start + 1; i < lines.length && !lines[i].startsWith('['); i += 1) {
    const match = lines[i].match(/^version\s*=\s*"([^"]+)"/);
    if (match) return match[1];
  }
  fail('Cargo.toml has no version in [workspace.package]');
}

/** Minimal KEY=VALUE reader. Values are returned, never logged. */
function readEnvFile(file) {
  if (!fs.existsSync(file)) return {};
  const values = {};
  for (const line of fs.readFileSync(file, 'utf8').split(/\r?\n/)) {
    const match = line.match(
      /^\s*(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$/,
    );
    if (!match) continue;
    let value = match[2];
    const quoted =
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"));
    if (quoted && value.length >= 2) value = value.slice(1, -1);
    values[match[1]] = value;
  }
  return values;
}

/** True/false when crates.io answers, undefined when the network does not. */
async function publishedVersion(name, version) {
  try {
    const response = await fetch(
      `https://crates.io/api/v1/crates/${name}/${version}`,
      {headers: {'User-Agent': USER_AGENT}},
    );
    if (response.status === 200) return true;
    if (response.status === 404) return false;
  } catch {
    // Offline. Let cargo make the decision and report the real error.
  }
  return undefined;
}

/** Sparse-index path for a crate name, e.g. ca/ll/caller-core. */
function indexPath(name) {
  const lower = name.toLowerCase();
  if (lower.length === 1) return `1/${lower}`;
  if (lower.length === 2) return `2/${lower}`;
  if (lower.length === 3) return `3/${lower[0]}/${lower}`;
  return `${lower.slice(0, 2)}/${lower.slice(2, 4)}/${lower}`;
}

async function indexHas(name, version) {
  try {
    const response = await fetch(`https://index.crates.io/${indexPath(name)}`, {
      headers: {'User-Agent': USER_AGENT},
    });
    if (!response.ok) return false;
    const body = await response.text();
    return body.split('\n').some((line) => line.includes(`"vers":"${version}"`));
  } catch {
    return false;
  }
}

async function waitForIndex(name, version, timeoutMs = 180_000) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    if (await indexHas(name, version)) return true;
    if (Date.now() >= deadline) return false;
    await sleep(3000);
  }
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    usage();
    return;
  }

  const version = readVersion();
  const tag = `v${version}`;

  const branch = git(['branch', '--show-current']).stdout;
  if (!options.allowBranch && branch !== 'main') {
    fail(`on branch "${branch || 'detached HEAD'}"; publish from main or pass --allow-branch`);
  }

  if (git(['status', '--porcelain']).stdout) {
    fail('working tree is dirty; commit the change so the published source matches the tag');
  }

  const tagCheck = git(['rev-parse', '--verify', '--quiet', `refs/tags/${tag}`]);
  if (tagCheck.status !== 0) {
    fail(`tag ${tag} does not exist; crates.io versions are immutable, so publish a tagged release`);
  }
  const head = git(['rev-parse', 'HEAD']).stdout;
  const tagCommit = git(['rev-list', '-n', '1', tag]).stdout;
  if (head !== tagCommit) {
    note(`HEAD is not ${tag} (typically a changelog commit after it); publishing the current tree`);
  }

  let token = TOKEN_KEYS.map((key) => process.env[key]).find(Boolean);
  if (options.yes && !token) {
    const envFile = readEnvFile(options.envFile);
    token = TOKEN_KEYS.map((key) => envFile[key]).find(Boolean);
    if (!token) {
      fail(
        `no ${TOKEN_KEYS.join(' or ')} in the environment or ${options.envFile}; run "cargo login" or add one to the file`,
      );
    }
  }

  note(`${options.yes ? 'publishing' : 'dry run for'} ${CRATES.join(', ')} ${version}`);

  let coreReady = (await publishedVersion(CRATES[0], version)) === true;

  for (const name of CRATES) {
    const state = await publishedVersion(name, version);
    if (state === true) {
      note(`${name} ${version} is already on crates.io; skipping`);
      if (name === CRATES[0]) coreReady = true;
      continue;
    }
    if (state === undefined) {
      note(`could not reach crates.io to check ${name}; cargo will decide`);
    }
    if (!options.yes && name !== CRATES[0] && !coreReady) {
      note(
        `skipping ${name}: ${CRATES[0]} ${version} is not on crates.io yet, so cargo cannot resolve it`,
      );
      continue;
    }

    const args = ['publish', '-p', name];
    if (!options.yes) args.push('--dry-run');
    note(`cargo ${args.join(' ')}`);
    const result = run('cargo', args, {
      env: token ? {...process.env, CARGO_REGISTRY_TOKEN: token} : process.env,
    });
    if (result.status !== 0) {
      fail(`cargo ${args.join(' ')} failed`);
    }

    if (options.yes) {
      note(`waiting for ${name} ${version} to reach index.crates.io`);
      if (!(await waitForIndex(name, version))) {
        note(
          `index.crates.io does not show ${name} ${version} yet; a rerun skips it once it does`,
        );
      }
      if (name === CRATES[0]) coreReady = true;
    }
  }

  note(
    options.yes
      ? 'done; both crates are on crates.io'
      : 'dry run complete; re-run with --yes to upload',
  );
}

await main();
