## 9. Platforms, packaging, and releases

### 9.1 Targets

| Platform | Build | Artifact |
| --- | --- | --- |
| Windows x64 | `cargo build --release` + Inno Setup | `ContraKaraoke-<version>-x64-setup.exe`, `...-portable.zip` |
| Linux x86_64 | `cargo build --release` | `.deb` via `cargo-deb`, `.AppImage`, `.flatpak` |
| Web | `trunk build --release` + `wasm-opt` | `contra-karaoke-web.zip`, hosted copy under the docs site `/app/` |
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

- Feature work from worktrees and pull requests merges into `dev` first. Test it
  on `dev`; only merge `dev` into `main` when a release is ready.
- `main` is the source for official release builds.
- The version only changes when the owner cuts a release with
  `scripts/release.mjs`.
- Each release carries a `v<version>` tag that matches the workspace version in
  `Cargo.toml`; the release workflow refuses a mismatched tag.
- The changelog keeps an `[Unreleased]` section.
- Release notes are generated from Conventional Commits.

### 9.4 Release checklist

- [ ] `cargo fmt`, `clippy`, and tests pass on Windows and Linux.
- [ ] Documentation screenshots are regenerated in one pass
      (`cd website && npm run screenshots`), reviewed, and committed with updated
      manifest dimensions and emphasis regions.
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
- [ ] No third-party content beyond the attributed defaults is bundled.
