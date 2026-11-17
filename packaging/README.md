# Packaging

- `windows/` — Inno Setup installer script and portable ZIP instructions.
- `linux/` — `.desktop` file and AppImage build script; `.deb` metadata lives in
  `crates/app/Cargo.toml`.
- `flatpak/` — Flatpak manifest and source-generation notes.
- `android/` — `cargo-apk` build script, TTS Java bridge, media-session
  activity, and launcher icons in `android/res/mipmap-*`.
- `web/` — placeholder for the Trunk web build.

## Media controls

Every packaged build registers with the operating system's media session, so the
hardware play/pause key and the system media widget control the transport while
the app is in the background. The integration has one sibling per platform:

| Platform | Session | Lives in |
| --- | --- | --- |
| Windows | System Media Transport Controls | `crates/app/src/media_keys.rs` (`souvlaki`, window handle from `raw-window-handle`) |
| Linux | MPRIS (`org.callerkaraoke.CallerKaraoke`) | `crates/app/src/media_keys.rs` (`souvlaki` with the zbus backend) |
| Android | `android.media.session.MediaSession` + MediaStyle notification | `packaging/android/java/org/callerkaraoke/app/MainActivity.java`, bridged by `crates/app/src/media_keys.rs` |

The session shows the loaded dance as the title, the tune as the artist, and
*Caller Karaoke* as the album. Keep the three in step when the metadata or the
transport commands change: they all map onto `MediaCommand`.

Platform icons and the source SVG are shared from `assets/icons/`. See
`docs/roadmap/ui-and-visual-design.md` §8.5. CI and release workflows consume
these files. See `ROADMAP.md` §9.
