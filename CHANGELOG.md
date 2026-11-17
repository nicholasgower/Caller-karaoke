# Changelog

All notable changes to Caller Karaoke are documented here.

## [Unreleased]

### Chores
- **deny:** Ignore the chrono unmaintained advisory.
- **brand:** Rename the app to Caller Karaoke.
- **legal:** Credit the copyright notices to Nicholas Gower.


### Documentation
- **changelog:** End every entry with a period.
- **website:** Serve the site from contrakaraoke.nicholasgower.com.
- **website:** Lead the browser page with the hosted copy.
- **changelog:** Skip changelog refresh commits.


### Features
- **website:** Add platform-aware download buttons.
- **prefs:** Add update check and feedback actions.
- **web:** Share a dance and tune set as a link.
- **prefs:** Add a settings search to Preferences.
- **website:** Preview the site on iOS and route iOS downloads to the web app.
- **glossary:** Move the dance glossary to its own tab.
- **timing:** Round call leads up and show them on the dance card.


### Fixes
- **release:** Create the tag only after the artifacts exist.
- **android:** Keep the soft keyboard stable for number fields.
- **website:** Remove the GitHub link from the navbar.
- **website:** Restore the unbuilt app fallback and size the mobile app frame.
- **music:** Resolve ending melody a fifth below the final chord.
- **release:** Bump internal crate requirements.
- **release:** Allow for the full cargo metadata output.


### Style
- **app:** Format windows legal copyright metadata.


## [0.1.0] — 2026-11-17

### Build
- Statically link the Windows CRT and satisfy cargo-deny.
- Pin the toolchain and keep formatting sweeps separate.


### CI
- Install trunk from release assets and verify web build.
- Allow the Unicode data license used by unicode-ident.
- Add release checksums and simplify cargo-deb install.
- **nightly:** Stop installing the removed Android tools package.
- **cache:** Only save rust caches from long-lived branches.
- **nightly:** Build nightlies from dev instead of main.
- **nightly:** Publish nightlies from main again.


### Chores
- Satisfy clippy and format workspace.
- Export current-tune MIDI and show source links.
- Remove temporary merge logs.
- Remove a stray SHA256SUMS.txt from the repository root.


### Documentation
- Add README, credits, changelog, packaging, proxy, and CI.
- Record implemented platform status and finish source filters.
- Document Android build and development samples.
- **website:** Add headless screenshots and Docusaurus guide.
- **website:** Rebase guide on main and cover new UI.
- **website:** Document plural actor placeholders.
- **website:** Cover Kokoro voices and pronunciation.
- **website:** Cover alternative call aliases.
- **website:** Document prompt card colour cues.
- **website:** Correct screenshot callout regions.
- **website:** Clarify alias and timing rebuild behavior.
- **website:** Cover the first-call TTS fix.
- **agents:** Require docs updates with code changes.
- **screenshots:** Refresh only when preparing a release.
- **process:** Require dev integration before main.
- **website:** Generate the sidebar from the docs folders.
- **roadmap:** Split the roadmap into chapter files.
- **roadmap:** Add calling and teaching research and backlog.
- **agents:** Require every user input to be MCP-drivable.
- **music:** Document the corrected gallop spacing.
- **website:** Use real Windows installer and portable screenshots.
- Describe the band list, data and storage, and memory paths.
- Document the platform detection helper.
- Describe prefetched playback and performance rules.
- **website:** Serve the documentation site from nicholasgower.
- **release:** Link the download pages at the releases list.


### Features
- **core:** Add dance timing, aliases, roles, and preferences.
- **music:** Parse ABC tunes, infer chords, and arrange 64-beat loops.
- **music:** Add deterministic MIDI export.
- **music:** Add built-in synthesizer, soundfont backend, and offline renderer.
- **app:** Add egui training UI, audio transport, TTS, and source pickers.
- **sources:** Add ContraDB and ABC Session Tunes providers and cache.
- **web:** Add web entry point, localStorage cache, and keyboard shortcuts.
- **web:** Build a runnable WASM app and configure Android packaging.
- **app:** Warn when the dance and tune lengths differ.
- **mcp:** Add agent-driven UI automation server.
- **alias:** Render gypsy as right shoulder round.
- **ui:** Animate tab, screen, and highlight transitions.
- **ui:** Prompt the next move while a figure executes.
- **music:** Vary the accompaniment on each loop.
- **ui:** Render sheet music in the tune preview.
- **ui:** Add keyboard shortcuts and submit-on-enter.
- **aliases:** Add plural actor placeholders.
- **tts:** Add local Kokoro voices and pronunciation review.
- **aliases:** Shuffle alternative call aliases.
- **ui:** Colour the prompt card for call-now and upcoming-move.
- **ui:** Add saved dance and tune libraries with folders.
- **ui:** Add an about window with project links.
- **music:** Add selectable accompaniment rhythms.
- **website:** Host the browser build on the docs site.
- **timing:** Make safety padding configurable.
- **ui:** Adapt the session tab to phone widths.
- **music:** Bundle a trimmed Fluid R3 GM subset as the default voice.
- **music:** Add a second accompaniment for a default trio.
- **session:** Add tune rotation and a slow ending.
- **tts:** Add commanding caller delivery and natural count phrasing.
- **ui:** Optimize every screen for phone widths.
- **app:** Handle media keys while the window is unfocused.
- **music:** Add optional foot-tap and drumset percussion.
- **ui:** Preview a saved card by clicking it.
- **tts:** Rescale the speech rate and lower its maximum.
- **music:** Add a gallop percussion rhythm as the foot-tap default.
- **mcp:** Add session-tab input tools and transport state.
- **icon:** Add the app mark and wire it into every target.
- **tts:** Replace Nicole with Sarah and add Onyx.
- **music:** Emphasize and vary the melody across loop passes.
- **session:** Move the call prompt to the top of the session.
- **music:** Add MIDI effect sends for exported files.
- **music:** Apply effects to in-app playback.
- **android:** Build a signed APK with the TTS bridge.
- **ui:** Remove back-to-session buttons from non-session screens.
- **roles:** Add Larks/Ravens and persist custom role terms.
- **sheet:** Enlarge chord symbols and clear row numbers from the clef.
- **theme:** Give the light theme a warm cream palette.
- **media:** Control Android playback from the system media session.
- **app:** Add swipe and arrow tab changes and scroll-safe sliders.
- **instruments:** Add per-part copies and rename the second accompaniment.
- **app:** Move About into Preferences behind a header gear.
- **ui:** Fill the picker panes to the bottom of the window.
- **audio:** Add sax and clarinet melody choices with a verified font rebuild.
- **music:** Add per-voice human timing with a tempo PID.
- **app:** Add undo and redo for settings and selections.
- **sources:** Add The Caller's Box dance source.
- **prefs:** Add band list, mix, variation, ending, and library foundations.
- **music:** Configure variations, humanize timing, dancer sounds, and endings.
- **library:** Import and export saved tunes and dances with an ABC editor.
- **audio:** Render a user-editable band with caller and band volumes.
- **session:** Nest preferences and make count-in and tempo settings consistent.
- **library:** Reorder folders, folder tune sets, and resizable panes.
- **session:** Make dance card padding compact by default.
- **ui:** Add an in-app dance glossary with linked terms.
- **ui:** Show glossary definitions on term hover and touch hold.
- **ui:** Add a default-on glossary setting.
- **ui:** Merge play and pause into one symbol button.
- **ui:** Swipe tabs only from a touch screen.
- **app:** Add a platform name for runtime feature gating.
- **playback:** Raise human timing to five and expose PID gains.
- **app:** Guard sliders on touch screens.
- **android:** Add local Kokoro neural voice.
- **tts:** Prepare and cache four deliveries per call.
- **android:** Build a signed APK with the TTS bridge.
- **release:** Publish releases only when they are cut by hand.


### Fixes
- **app:** Stop speech when the audio stream is reset.
- **timing:** Calibrate call lead to measured 2.25 second delivery.
- **prefs:** Migrate old lead defaults to the measured calibration.
- **music:** Keep pickups out of the repeated loop.
- **music:** Make the Sandy Boys AABB body a full 64 beats.
- **timing:** Call the first move at the loop end and add 0.5 s padding.
- **sources:** Repair the live ContraDB and ABC Session Tunes searches.
- **ui:** Give the light theme its own card and text palette.
- **app:** Keep playback running when settings change.
- **ui:** Only switch tabs when a number key is pressed.
- **tts:** Call the first move when a dance starts.
- **music:** Normalize raw ABC before parsing.
- **tts:** Stress the first syllable in petronella.
- **app:** Keep the window size when moving between monitors.
- **timing:** Correct count-in to four dance beats.
- **music:** Alternate guitar strum direction.
- **app:** Bring the window to the front when the app starts.
- **music:** Make foot taps audible and add percussion rhythms.
- **music:** Make the gallop a one-and-a triplet.
- **music:** Keep accompaniment clear of the melody register.
- **music:** Use eighth-plus-two-sixteenths for gallop.
- **music:** Halve the gallop foot-tap frequency.
- **ui:** Center the About label with the tab labels.
- **music:** Give the gallop a long first tap and short third.
- **ui:** Match button heights in saved-item rows.
- **android:** Keep the UI inside the system bar insets.
- **docs:** Stage the browser app before the dev server.
- **ui:** Fit the session progress bar to the panel width.
- **app:** Keep music and calls running while the window is minimized.
- **app:** Drop the unused role terms import.
- **session:** Keep the prompt card a constant height.
- **app:** Stop the tab header growing after a window resize.
- **session:** Finalize the in-card next-call footer.
- **session:** Show the next call inside the prompt card.
- **web:** Use a WASM-safe clock so the browser build starts.
- **music:** Keep the copy count loudness-neutral.
- **android:** Pass the activity class to the media bridge.
- **ui:** Restore UTF-8 punctuation after the merge.
- **ui:** Align the phone settings category rows.
- **ui:** Let controls keep the gesture instead of changing tabs.
- **ui:** Anchor the desktop glossary tooltip to the pointer.
- **audio:** Temporarily disable dancer sounds.
- **ui:** Keep buttons as tall as the session Play button.
- **android:** Summon the system keyboard for text fields.
- **packaging:** Resolve the Debian desktop asset from the workspace root.
- **android:** Add the launcher icons the manifest references.
- **packaging:** Give the AppImage its desktop entry and icon.
- **android:** Install the Android targets for the repo toolchain.
- **nightly:** Checksum the release files instead of the artifact folders.
- **links:** Retarget project references from storytold to nicholasgower.
- **roles:** Map lady to robin and gent to lark.
- **ci:** Install the wasm target on the pinned toolchain.
- **release:** Keep appimagetool out of the published artifacts.
- **docs:** Install the wasm target on the pinned toolchain.
- **release:** Snapshot the changelog even when the version is unchanged.


### Performance
- Profile, cache, and parallelize music rendering.
- Profile the new human timing and slider guard paths.
- Profile the touch tab swipe in contra-perf.


### Refactoring
- **app:** Split the screen implementations into app/ui modules.
- **app:** Drive the tabs and screen dispatch from one registry.
- **prefs:** Make preference changes merge-safe.
- **app:** Split the preferences screen into section functions.
- **app:** Group the headless screenshots by screen.
- **app:** Group the picker state into one struct.


### Release
- **release:** V0.1.0.


### Testing
- **samples:** Add curated dance and tune samples.
- **prefs:** Use struct updates for v1 migration fixtures.
- **timing:** Pin the wrapped first-call start beat.
- **app:** Align tab and header tests with the three-tab header.
- **app:** Add reversibility, layout, and memory coverage.
- **app:** Cover the ABC editor screen in the layout matrix.



