#!/usr/bin/env python3
"""Check composer chrome/caret against the locked, embedded Bevy default font.

Uses only Python's standard library; does not build Rust or inspect user input.
Run after Cargo fetch, or pass --font with the cached bevy_text font path.
This is source/cmap coverage, not text shaping or rendered UI acceptance.
"""

import argparse
import ast
import hashlib
import os
from pathlib import Path
import re
import struct
import sys


FONT_SHA256 = "45131c5e913a5c7b4311101303e3c320a97b93a27a64b7262b533ee0d31ae03c"
SOURCE = "crates/alife_game_app/src/production_conversation_lineage_ui.rs"


def unicode_cmap(data):
    """Read the format-4 Unicode cmap in the pinned Fira Mono subset."""
    table_count = struct.unpack_from(">H", data, 4)[0]
    for index in range(table_count):
        tag, _, offset, length = struct.unpack_from(">4sIII", data, 12 + index * 16)
        if tag == b"cmap":
            cmap = data[offset : offset + length]
            break
    else:
        raise ValueError("font has no cmap")
    record_count = struct.unpack_from(">H", cmap, 2)[0]
    for index in range(record_count):
        platform, encoding, offset = struct.unpack_from(">HHI", cmap, 4 + index * 8)
        if platform == 0 or (platform == 3 and encoding in (1, 10)):
            if struct.unpack_from(">H", cmap, offset)[0] == 4:
                break
    else:
        raise ValueError("pinned font has no format-4 Unicode cmap")
    length, _, twice_segments = struct.unpack_from(">HHH", cmap, offset + 2)
    table = cmap[offset : offset + length]
    segments = twice_segments // 2
    covered = set()
    for index in range(segments):
        end = struct.unpack_from(">H", table, 14 + 2 * index)[0]
        start = struct.unpack_from(">H", table, 16 + 2 * segments + 2 * index)[0]
        delta = struct.unpack_from(">h", table, 16 + 4 * segments + 2 * index)[0]
        range_position = 16 + 6 * segments + 2 * index
        range_offset = struct.unpack_from(">H", table, range_position)[0]
        for codepoint in range(start, min(end, 0xFFFE) + 1):
            glyph = (codepoint + delta) & 0xFFFF
            if range_offset:
                glyph = struct.unpack_from(
                    ">H", table, range_position + range_offset + 2 * (codepoint - start)
                )[0]
                if glyph:
                    glyph = (glyph + delta) & 0xFFFF
            if glyph:
                covered.add(codepoint)
    return covered


def composer_template(source):
    match = re.search(
        r'fn speech_entry_text\([^\n]*\) -> String \{\s*format!\(\s*'
        r'("(?:[^"\\]|\\.)*")(?P<arguments>.*?)\n    \)\n\}',
        source,
        re.DOTALL,
    )
    if not match or not re.search(r",\s*state\.input\s*$", match["arguments"]):
        raise ValueError("cannot verify the existing unmodified input display argument")
    template = ast.literal_eval(match[1])
    caret = re.search(r"\n\{\}([^\n])\n", template)
    if template.count("{}") != 2 or not caret:
        raise ValueError("expected two display arguments and a single end-caret character")
    return template, caret[1]


def main():
    root = Path(__file__).resolve().parent.parent
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=root / SOURCE)
    parser.add_argument("--lockfile", type=Path, default=root / "Cargo.lock")
    parser.add_argument("--font", type=Path)
    args = parser.parse_args()
    try:
        lock = args.lockfile.read_text(encoding="utf-8")
        versions = re.findall(r'\[\[package\]\]\s*name = "bevy_text"\s*version = "([^"]+)"', lock)
        if versions != ["0.18.1"]:
            raise ValueError("locked bevy_text changed; review the embedded font and coverage pin")
        font = args.font
        if font is None:
            cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
            candidates = sorted(cargo_home.glob("registry/src/*/bevy_text-0.18.1/src/FiraMono-subset.ttf"))
            if len(candidates) != 1:
                raise ValueError("expected one cached Bevy font; pass --font explicitly")
            font = candidates[0]
        data = font.read_bytes()
        if hashlib.sha256(data).hexdigest() != FONT_SHA256:
            raise ValueError("font bytes do not match the locked embedded Bevy default font")
        covered = unicode_cmap(data)
        template, caret = composer_template(args.source.read_text(encoding="utf-8"))
        fixed_chrome = template.replace("{}", "").replace("\n", "")
        missing = sorted({ord(character) for character in fixed_chrome} - covered)
        if missing:
            raise ValueError("composer chrome/caret lacks glyphs: " + ", ".join(f"U+{value:04X}" for value in missing))
        print(f"PASS: composer end caret U+{ord(caret):04X} and fixed chrome covered by locked Bevy font ({len(covered)} codepoints)")
        print(f"font_sha256={FONT_SHA256}")
        print("Source/cmap gate only; user input is not coverage-filtered and rendering is not established")
        return 0
    except (OSError, ValueError, struct.error, SyntaxError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
