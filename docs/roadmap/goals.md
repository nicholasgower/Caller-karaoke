## 1. Goals

1. Let a caller practise a real contra dance against real ABC tune playback.
2. Show calls early enough that dancers hear them before they must move.
3. Make the caller's spoken wording adaptable through aliases and role terms.
4. Work offline after content has been cached.
5. Keep dance/tune parsing replaceable so new sources can be added without
   touching the session engine or UI.
6. Ship as a small, MIT-licensed application with reproducible nightly builds.

### Two audiences

The app serves two related goals. The first is to train a human caller: show the
call early, teach the dance, model good wording, and let the learner thin the
calls as they gain confidence. The second is to call a dance on its own with the
judgment of a competent caller; TTS already speaks the calls in 0.1.0, while the
judgment (variants, thinning, endings) is post-0.1.0. Both use the same timeline
and the same rendered calls; §17 describes the shared design and the staged
work.

The human-caller goal sets the 0.1.0 scope. Autonomous calling must not turn the
trainer into a machine that does the learning for the caller. Features that
depend on choreography stay behind the dance-model gate in §17.

### Non-goals for 0.1.0

- Authoring or editing dances and tunes inside the app.
- Multi-user accounts, cloud sync, or telemetry.
- Recording or scoring the caller's voice.
- iOS/macOS releases (the code should remain portable, but these are not
  release targets yet).
- Bundling a full general-purpose soundfont bank, or redistributing a
  third-party tune or dance database.
