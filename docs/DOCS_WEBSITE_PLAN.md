# Documentation Website and Automated Screenshot Plan

**Status:** documentation complete; 69 in-app screenshots rendered headlessly
**Baseline:** local `main` at `108e1de`
**Date:** 2026-11-14
**Owner:** docs track, running alongside app generation
**Targets:** Docusaurus 3.10 · Node 24 LTS · `egui`/`eframe` 0.37 · `egui_kittest` 0.37 · Rust 1.95
**Applies to:** Contra Karaoke 0.1.0 nightly

This plan describes a public GitHub Pages documentation site in which every
in-app instruction is paired with a screenshot generated from the real
application. It replaces the first draft with a version aligned to the code
that now exists on `main`.

> **Screenshot cadence (2026-11-15):** Screenshots are regenerated in one pass
> when preparing a release, not in every feature worktree. Prose and the
> manifest still change with the code. CI validates the manifest and builds the
> site; it does not regenerate PNGs. Sections below that describe a per-PR
> screenshot diff workflow are superseded by this decision.

---

## 0. Baseline after the rebase

`main` now contains the first complete application slice, not just the plan:

| Area | Current code |
| --- | --- |
| Entry point | `crates/app/src/main.rs` — native `eframe` bootstrap |
| Application state | `crates/app/src/app.rs` — `ContraApp`, `eframe::App::ui`, startup I/O, source events, playback, preferences |
| Screens | `crates/app/src/ui.rs` — included into `app.rs`; session, dance picker, tune picker, preferences, aliases |
| Audio | `crates/app/src/audio.rs` — concrete `cpal` engine with `silent()` fallback |
| Sources | `crates/app/src/source.rs` — concrete `ehttp` client, cache, proxy |
| TTS | `crates/app/src/tts.rs` — concrete system `tts` wrapper |
| Theme | `crates/app/src/theme.rs` — dark/light grey visuals |
| Defaults | `crates/app/src/defaults.rs` — embedded *Another Last Dance* and *Sandy Boys* |
| Sources crate | `crates/sources` — provider traits, ContraDB and ABC Session Tunes parsers, cache |
| Music crate | `crates/music` — ABC parser, arranger, synth, SF2, MIDI, offline renderer |
| Tests/samples | `samples/` — curated third-party parser fixtures; not documentation content |

### 0.1 What changed from the first draft

- The app now has `crates/app/src/lib.rs` with shared desktop, web, and Android
  entry points, `crates/web` for the Trunk/WASM page, and `crates/mcp` plus an
  `inspection` feature for live agent automation.
- `ContraApp::new` still reads real config/cache directories, opens the audio
  device, initializes system TTS, and may read a soundfont at startup.
  `ContraApp::new_headless` and `contra-headless` provide the deterministic
  render path used for documentation.
- Playback is a concrete `AudioEngine`; the headless build adds a silent clock
  and seek helpers, but there is no general playback trait.
- Sources are a concrete `SourceClient` with hard-coded providers and `ehttp`.
  Headless scenarios seed results/previews directly instead of using a fixture
  provider.
- Keyboard shortcuts are implemented: Space play/pause, R restart, Esc return
  to the session. The dance picker now exposes user, hook, and figure-type
  filters, and previews link to ContraDB / ABC Session Tunes.
- The session now warns when the dance and tune lengths differ, and the
  arranger warnings are visible after rendering. The UI still lacks page-size
  selection, an offline banner, a “preparing music” progress state, cache
  size/refresh, TTS voice selection, and SoundFont fallback messaging.
- Many inputs are not associated with their labels, and several labels repeat
  (`Preview`, `Reset to default`). The headless renderer disables the automatic
  accessibility check; a label pass remains future work.
- The branch is rebased onto `main` at `108e1de`. The `mcp` crate and
  `inspection` feature add a live-app automation path; the docs screenshots use
  the separate headless binary instead.

The Docusaurus, Pages, and `egui_kittest` parts of the first draft remain
valid. The coverage matrix and app hand-off have been corrected to match the
actual UI.

### 0.2 Implementation progress

The first writing pass now exists under `website/`:

- Docusaurus 3.10 scaffold, base URL, theme tokens, landing page, download
  page, and sidebars;
- 25 user-facing pages covering installation on four platforms, the actual
  session, pickers, preferences, aliases, instruments, timing, TTS, cache,
  themes, MIDI, troubleshooting, keyboard shortcuts, limitations, and reference
  material;
- `<Shot>`, `<Step>`, `<Steps>`, and `<CommandStep>` MDX components;
- a 78-entry screenshot manifest with alt text and captions;
- 69 rendered in-app screenshots plus nine manual OS placeholders;
- a dependency-free manifest/docs checker (`npm run check:shots`);
- a successful production build under `/Contra-karaoke/`.

Still missing: the screenshot verification workflow, manual OS installer
images, and the first Pages deployment. The app test seam and real in-app
screenshots are now handled by the headless renderer below.

The docs workflow also builds the WASM app and stages it at `/app/`, so the
`/play` page can embed a hosted copy of the browser build without leaving the
documentation site.

### 0.3 Headless renderer (implemented)

`crates/app` has a `headless` feature and a `contra-headless` binary. It
constructs the real `ContraApp` with a silent transport, in-memory source cache,
stub speech backend, and no preference file, then renders it through
`egui_kittest` with wgpu. No window, audio device, network, or system TTS is
opened.

```sh
cd website
npm run screenshots
```

`website/scripts/capture-screenshots.mjs` renders the raw PNGs into
`website/screenshots/raw/`, then `website/scripts/annotate-screenshots.mjs`
drives `photocraft-cli` to draw the focus ring from
`website/screenshots/emphasis.json` and writes the final PNGs to
`website/static/img/screens/`. It generates all in-app shots; the nine
remaining images are OS installer dialogs that are not part of the app. The old
`xtask screenshots` design is superseded for in-app captures.

---

## 1. Goals, non-goals, and success criteria

### 1.1 Goals

1. Publish user documentation at
   `https://storytold.github.io/Contra-karaoke/` from `main`.
2. Explain the complete implemented 0.1.0 journey: install, choose a dance,
   choose a tune, configure roles/aliases/instruments/timing, run a session,
   use TTS, clear the source cache, export MIDI, troubleshoot.
3. Pair every in-app instruction with a current screenshot.
4. Generate those screenshots from the real `eframe` application with fixed
   viewports, themes, and fixture data. No mockups and no manual crops for
   in-app screens.
5. Make the release screenshot refresh a deliberate, reviewed step while CI
   validates the manifest and builds the site.
6. Keep the site offline-friendly, private, accessible, and consistent with the
   app's grey theme.

### 1.2 Non-goals

- Versioned docs before 1.0.
- Translations in 0.1.0.
- Video or animated GIF tutorials.
- Documenting features that are not in the build. The docs must not promise
  keyboard shortcuts, warning banners, or settings the UI does not have.
- ~~Publishing the web/WASM build from the docs workflow.~~ Superseded: the docs
  workflow now stages the WASM build under `/app/` and the `/play` page embeds
  it. That build contains only the two attributed defaults; the docs site still
  does not host a dance or tune database or a soundfont.
- Redistributing third-party dances, tunes, or soundfonts. Screenshots use the
  two bundled defaults or synthetic fixtures only.
- Replacing internal docs in `docs/`. Public docs live in `website/docs/`.

### 1.3 Success criteria

- [ ] The site builds with `onBrokenLinks: 'throw'` and deploys from `main`
      after owner approval.
- [ ] Every task step uses `<Step shot="…">` (or `<CommandStep>` for CLI-only
      steps) and therefore cannot exist without an image or an explicit reason.
- [ ] Every screenshot has authored alt text and a caption; every image is
      referenced; no orphan PNGs.
- [ ] `cargo xtask screenshots --check` passes on a clean checkout in CI.
- [ ] A UI change that alters a screenshot produces a failing check and a
      reviewable diff artifact.
- [ ] The site is usable at 320 px, keyboard-only, and in both themes.
- [ ] The built site makes no third-party requests.
- [ ] Each screenshot is ≤ 300 KB; each page's initial payload is ≤ 1 MB.
- [ ] Screenshots contain no user data, tokens, paths, or third-party content.

### 1.4 What can start immediately

1. Scaffold `website/`, the theme, navigation, landing page, download page, and
   the `Shot`/`Step` component API. None of this depends on the app.
2. Author the P1 screenshot manifest with IDs, viewports, alt text, and
   captions. These are content decisions that need no pixels.
3. Write `check-screenshots.mjs` and the docs/screenshot workflows; they can
   pass with an empty manifest.
4. Land the app test seams in §4.2–4.4 and the label pass in §4.4. These are
   the blocking dependencies on the app track.
5. Prove the pipeline with one disposable demo shot outside the published set.

Do not commit fake screenshots. The first published shots must come from the
real app.

---

## 2. Key decisions

| Area | Decision | Why |
| --- | --- | --- |
| Site | Docusaurus 3.10 classic, TypeScript, blog disabled | MDX components, static output, Pages-native |
| Location | `website/` in this monorepo | Docs and screenshots version with the app |
| Hosting | GitHub Actions → Pages artifact | Atomic deploys, no `gh-pages` branch |
| URL | Project Pages, `baseUrl: '/Contra-karaoke/'` | Matches the repository URL |
| Search | `@easyops-cn/docusaurus-search-local` | Offline index, MIT, no external service |
| Screenshots | `egui_kittest` 0.37 + wgpu test renderer | Official egui harness; drives the real `eframe::App` through AccessKit |
| Capture | Desktop 1280×800 @1×, mobile 390×844 @1× | 1280-wide PNGs stay under the docs size budget; text remains crisp at the 760 px display width |
| Theme | Dark for most shots; light only on theme pages | App default is dark |
| Data | Bundled defaults + synthetic fixtures, offline | Deterministic, no network, no licensing surprises |
| Storage | PNGs committed under `website/static/img/screens/` | Reviewable diffs, fast docs builds |
| Instruction API | `<Step shot="id">` MDX component | Makes “instruction without screenshot” impossible |
| App baseline | `108e1de`, headless renderer added | Library, `contra-headless`, sheet-music preview, keyboard shortcuts, Kokoro voices, and live MCP inspection exist |
| Manual images | Windows/Android OS dialogs only | Automating OS UI is disproportionate for three images |
| Analytics | None | Matches app privacy rules |

### 2.1 Alternatives considered

| Option | Why not |
| --- | --- |
| Hand-captured screenshots | Accurate once, stale forever; no review signal |
| Hidden native window + OS screenshot | Needs a display, captures window chrome/cursor, cannot crop to a control, flaky in CI |
| Playwright against the web build | Exercises the proxy/web renderer, not the desktop app; cannot reach native states |
| Mock/storybook UI | Drifts from the real app and proves nothing about interactions |
| `egui_kittest` + wgpu | Chosen: official egui harness, real `App::ui`, headless, deterministic, label-based crops |
| mdBook | Simpler, but no React components or manifest-driven validation |
| External docs host | Adds an account/service and weakens privacy and styling control |

---

## 3. Information architecture

### 3.1 Repository layout

```
website/
  docusaurus.config.ts
  sidebars.ts
  package.json
  package-lock.json
  tsconfig.json
  docs/
    intro.md
    getting-started/
      install-windows.md
      install-linux.md
      install-web.md
      install-android.md
      first-session.md
    guides/
      choosing-a-dance.md
      choosing-a-tune.md
      session-controls.md
      role-terms.md
      aliases.md
      instruments-and-soundfonts.md
      timing-and-lead.md
      text-to-speech.md
      cache-and-offline.md
      themes-and-accessibility.md
      export-midi.md
    reference/
      how-calls-are-timed.md
      data-sources.md
      privacy.md
      troubleshooting.md
    contributing/
      docs-and-screenshots.md
  screenshots/
    manifest.json          # authored: id, scenario, viewport, alt, caption
    generated.json         # generated: dimensions, sha256, versions
  src/
    components/{Shot,Step,Steps,CommandStep,Figure,PlatformTabs,ScreenshotGallery}/
    css/custom.css
    pages/{index,download,screenshots}.tsx
  static/img/screens/*.png
  static/img/manual/*.png
  scripts/check-screenshots.mjs
xtask/                     # dev-only screenshot harness
.github/workflows/{docs,screenshots}.yml
```

`website/docs` is the public site. `docs/` remains internal engineering
documentation and is not published verbatim.

### 3.2 Navigation

- **Guide** (sidebar): Getting started → Guides → Reference → Contributing.
- **Download**: static links to the rolling `nightly` release.
- **GitHub**: repository.
- Footer: MIT, default-content attribution, privacy, “screenshots generated
  from the current nightly build”.

### 3.3 Page outline mapped to the actual UI

| Page | Actual screen/state | Key content |
| --- | --- | --- |
| `intro.md` | — | What the app is, karaoke analogy, call-before-move promise, platforms, quickstart |
| `install-*` | OS/browser UI | Download, verify, install, first launch, uninstall |
| `first-session.md` | Session, pickers | Default dance/tune, search/select, tempo, Play, count-in, call-now, pause, finish |
| `choosing-a-dance.md` | **Select dance** screen | Title, choreographer, formation, figure words, verified, sort, page through, preview, use |
| `choosing-a-tune.md` | **Select Tune** screen | Search, composer, key, meter, genre, preview ABC, use |
| `session-controls.md` | Session header/controls | Session tab, tempo, Play/Pause/Restart, Loop, Count-in, Speak calls, Volume, tune summary, dance card |
| `role-terms.md` | **Preferences** → Role terms | Presets (Larks/Robins default), custom singular/plural terms, live example |
| `aliases.md` | **Preferences** → **Dance move aliases** | Filter, per-move template, Reset to default, Reset all, placeholders, custom aliases |
| `instruments-and-soundfonts.md` | Preferences → Instruments/Soundfont | Fiddle/Accordion melody, Piano/Guitar/Accordion accompaniment, volumes, Load/Remove `.sf2` |
| `timing-and-lead.md` | Preferences → Calling lead; Session | Auto/manual, min/max, speech rate, Rebuild call timing, count-in, call countdown |
| `text-to-speech.md` | Preferences → Text to speech; Session | Speak calls, rate, unavailable message, stop on pause |
| `cache-and-offline.md` | Preferences → Maintenance; error panel | Clear source cache (confirm), automatic stale fallback (no banner), error dismiss |
| `themes-and-accessibility.md` | Preferences → Theme | Dark/Light, phone width, touch targets, keyboard, screen-reader labels |
| `export-midi.md` | Preferences → Maintenance | Desktop-only export, save dialog, what is exported |
| `reference/keyboard-shortcuts.md` | — | Space, R, Esc, focus behavior |
| `reference/how-calls-are-timed.md` | — | Dance beat, syllables, clamp, examples, negative first call |
| `reference/data-sources.md` | — | ContraDB, ABC Session Tunes, proxy, cache TTLs |
| `reference/privacy.md` | — | No telemetry, local cache, proxy token |
| `reference/known-limitations.md` | — | Platform, feature, grid, accessibility, release caveats |
| `reference/troubleshooting.md` | Error/loading states | Network, parse, audio, TTS, SF2, cache |
| `contributing/docs-and-screenshots.md` | — | Generation, blessing, review, label rules |

### 3.4 Do not document these yet

The UI does not implement them; screenshots would be fake or unreachable:

- page-size selector; fixed at 10
- dance non-64-beat warning
- tune arrangement warning
- offline “using cached data” banner
- “Preparing music…” progress/cancel state (rendering is synchronous on Play)
- cache size display and manual refresh
- TTS voice picker (rate only)
- SoundFont missing-program fallback message
- explicit paused/playing status label in the session header

Add them to the docs when the UI ships, with their own screenshots.

---

## 4. Screenshot automation architecture

### 4.1 Pipeline

```
website/screenshots/manifest.json  (authored: id, scenario, viewport, alt, caption)
                │
                ├──────────────► cargo xtask screenshots
                │                     │
                │                     ├─ build real ContraApp with screenshot deps
                │                     ├─ drive it with egui_kittest (AccessKit)
                │                     ├─ render @2× with wgpu, crop via label bounds
                │                     └─ write PNG + website/screenshots/generated.json
                │
                ▼
        Docusaurus <Shot id="…"> ──► build ──► GitHub Pages
                ▲
                └─ <Step shot="…"> in MDX (checker enforces 1:1)
```

Two directions are enforced:

- every manifest shot is referenced by at least one page;
- every `<Step shot="x">` has a manifest entry and a generated PNG.

### 4.2 Current blockers and the required test seam

The harness must run the production UI without a window, audio device, network,
TTS, or user config. Current code blocks each of those:

| Requirement | Current code | Required change |
| --- | --- | --- |
| Library entry | `src/lib.rs` exists and exports `ContraApp`; platform entry points are shared | Keep it; expose the test constructor below |
| Constructor | `ContraApp::new(cc)` reads prefs, creates dirs, opens audio/TTS, loads SF2 | Add `ContraApp::with_deps(cc, AppDeps)`; `new` delegates |
| Playback | Concrete `AudioEngine`; `current_beat` is 0 without a buffer | Add `Playback` trait + `FakePlayback` (preferred), or `AudioEngine::silent` + `seek_beat`/`force_finished` test helpers |
| Sources | Concrete `SourceClient`; `ehttp` only | Add a fixture backend or `SourceService` trait; fixture returns results/errors synchronously |
| TTS | `Speech::new()` initializes the system voice | Add `Speech::stub()` or a `Speaker` trait |
| Preferences | Always reads/writes the real config path (or localStorage on web) | `prefs_path: Option<PathBuf>` or `PrefStore`; `None` = no-op saves |
| Screen state | `Screen` private; starts at Session | Make `Screen` public and allow an initial screen (or drive tabs) |
| Time states | No seek API | Fake playback with settable beat and finished flag |
| Labels | Many inputs unlabeled; `Preview` and `Reset to default` repeat | Label pass in §4.4 |

The clean split is:

```rust
// crates/app/src/lib.rs
pub mod app;
pub mod audio;
pub mod defaults;
pub mod source;
pub mod theme;
pub mod tts;
pub use app::ContraApp;
```

```rust
// crates/app/src/app.rs (illustrative)
pub struct AppDeps {
    pub prefs: Preferences,
    pub prefs_path: Option<PathBuf>,
    pub audio: Box<dyn Playback>,
    pub speech: Box<dyn Speaker>,
    pub source: SourceClient,
    pub source_rx: Receiver<SourceEvent>,
    pub screen: Screen,
    pub ready: bool,
}

impl ContraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::with_deps(cc, AppDeps::native())
    }

    pub fn with_deps(cc: &eframe::CreationContext<'_>, deps: AppDeps) -> Self {
        // existing construction, but no I/O beyond what is in deps
    }
}
```

`AppDeps::native()` builds the current `FileCache`, `ehttp`-backed
`SourceClient`, `cpal` engine, and system `Speech`. The screenshot harness
builds the fixture version. This preserves production behavior while removing
startup I/O from the test path.

If the app track prefers a smaller change, the minimum viable seam is:

- `src/lib.rs` + public modules;
- `ContraApp::with_deps`;
- `AudioEngine::silent` + `seek_beat` + `force_finished`;
- `Speech::stub` (available, no-op);
- `SourceClient::fixture` that sends prebuilt `SourceEvent`s;
- `prefs_path: Option<PathBuf>`;
- a `ready: bool`/`needs_render: false` initial state.

Do not add `cfg` branches inside the UI. The screenshots must render the same
code paths users see.

> The implemented capture path is `contra-headless` (§0.3) and
> `website/scripts/capture-screenshots.mjs`. The `xtask`/`egui_kittest` sketches
> in this section describe the original design; use the headless binary for
> current captures.

### 4.3 Why fixture sources are required

`SourceClient` currently calls `ehttp::fetch` and caches by
`format!("{query:?}")`. There is no way to inject search results, and network
access in tests is forbidden. Do not work around this by populating the cache:
cache keys include `Debug` output and TTL timestamps, so they are brittle.

Add a fixture path to `SourceClient`, for example:

```rust
pub struct SourceFixture {
    pub dance_search: DanceSearchPage,
    pub dance_demo: Dance,
    pub tune_search: TuneSearchPage,
    pub tune_demo: Tune,
    pub error: Option<String>,
}
```

`search_dances`, `load_dance`, `search_tunes`, and `load_tune` send the
corresponding `SourceEvent` immediately when a fixture is present. The fixture
must come from the bundled defaults plus synthetic cards, never from
`samples/` (which contains third-party parser material).

### 4.4 Accessibility and label pass (blocking)

`egui_kittest` queries controls by accessible name and runs an accessibility
check before every snapshot. The current UI will fail or hit the wrong node:

- `Play`, `Pause`, and `Restart` appear both in the top header and the session
  controls. Either remove the header duplicates or label them
  `Header: Play`, etc. Removing duplicates is better UX.
- Every `Preview` button has the same label. Use
  `Preview: Another Last Dance` or query by ancestor.
- `Reset to default` repeats for each alias. Use `Reset Swing alias`.
- `Verified only` is a checkbox with an empty label next to a grid label. Tie
  the checkbox to the label with `Response::labelled_by`.
- Search fields (`Title`, `Choreographer`, `Figure contains`, `Search`,
  `Composer`, proxy URL/token, alias fields) are separate `TextEdit`s with
  adjacent labels. Associate them or use `labelled_by`.
- Combo boxes (`Formation`, `Sort`, `Key`, `Meter`, `Genre`, role presets) use
  `from_id_salt` with no accessible name. Associate the grid label.
- Sliders (`Tempo`, `Volume`, min/max lead, speech rate, TTS rate, alias
  volumes) need labels with units; `.text("beats")` alone is not enough.

Add a test in `xtask` or `crates/app` that builds each screen with the harness
and asserts `harness.unnamed_widgets().is_empty()`. This is cheaper than
discovering labels one screenshot at a time.

### 4.5 Session-state helper

The session is time-driven from `audio.position_beat()`. With a fake playback:

```rust
impl FakePlayback {
    pub fn set_beat(&self, beat: f64) { ... }
    pub fn set_finished(&self, finished: bool) { ... }
    pub fn set_playing(&self, playing: bool) { ... }
}
```

Scenarios set the beat instead of sleeping. Screenshots of playing states must
not call `Harness::run()` (the app requests a repaint every 16 ms while
playing); use `run_steps(3)`.

The initial app has `needs_render: true`, so the first screen shows “Changes
pending — press Play to rebuild the music.” A screenshot fixture must set
`needs_render: false` (or call `ensure_ready`) for states that should not show
that message. Keep a dedicated `main-changes-pending` shot for the real state.

### 4.6 Harness

`xtask` is a dev-only workspace member:

```toml
# xtask/Cargo.toml
[package]
name = "xtask"
version.workspace = true
edition.workspace = true
publish = false

[dependencies]
anyhow.workspace = true
contra-app = { path = "../crates/app" }
egui_kittest = { version = "0.37", features = ["eframe", "wgpu"] }
image = { version = "0.25", default-features = false, features = ["png"] }
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
```

Add a Cargo alias:

```toml
# .cargo/config.toml
[alias]
xtask = "run -p xtask --"
```

Commands:

```sh
cargo xtask screenshots                 # regenerate all PNGs + generated.json
cargo xtask screenshots --filter session
cargo xtask screenshots --check         # compare only; never writes docs images
cargo xtask screenshots --list
```

Capture routine:

```rust
use egui::{Theme, Vec2, os::OperatingSystem};
use egui_kittest::{Harness, kittest::Queryable};

fn capture(spec: &ShotSpec) -> anyhow::Result<image::RgbaImage> {
    let vp = spec.viewport;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(vp.width, vp.height))
        .with_pixels_per_point(vp.scale)          // 1.0
        .with_theme(match spec.theme {
            ThemeName::Dark => Theme::Dark,
            ThemeName::Light => Theme::Light,
        })
        .with_os(OperatingSystem::Windows)        // stable Ctrl/Alt glyphs
        .with_render_cursor(false)
        .with_max_steps(10)
        .with_context_setup(|ctx| {
            ctx.style_mut(|s| s.animation_time = 0.0);
        })
        .build_eframe(|cc| {
            contra_app::ContraApp::with_deps(cc, screenshot_deps(spec))
        });

    scenarios::run(&spec.scenario, &mut harness)?;
    settle(&mut harness);
    harness.remove_cursor();
    harness.check_accessibility();
    let image = harness.render().map_err(anyhow::Error::msg)?;

    Ok(match &spec.focus {
        Some(labels) => crop_to_labels(&harness, image, labels),
        None => image,
    })
}

fn settle(h: &mut Harness<contra_app::ContraApp>) {
    h.run_steps(3);
}
```

Notes validated against `egui_kittest` 0.37 / `eframe` 0.37:

- `eframe::App` now has a required `ui(&mut self, ui, frame)` method; the
  actual app implements it. `build_eframe` works with this trait.
- The harness disables scroll animation and cursor blinking; `with_theme` and
  `with_os` remove environment drift.
- `render()` returns an `RgbaImage`; write it ourselves for paths and hashing.
- `RendererOptions::PREDICTABLE` is the default; do not enable MSAA.
- `check_accessibility()` runs automatically before snapshots; call it
  explicitly for a clearer failure.
- `Node` exposes `rect()` in logical coordinates, `scroll_to_me()`,
  `click_accesskit()`, and `type_text()`.
- The harness uses wgpu while the app uses glow. Layout and colours match;
  anti-aliasing can differ by a pixel.

### 4.7 Scenario model

`website/screenshots/manifest.json` is authored; docs authors own `alt` and
`caption`, the Rust harness owns how to reach the state.

```json
{
  "schemaVersion": 1,
  "viewports": {
    "desktop": { "width": 1280, "height": 800, "scale": 1 },
    "mobile":  { "width": 390,  "height": 844, "scale": 1 }
  },
  "shots": [
    {
      "id": "session-call-now",
      "scenario": "session_call_now",
      "viewport": "desktop",
      "theme": "dark",
      "focus": ["Tempo", "Dance card"],
      "alt": "Session screen during the first figure: a large CALL NOW prompt with the lead countdown, the next call, and the active dance-card row highlighted.",
      "caption": "The large prompt is the call to say now; the next call is previewed below it.",
      "priority": true
    }
  ]
}
```

`scenario` maps to a typed Rust function. Compare the manifest's scenario names
with the match arms in a unit test so neither side can drift.

Scenario rules:

- Prefer real clicks and typing over state injection.
- Use injection only for time, fixture results, theme, and errors.
- Every scenario starts from a fresh harness.
- Use `settle()` (`run_steps(3)`) after each action. Never `run()` on an active
  session.
- Keep scenarios under ~2 seconds; the suite should stay under 3 minutes.

### 4.8 Focus crops and callouts

The implemented annotation pass uses `website/screenshots/emphasis.json`, a
per-shot list of pixel rectangles, and `website/scripts/annotate-screenshots.mjs`
drives `photocraft-cli` to draw a rounded amber focus ring with a soft shadow.
The file also stores the expected image size; a layout change fails the pass
instead of drawing the ring in the wrong place. `npm test` checks that every
ready shot has a region and that every region is inside its image.

Deriving the regions from AccessKit bounds remains future work, as do numbered
callout badges for multi-step figures.

### 4.9 Determinism rules

| Source of drift | Rule |
| --- | --- |
| Wall clock | UI uses the playback clock; fake playback sets the beat |
| Animations | `animation_time = 0` |
| Cursor | `with_render_cursor(false)`, `remove_cursor()` |
| Fonts | egui embedded fonts only; no system fonts |
| DPI | Fixed `pixels_per_point = 1.0` |
| OS | `with_os(Windows)` |
| Theme | Set both harness theme and `Preferences.theme`; app calls `theme::apply` each frame |
| Repaint | `run_steps(3)` for playing states |
| Data | Bundled defaults + synthetic fixtures; `samples/` excluded |
| Network | Fixture source; no `ehttp` |
| Filesystem | `prefs_path = None`; in-memory cache only |
| Initial state | Explicit `needs_render`, beat, playing, finished |
| Time text | No “today”, timestamps, uptime, or build time |
| Randomness | None; stable ordering |
| Renderer | wgpu software Vulkan (lavapipe) in CI |
| PNG | Fixed encoder, metadata stripped, no timestamps |

The app currently calls `theme::apply` on every frame and writes prefs in
several places. Both are fine once `prefs_path` is optional and the screenshot
prefs are fixed.

### 4.10 Output manifest

`website/screenshots/generated.json`:

```json
{
  "schemaVersion": 1,
  "appVersion": "0.1.0",
  "appCommit": "108e1de",
  "egui": "0.37.0",
  "renderer": "wgpu-vulkan-llvmpipe",
  "shots": {
    "session-call-now": {
      "file": "session-call-now.png",
      "width": 2560,
      "height": 1600,
      "logicalWidth": 1280,
      "logicalHeight": 800,
      "sha256": "…"
    }
  }
}
```

No capture timestamp. `appCommit` lets the docs show the source version;
`sha256` detects a PNG edited without regeneration.

### 4.11 Docs components

Register `Shot`, `Step`, `Steps`, `CommandStep`, `Figure`, and `PlatformTabs`
globally.

```mdx
<Steps>
  <Step shot="dance-picker-open">
    Select **Select dance** at the top of the window.
  </Step>
  <Step shot="dance-picker-search">
    Enter part of a title in **Title**, then select **Search dances**.
  </Step>
</Steps>
```

`Shot` looks up the manifest and generated metadata, throws at build time if
either is missing, renders `alt`, logical `width`/`height`, lazy loading, and a
link to the full-resolution PNG. `Step` renders the numbered instruction plus
the shot. `CommandStep` is only for CLI/install commands; it has no `shot`.
`Figure` is only for the tracked manual OS images and requires a source.

### 4.12 Manual screenshots

Automate all in-app shots. These are the only manual images:

| Image | Why manual | Update trigger |
| --- | --- | --- |
| Windows installer wizard | OS UI, not the app | Installer changes |
| Windows SmartScreen | OS security UI | Verify wording each release |
| Android install-unknown-apps / package installer | OS/device UI | Android release checklist |
| Optional AppImage permissions dialog | OS UI | Linux release checklist |

Rules: store under `static/img/manual/`, require `source` and date, redact
names/device IDs/paths, never use them for an in-app step. Fully automating OS
UI would need a Windows VM/Sandbox with UI Automation and an Android emulator;
both are deferrable because the app itself is fully automated.

### 4.13 Review flow

- PRs touching `crates/app`, `crates/core`, `assets/`, `xtask/`, or
  `website/screenshots/` run `screenshots.yml`.
- Failure uploads `target/screenshot-diffs/` with `.new.png` and `.diff.png`.
- A maintainer reviews the artifact and blesses on canonical Linux/lavapipe.
- The internal `/screenshots` gallery lists every shot and its pages.
- Branch protection requires both docs and screenshots checks. Otherwise
  `docs.yml` would deploy the last committed PNGs after a UI change.

---

## 5. Instruction and screenshot coverage

Priority: **P1** for first public launch, **P2** for follow-up. Every row is a
`<Step>` except the landing hero and manual `Figure` rows. IDs must match
actual labels and states.

### 5.1 Landing and first session

| Page | Instruction | Shot id | Viewport | Priority |
| --- | --- | --- | --- | --- |
| Landing | See the app at a glance | `overview-main` | desktop | P1 |
| `first-session` | Launch the app; note the default dance and tune | `main-idle` | desktop | P1 |
| `first-session` | Open the dance picker | `dance-picker-open` | desktop | P1 |
| `first-session` | Search for a dance | `dance-picker-search` | desktop | P1 |
| `first-session` | Preview the figures | `dance-picker-preview` | desktop | P1 |
| `first-session` | Use the dance | `main-dance-loaded` | desktop | P1 |
| `first-session` | Open the tune picker | `tune-picker-open` | desktop | P1 |
| `first-session` | Search and preview a tune | `tune-picker-preview` | desktop | P1 |
| `first-session` | Use the tune | `main-tune-loaded` | desktop | P1 |
| `first-session` | Set the tempo | `transport-tempo` | desktop | P1 |
| `first-session` | Start the session | `session-count-in` | desktop | P1 |
| `first-session` | Read the call-now prompt | `session-call-now` | desktop | P1 |
| `first-session` | Change a setting and rebuild | `session-changes-pending` | desktop | P1 |
| `first-session` | Let the dance finish | `session-finished` | desktop | P1 |
| `first-session` | Use the phone layout | `main-mobile` | mobile | P1 |

### 5.2 Choosing a dance

| Page | Instruction | Shot id | Viewport | Priority |
| --- | --- | --- | --- | --- |
| `choosing-a-dance` | Filter by title | `dance-search-title` | desktop | P2 |
| `choosing-a-dance` | Filter by choreographer | `dance-search-choreographer` | desktop | P2 |
| `choosing-a-dance` | Filter by formation | `dance-filter-formation` | desktop | P2 |
| `choosing-a-dance` | Search figure words | `dance-filter-figure` | desktop | P2 |
| `choosing-a-dance` | Show verified dances only | `dance-filter-verified` | desktop | P2 |
| `choosing-a-dance` | Change sort order | `dance-sort` | desktop | P2 |
| `choosing-a-dance` | Page through results | `dance-pagination` | desktop | P2 |
| `choosing-a-dance` | Read a card whose total is not 64 beats | `dance-preview-beats` | desktop | P2 |
| `choosing-a-dance` | Recover from a search error | `error-dance-load` | desktop | P2 |

### 5.3 Choosing a tune

| Page | Instruction | Shot id | Viewport | Priority |
| --- | --- | --- | --- | --- |
| `choosing-a-tune` | Search by title or composer | `tune-search` | desktop | P2 |
| `choosing-a-tune` | Filter by key | `tune-filter-key` | desktop | P2 |
| `choosing-a-tune` | Filter by meter | `tune-filter-meter` | desktop | P2 |
| `choosing-a-tune` | Filter by genre | `tune-filter-genre` | desktop | P2 |
| `choosing-a-tune` | Inspect the ABC source | `tune-preview` | desktop | P2 |
| `choosing-a-tune` | Recover from a tune error | `error-tune-load` | desktop | P2 |

### 5.4 Session controls

| Page | Instruction | Shot id | Viewport | Priority |
| --- | --- | --- | --- | --- |
| `session-controls` | Change tempo | `transport-tempo` | desktop | P1 |
| `session-controls` | Play and pause | `session-paused` | desktop | P1* |
| `session-controls` | Restart | `session-restarted` | desktop | P2 |
| `session-controls` | Turn looping off | `transport-loop-off` | desktop | P2 |
| `session-controls` | Toggle spoken calls | `transport-tts-on` | desktop | P1 |
| `session-controls` | Set master volume | `transport-volume` | desktop | P2 |
| `session-controls` | Read the tune and instrument summary | `transport-summary` | desktop | P2 |
| `session-controls` | Use controls on a phone | `session-mobile-controls` | mobile | P2 |

\* The session header has no visible “Paused” status. Add a small status label
before treating this shot as P1; otherwise the image is indistinguishable from
call-now.

### 5.5 Preferences and aliases

| Page | Instruction | Shot id | Viewport | Priority |
| --- | --- | --- | --- | --- |
| `role-terms` | Open Preferences | `prefs-open` | desktop | P1 |
| `role-terms` | Choose a role preset | `prefs-roles-presets` | desktop | P1 |
| `role-terms` | Enter custom role terms | `prefs-roles-custom` | desktop | P2 |
| `role-terms` | See roles replaced | `session-roles-applied` | desktop | P2 |
| `aliases` | Open the alias editor | `prefs-alias-list` | desktop | P1 |
| `aliases` | Edit one alias template | `prefs-alias-editor` | desktop | P1 |
| `aliases` | Understand the placeholders | `prefs-alias-placeholders` | desktop | P2 |
| `aliases` | Reset one alias | `prefs-alias-reset` | desktop | P2 |
| `aliases` | Reset all aliases | `prefs-alias-reset-all` | desktop | P2 |
| `aliases` | Add a custom figure alias | `prefs-alias-custom` | desktop | P1 |
| `aliases` | Hear the alias in a session | `session-alias-call` | desktop | P1 |
| `instruments` | Choose the melody instrument | `prefs-instrument-melody` | desktop | P1 |
| `instruments` | Choose the accompaniment | `prefs-instrument-accompaniment` | desktop | P1 |
| `instruments` | Set the instrument mix | `prefs-instrument-mix` | desktop | P2 |
| `instruments` | Load a SoundFont | `prefs-sf2-loaded` | desktop | P1 |
| `instruments` | Remove a SoundFont | `prefs-sf2-removed` | desktop | P2 |
| `instruments` | See a SoundFont load error | `error-sf2` | desktop | P2 |
| `timing` | Choose automatic or manual lead | `prefs-timing-mode` | desktop | P1 |
| `timing` | Adjust min/max lead | `prefs-timing-bounds` | desktop | P2 |
| `timing` | Adjust speech rate | `prefs-timing-speech-rate` | desktop | P2 |
| `timing` | Rebuild call timing | `prefs-timing-rebuild` | desktop | P2 |
| `timing` | Watch the lead countdown | `session-lead-countdown` | desktop | P1 |
| `timing` | See a negative first call | `session-first-call` | desktop | P2 |
| `text-to-speech` | Enable spoken calls | `transport-tts-on` | desktop | P1 |
| `text-to-speech` | Change the TTS rate | `prefs-tts-rate` | desktop | P2 |
| `text-to-speech` | Understand an unavailable voice | `prefs-tts-unavailable` | desktop | P1 |
| `text-to-speech` | Pause stops speech | `session-paused-tts` | desktop | P2 |
| `cache` | Clear the source cache | `prefs-cache-confirm` | desktop | P1 |
| `cache` | Confirm the clear | `prefs-cache-cleared` | desktop | P2 |
| `cache` | Configure the web proxy | `prefs-proxy` | desktop | P2 |
| `themes` | Switch to the light theme | `prefs-theme-light` | desktop | P1 |
| `themes` | See the light session screen | `main-light` | desktop | P1 |
| `export-midi` | Open Maintenance | `prefs-maintenance` | desktop | P1 |

The MIDI save dialog is a native OS dialog and cannot be rendered by the egui
harness. Document the export as prose plus the visible Maintenance button; do
not promise a screenshot of the save dialog. The SoundFont file picker is also
native: `prefs-sf2-*` shots show the resulting Preferences state, not the
picker.

Offline use has no visible banner in the current UI: stale cache is used
silently. Explain that behavior in prose and keep screenshots for the error and
cache-clear states instead.

### 5.6 Errors and loading

| Page | Instruction | Shot id | Viewport | Priority |
| --- | --- | --- | --- | --- |
| `troubleshooting` | Dismiss a source error | `error-network` | desktop | P1 |
| `troubleshooting` | Recover from a parse failure | `error-parse` | desktop | P2 |
| `troubleshooting` | See a missing audio device | `error-audio` | desktop | P2 |
| `troubleshooting` | See a SoundFont load error | `error-sf2` | desktop | P2 |
| `first-session` | See the pending-rebuild message | `session-changes-pending` | desktop | P1 |

### 5.7 Manual OS images

| Page | Image | Priority |
| --- | --- | --- |
| `install-windows` | Setup wizard, portable ZIP contents | P1 |
| `install-windows` | SmartScreen prompt | P1 |
| `install-android` | Install unknown apps, package installer | P1 |
| `install-linux` | Optional AppImage permissions dialog | P2 |

Totals: roughly 30 P1 app screenshots, 30 P2, and 4 manual.

The implemented site also has four platform installation pages and a keyboard
shortcut reference page; their shots are in the manifest under `install-*` and
`dance-filter-*` / `*-source-link`.

### 5.8 Future UI — add screenshots when implemented

| Feature | Why no shot today |
| --- | --- |
| Page-size selector | Fixed at 10 |
| Non-64-beat warning | Preview shows total beats but no warning |
| Arrangement warning | Not shown in tune preview |
| Offline banner | Stale cache is used silently |
| Preparing/cancel | Music render is synchronous on Play |
| Cache size/refresh | Only “Clear source cache” exists |
| TTS voice picker | Rate slider only |
| SoundFont fallback | Only a generic load error is shown |
| Paused/playing status | No status label in the session header |

### 5.9 Alt text and caption rules

- Alt describes what is visible and the state, not “screenshot of”.
- Use the exact control names from the UI.
- Do not repeat the caption.
- Keep alt under ~180 characters and captions under ~140.
- No personal data, tokens, or absolute paths.
- Manual images name the OS and version.

---

## 6. Docusaurus implementation

### 6.1 Scaffold

```sh
npx create-docusaurus@latest website classic --typescript
```

Remove the blog, set `onBrokenLinks`/`onBrokenMarkdownLinks`/`onBrokenAnchors`
to `'throw'`, add local search, commit `package-lock.json`, add `.nvmrc` with
`24`.

### 6.2 `docusaurus.config.ts` (key fields)

```ts
const config: Config = {
  title: 'Contra Karaoke',
  tagline: 'Learn to call contra dances in time with the music',
  url: 'https://storytold.github.io',
  baseUrl: '/Contra-karaoke/',
  organizationName: 'storytold',
  projectName: 'Contra-karaoke',
  trailingSlash: true,
  onBrokenLinks: 'throw',
  onBrokenMarkdownLinks: 'throw',
  onBrokenAnchors: 'throw',
  i18n: { defaultLocale: 'en', locales: ['en'] },
  presets: [['classic', {
    docs: {
      sidebarPath: './sidebars.ts',
      editUrl: 'https://github.com/storytold/Contra-karaoke/edit/main/website/',
    },
    blog: false,
    theme: { customCss: './src/css/custom.css' },
  }]],
  plugins: [['@easyops-cn/docusaurus-search-local', {
    hashed: true,
    language: ['en'],
    indexDocs: true,
    docsRouteBasePath: '/docs',
  }]],
  themeConfig: {
    colorMode: {
      defaultMode: 'dark',
      disableSwitch: false,
      respectPrefersColorScheme: false,
    },
    navbar: {
      title: 'Contra Karaoke',
      items: [
        { type: 'docSidebar', sidebarId: 'docs', position: 'left', label: 'Guide' },
        { to: '/download', label: 'Download', position: 'left' },
        { href: 'https://github.com/storytold/Contra-karaoke', label: 'GitHub', position: 'right' },
      ],
    },
    footer: { style: 'dark' },
    tableOfContents: { minHeadingLevel: 2, maxHeadingLevel: 3 },
  },
};
```

No `gtag`, Algolia, or external fonts. The site must work offline after load.

### 6.3 Theme

Match the actual `theme.rs` values:

```css
:root {
  --ifm-background-color: #141414;
  --ifm-background-surface-color: #232323;
  --ifm-color-content: #e6e6e6;
  --ifm-color-content-secondary: #9a9a9a;
  --ifm-link-color: #e6e6e6;
  --ifm-link-hover-color: #ffffff;
  --ifm-toc-border-color: #3a3a3a;
  --ifm-color-emphasis-300: #3a3a3a;
  --ifm-code-background: #2e2e2e;
  --ifm-global-radius: 6px;
  --shot-max-width: 760px;

  --ifm-color-primary: #4a4a4a;
  --ifm-color-primary-dark: #3a3a3a;
  --ifm-color-primary-darker: #303030;
  --ifm-color-primary-darkest: #232323;
  --ifm-color-primary-light: #5a5a5a;
  --ifm-color-primary-lighter: #6a6a6a;
  --ifm-color-primary-lightest: #7a7a7a;
  --ifm-color-primary-contrast-background: #2e2e2e;
  --ifm-color-primary-contrast-foreground: #f2f2f2;
}

[data-theme='light'] {
  --ifm-background-color: #e6e6e6;
  --ifm-background-surface-color: #f2f2f2;
  --ifm-color-content: #1c1c1c;
  --ifm-color-content-secondary: #5c5c5c;
  --ifm-link-color: #1c1c1c;
  --ifm-link-hover-color: #000000;
  --ifm-code-background: #d6d6d6;
}
```

No accent colour. Links use underline and weight; focus outlines stay visible.
Use a muted Prism theme for code.

### 6.4 Screenshot styling

- Display at `min(100%, 760px)`, centred, 1 px `#3a3a3a` border, 6 px radius.
- No fake OS title bar.
- Caption states viewport when relevant (“Phone layout, 390 px wide”).
- Lazy by default; eager for the hero and first step.
- Click opens the full-resolution PNG.
- Print styles hide the full-size hint.

### 6.5 Content standards

- Second person, present tense, one action per step.
- A step's screenshot shows the result, with the next control visible.
- No more than ~7 steps per task page.
- Use `<PlatformTabs>` for OS differences.
- Use `<kbd>` only for keys that actually work (none in 0.1.0).
- Admonitions for the synchronous rebuild and silent stale-cache fallback.
- Explain one dance beat = half a bar once, on the timing page.
- Every page ends with “Next”.

### 6.6 Search, SEO, metadata

- Local search only; no network.
- Per-page descriptions; sitemap on.
- Canonical URLs from `url` + `baseUrl`.
- Static OG image; `/screenshots` gallery `noindex`.
- No cookies, consent banner, or analytics.

### 6.7 Download page

- Link to `https://github.com/storytold/Contra-karaoke/releases/tag/nightly`.
- List expected artifact names; note nightly names include date/SHA.
- Explain SmartScreen and unsigned nightly builds.
- No GitHub API fetch at build/runtime.
- Offer a hosted browser copy on `/play` and link it from the download page and
  landing page. The app itself is staged under `/app/` by the docs workflow;
  keep the download bundle and the hosted copy built from the same sources.

### 6.8 Version banner

“These docs track the 0.1.0 nightly build. Screenshots are regenerated from
`main` when a release is prepared.” One version constant; no dropdown until
1.0.

---

## 7. CI/CD

### 7.1 `docs.yml`

```yaml
name: docs
on:
  push:
    branches: [main]
    paths: ['website/**', 'docs/**', '.github/workflows/docs.yml']
  pull_request:
    paths: ['website/**', 'docs/**', '.github/workflows/docs.yml']
  workflow_dispatch:

permissions:
  contents: read
  pages: write
  id-token: write

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 24
          cache: npm
          cache-dependency-path: website/package-lock.json
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - name: Install Trunk
        run: |
          curl -L --fail --retry 3 \
            -o trunk.tar.gz \
            https://github.com/trunk-rs/trunk/releases/latest/download/trunk-x86_64-unknown-linux-gnu.tar.gz
          tar -xzf trunk.tar.gz
          sudo mv trunk /usr/local/bin/
      - run: npm ci
        working-directory: website
      - name: Build the hosted browser app
        working-directory: website
        run: npm run build:app
      - run: npm run check
        working-directory: website
      - uses: actions/configure-pages@v5
      - uses: actions/upload-pages-artifact@v3
        with:
          path: website/build

  deploy:
    if: github.event_name != 'pull_request' && github.ref == 'refs/heads/main'
    needs: build
    runs-on: ubuntu-24.04
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - id: deployment
        uses: actions/deploy-pages@v4
```

Enable Pages → Source: GitHub Actions. First deploy requires owner approval.

### 7.2 `screenshots.yml`

```yaml
name: screenshots
on:
  pull_request:
    paths:
      - 'crates/**'
      - 'assets/**'
      - 'xtask/**'
      - 'website/screenshots/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '.github/workflows/screenshots.yml'
  push:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: screenshots-${{ github.ref }}
  cancel-in-progress: true

jobs:
  verify:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: '1.95'
      - uses: Swatinem/rust-cache@v2
      - name: Install Linux build and software Vulkan dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libvulkan1 mesa-vulkan-drivers \
            libasound2-dev libspeechd-dev \
            libxkbcommon-dev libwayland-dev \
            libx11-dev libxi-dev libxcursor-dev libxrandr-dev \
            libegl1-mesa-dev libgl1-mesa-dev
      - name: Verify screenshots
        env:
          WGPU_BACKEND: vulkan
        run: cargo xtask screenshots --check
      - uses: actions/upload-artifact@v4
        if: failure()
        with:
          name: screenshot-diffs
          path: target/screenshot-diffs/
          if-no-files-found: ignore
```

The apt list matches the app's Linux build needs (`cpal`/ALSA, `tts`/Speech
Dispatcher, `winit`/X11, `rfd`) plus Mesa's software Vulkan. Share it with
`ci.yml` as a composite action.

`--check` writes to `target/screenshot-work/`, compares against committed PNGs,
and writes `*.new.png`/`*.diff.png` on mismatch. It never modifies `website/`.
Assert the wgpu adapter name contains `llvmpipe`.

Require both `docs` and `screenshots` checks in branch protection on `main`.

### 7.3 Local workflow

```sh
cargo xtask screenshots --list
cargo xtask screenshots --filter prefs
cargo xtask screenshots
cargo xtask screenshots --check

cd website
npm ci
npm start          # http://localhost:3000/Contra-karaoke/
npm run check
```

Canonical blessing is Linux + lavapipe. Windows iteration is fine, but bless in
WSL/Docker or via the CI artifact.

### 7.4 Cost

- Rust cache + npm cache.
- Docs build is Node-only; screenshots are committed and refreshed at release
  prep.
- Estimated: release screenshot refresh 4–8 min cold, 2–3 min warm; docs 1–2
  min.

---

## 8. Testing and quality gates

### 8.1 Screenshot checker

`scripts/check-screenshots.mjs`, Node standard library only:

1. Validate manifest/generated schemas and unique IDs.
2. Require alt (20–180 chars, not starting “screenshot”), caption (≤ 160),
   known viewport/theme.
3. Parse `<Step>`/`<Shot>` tags with a strict one-tag-per-line rule.
4. Fail on a step without `shot`, an unknown shot, or an unreferenced shot.
5. Fail on a missing PNG, wrong dimensions, or > 300 KB.
6. Hash every ready PNG with SHA-256 and fail if two screenshots are identical.
7. Require an `emphasis.json` region for every ready shot and keep it in bounds.
8. Reject raw `<img>`/Markdown images outside manual images.
9. Print a compact summary.

### 8.2 Docs build

- Broken links/anchors/markdown throw.
- TypeScript compile via build.
- `useBaseUrl` everywhere; verify base path.
- Serve with `docusaurus serve` under `/Contra-karaoke/`.

### 8.3 Optional browser smoke test

Phase 3: Playwright checks the built site at 390 px, screenshot alt text,
theme toggle, local search, and zero third-party requests. Apache-2.0; add to
`CREDITS.md` if adopted.

### 8.4 Manual QA

- Keyboard-only navigation; 320/390/768/1280 px.
- Dark/light AA contrast; 200% zoom; reduced motion.
- Screen-reader pass on the first-session page.
- Screenshots match the current build.
- Manual OS images current/redacted.
- No third-party requests.
- 404 page under the project base path.

### 8.5 Done

- All P1 pages written and reviewed.
- All P1 steps generated with alt/caption.
- Both workflows green on `main`.
- Live site with owner approval.
- `CREDITS.md` and README updated.
- No version bump; 0.1.0 nightly.

---

## 9. Licensing, attribution, and privacy

### 9.1 Tooling

| Component | License | Action |
| --- | --- | --- |
| Docusaurus / React | MIT | `CREDITS.md` |
| `@easyops-cn/docusaurus-search-local` | MIT | `CREDITS.md` |
| `egui_kittest` 0.37 | MIT OR Apache-2.0 | `CREDITS.md` |
| `image` 0.25 | MIT OR Apache-2.0 | `CREDITS.md` |
| wgpu / Mesa lavapipe | MIT / MIT | CI-only; note |
| Playwright (optional) | Apache-2.0 | if adopted |

No GPL/AGPL/LGPL tooling. Run `cargo deny check` and `cargo machete`.

### 9.2 Content

- Screenshots use only the bundled defaults or synthetic fixtures. Do not use
  `samples/`; those are third-party parser fixtures, not docs material.
- Attribute the defaults on the first-session page and “About screenshots”:
  - *Another Last Dance* — ContraDB dance 2409, Erik Hoffman.
  - *Sandy Boys* — ABC Session Tunes tune 75815, transcription credit.
- Do not show a commercial soundfont filename. Use “None (built-in synth)” or
  a synthetic `demo.sf2` that is never distributed.
- App and docs code are MIT; the footer links `LICENSE`.
- Screenshots are derivative UI works published under the same license.

### 9.3 Privacy

- No analytics, telemetry, cookies, adverts, or third-party fonts.
- Local search stays in the browser.
- Only GitHub, ContraDB, and ABC Session Tunes are external links.
- The screenshot harness is offline with fixture sources.
- Screenshots must not show proxy tokens, usernames, home directories, device
  IDs, or cached third-party content.
- Use a fake token such as `test-token` or leave the field blank.
- GitHub Pages request logs are GitHub's; link their policy rather than
  claiming no logs.

---

## 10. Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Constructor does I/O (prefs/audio/TTS/SF2) | Non-deterministic or failing CI | `AppDeps::with_deps`; native deps only in `new` |
| No seek/fake playback | Cannot capture count-in/call-now/finished | `Playback` trait or `AudioEngine` test helpers |
| No fixture source | Search/preview screenshots need network | Fixture backend returning `SourceEvent`s |
| Duplicate/missing labels | `get_by_label` ambiguous; accessibility check fails | Label pass; remove duplicate transport buttons; per-item labels |
| User prefs leak into screenshots | Different images per machine | `prefs_path = None`, fixed fixture prefs |
| Synchronous music render | Play blocks; no “preparing” screen | Do not screenshot a state that does not exist; document the pending-rebuild message |
| No paused/playing status label | Pause screenshot is ambiguous | Add a session status label before P1, or drop the shot |
| Mobile two-column pickers | Cramped at 390 px; bad screenshots | Do a responsive pass or use full-width stacked layout |
| UI churn | Every release changes images | Focus crops, gallery, one release refresh |
| GPU raster differences | Flaky checks | Linux + lavapipe, PREDICTABLE, tolerance, artifact |
| Screenshot suite slow | Slow CI | Cache, `--filter`, parallel prefixes |
| PNG bloat | Large clone | 2× only, 300 KB budget, lazy loading, LFS/artifacts if > 25 MB |
| Manual OS images stale | Misleading install docs | Version-stamped `Figure`, release checklist |
| Project Pages base URL | Broken assets | `useBaseUrl`, `trailingSlash: true`, base-path QA |
| Docs claim unbuilt features | Wrong instructions | §3.4 exclusion list; review against the UI |
| Samples leak into docs | Licensing risk | Fixture rule; checker review |

---

## 11. Phased delivery

### Phase 0 — Start now (3–6 days)

**App seams (blocking):**

- [ ] Add `crates/app/src/lib.rs`; move `mod` declarations from `main.rs`;
      make `main.rs` use `contra_app::ContraApp`.
- [ ] Add `AppDeps` and `ContraApp::with_deps`; keep `new()` as the native
      wrapper. No startup I/O when deps are supplied.
- [ ] Add a fixture source backend and a no-op prefs path.
- [ ] Add test playback (trait or `AudioEngine` helpers) and `Speech::stub`.
- [ ] Add an explicit initial state (`needs_render`, beat, playing, finished).
- [ ] Make `Screen` public or add a screen setter for harness scenarios.
- [ ] Label pass: unique Play/Pause/Restart, per-result Preview, per-alias
      Reset, associated text fields/combos/sliders, `Verified only`.
- [ ] Add a session status label (playing/paused/finished) if the pause shot is
      to be P1.
- [ ] Add a narrow-width pass for the picker columns and header controls.

**Docs scaffolding (parallel):**

- [x] Scaffold `website/`, theme, sidebars, landing, download, privacy stub.
- [x] Add `Shot`, `Step`, `Steps`, and `CommandStep`; `Figure` and
      `PlatformTabs` remain for the manual/platform pages.
- [x] Author the P1 manifest with alt/caption/viewport (78 planned shots).
- [x] Add `check-screenshots.mjs` and `npm run check`.
- [x] Write the first 25 user-facing pages and land a successful production
      build under `/Contra-karaoke/`.
- [x] Implement the headless renderer and generate all in-app screenshots.
- [ ] Add the screenshot verification workflow.
- [ ] Capture the manual OS installer images.
- [ ] Prove drift detection with a disposable demo shot.

Exit: `cargo xtask screenshots --filter demo` produces a PNG from the real
app; docs skeleton builds; CI jobs exist.

### Phase 1 — Pilot (3–5 days after Phase 0)

- [ ] Capture `main-idle`, `dance-picker-open`, `dance-picker-search`,
      `prefs-open`, `prefs-alias-list`, `session-call-now`, `main-mobile`,
      `main-light`.
- [ ] Tune determinism, crops, and tolerance.
- [ ] Verify the check detects a one-pixel/label change.
- [ ] Owner review of visual style and crops.

Exit: pilot PNGs committed, `--check` green, page renders locally.

### Phase 2 — Full content (2–3 weeks)

- [ ] Write the task pages and guides; P1 first.
- [ ] Add remaining scenarios and shots.
- [ ] Add manual OS images with attribution.
- [ ] Add error states and fixtures.
- [ ] Add the gallery and “About screenshots”.
- [ ] Full accuracy review against the actual UI.

Exit: all P1 instructions paired and generated; no broken links; manual QA.

### Phase 3 — Polish and launch (3–5 days)

- [ ] Local search tuning.
- [ ] Light-theme set and contrast audit.
- [ ] Optional Playwright smoke test.
- [ ] Image budgets and page performance.
- [ ] Custom domain decision.
- [ ] Enable Pages, deploy, verify URL and 404.
- [ ] Link from README and nightly notes.

Total: roughly 4–6 weeks part-time, with Phase 0 overlapping app work.

---

## 12. Open questions

1. **Custom domain?** Start on the project Pages URL; add one later if desired.
2. **Commit screenshots?** Recommend yes; revisit at ~25 MB.
3. **Host the web app on the same Pages site?** Recommend no; separate pipeline.
4. **Manual Windows/Android images?** Recommend yes for three images; automate
   only if the owner requires zero manual captures.
5. **Callouts in v1?** No; focus crops and captions first.
6. **Exact timing formula in user docs?** Plain-language version plus one
   worked example; link to the roadmap for contributors.
7. **Add the missing UI states (warning banners, preparing, status label,
   shortcuts) before launch?** This affects which screenshots are possible.
   Recommendation: add the status label and responsive pass; defer the rest.
8. **Docs review cadence?** Every release checklist and every UI PR that
   changes a shot.

---

## 13. Appendix A — Example scenarios against the actual UI

These labels assume the label pass in §4.4 has landed.

```rust
// xtask/src/screenshots/scenarios.rs

pub fn run(name: &str, h: &mut Harness<ContraApp>) -> anyhow::Result<()> {
    match name {
        "main_idle" => {
            // default dance/tune; screenshot deps set needs_render=true
        }

        "dance_picker_open" => {
            h.get_by_label("Select dance").click();
        }

        "dance_picker_search" => {
            h.get_by_label("Select dance").click();
            settle(h);
            h.get_by_label("Dance title").type_text("last");
            h.get_by_label("Search dances").click();
        }

        "dance_picker_preview" => {
            h.get_by_label("Select dance").click();
            settle(h);
            h.get_by_label("Search dances").click();
            settle(h);
            h.get_by_label("Preview: Another Last Dance").click();
        }

        "tune_picker_preview" => {
            h.get_by_label("Select Tune").click();
            settle(h);
            h.get_by_label("Search tunes").click();
            settle(h);
            h.get_by_label("Preview: Sandy Boys").click();
        }

        "session_count_in" => {
            h.state_mut().screenshot_set_beat(-2.0);
            h.state_mut().screenshot_set_playing(true);
            h.state_mut().screenshot_set_ready(true);
        }

        "session_call_now" => {
            h.state_mut().screenshot_set_beat(2.0);
            h.state_mut().screenshot_set_playing(true);
            h.state_mut().screenshot_set_ready(true);
        }

        "session_finished" => {
            h.state_mut().screenshot_set_finished(true);
            h.state_mut().screenshot_set_ready(true);
        }

        "prefs_alias_editor" => {
            h.get_by_label("Preferences").click();
            settle(h);
            h.get_by_label("Edit dance move aliases…").click();
            settle(h);
            h.get_by_label("Swing alias").click();
        }

        "main_light" => {
            h.state_mut().screenshot_set_theme(Theme::Light);
        }

        other => anyhow::bail!("unhandled scenario {other:?}"),
    }
    settle(h);
    Ok(())
}

fn settle(h: &mut Harness<ContraApp>) {
    h.run_steps(3);
}
```

The exact accessor names are illustrative; what matters is that the harness can
set the clock/playback state and fixture results without touching the disk,
network, audio device, or system TTS.

## 14. Appendix B — Screenshot ID conventions

- Lowercase kebab-case ASCII: `^[a-z0-9]+(-[a-z0-9]+)*$`.
- Prefix with the screen: `main-`, `dance-picker-`, `tune-picker-`,
  `transport-`, `session-`, `prefs-`, `error-`, `offline-`, `export-`.
- Describe the state, not the click: `session-call-now`, not `click-play`.
- One PNG per ID under `website/static/img/screens/<id>.png`.
- Never reuse an ID for a different state.
- Manual images use `manual-` and live under `img/manual/`.

## 15. Appendix C — App-track checklist, against current files

- [x] `crates/app/src/lib.rs` exists; `main.rs` is a thin platform wrapper.
- [ ] `app.rs`: `pub struct AppDeps`; `ContraApp::with_deps`; `new` delegates.
- [ ] `app.rs`: `prefs_path: Option<PathBuf>`; `save_prefs` no-ops when `None`.
- [ ] `audio.rs`: `Playback` trait + `FakePlayback`, or `silent` + `seek_beat` +
      `force_finished`.
- [ ] `tts.rs`: `Speaker` trait + `Speech::stub()` (available, no-op).
- [ ] `source.rs`: fixture backend returning `SourceEvent`s; no `ehttp` in
      screenshot mode.
- [ ] `defaults.rs`: public fixture constructors for the bundled dance/tune.
- [ ] `app.rs`: public `Screen` or setter; initial screen for picker shots.
- [ ] `app.rs`: screenshot setters for beat, playing, finished, theme, error.
- [ ] `ui.rs`: unique `Preview: <title>` and `Reset <Move> alias` labels.
- [ ] `ui.rs`: associate every text field, combo, slider, and checkbox with its
      visible label; `harness.unnamed_widgets()` is empty on every screen.
- [ ] `ui.rs`: session status label for playing/paused/finished.
- [ ] `ui.rs`: narrow-width layout for the two-column pickers and header.
- [ ] `crates/app` builds and the label test runs on Linux with the CI apt list.

If the app track can land only the first five items, the pilot can still start;
the rest can follow during Phase 1.
