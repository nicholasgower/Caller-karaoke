## 1. Goals

1. Let a caller practise a real contra dance against real ABC tune playback.
2. Show calls early enough that dancers hear them before they must move.
3. Make the caller's spoken wording adaptable through aliases and role terms.
4. Work offline after content has been cached.
5. Keep dance/tune parsing replaceable so new sources can be added without
   touching the session engine or UI.
6. Ship as a small, MIT-licensed application with reproducible nightly builds.

### Non-goals for 0.1.0

- Authoring or editing dances and tunes inside the app.
- Multi-user accounts, cloud sync, or telemetry.
- Recording or scoring the caller's voice.
- iOS/macOS releases (the code should remain portable, but these are not
  release targets yet).
- Bundling a general-purpose soundfont or redistributing a third-party tune or
  dance database.
