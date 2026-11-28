#!/usr/bin/env python3
"""Translate pinned Flutter numeric platform key tables."""

from pathlib import Path
import re
import subprocess
import sys

REVISION = "ab2912035e160d9cad6306df307576da6128ebdb"
platform = sys.argv[1]
assert platform in ("Android", "Linux", "Windows"), platform
root, output = map(Path, sys.argv[2:])


def pinned(name):
    path = f"packages/flutter/lib/src/services/{name}"
    return subprocess.check_output(
        ["git", "-C", str(root), "show", f"{REVISION}:{path}"], text=True
    )


maps = pinned("keyboard_maps.g.dart")
keys = pinned("keyboard_key.g.dart")
script = (
    "extract-numeric-raw-key-catalog.py Linux"
    if platform == "Linux"
    else f"extract-{platform.lower()}-raw-key-catalog.py"
)
rows = [
    "// Copyright 2014 The Flutter Authors. All rights reserved.",
    "// Use of this source code is governed by the BSD-style license in raw_key_fuchsia.LICENSE.",
    f"// Generated from Flutter 3.50.0 by {script}. Do not hand edit.",
    "",
]
tables = (
    (f"k{platform}ToPhysicalKey", "PhysicalKeyboardKey", "PHYSICAL", "u32"),
    (f"k{platform}ToLogicalKey", "LogicalKeyboardKey", "LOGICAL", "(u64, &str)"),
    (f"k{platform}NumPadMap", "LogicalKeyboardKey", "NUM_PAD", "u64"),
)
if platform == "Linux":
    tables = (tables[0],) + tuple(
        (name, "LogicalKeyboardKey", f"{toolkit.upper()}_{constant}", rust_type)
        for toolkit in ("Gtk", "Glfw")
        for name, constant, rust_type in (
            (f"k{toolkit}ToLogicalKey", "LOGICAL", "(u64, &str)"),
            (f"k{toolkit}NumpadMap", "NUM_PAD", "u64"),
        )
    )
for name, kind, constant, rust_type in tables:
    ids = {
        identifier: int(value, 16)
        for identifier, value in re.findall(
            rf"static const {kind} (\w+) = {kind}\((0x[0-9a-f]+)\);", keys
        )
    }
    start = maps.index("{\n", maps.index(f" {name} = ")) + 2
    body = maps[start : maps.index("\n};", start)]
    entries = [
        (int(code, 0), ids[identifier], identifier)
        for code, identifier in re.findall(rf"(0x[0-9a-f]+|\d+): {kind}\.(\w+),", body)
    ]
    assert entries and len(entries) == body.count(f"{kind}.")
    rows.extend(["#[rustfmt::skip]", f"pub(super) const {constant}: &[(u32, {rust_type})] = &["])
    for code, value, identifier in sorted(entries):
        right = f"{value:#x}"
        if rust_type == "(u64, &str)":
            right = f'({right}, "{identifier[0].upper()}{identifier[1:]}")'
        rows.append(f"    ({code:#x}, {right}),")
    rows.extend(["];", ""])

output.write_text("\n".join(rows), encoding="utf-8")
