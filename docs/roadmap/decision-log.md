## 15. Decision log

| Date | Decision | Reason |
| --- | --- | --- |
| 2026-11-14 | Rust + `eframe`/`egui` | One codebase for desktop, web, and Android; mature Rust ecosystem |
| 2026-11-14 | `cpal` for all audio | Supports WASAPI, ALSA, AAudio, and WebAudio |
| 2026-11-14 | `abc-parser` + custom sequencer | MIT parser, but playback/repeat handling is project-specific |
| 2026-11-14 | Built-in synth default, optional user SF2 | Small default download, clean licensing, upgrade path to realistic instruments |
| 2026-11-15 | Bundle a trimmed Fluid R3 GM subset (MIT) as the default voice | Sampled instruments by default; 15 MB instead of the 141 MB bank; keeps the procedural synth as fallback |
| 2026-11-14 | Cloudflare Worker proxy | Only practical way to give WASM online search without redistributing content |
| 2026-11-14 | App name Contra Karaoke | Matches repository and product intent |
| 2026-11-14 | Version stays 0.1.0 | Owner instruction until declared released |
| 2026-11-14 | Dated nightly tags + rolling nightly | Traceable builds without changing semantic version |
| 2026-11-15 | Refresh documentation screenshots only at release prep | Per-worktree regeneration was slow and noisy; prose still tracks every change |
| 2026-11-15 | Host the web build on the documentation site | Lets visitors try the app without downloading; the docs deploy already owns the Pages artifact and can rebuild it from `main` |
| 2026-11-15 | Count-in is four dance beats ("four potatoes") | Owner correction: the earlier three-beat choice was wrong. Four is the contra convention and gives the first call more room |
| 2026-11-15 | Pursue both human-caller training and autonomous calling | The same timeline supports both; an autonomous caller is a strong model for a learner |
| 2026-11-15 | Calls should fit the beat via long/medium/short variants | Research: good callers use 4/2/1/0-beat variants and thin over the dance; stretching every alias is a training crutch |
| 2026-11-15 | Defer the dance model and everything that depends on it | Who/where and positional calling need a real choreography model; roadmap only until it is designed and accepted |
| 2026-11-15 | Commanding caller tone by default, via a pitch shift and clear-speech phoneme/EQ pass | Kokoro has no emotion input and drops punctuation; holding stressed vowels and lifting presence makes diction carry without changing the syllable-based lead |
| 2026-11-15 | Omit the word “once” and connect “and a half” in calls | Callers do not say a one-count; the connected tail removes the four-word pause without changing the card |
