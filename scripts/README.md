# Scripts

Helper scripts for contributors and agents. None of these are required to run
the application.

- `check.ps1` — run formatting, clippy, and tests on Windows.
- `check.sh` — the same checks on Linux/macOS.
- `inspect.ps1` / `inspect.sh` — run the desktop app with the `inspection`
  feature for MCP automation (see `docs/automation.md`).
- `soundfont-subset.py` — rebuild `assets/defaults/fluid-r3-gm-subset.sf2` from
  the full Fluid R3 GM bank. Needs only Python 3; see the file header for the
  command, then run `cargo test -p contra-music --test bundled_soundfont` with
  `CONTRA_REFERENCE_SF2` set to confirm the subset still matches the source.

Run from the repository root. They assume `cargo` is on `PATH`.
