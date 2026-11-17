#!/usr/bin/env python3
"""Rebuild the bundled Fluid R3 GM subset from the full MIT-licensed bank.

`assets/defaults/fluid-r3-gm-subset.sf2` is a derived work: the programs listed
in `scripts/soundfont-keep.txt`, copied byte for byte out of FluidR3_GM (MIT,
Frank Wen, Toby Smithe). This wrapper downloads the pinned official bank,
verifies its SHA-256, runs the low-level editor `soundfont-subset.py`, checks
that every kept program survived, and prints the result. Adding an instrument
means editing the keep-list and running this script; nothing else changes.

    python scripts/rebuild-soundfont.py                 # rebuild the asset
    python scripts/rebuild-soundfont.py --check         # verify it, offline
    python scripts/rebuild-soundfont.py --source C:\\path\\FluidR3_GM.sf2
    python scripts/rebuild-soundfont.py --keep 0,21,40,65,71

Only the Python 3 standard library is used, so it runs on Windows without a
toolchain, and no archive/zip tool is needed: the pinned release is a .tar.bz2,
which the `tarfile` module reads directly.

Licensing: Fluid R3 GM is MIT licensed. The subset keeps its generators,
modulators, and sample data, so the MIT notice in CREDITS.md still applies to
the shipped file. The full 141 MB bank is never committed or redistributed.
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import shutil
import sys
import tarfile
import urllib.error
import urllib.request
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent
DEFAULT_OUTPUT = REPO_ROOT / "assets" / "defaults" / "fluid-r3-gm-subset.sf2"
DEFAULT_CACHE = REPO_ROOT / "target" / "soundfont-cache"

# --- pinned source ---------------------------------------------------------
# The official releases of the MIT-licensed bank. Pin the asset URL and both
# hashes: the archive proves the download, and the member hash proves that the
# extraction step picked the file we hashed in the first place.
SOURCE_URL = (
    "https://github.com/Jacalz/fluid-soundfont/releases/download/v1.0/"
    "GM-and-SF3.tar.bz2"
)
SOURCE_ARCHIVE_SHA256 = (
    "7cf968778cfbd0abdb80c57ca5e1c9e29dd9c4251042c538522a5f364a4d0379"
)
SOURCE_SF2_MEMBER = "GM&SF3/FluidR3_GM.sf2"
SOURCE_SF2_SHA256 = "74594e8f4250680adf590507a306655a299935343583256f3b722c48a1bc1cb0"
SOURCE_SF2_BYTES = 148_398_306

USER_AGENT = "caller-karaoke-soundfont-rebuild/1.0 (+https://github.com/nicholasgower/Caller-karaoke)"


def load_subsetter():
    """Import `soundfont-subset.py`, whose hyphenated name is not a module."""
    path = SCRIPT_DIR / "soundfont-subset.py"
    spec = importlib.util.spec_from_file_location("soundfont_subset", path)
    if spec is None or spec.loader is None:
        raise SystemExit(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def human_size(count: int) -> str:
    return f"{count:,} bytes ({count / (1024 * 1024):.1f} MB)"


def download(url: str, destination: Path) -> None:
    """Download `url` to `destination`, showing a plain progress line."""
    destination.parent.mkdir(parents=True, exist_ok=True)
    partial = destination.with_suffix(destination.suffix + ".part")
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    print(f"downloading {url}")
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            total = int(response.headers.get("Content-Length", "0"))
            done = 0
            last_percent = -10
            with partial.open("wb") as handle:
                while True:
                    block = response.read(1 << 20)
                    if not block:
                        break
                    handle.write(block)
                    done += len(block)
                    if total:
                        percent = 100.0 * done / total
                        # A fresh line every tenth keeps a piped log readable
                        # instead of one line per megabyte.
                        if percent - last_percent >= 10.0:
                            last_percent = percent
                            print(
                                f"\r  {percent:5.1f}%  {human_size(done)}",
                                end="",
                                flush=True,
                            )
            print(f"\r  100.0%  {human_size(done)}")
            print()
    except (urllib.error.URLError, TimeoutError) as error:
        partial.unlink(missing_ok=True)
        raise SystemExit(
            f"download failed: {error}\n"
            "Pass --source with a local FluidR3_GM.sf2 or the release archive."
        ) from error
    partial.replace(destination)


def extract_sf2(archive: Path, cache: Path) -> Path:
    """Copy `FluidR3_GM.sf2` out of the pinned release archive."""
    target = cache / "FluidR3_GM.sf2"
    print(f"extracting {SOURCE_SF2_MEMBER} from {archive.name}")
    try:
        with tarfile.open(archive, "r:bz2") as bundle:
            member = next(
                (item for item in bundle.getmembers() if item.name.endswith("FluidR3_GM.sf2")),
                None,
            )
            if member is None:
                raise SystemExit(f"{archive} does not contain FluidR3_GM.sf2")
            source = bundle.extractfile(member)
            if source is None:
                raise SystemExit(f"cannot read {member.name} from {archive}")
            with target.open("wb") as handle:
                shutil.copyfileobj(source, handle)
    except tarfile.TarError as error:
        raise SystemExit(f"{archive} is not a readable .tar.bz2: {error}") from error
    return target


def verify_source(path: Path, allow_unverified: bool) -> None:
    """Check a local source file against the pinned hashes."""
    name = path.name.lower()
    if name.endswith((".tar.bz2", ".tbz2", ".tbz")):
        digest = sha256_of(path)
        if digest != SOURCE_ARCHIVE_SHA256:
            if not allow_unverified:
                raise SystemExit(
                    f"{path} is not the pinned release archive.\n"
                    f"  expected {SOURCE_ARCHIVE_SHA256}\n"
                    f"  found    {digest}\n"
                    "Pass --allow-unverified-source to use a different bank."
                )
            print(f"warning: {path} is not the pinned archive; using it anyway")
        return

    size = path.stat().st_size
    digest = sha256_of(path)
    if digest == SOURCE_SF2_SHA256:
        print(f"source {path.name}: pinned Fluid R3 GM ({human_size(size)})")
        return
    if allow_unverified:
        print(
            f"warning: {path.name} is not the pinned bank "
            f"({digest}, {human_size(size)}); using it anyway"
        )
        return
    raise SystemExit(
        f"{path} is not the pinned Fluid R3 GM bank.\n"
        f"  expected {SOURCE_SF2_SHA256} ({SOURCE_SF2_BYTES:,} bytes)\n"
        f"  found    {digest} ({size:,} bytes)\n"
        "Pass --allow-unverified-source to build from a different bank."
    )


def obtain_source(args) -> Path:
    """Return the full bank: a local --source, or the verified download."""
    if args.source is not None:
        path = Path(args.source).expanduser().resolve()
        if not path.is_file():
            raise SystemExit(f"--source {path} does not exist")
        verify_source(path, args.allow_unverified_source)
        if path.name.lower().endswith((".tar.bz2", ".tbz2", ".tbz")):
            extracted = extract_sf2(path, args.cache_dir)
            if sha256_of(extracted) != SOURCE_SF2_SHA256 and not args.allow_unverified_source:
                raise SystemExit(
                    f"{path} contains a different FluidR3_GM.sf2; "
                    "pass --allow-unverified-source to use it"
                )
            return extracted
        return path

    archive = args.cache_dir / Path(SOURCE_URL).name
    if archive.is_file() and sha256_of(archive) == SOURCE_ARCHIVE_SHA256:
        print(f"using cached {archive}")
    else:
        if archive.is_file():
            print(f"discarding stale {archive}")
            archive.unlink()
        download(SOURCE_URL, archive)
        digest = sha256_of(archive)
        if digest != SOURCE_ARCHIVE_SHA256:
            archive.unlink()
            raise SystemExit(
                "downloaded archive failed SHA-256 verification\n"
                f"  expected {SOURCE_ARCHIVE_SHA256}\n"
                f"  found    {digest}"
            )
    extracted = extract_sf2(archive, args.cache_dir)
    digest = sha256_of(extracted)
    if digest != SOURCE_SF2_SHA256:
        raise SystemExit(
            "extracted FluidR3_GM.sf2 failed SHA-256 verification\n"
            f"  expected {SOURCE_SF2_SHA256}\n"
            f"  found    {digest}"
        )
    return extracted


def presets_of(font) -> dict[int, str]:
    """Bank-0 programs in a parsed font, mapped to their preset names."""
    return {
        preset[1]: preset[0].split(b"\0", 1)[0].decode("latin-1")
        for preset in font.phdr[:-1]
        if preset[2] == 0
    }


def report(path: Path, keep: list[int], subsetter) -> None:
    """Check that `path` holds exactly `keep` and print what it contains."""
    font = subsetter.SoundFont(path.read_bytes())
    presets = presets_of(font)
    missing = [program for program in keep if program not in presets]
    extra = sorted(program for program in presets if program not in keep)
    if missing:
        raise SystemExit(
            f"{path} is missing programs {missing}; "
            "run `python scripts/rebuild-soundfont.py` to regenerate it"
        )
    if extra:
        raise SystemExit(f"{path} has programs not in the keep-list: {extra}")
    print(
        f"verified {path.relative_to(REPO_ROOT) if path.is_relative_to(REPO_ROOT) else path}"
    )
    print(f"  size    {human_size(path.stat().st_size)}")
    print(f"  sha256  {sha256_of(path)}")
    print(f"  samples {len(font.shdr) - 1:,} ({font.smpl_size:,} sample bytes)")
    print(f"  programs ({len(keep)})")
    for program in keep:
        print(f"    {program:3d}  {presets[program]}")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="verify the committed subset against the keep-list and exit (offline)",
    )
    parser.add_argument(
        "--source",
        help="full FluidR3_GM.sf2 or the pinned release archive, instead of downloading",
    )
    parser.add_argument(
        "--keep",
        help="comma-separated bank-0 GM programs, overriding scripts/soundfont-keep.txt",
    )
    parser.add_argument(
        "--output",
        default=str(DEFAULT_OUTPUT),
        help="output .sf2 (default: assets/defaults/fluid-r3-gm-subset.sf2)",
    )
    parser.add_argument(
        "--cache-dir",
        default=str(DEFAULT_CACHE),
        help="where the download and the extracted bank are kept (default: target/soundfont-cache)",
    )
    parser.add_argument(
        "--allow-unverified-source",
        action="store_true",
        help="accept a --source bank that does not match the pinned hashes",
    )
    args = parser.parse_args(argv[1:])
    args.cache_dir = Path(args.cache_dir).expanduser().resolve()
    output = Path(args.output).expanduser().resolve()

    subsetter = load_subsetter()
    if args.keep is None:
        keep = subsetter.read_keep_list()
    else:
        keep = sorted({int(value) for value in args.keep.split(",") if value.strip()})
    if not keep:
        raise SystemExit("the keep-list is empty")

    if args.check:
        print(f"pinned source {SOURCE_URL}")
        print(f"  archive sha256 {SOURCE_ARCHIVE_SHA256}")
        print(f"  bank    sha256 {SOURCE_SF2_SHA256}")
        if not output.is_file():
            raise SystemExit(
                f"{output} is missing; run `python scripts/rebuild-soundfont.py`"
            )
        report(output, keep, subsetter)
        print("OK")
        return 0

    source = obtain_source(args)
    font = subsetter.SoundFont(source.read_bytes())
    print(f"keeping programs {keep}")
    out = subsetter.subset(font, set(keep))

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(out)
    print(f"wrote {output.relative_to(REPO_ROOT) if output.is_relative_to(REPO_ROOT) else output}")
    report(output, keep, subsetter)

    print()
    print("Next: cargo test -p caller-music --test bundled_soundfont")
    print(
        "      CALLER_REFERENCE_SF2=%s cargo test -p caller-music "
        "--test bundled_soundfont -- --ignored" % source
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
