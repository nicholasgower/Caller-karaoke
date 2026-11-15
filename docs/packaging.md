# Packaging and releases

The release matrix is defined in `.github/workflows/nightly.yml`.

| Platform | Output | Tooling |
| --- | --- | --- |
| Windows x64 | `-setup.exe`, `-portable.zip` | Inno Setup, Compress-Archive |
| Linux x86_64 | `.deb`, `.AppImage`, `.flatpak` | cargo-deb, appimagetool, flatpak-builder |
| Web | `contra-karaoke-web.zip` | Trunk + wasm-opt |
| Android | `.apk` | cargo-apk, minSdk 26 |

Nightly releases are created on every push to `main` and use version `0.1.0`
with the commit SHA in the release name. The changelog is maintained by
`git-cliff`.

Before a release, run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```
