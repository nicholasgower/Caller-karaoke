# Scripts

Helper scripts for contributors and agents. None of these are required to run
the application.

- `check.ps1` — run formatting, clippy, and tests on Windows.
- `check.sh` — the same checks on Linux/macOS.
- `inspect.ps1` / `inspect.sh` — run the desktop app with the `inspection`
  feature for MCP automation (see `docs/automation.md`).
- `profile.ps1` / `profile.sh` — build `contra-perf` in release mode and print
  the CPU profile of every screen and user-facing action. Pass a sample count
  as the first argument; five samples is the default. The report names the
  functions over the frame or action budget and writes nothing except to
  stdout.
- `rebuild-soundfont.py` — regenerate
  `assets/defaults/fluid-r3-gm-subset.sf2` from the full Fluid R3 GM bank. Needs
  only Python 3: it downloads the pinned official release, verifies its
  SHA-256, runs the subsetter below, and prints the kept programs and size.
- `soundfont-keep.txt` — the one keep-list of General MIDI programs in the
  bundled font. Adding an instrument to the preferences means adding its
  program here, then running the rebuild script.
- `soundfont-subset.py` — the low-level SF2 editor the wrapper calls. It copies
  the chosen presets, their generators and their sample data into a new file;
  use it directly only when you already have the bank on disk.

## Rebuild the bundled soundfont

From the repository root, with Python 3 and no other tools:

```sh
python scripts/rebuild-soundfont.py
```

On Windows either `python` or `py -3` works.

That downloads the MIT-licensed Fluid R3 GM release
(`https://github.com/Jacalz/fluid-soundfont/releases/download/v1.0/GM-and-SF3.tar.bz2`,
archive SHA-256 `7cf96877…4d0379`, `FluidR3_GM.sf2` SHA-256 `74594e8f…bc1cb0`),
verifies both hashes, extracts the bank into `target/soundfont-cache/`, and
rewrites the bundled subset from `scripts/soundfont-keep.txt`. The download is
cached, so a second run is offline.

Options:

| Option | Effect |
| --- | --- |
| `--check` | Verify the committed subset against the keep-list and exit. Offline; prints the size, hash, samples, and program names. |
| `--source PATH` | Use a local `FluidR3_GM.sf2` or the pinned `.tar.bz2` instead of downloading. A file that does not match the pinned hashes is rejected unless `--allow-unverified-source` is passed. |
| `--keep 0,21,40` | Override the keep-list for one run (comma-separated bank-0 GM programs). |
| `--output PATH` | Write somewhere other than `assets/defaults/fluid-r3-gm-subset.sf2`. |
| `--cache-dir PATH` | Keep the download and extracted bank elsewhere (default `target/soundfont-cache`). |

After a rebuild, confirm the file still matches the full bank for the kept
programs:

```sh
cargo test -p contra-music --test bundled_soundfont
CONTRA_REFERENCE_SF2=/path/to/FluidR3_GM.sf2 \
    cargo test -p contra-music --test bundled_soundfont -- --ignored
```

`crates/music/tests/bundled_soundfont.rs` also checks that the keep-list matches
the `MelodyInstrument`, `AccompanimentInstrument`, and `Countermelody` enums, so
a missed entry fails in CI rather than at runtime.

## Licensing and attribution

Fluid R3 GM is MIT licensed (© 2000–2002, 2008 Frank Wen; © 2008 Toby Smithe).
The subset is a derived work: it keeps the bank's generators, modulators, and
sample data unchanged, so the MIT notice still applies. `CREDITS.md` and the
app's About section carry the notice. Only the trimmed subset is committed —
never the full bank, a dance database, or a tune database. See `AGENTS.md` §14.

Run from the repository root. They assume `cargo` is on `PATH`.
