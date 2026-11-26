#!/usr/bin/env python3
"""Extract the pinned flutter_tester fixture without adding a font binary."""

from pathlib import Path
import re
import subprocess
import sys

REVISION = "ab2912035e160d9cad6306df307576da6128ebdb"
SOURCE = "engine/src/flutter/runtime/test_font_data.cc"

if len(sys.argv) != 3:
    raise SystemExit("usage: extract-flutter-test-font.py FLUTTER_ROOT OUTPUT_TTF")

root, output = map(Path, sys.argv[1:])
source = subprocess.check_output(
    ["git", "-C", str(root), "show", f"{REVISION}:{SOURCE}"], text=True
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
output.write_bytes(font)
