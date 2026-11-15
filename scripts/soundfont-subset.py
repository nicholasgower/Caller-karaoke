#!/usr/bin/env python3
"""Produce a small SF2 from a larger one, keeping only chosen GM programs.

Contra Karaoke bundles a subset of FluidR3_GM (MIT) so the default playback
uses sampled instruments without shipping the whole 141 MB bank. Nothing in the
Rust crates can write an SF2, and the usual editors (Polyphone, Swami) are
GPL GUI tools, so this dependency-free script does the edit.

Usage:
    py -3 scripts/soundfont-subset.py FluidR3_GM.sf2 out.sf2 --keep 0,21,24,25,40

`--keep` is a comma-separated list of General MIDI program numbers (bank 0).
The output keeps every sample the selected presets reach, rewrites the preset,
instrument, bag, generator and sample-header tables, and copies the modulator
tables unchanged (their indices stay valid because zones keep their order).

The result renders identically to the source for those programs: the same
generators, the same sample data, the same modulators. It is a derived work, so
the source license (MIT for FluidR3_GM) still applies.
"""

from __future__ import annotations

import argparse
import struct
import sys

PHDR = 38
PBAG = 4
PGEN = 4
PMOD = 10
INST = 22
IBAG = 4
IGEN = 4
IMOD = 10
SHDR = 46

GEN_INSTRUMENT = 41
GEN_SAMPLE_ID = 53

# sampleType bits: 0x0002 left, 0x0004 right; a stereo pair links to its twin.
STEREO_MASK = 0x0006


def four_cc(value: bytes) -> str:
    return value.decode("latin-1")


def read_chunks(data: bytes) -> tuple[dict[str, tuple[int, int]], list[str]]:
    """Map "LIST/info", "pdta/phdr", ... to (offset, size) and remember order."""
    if data[0:4] != b"RIFF" or data[8:12] != b"sfbk":
        raise SystemExit("not a SoundFont 2 file (RIFF/sfbk missing)")

    chunks: dict[str, tuple[int, int]] = {}

    def walk(offset: int, end: int, prefix: str) -> None:
        while offset + 8 <= end:
            chunk_id = data[offset : offset + 4]
            size = struct.unpack_from("<I", data, offset + 4)[0]
            body = offset + 8
            if body + size > len(data):
                raise SystemExit(f"chunk {chunk_id!r} runs past the end of the file")
            if chunk_id == b"LIST":
                key = prefix + four_cc(data[body : body + 4])
                chunks[key] = (body + 4, size - 4)
                walk(body + 4, body + size, key + "/")
            else:
                key = prefix + four_cc(chunk_id)
                chunks[key] = (body, size)
            offset = body + size + (size & 1)

    riff_size = struct.unpack_from("<I", data, 4)[0]
    walk(12, min(riff_size + 8, len(data)), "")
    return chunks


def unpack_records(data: bytes, chunk: tuple[int, int], size: int, fmt: str) -> list[tuple]:
    body, length = chunk
    if length % size:
        raise SystemExit(f"chunk length {length} is not a multiple of {size}")
    return [
        struct.unpack_from("<" + fmt, data, body + index * size)
        for index in range(length // size)
    ]


def name_of(raw: bytes) -> bytes:
    return raw.split(b"\0", 1)[0]


class SoundFont:
    def __init__(self, data: bytes) -> None:
        self.data = data
        chunks = read_chunks(data)
        self.chunks = chunks

        self.phdr = unpack_records(data, chunks["pdta/phdr"], PHDR, "20sHHHIII")
        self.pbag = unpack_records(data, chunks["pdta/pbag"], PBAG, "HH")
        self.pmod = unpack_records(data, chunks["pdta/pmod"], PMOD, "HHhHH")
        self.pgen = unpack_records(data, chunks["pdta/pgen"], PGEN, "HH")
        self.inst = unpack_records(data, chunks["pdta/inst"], INST, "20sH")
        self.ibag = unpack_records(data, chunks["pdta/ibag"], IBAG, "HH")
        self.imod = unpack_records(data, chunks["pdta/imod"], IMOD, "HHhHH")
        self.igen = unpack_records(data, chunks["pdta/igen"], IGEN, "HH")
        self.shdr = unpack_records(
            data, chunks["pdta/shdr"], SHDR, "20sIIIIIBbHH"
        )
        self.smpl_body, self.smpl_size = chunks["sdta/smpl"]
        if "sdta/sm24" in chunks:
            raise SystemExit("24-bit sm24 data is not supported by this script")


def zone_generators(bags: list[tuple], generators: list[tuple], start: int, end: int):
    """Yield (bag, [generators]) for the zones of one preset or instrument.

    The last record of every generator chunk is the terminal entry; zones end
    at the next bag's generator index, or at that terminal entry.
    """
    limit = len(generators) - 1
    for index in range(start, end):
        gen_start = bags[index][0]
        gen_end = bags[index + 1][0] if index + 1 < len(bags) else limit
        gen_end = min(gen_end, limit)
        if gen_start > gen_end:
            raise SystemExit("generator indices are not monotonic")
        yield bags[index], generators[gen_start:gen_end]


def subset(source: SoundFont, keep_programs: set[int]) -> bytes:
    presets = source.phdr[:-1]
    wanted = [
        index
        for index, preset in enumerate(presets)
        if preset[2] == 0 and preset[1] in keep_programs
    ]
    found = {presets[index][1] for index in wanted}
    missing = keep_programs - found
    if missing:
        raise SystemExit(f"programs not present in the source font: {sorted(missing)}")

    # Presets -> instruments they reference through the `instrument` generator.
    instruments: list[int] = []
    for preset_index in wanted:
        start = source.phdr[preset_index][3]
        end = source.phdr[preset_index + 1][3]
        for _, generators in zone_generators(source.pbag, source.pgen, start, end):
            for oper, amount in generators:
                if oper == GEN_INSTRUMENT and amount not in instruments:
                    instruments.append(amount)

    # Instruments -> sample headers they reference through `sampleID`.
    samples: list[int] = []
    for instrument_index in instruments:
        start = source.inst[instrument_index][1]
        end = source.inst[instrument_index + 1][1]
        for _, generators in zone_generators(source.ibag, source.igen, start, end):
            for oper, amount in generators:
                if oper == GEN_SAMPLE_ID and amount not in samples:
                    samples.append(amount)
    samples.sort()
    if not samples:
        raise SystemExit("no samples selected")

    # Keep both halves of a stereo pair so the `sampleLink` stays valid.
    pending = list(samples)
    while pending:
        sample_id = pending.pop()
        _, start, end, _, _, _, _, _, link, sample_type = source.shdr[sample_id]
        if sample_type & STEREO_MASK and link and link < len(source.shdr) - 1:
            if link not in samples:
                samples.append(link)
                samples.sort()
                pending.append(link)

    sample_map = {old: new for new, old in enumerate(samples)}
    instrument_map = {old: new for new, old in enumerate(instruments)}

    # --- sample data -------------------------------------------------------
    # SF2 sample offsets count sample frames (16-bit words), not bytes.
    smpl = bytearray()
    new_shdr: list[tuple] = []
    for old in samples:
        name, start, end, loop_start, loop_end, rate, pitch, correction, link, kind = (
            source.shdr[old]
        )
        if end <= start:
            raise SystemExit(f"sample {name!r} has an unusable range {start}..{end}")
        if end > source.smpl_size // 2:
            raise SystemExit(f"sample {name!r} runs past the sample data")
        base = len(smpl) // 2
        smpl.extend(source.data[source.smpl_body + start * 2 : source.smpl_body + end * 2])
        new_shdr.append(
            (
                name,
                base,
                base + (end - start),
                base + (loop_start - start),
                base + (loop_end - start),
                rate,
                pitch,
                correction,
                sample_map.get(link, 0) if kind & STEREO_MASK else 0,
                kind,
            )
        )
    new_shdr.append((b"EOS", 0, 0, 0, 0, 0, 0, 0, 0, 0))
    # Trail a little silence: rustysynth rejects a region whose end equals the
    # sample-data length, and tight packing puts the last sample right at it.
    smpl.extend(b"\0\0" * 64)

    # --- instrument zones --------------------------------------------------
    new_ibag: list[tuple] = []
    new_igen: list[tuple] = []
    new_inst: list[tuple] = []
    for old in instruments:
        name, bag_start = source.inst[old]
        bag_end = source.inst[old + 1][1]
        new_inst.append((name, len(new_ibag)))
        for bag, generators in zone_generators(
            source.ibag, source.igen, bag_start, bag_end
        ):
            new_ibag.append((len(new_igen), bag[1]))
            for oper, amount in generators:
                if oper == GEN_SAMPLE_ID:
                    amount = sample_map[amount]
                new_igen.append((oper, amount))
    new_igen.append((0, 0))
    # The instrument list needs a terminal bag and a terminating instrument
    # record whose bag index points at it.
    new_ibag.append((len(new_igen) - 1, source.ibag[-1][1]))
    new_inst.append((b"EOI", len(new_ibag) - 1))

    # --- preset zones ------------------------------------------------------
    new_pbag: list[tuple] = []
    new_pgen: list[tuple] = []
    new_phdr: list[tuple] = []
    for preset_index in wanted:
        name, preset, bank, bag_start, library, genre, morphology = source.phdr[
            preset_index
        ]
        # phdr keeps a terminating record after the last preset, so this is
        # always in range, including for the final preset of the file.
        bag_end = source.phdr[preset_index + 1][3]
        new_phdr.append((name, preset, bank, len(new_pbag), library, genre, morphology))
        for bag, generators in zone_generators(
            source.pbag, source.pgen, bag_start, bag_end
        ):
            new_pbag.append((len(new_pgen), bag[1]))
            for oper, amount in generators:
                if oper == GEN_INSTRUMENT:
                    amount = instrument_map[amount]
                new_pgen.append((oper, amount))
    new_pgen.append((0, 0))
    new_phdr.append((b"EOP", 0, 0, len(new_pbag), 0, 0, 0))
    new_pbag.append((len(new_pgen) - 1, source.pbag[-1][1]))

    # --- INFO --------------------------------------------------------------
    info: list[tuple[str, bytes]] = []
    for key in ("ifil", "isng"):
        body, size = source.chunks["INFO/" + key]
        info.append((key, source.data[body : body + size]))
    body, size = source.chunks["INFO/INAM"]
    original = name_of(source.data[body : body + size]).decode("latin-1")
    info.append(("INAM", (original + " (Contra Karaoke subset)").encode("latin-1") + b"\0"))
    for key in ("ICRD", "IENG", "ICOP", "ICMT"):
        if "INFO/" + key in source.chunks:
            body, size = source.chunks["INFO/" + key]
            info.append((key, source.data[body : body + size]))
    info.append(
        (
            "ISFT",
            b"contra-karaoke soundfont-subset.py\0",
        )
    )
    # rustysynth's INFO reader ignores the RIFF pad byte after an odd-sized
    # chunk, so keep every INFO payload even rather than relying on padding.
    info = [
        (key, value + b"\0" if len(value) & 1 else value)
        for key, value in info
    ]

    # --- write -------------------------------------------------------------
    def chunk(cid: bytes, payload: bytes) -> bytes:
        return cid + struct.pack("<I", len(payload)) + payload + (b"\0" if len(payload) & 1 else b"")

    def list_chunk(kind: bytes, payload: bytes) -> bytes:
        return chunk(b"LIST", kind + payload)

    def records(rows: list[tuple], fmt: str) -> bytes:
        return b"".join(struct.pack("<" + fmt, *row) for row in rows)

    info_bytes = list_chunk(
        b"INFO",
        b"".join(chunk(key.encode("latin-1"), value) for key, value in info),
    )
    sdta_bytes = list_chunk(b"sdta", chunk(b"smpl", bytes(smpl)))
    pdta_bytes = list_chunk(
        b"pdta",
        chunk(b"phdr", records(new_phdr, "20sHHHIII"))
        + chunk(b"pbag", records(new_pbag, "HH"))
        + chunk(b"pmod", records(source.pmod, "HHhHH"))
        + chunk(b"pgen", records(new_pgen, "HH"))
        + chunk(b"inst", records(new_inst, "20sH"))
        + chunk(b"ibag", records(new_ibag, "HH"))
        + chunk(b"imod", records(source.imod, "HHhHH"))
        + chunk(b"igen", records(new_igen, "HH"))
        + chunk(b"shdr", records(new_shdr, "20sIIIIIBbHH")),
    )
    body = b"sfbk" + info_bytes + sdta_bytes + pdta_bytes
    return b"RIFF" + struct.pack("<I", len(body)) + body


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", help="input .sf2")
    parser.add_argument("output", help="output .sf2")
    parser.add_argument(
        "--keep",
        default="0,21,25,40",
        help="comma-separated bank-0 GM programs to keep (default: %(default)s)",
    )
    args = parser.parse_args(argv[1:])

    keep = {int(value) for value in args.keep.split(",") if value.strip()}
    data = open(args.source, "rb").read()
    font = SoundFont(data)
    out = subset(font, keep)

    with open(args.output, "wb") as handle:
        handle.write(out)

    # Read the result back with the same parser so a malformed file fails here.
    check = SoundFont(open(args.output, "rb").read())
    print(
        f"{args.output}: {len(out):,} bytes, "
        f"{len(check.phdr) - 1} presets, {len(check.shdr) - 1} samples, "
        f"{check.smpl_size:,} sample bytes"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
