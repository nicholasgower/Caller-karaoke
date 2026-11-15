# Packaging

- `windows/` — Inno Setup installer script and portable ZIP instructions.
- `linux/` — `.desktop` file and AppImage build script; `.deb` metadata lives in
  `crates/app/Cargo.toml`.
- `flatpak/` — Flatpak manifest and source-generation notes.
- `android/` — placeholder for the Android packaging steps.
- `web/` — placeholder for the Trunk web build.

CI and nightly workflows consume these files. See `ROADMAP.md` §9.
