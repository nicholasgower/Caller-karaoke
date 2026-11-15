## 13. Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| CORS blocks web search | No online search in WASM | Proxy + bundled defaults; configurable URL; document deployment |
| ABC parser gaps on messy tunes | Tunes fail to play | Normalize input; keep the previous tune; add fixtures; report source URL |
| Soundfont format/size problems | Long render, high memory | Render offline with progress; session-only web fallback; built-in synth fallback |
| Audio latency/clock drift | Calls visually out of sync | Use cpal playback timestamps; latency compensation; ±20 ms target; manual offset setting if needed |
| Timing formula too rigid | Calls feel early/late | User-adjustable min/max lead, syllables/s, scale; alias lengths; test with a caller |
| ContraDB HTML changes | Parser breaks | Provider isolation, fixtures, cached raw HTML, clear errors |
| Android build complexity | APK missing from nightly | Spike early; isolate JNI picker; allow built-in synth fallback |
| Flatpak/AppImage CI complexity | Missing Linux assets | Separate jobs, pinned tool versions, cached dependencies |
| Content licensing | Legal risk | Bundle only attributed defaults; local cache only; no database redistribution |
| egui mobile keyboard quirks | Poor search UX | Test early; use full-screen sheets and focus management |
| Call variants or thinning feel artificial | The trainer teaches bad habits | Review scripts with an experienced caller; keep every automatic choice visible and overridable; start with conservative defaults |
| Thinning drops a call the learner still needs | Confusion, loss of trust | Keep first move, post-swing move, and unusual figures; offer Rescue; let the learner choose Fixed/Full support |
| Dance-model scope creep | Delays release and distorts the product | Gate all who/where features behind §17 C13; ship the metadata-only checks first |
| Beat-aware TTS is costly to tune | Poor return on effort | Start with the visual rhythm strip and a click track; treat TTS rhythm as P2 |
