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
parser.add_argument(
    "--emoji-caret-fixture",
    action="store_true",
    help="give the caret test's emoji and Hebrew scalars the pinned missing-glyph advance",
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
if not (args.arabic_lam_fixture or args.emoji_caret_fixture):
    args.output_ttf.write_bytes(font)
else:
    try:
        from fontTools.ttLib import TTFont
        from fontTools.ttLib.tables._c_m_a_p import CmapSubtable
    except ImportError:
        parser.error("the optional fixture switches require Python fontTools")
    # flutter_tester sizes unsupported scalars using the full-width
    # missing glyph. Keep their original Unicode properties while
    # removing platform-specific system fallback in exact caret tests.
    characters = ("ل" if args.arabic_lam_fixture else "") + (
        "👩👦👧👏🏽🚀מילים" if args.emoji_caret_fixture else ""
    )
    with TTFont(io.BytesIO(font)) as face:
        square = face.getBestCmap()[ord("a")]
        assert face["hmtx"][square][0] == face["hmtx"][".notdef"][0]
        cmap = face["cmap"]
        if args.emoji_caret_fixture and not any(
            encoding.isUnicode() and encoding.format == 12 for encoding in cmap.tables
        ):
            encoding = CmapSubtable.newSubtable(12)
            encoding.platformID, encoding.platEncID, encoding.language = 3, 10, 0
            encoding.cmap = dict(face.getBestCmap())
            cmap.tables.append(encoding)
        for encoding in cmap.tables:
            if encoding.isUnicode():
                if args.emoji_caret_fixture:
                    encoding.cmap[0x200D] = "Zero Advance"
                for character in characters:
                    if ord(character) <= 0xFFFF or encoding.format == 12:
                        encoding.cmap[ord(character)] = square
        face.save(args.output_ttf)
