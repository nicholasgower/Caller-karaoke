## 9. Platforms, packaging, and releases

### 9.1 Targets

| Platform | Build | Artifact |
| --- | --- | --- |
| Windows x64 | `cargo build --release` + Inno Setup | `CallerKaraoke-<version>-x64-setup.exe`, `...-portable.zip` |
| Linux x86_64 | `cargo build --release` | `.deb` via `cargo-deb`, `.AppImage`, `.flatpak` |
| Web | `trunk build --release` + `wasm-opt` | `caller-karaoke-web.zip`, hosted copy under the docs site `/app/` |
| Android | `python packaging/android/build-apk.py` (wraps `cargo apk`) | universal APK (arm64 + armv7), minSdk 26 |

Notes:

- Windows: Inno Setup for the installer; ZIP is portable (no registry writes).
- Linux: AppImage via `linuxdeploy`/`appimagetool`; Flatpak manifest under
  `packaging/flatpak/`; `.deb` metadata under `packaging/deb/`.
- Web: `eframe` with the `glow` renderer; WebGL2 is the baseline.
- Android: `eframe` with `android-native-activity` and `glow`; `cpal` uses
  AAudio; `tts` requires minSdk 26. The packaging script adds the pinned ONNX
  Runtime `libonnxruntime.so` for the local Kokoro voice.
- No macOS/iOS artifacts in 0.1.0, but do not add platform-specific code that
  would prevent them.

### 9.2 CI

**`ci.yml`** on pull requests and pushes to `main` or `dev`:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo check --target wasm32-unknown-unknown` for the web crate
- Android check/build smoke test
- `cargo deny check` with an allowlist of permissive licenses
- `cargo machete`

**`release.yml`** by hand with `workflow_dispatch`, started by `scripts/release.mjs`:

- Build all release targets.
- Write release notes with the version, commit, and build time.
- Create the `v<version>` tag at the built commit and publish the GitHub release.
- Upload all artifacts with checksums.
- Publish `caller-core` and then `caller-sources` to crates.io from the tagged
  commit, in a final `crates` job (see §9.4).

Releases are cut with `node scripts/release.mjs <patch|minor|major|X.Y.Z>`, which
bumps the workspace version, commits, and pushes `main`. The workflow creates the
tag only after every build succeeds, so it never points at an unbuilt commit.

**`changelog.yml`** on push to `main`:

- Regenerate `CHANGELOG.md` with `git-cliff`.
- Commit with `[skip ci]` and path filters to avoid loops.
- Release builds ignore changelog-only commits.

**`docs.yml`** builds and deploys the documentation site to GitHub Pages. It
also builds the WASM app and stages it under `/app/` so the guide can embed a
hosted copy.

### 9.3 Versioning and branches

- Feature work from worktrees and pull requests merges into `main`.
- `main` is the source for official release builds.
- The version only changes when the owner cuts a release with
  `scripts/release.mjs`.
- Each release carries a `v<version>` tag that matches the workspace version in
  `Cargo.toml`; the release workflow refuses a mismatched tag.
- The changelog keeps an `[Unreleased]` section.
- Release notes are generated from Conventional Commits.

### 9.4 Crate publishing

`caller-core` and `caller-sources` are published to crates.io from this
workspace; keeping them in the repository needs no split. `caller-sources`
requires the same version of `caller-core` from the registry, so the order is
fixed: core first, then sources.

- `release.yml` has a final `crates` job that runs after the tag and release
  exist, checks out the tag, and runs
  `node scripts/publish-crates.mjs --yes --allow-branch` with the
  `CARGO_REGISTRY_TOKEN` repository secret. Without that secret the job fails
  with a message instead of silently skipping the crates.
- Publishing last is deliberate: a crates.io version is immutable, so a crate
  must never appear for a version whose platform builds failed. A failed publish
  job leaves the release intact and can be re-run; the script skips versions
  already on crates.io.
- Run the script by hand only as a fallback when CI is unavailable: from `main`,
  `node scripts/publish-crates.mjs` dry-runs both crates and `--yes` uploads.
- The script refuses a dirty tree, a branch other than `main` (CI passes
  `--allow-branch` for the detached tag checkout), or a missing tag, and reads
  `CARGO_REGISTRY_TOKEN` (or `CRATES_API_KEY`) from the environment or
  `--env-file` (default `.env`) without printing it.
- A published version is immutable: yanking is the only undo. Never upload from
  an untagged or dirty commit.

### 9.5 Release checklist

- [ ] `cargo fmt`, `clippy`, and tests pass on Windows and Linux.
- [ ] Documentation screenshots are regenerated in one pass
      (`cd website && npm run screenshots`), reviewed, and committed with updated
      manifest dimensions.
- [ ] Web build loads and plays audio in Chrome and Firefox, both from the
      downloaded bundle and from the docs site `/play` page.
- [ ] A web share link (`#set=…`) opens the same dance and tune set in a fresh
      browser profile, and a damaged link leaves the default session in place.
- [ ] Android APK installs and plays audio on a real device.
- [ ] Default dance and tune load offline.
- [ ] Proxy-backed search works in the web build.
- [ ] SF2 loading works on Windows, Linux, and web; Android path is verified or
      documented as a known limitation.
- [ ] `.deb`, AppImage, Flatpak, installer, portable ZIP, web ZIP, and APK are
      attached to the release.
- [ ] `CREDITS.md`, `LICENSE`, and `CHANGELOG.md` are present in the artifacts
      where applicable.
- [ ] The `crates` job published `caller-core` and `caller-sources` at the
      workspace version (re-run the job if it failed).
- [ ] No third-party content beyond the attributed defaults is bundled.
