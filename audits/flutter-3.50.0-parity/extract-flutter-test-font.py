#!/usr/bin/env python3
"""Extract the pinned flutter_tester fixture without adding a font binary."""

from pathlib import Path
import argparse
import io
import re
import subprocess

REVISION = "ab2912035e160d9cad6306df307576da6128ebdb"
SOURCE = "engine/src/flutter/runtime/test_font_data.cc"

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("flutter_root", type=Path)
parser.add_argument("output_ttf", type=Path)
parser.add_argument(
    "--arabic-lam-fixture",
    action="store_true",
    help="map U+0644 to the test face's full-width square instead of using system fallback",
)
args = parser.parse_args()

source = subprocess.check_output(
    ["git", "-C", str(args.flutter_root), "show", f"{REVISION}:{SOURCE}"], text=True
)
match = re.search(
    r"kFlutterTestFont\[\] = \{(.*?)\};\s+"
    r"static const unsigned int kFlutterTestFontLength = (\d+);",
    source, re.DOTALL,
)
if match is None:
    raise SystemExit("Pinned engine is missing its FlutterTest font")
font = bytes(int(value, 16) for value in re.findall(r"0x([\da-fA-F]{2})", match[1]))
if len(font) != int(match[2]) or not font.startswith(b"\0\x01\0\0"):
    raise SystemExit("Unexpected embedded FlutterTest font size or header")
if not args.arabic_lam_fixture:
    args.output_ttf.write_bytes(font)
else:
    try:
        from fontTools.ttLib import TTFont
    except ImportError:
        parser.error("--arabic-lam-fixture requires the optional Python fontTools package")
    # The original Linux Flutter test sizes an unsupported lam with the
    # full-width missing glyph. Parley searches system fallback instead.
    # Keep the script character and bidi behavior, but give it that same
    # test advance without changing production font selection.
    with TTFont(io.BytesIO(font)) as face:
        square = face.getBestCmap()[ord("a")]
        assert face["hmtx"][square][0] == face["hmtx"][".notdef"][0]
        for encoding in face["cmap"].tables:
            if encoding.isUnicode():
                encoding.cmap[0x0644] = square
        face.save(args.output_ttf)
