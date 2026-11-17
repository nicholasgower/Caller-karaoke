#!/usr/bin/env node
/**
 * Cut a release.
 *
 * Bumps the workspace version in `Cargo.toml`, refreshes `Cargo.lock` and the
 * changelog, commits the result, tags `v<version>`, and pushes the commit and
 * the tag. The tag starts `.github/workflows/release.yml`, which builds every
 * platform and publishes the GitHub release.
 *
 * Usage:
 *   node scripts/release.mjs patch              # 0.1.0 -> 0.1.1
 *   node scripts/release.mjs minor              # 0.1.0 -> 0.2.0
 *   node scripts/release.mjs major              # 0.1.0 -> 1.0.0
 *   node scripts/release.mjs 0.1.0              # tag the current version as-is
 *   node scripts/release.mjs patch --dry-run    # print the plan, change nothing
 *   node scripts/release.mjs patch --no-push    # commit and tag, push nothing
 *
 * The script refuses to run on a branch other than `main`, with a dirty working
 * tree, behind `origin/main`, or when the tag already exists. `--allow-branch`
 * skips the branch and up-to-date checks.
 */
import {spawnSync} from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const CARGO_TOML = path.join(ROOT, 'Cargo.toml');

function fail(message) {
  console.error(`release: ${message}`);
  process.exit(1);
}

function run(cmd, args, {capture = false} = {}) {
  const result = spawnSync(cmd, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: capture ? 'pipe' : 'inherit',
  });
  if (result.error) fail(`cannot run ${cmd}: ${result.error.message}`);
  if (result.status !== 0) {
    fail(`${cmd} ${args.join(' ')} exited with ${result.status}`);
  }
  return capture ? (result.stdout ?? '').trim() : '';
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

function writeVersion(next) {
  const lines = fs.readFileSync(CARGO_TOML, 'utf8').split('\n');
  const start = lines.findIndex((line) => line.trim() === '[workspace.package]');
  for (let i = start + 1; i < lines.length && !lines[i].startsWith('['); i += 1) {
    if (/^version\s*=\s*"/.test(lines[i])) {
      lines[i] = lines[i].replace(/"[^"]*"/, `"${next}"`);
      fs.writeFileSync(CARGO_TOML, lines.join('\n'));
      return;
    }
  }
  fail('Cargo.toml has no version in [workspace.package]');
}

function nextVersion(current, spec) {
  const match = current.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) fail(`cannot parse version "${current}" in Cargo.toml`);
  const [major, minor, patch] = match.slice(1).map(Number);
  if (spec === 'major') return `${major + 1}.0.0`;
  if (spec === 'minor') return `${major}.${minor + 1}.0`;
  if (spec === 'patch') return `${major}.${minor}.${patch + 1}`;
  if (/^\d+\.\d+\.\d+$/.test(spec)) return spec;
  fail(`expected patch, minor, major, or X.Y.Z, got "${spec}"`);
}

function hasGitCliff() {
  const result = spawnSync('git-cliff', ['--version'], {cwd: ROOT, stdio: 'ignore'});
  return !result.error && result.status === 0;
}

const argv = process.argv.slice(2);
const flags = new Set(argv.filter((arg) => arg.startsWith('--')));
const positional = argv.filter((arg) => !arg.startsWith('--'));
if (positional.length !== 1) {
  fail('usage: node scripts/release.mjs <patch|minor|major|X.Y.Z> [--dry-run] [--no-push] [--allow-branch]');
}
const dryRun = flags.has('--dry-run');
const noPush = flags.has('--no-push');
const allowBranch = flags.has('--allow-branch');
const known = new Set(['--dry-run', '--no-push', '--allow-branch']);
for (const flag of flags) {
  if (!known.has(flag)) fail(`unknown option ${flag}`);
}

const current = readVersion();
const next = nextVersion(current, positional[0]);
const tag = `v${next}`;

if (run('git', ['status', '--porcelain'], {capture: true}) !== '') {
  fail('the working tree is not clean; commit or stash first');
}

const branch = run('git', ['rev-parse', '--abbrev-ref', 'HEAD'], {capture: true});
if (branch !== 'main' && !allowBranch) {
  fail(`releases are cut from main, not ${branch} (use --allow-branch to override)`);
}

run('git', ['fetch', 'origin', 'main'], {capture: true});
const head = run('git', ['rev-parse', 'HEAD'], {capture: true});
const upstream = run('git', ['rev-parse', 'origin/main'], {capture: true});
if (head !== upstream && !allowBranch) {
  fail('HEAD is not at origin/main; pull or merge first (use --allow-branch to override)');
}

const existingTag = spawnSync('git', ['rev-parse', '-q', '--verify', `refs/tags/${tag}`], {
  cwd: ROOT,
  stdio: 'ignore',
});
if (existingTag.status === 0) fail(`tag ${tag} already exists`);

const changed = next !== current;
console.log(`release: ${current} -> ${next}${changed ? '' : ' (version unchanged)'}`);
console.log(`release: branch ${branch}, tag ${tag}${dryRun ? ', dry run' : ''}`);

if (dryRun) {
  if (changed) {
    console.log(`release: would set [workspace.package] version to ${next}`);
    console.log('release: would refresh Cargo.lock and CHANGELOG.md');
    console.log(`release: would commit "chore(release): ${tag}" and tag ${tag}`);
  } else {
    console.log(`release: would tag the current commit as ${tag}`);
  }
  console.log(noPush ? 'release: would not push' : `release: would push main and ${tag} to origin`);
  process.exit(0);
}

if (changed) {
  writeVersion(next);
  run('cargo', ['metadata', '--format-version', '1'], {capture: true});
  if (hasGitCliff()) {
    run('git-cliff', ['--tag', tag, '--output', 'CHANGELOG.md']);
  } else {
    console.warn('release: git-cliff is not installed; leaving CHANGELOG.md alone');
  }
  const paths = ['Cargo.toml', 'Cargo.lock', 'CHANGELOG.md'].filter((file) =>
    fs.existsSync(path.join(ROOT, file)),
  );
  run('git', ['add', ...paths]);
  run('git', ['commit', '-m', `chore(release): ${tag}`]);
}

run('git', ['tag', '-a', tag, '-m', `Contra Karaoke ${tag}`]);

if (noPush) {
  console.log(`release: committed and tagged ${tag}; push with:`);
  console.log(`release:   git push origin main && git push origin ${tag}`);
} else {
  run('git', ['push', 'origin', 'main']);
  run('git', ['push', 'origin', tag]);
  console.log(`release: pushed ${tag}; .github/workflows/release.yml is building it`);
}
