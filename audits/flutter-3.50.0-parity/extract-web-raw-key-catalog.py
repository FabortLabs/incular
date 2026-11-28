#!/usr/bin/env python3
"""Translate the three pinned Flutter DOM keyboard lookup tables."""

import ast
import json
from pathlib import Path
import re
import subprocess
import sys

REVISION = "ab2912035e160d9cad6306df307576da6128ebdb"
root, output = map(Path, sys.argv[1:])
services = "packages/flutter/lib/src/services/"


def pinned(name):
    return subprocess.check_output(
        ["git", "-C", str(root), "show", f"{REVISION}:{services}{name}"], text=True
    )


maps = pinned("keyboard_maps.g.dart")
keys = pinned("keyboard_key.g.dart")
identifiers = {
    kind: {
        name: int(value, 16)
        for name, value in re.findall(
            rf"static const {kind} (\w+) = {kind}\((0x[0-9a-f]+)\);", keys
        )
    }
    for kind in ("LogicalKeyboardKey", "PhysicalKeyboardKey")
}
dart_string = r"'(?:[^'\\]|\\.)*'"


def spelling(token):
    return json.dumps(ast.literal_eval(token.replace(r"\$", "$")), ensure_ascii=False)


def table(name):
    start = maps.index("{\n", maps.index(f" {name} = ")) + 2
    return maps[start : maps.index("\n};", start)]


rows = [
    "// Copyright 2014 The Flutter Authors. All rights reserved.",
    "// Use of this source code is governed by the BSD-style license in raw_key_fuchsia.LICENSE.",
    "// Generated from Flutter 3.50.0 by extract-web-raw-key-catalog.py. Do not hand edit.",
    "",
]
for name, kind, constant, rust_type in (
    ("kWebToPhysicalKey", "PhysicalKeyboardKey", "PHYSICAL", "u32"),
    ("kWebToLogicalKey", "LogicalKeyboardKey", "LOGICAL", "u64"),
):
    body = table(name)
    entries = [
        (ast.literal_eval(key), key, identifiers[kind][identifier])
        for key, identifier in re.findall(rf"({dart_string}): {kind}\.(\w+),", body)
    ]
    assert len(entries) == len(re.findall(rf"\b{kind}\.", body)), name
    rows.extend(["#[rustfmt::skip]", f"pub(super) const {constant}: &[(&str, {rust_type})] = &["])
    rows.extend(f"    ({spelling(key)}, {value:#x})," for _, key, value in sorted(entries))
    rows.extend(["];", ""])

locations = []
for key, values in re.findall(rf"({dart_string}): <LogicalKeyboardKey\?>\[([^\]]+)\],", table("kWebLocationMap")):
    choices = re.findall(r"LogicalKeyboardKey\.(\w+)|(null)", values)
    assert len(choices) == 4, (key, choices)
    slots = [
        f"Some({identifiers['LogicalKeyboardKey'][name]:#x})" if name else "None"
        for name, _ in choices
    ]
    locations.append((ast.literal_eval(key), spelling(key), ", ".join(slots)))
assert len(locations) == table("kWebLocationMap").count("<LogicalKeyboardKey?>[")
rows.extend(["#[rustfmt::skip]", "pub(super) const LOCATION: &[(&str, [Option<u64>; 4])] = &["])
rows.extend(f"    ({key}, [{values}])," for _, key, values in sorted(locations))
rows.extend(["];", ""])
output.write_text("\n".join(rows), encoding="utf-8")
