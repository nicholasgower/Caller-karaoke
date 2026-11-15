# Sources

Adding a dance or tune website should not require changes to the session engine.

1. Add a module under `crates/sources/src/` implementing
   `DanceProvider` or `TuneProvider`.
2. Keep request building and parsing as pure functions.
3. Describe search fields with `FilterSpec`.
4. Register the provider in `provider.rs`.
5. Add fixture tests; never call the network in tests.
6. Add attribution to `CREDITS.md`.
7. Add a URL parser for direct links.

Current endpoints and search behavior are documented in
`docs/roadmap/data-sources.md` (§7).
