# Packaging

- `windows/` — Inno Setup installer script and portable ZIP instructions.
- `linux/` — `.desktop` file and AppImage build script; `.deb` metadata lives in
  `crates/app/Cargo.toml`.
- `flatpak/` — Flatpak manifest and source-generation notes.
- `android/` — `cargo-apk` build script, TTS Java bridge, and launcher icons in
  `android/res/mipmap-*`.
- `web/` — placeholder for the Trunk web build.

Platform icons and the source SVG are shared from `assets/icons/`. See
`docs/roadmap/ui-and-visual-design.md` §8.5. CI and nightly workflows consume
these files. See `ROADMAP.md` §9.
