#!/usr/bin/env python3
"""Translate the pinned Flutter Windows scan-code and virtual-key tables."""

from pathlib import Path
import re
import subprocess
import sys

REVISION = "ab2912035e160d9cad6306df307576da6128ebdb"
root, output = map(Path, sys.argv[1:])


def pinned(name):
    path = f"packages/flutter/lib/src/services/{name}"
    return subprocess.check_output(
        ["git", "-C", str(root), "show", f"{REVISION}:{path}"], text=True
    )


maps = pinned("keyboard_maps.g.dart")
keys = pinned("keyboard_key.g.dart")
rows = [
    "// Copyright 2014 The Flutter Authors. All rights reserved.",
    "// Use of this source code is governed by the BSD-style license in raw_key_fuchsia.LICENSE.",
    "// Generated from Flutter 3.50.0 by extract-windows-raw-key-catalog.py. Do not hand edit.",
    "",
]
for name, kind, constant, rust_type in (
    ("kWindowsToPhysicalKey", "PhysicalKeyboardKey", "PHYSICAL", "u32"),
    ("kWindowsToLogicalKey", "LogicalKeyboardKey", "LOGICAL", "(u64, &str)"),
    ("kWindowsNumPadMap", "LogicalKeyboardKey", "NUM_PAD", "u64"),
):
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
        if constant == "LOGICAL":
            right = f'({right}, "{identifier[0].upper()}{identifier[1:]}")'
        rows.append(f"    ({code:#x}, {right}),")
    rows.extend(["];", ""])

output.write_text("\n".join(rows), encoding="utf-8")
