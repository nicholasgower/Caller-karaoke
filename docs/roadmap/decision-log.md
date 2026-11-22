## 15. Decision log

| Date | Decision | Reason |
| --- | --- | --- |
| 2026-11-14 | Rust + `eframe`/`egui` | One codebase for desktop, web, and Android; mature Rust ecosystem |
| 2026-11-14 | `cpal` for all audio | Supports WASAPI, ALSA, AAudio, and WebAudio |
| 2026-11-14 | `abc-parser` + custom sequencer | MIT parser, but playback/repeat handling is project-specific |
| 2026-11-14 | Built-in synth default, optional user SF2 | Small default download, clean licensing, upgrade path to realistic instruments |
| 2026-11-15 | Bundle a trimmed Fluid R3 GM subset (MIT) as the default voice | Sampled instruments by default; 15 MB instead of the 141 MB bank; keeps the procedural synth as fallback |
| 2026-11-15 | Add alto sax, tenor sax, and clarinet melody choices and a verified rebuild wrapper | Real contra pickup bands carry winds; `scripts/rebuild-soundfont.py` regenerates the subset from a pinned MIT source with SHA-256 checks, and `scripts/soundfont-keep.txt` is the one keep-list. The font grows to ~22 MB, still a fraction of the 141 MB bank |
| 2026-11-14 | Cloudflare Worker proxy | Only practical way to give WASM online search without redistributing content |
| 2026-11-14 | App name Contra Karaoke | Matches repository and product intent |
| 2026-11-17 | Rename the app to Caller Karaoke (`caller-karaoke`, crates `caller-*`, Android ID `org.callerkaraoke.app`, docs host `callerkaraoke.nicholasgower.com`) | Owner instruction: the product is a caller trainer, so the name names the caller, not the dance. Files exported under the old format marker still import |
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
| 2026-11-15 | Replace the Nicole TTS voice with Sarah, and add Onyx | Nicole’s soft, whispery delivery did not carry over dance music; Sarah is a clear US female voice and Onyx adds a resonant US male option. Stored Nicole preferences migrate to Sarah |
| 2026-11-16 | Package ONNX Runtime in the Android APK for the Kokoro voice | Keeps the same voice, pronunciation, and caller tone on phones; the pinned 55 MB AAR is verified at build time, only `libonnxruntime.so` is added, both libraries are 16 KB page aligned, and the platform voice remains the fallback |
| 2026-11-17 | Web share links embed the dance and tunes in the `#set=` URL fragment, compressed and base64url-encoded | A self-contained link reconstructs the playlist with no download or proxy; the fragment is never sent to the static host, the payload is versioned, length-capped, and validated as untrusted input. Preferences stay with the recipient, and the control is web-only. A `?set=` query is accepted for hand-edited links |
| 2026-11-22 | Try a checksum-verified Hugging Face mirror when the canonical host fails | A 404 or blocked host stopped first-run neural voice setup; every asset is pinned by SHA-256, so a mirror cannot alter the model and the fallback stays automatic |
| 2026-11-22 | Publish the Kokoro assets as a stable, non-latest GitHub release and make them the default download source | Removes Hugging Face as a single point of failure without Git LFS or growing the installers; every file is pinned by SHA-256, and the asset release is kept out of `/releases/latest` so app update checks still see the newest application release |
| 2026-11-22 | Host the Linux ONNX Runtime archive on that release too | Microsoft's pinned v1.31.0 release URL returned 404, leaving the neural voice unusable; the project-hosted archive is byte-identical to the published 1.31.0 archive, and its checksum and extracted `.so` hash match the existing pins |
