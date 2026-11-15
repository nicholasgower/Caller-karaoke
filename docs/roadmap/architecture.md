## 4. Architecture

### 4.1 Workspace layout

```
contra-karaoke/
  Cargo.toml                 workspace, shared version 0.1.0
  crates/
    core/                    domain, timing, aliases, role terms, preferences
    music/                   ABC sequencing, arrangement, synth, MIDI export
    sources/                 provider traits, ContraDB, ABC Session Tunes,
                             HTTP, cache
    app/                     egui UI, transport, TTS, platform bootstrap
    web/                     WASM entry point and Trunk page
  xtask/                     packaging/changelog helper commands
  scripts/                   Scripts to aid agents in development
  assets/
    defaults/                bundled default dance and tune (attributed)
    icons/                   app icons
  proxy/                     Cloudflare Worker for web CORS
  fixtures/                  small, attributed parser test fixtures
  docs/                      architecture, call timing, sources, packaging
  .github/workflows/         CI, nightly, changelog, optional Pages deploy
  ROADMAP.md
  AGENTS.md
  CREDITS.md
  CHANGELOG.md
  README.md
```

### 4.2 Dependency direction

```
app  ──▶ sources ──▶ core
 │          │          ▲
 └────────▶ music ─────┘
```

Rules:

- `core` has no UI, HTTP, filesystem, audio, or platform dependencies.
- `music` depends only on `core` and pure-Rust libraries.
- `sources` depends on `core`; HTTP is abstracted behind a small client.
- `app` is the only crate that touches `egui`, `cpal`, `tts`, file dialogs, and
  platform storage.
- Parsers are pure functions: bytes in, domain model out. A parser never
  performs a request.

### 4.3 Data flow

```
UI action
  └─▶ SourceService (provider + cache + HTTP)
        └─▶ Dance / Tune domain models
              ├─▶ AliasEngine + RoleTerms
              ├─▶ PracticeTimeline (call events)
              └─▶ ABC parser ─▶ Score ─▶ Arranger (64-beat loop)
                                    └─▶ SynthBackend ─▶ PCM buffer
                                    └─▶ MIDI export (.mid)
        └─▶ Transport (cpal) ─▶ audio clock ─▶ karaoke timeline
```

### 4.4 Provider abstraction

Dance and tune sources implement traits. The UI never knows a provider's URL
shape or JSON/HTML format.

```rust
pub trait DanceProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn filter_specs(&self) -> Vec<FilterSpec>;
    fn search(&self, query: &DanceQuery) -> HttpCall;
    fn parse_search(&self, body: &[u8]) -> Result<DanceSearchPage>;
    fn detail(&self, id: &str) -> HttpCall;
    fn parse_dance(&self, body: &[u8], url: &str) -> Result<Dance>;
    fn parse_url(&self, url: &str) -> Option<String>;
}

pub trait TuneProvider: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn filter_specs(&self) -> Vec<FilterSpec>;
    fn search(&self, query: &TuneQuery) -> HttpCall;
    fn parse_search(&self, body: &[u8]) -> Result<TuneSearchPage>;
    fn detail(&self, id: &str) -> HttpCall;
    fn parse_tune(&self, body: &[u8], url: &str) -> Result<Tune>;
    fn parse_url(&self, url: &str) -> Option<String>;
}
```

`FilterSpec` describes a field generically (text, select, checkbox, keyword
row), so the picker screens can render any provider's search options.
