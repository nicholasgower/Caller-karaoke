## 16. Open items

- Final proxy deployment host/domain and token policy.
- Data proxy URL to bake into the hosted web copy for online search.
- Android SF2 picker: JNI intent vs. app-folder import fallback.
- Review of default alias templates, call variants, and teaching scripts with
  an experienced caller.
- Permission/attribution wording for the bundled default dance and tune.
- Windows code signing (not required for these test builds; note SmartScreen
  warnings).
- A simplified 16/32 px app icon variant; the detailed mark (four couples,
  set links) turns into a pattern below 32 px.
- Final visual polish.
- Flatpak runtime/SDK version pinning.
- Autonomous-caller default support level and whether performance mode is a
  separate setting (§17 C5/C11).
- Smallest useful dance model and the gate for who/where features (§17 C13).
- How to evaluate "good calling" without a room: recordings, expert review, and
  finish-before-the-figure checks at 224, 232, and 240 BPM (§17 C10).
- Whether beat-aware TTS is worth the complexity or a visual rhythm strip plus
  a click track is enough (§17 C11).
- Dancer sounds: the derived rhythms do not match what dancers actually do.
  Claps (balance, figures that say "clap") and foot stomps (petronella, long
  lines) are both disabled in 0.1.0 behind `DANCER_SOUNDS_ENABLED` in
  `crates/core/src/dancer.rs`; foot stomps also keep `FOOT_STOMPS_ENABLED` off.
  Collect the real per-move rhythms, cover them with timing tests, then
  re-enable the layer (§6.3).
