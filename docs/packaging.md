# Packaging and releases

The release matrix is defined in `.github/workflows/release.yml`.

| Platform | Output | Tooling |
| --- | --- | --- |
| Windows x64 | `-setup.exe`, `-portable.zip` | Inno Setup, Compress-Archive |
| Linux x86_64 | `.deb`, `.AppImage`, `.flatpak` | cargo-deb, appimagetool, flatpak-builder |
| Web | `caller-karaoke-web.zip` | Trunk + wasm-opt |
| Android | `.apk` | cargo-apk, minSdk 26 |

Releases are cut by hand with `node scripts/release.mjs <patch|minor|major|X.Y.Z>`.
The script bumps the workspace version, refreshes `Cargo.lock` and `CHANGELOG.md`,
commits, pushes `main`, and starts `.github/workflows/release.yml`. The workflow
builds every platform and creates the `v<version>` tag and release only when all
of them succeed. Feature work merges into `main`, which is also the source for
releases. The changelog is maintained by `git-cliff`.

Before a release, run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```
