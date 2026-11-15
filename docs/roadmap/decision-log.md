## 15. Decision log

| Date | Decision | Reason |
| --- | --- | --- |
| 2026-11-14 | Rust + `eframe`/`egui` | One codebase for desktop, web, and Android; mature Rust ecosystem |
| 2026-11-14 | `cpal` for all audio | Supports WASAPI, ALSA, AAudio, and WebAudio |
| 2026-11-14 | `abc-parser` + custom sequencer | MIT parser, but playback/repeat handling is project-specific |
| 2026-11-14 | Built-in synth default, optional user SF2 | Small default download, clean licensing, upgrade path to realistic instruments |
| 2026-11-14 | Cloudflare Worker proxy | Only practical way to give WASM online search without redistributing content |
| 2026-11-14 | App name Contra Karaoke | Matches repository and product intent |
| 2026-11-14 | Version stays 0.1.0 | Owner instruction until declared released |
| 2026-11-14 | Dated nightly tags + rolling nightly | Traceable builds without changing semantic version |
| 2026-11-15 | Refresh documentation screenshots only at release prep | Per-worktree regeneration was slow and noisy; prose still tracks every change |
| 2026-11-15 | Host the web build on the documentation site | Lets visitors try the app without downloading; the docs deploy already owns the Pages artifact and can rebuild it from `main` |
