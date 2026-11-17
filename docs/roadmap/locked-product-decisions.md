## 2. Locked product decisions

| Decision | Choice |
| --- | --- |
| Product name | **Contra Karaoke** |
| Crate/binary prefix | `contra-karaoke`, `contra-core`, `contra-music`, `contra-sources`, `contra-app` |
| Android application ID | `org.contrakaraoke.app` |
| Desktop UI toolkit | `eframe` / `egui` |
| Audio I/O | `cpal` |
| ABC parsing | `abc-parser` plus a project-owned sequencer |
| Default instruments | Editable band list: two melody fiddles, one piano, two countermelody fiddles, three foot taps; any pitched part can use any voice |
| Output layout | Stereo by default; mono and five-channel surround (front L/R, centre, rear L/R) optional |
| Caller level | Separate from the band master; both persist with the other preferences |
| Soundfont support | Bundled Fluid R3 GM subset by default; optional user-supplied SF2 |
| Web data access | Small Cloudflare Worker CORS proxy; bundled defaults as fallback |
| Theme | Dark grey by default; light grey option |
| Count-in | Four dance beats ("four potatoes"), built from the tune's first chord |
| Version until further notice | **0.1.0** |
| Windows packaging | Inno Setup installer + portable ZIP |
| Releases | Manual, version-tagged `v<version>` builds cut with `scripts/release.mjs` |
| Branching | Feature work merges into `dev`; `main` advances only for releases |

The default dance is **Another Last Dance** (ContraDB dance 2409).
The default tune is **Sandy Boys** (ABC Session Tunes tune 75815).
