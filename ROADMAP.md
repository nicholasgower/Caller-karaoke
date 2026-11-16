# Contra Karaoke — Roadmap

**Status:** 0.1.0 development — desktop and web MVP implemented; Android and
release hardening in progress · **Version:** 0.1.0 (unreleased) · **License:** MIT

Contra Karaoke is a caller-training app for contra dancing. It behaves like a
karaoke machine for callers: the dance card and a moving call prompt show the
caller what to say, while ABC-sourced music plays in sync. Calls are timed to
land **before** the dancers execute each figure, with lead time derived from how
long the words take to say. The same engine is also intended to call a dance on
its own after 0.1.0; §17 records both goals. The same Rust codebase targets
Windows, Linux, the web (WASM), and Android.

The chapters under [`docs/roadmap/`](docs/roadmap/goals.md) are the source of
truth; this index only links them. Each chapter keeps its original section
number, so a reference such as "ROADMAP.md §7" still points at the same text.

---

1. [Goals](docs/roadmap/goals.md)
2. [Locked product decisions](docs/roadmap/locked-product-decisions.md)
3. [User-facing feature set](docs/roadmap/user-facing-feature-set.md)
4. [Architecture](docs/roadmap/architecture.md)
5. [Timing model (critical element)](docs/roadmap/timing-model.md)
6. [Music pipeline](docs/roadmap/music-pipeline.md)
7. [Data sources](docs/roadmap/data-sources.md)
8. [UI and visual design](docs/roadmap/ui-and-visual-design.md)
9. [Platforms, packaging, and releases](docs/roadmap/platforms-packaging-and-releases.md)
10. [Testing strategy](docs/roadmap/testing-strategy.md)
11. [Documentation and licensing](docs/roadmap/documentation-and-licensing.md)
12. [Milestones](docs/roadmap/milestones.md)
13. [Risks and mitigations](docs/roadmap/risks-and-mitigations.md)
14. [Future work (post-0.1.0)](docs/roadmap/future-work.md)
15. [Decision log](docs/roadmap/decision-log.md)
16. [Open items](docs/roadmap/open-items.md)
17. [Calling and teaching](docs/roadmap/calling-and-teaching.md)

Related design note: [ABC editor roadmap](docs/roadmap/abc-editor.md).

---

## Developer references

- [Memory use](docs/memory.md) — where the process memory goes and how to
  reduce it, with the in-app memory table.
- [Automation](docs/automation.md) — driving the running app through the MCP
  server for UI testing.
