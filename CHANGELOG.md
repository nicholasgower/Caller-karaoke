# Changelog

All notable changes to Contra Karaoke are documented here.

## [Unreleased]

### Build
- Statically link the Windows CRT and satisfy cargo-deny


### CI
- Install trunk from release assets and verify web build
- Allow the Unicode data license used by unicode-ident
- Add release checksums and simplify cargo-deb install
- **nightly:** Stop installing the removed Android tools package
- **cache:** Only save rust caches from long-lived branches


### Chores
- Satisfy clippy and format workspace
- Export current-tune MIDI and show source links


### Documentation
- Add README, credits, changelog, packaging, proxy, and CI
- Record implemented platform status and finish source filters
- Document Android build and development samples
- **website:** Add headless screenshots and Docusaurus guide
- **website:** Rebase guide on main and cover new UI
- **website:** Document plural actor placeholders
- **website:** Cover Kokoro voices and pronunciation
- **website:** Cover alternative call aliases
- **website:** Document prompt card colour cues
- **website:** Correct screenshot callout regions
- **website:** Clarify alias and timing rebuild behavior
- **website:** Cover the first-call TTS fix
- **agents:** Require docs updates with code changes
- **screenshots:** Refresh only when preparing a release
- Update changelog [skip ci]
- Update changelog [skip ci]
- Update changelog [skip ci]
- Update changelog [skip ci]


### Features
- **core:** Add dance timing, aliases, roles, and preferences
- **music:** Parse ABC tunes, infer chords, and arrange 64-beat loops
- **music:** Add deterministic MIDI export
- **music:** Add built-in synthesizer, soundfont backend, and offline renderer
- **app:** Add egui training UI, audio transport, TTS, and source pickers
- **sources:** Add ContraDB and ABC Session Tunes providers and cache
- **web:** Add web entry point, localStorage cache, and keyboard shortcuts
- **web:** Build a runnable WASM app and configure Android packaging
- **app:** Warn when the dance and tune lengths differ
- **mcp:** Add agent-driven UI automation server
- **alias:** Render gypsy as right shoulder round
- **ui:** Animate tab, screen, and highlight transitions
- **ui:** Prompt the next move while a figure executes
- **music:** Vary the accompaniment on each loop
- **ui:** Render sheet music in the tune preview
- **ui:** Add keyboard shortcuts and submit-on-enter
- **aliases:** Add plural actor placeholders
- **tts:** Add local Kokoro voices and pronunciation review
- **aliases:** Shuffle alternative call aliases
- **ui:** Colour the prompt card for call-now and upcoming-move
- **ui:** Add an about window with project links
- **android:** Build a signed APK with the TTS bridge


### Fixes
- **app:** Stop speech when the audio stream is reset
- **timing:** Calibrate call lead to measured 2.25 second delivery
- **prefs:** Migrate old lead defaults to the measured calibration
- **music:** Keep pickups out of the repeated loop
- **music:** Make the Sandy Boys AABB body a full 64 beats
- **timing:** Call the first move at the loop end and add 0.5 s padding
- **sources:** Repair the live ContraDB and ABC Session Tunes searches
- **ui:** Give the light theme its own card and text palette
- **app:** Keep playback running when settings change
- **ui:** Only switch tabs when a number key is pressed
- **tts:** Call the first move when a dance starts
- **music:** Normalize raw ABC before parsing
- **tts:** Stress the first syllable in petronella
- **packaging:** Resolve the Debian desktop asset from the workspace root
- **android:** Add the launcher icons the manifest references
- **packaging:** Give the AppImage its desktop entry and icon
- **android:** Install the Android targets for the repo toolchain
- **nightly:** Checksum the release files instead of the artifact folders


### Testing
- **samples:** Add curated dance and tune samples
- **prefs:** Use struct updates for v1 migration fixtures
- **timing:** Pin the wrapped first-call start beat



