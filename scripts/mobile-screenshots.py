#!/usr/bin/env python3
"""Build every `incular` example for Android or iOS and screenshot it.

Usage:
    python scripts/mobile-screenshots.py android [--build-only] [--abi x86_64]
    python scripts/mobile-screenshots.py ios [--build-only]
    python scripts/mobile-screenshots.py android --examples counter,scroll

Builds the `incular` CLI (tools/incular-cli) once, then runs
`incular build|run <platform> --example <name>` for each example whose
required features fit the mobile feature set. `run` needs a booted emulator
or simulator. Screenshots and logs land in `--out` (default
target/incular/screenshots/<platform>). Every example is attempted; the exit
status is non-zero when any failed, and a Markdown summary is appended to
$GITHUB_STEP_SUMMARY when it is set.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FEATURES = "mobile,controls,material"
# `mobile` enables the facade's `runner` feature.
ENABLED = {*FEATURES.split(","), "runner"}
PNG_SIGNATURE = bytes([0x89]) + b"PNG" + bytes([13, 10, 26, 10])


def examples(selected: list[str] | None) -> tuple[list[str], list[str]]:
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT
        )
    )
    package = next(p for p in metadata["packages"] if p["name"] == "incular")
    runnable, skipped = [], []
    for target in package["targets"]:
        if "example" not in target["kind"]:
            continue
        name = target["name"]
        if selected and name not in selected:
            continue
        required = set(target.get("required-features") or [])
        (runnable if required <= ENABLED else skipped).append(name)
    return sorted(runnable), sorted(skipped)


def png_content_colors(path: Path) -> int:
    """Distinct colors in a PNG below the status bar, sampled on a grid.

    One color means the app drew nothing but its background."""
    data = path.read_bytes()
    if data[:8] != PNG_SIGNATURE:
        return 0
    position, idat = 8, bytearray()
    width = height = channels = 0
    while position < len(data):
        length = int.from_bytes(data[position : position + 4], "big")
        kind = data[position + 4 : position + 8]
        chunk = data[position + 8 : position + 8 + length]
        if kind == b"IHDR":
            width, height = int.from_bytes(chunk[:4], "big"), int.from_bytes(chunk[4:8], "big")
            if chunk[8] != 8 or chunk[12] != 0:
                return -1  # Only 8-bit, non-interlaced images are analyzed.
            channels = {0: 1, 2: 3, 4: 2, 6: 4}.get(chunk[9], 0)
        elif kind == b"IDAT":
            idat += chunk
        position += 12 + length
    if not channels:
        return -1
    raw = zlib.decompress(bytes(idat))
    stride = width * channels
    previous = bytearray(stride)
    colors: set[bytes] = set()
    # Skip the status and navigation bars, which the system draws.
    top, bottom = height // 20, height * 19 // 20
    step = max(1, width // 256)
    for row in range(bottom):
        start = row * (stride + 1)
        kind, line = raw[start], bytearray(raw[start + 1 : start + 1 + stride])
        for i in range(stride):
            left = line[i - channels] if i >= channels else 0
            up = previous[i]
            if kind == 1:
                line[i] = (line[i] + left) & 0xFF
            elif kind == 2:
                line[i] = (line[i] + up) & 0xFF
            elif kind == 3:
                line[i] = (line[i] + ((left + up) >> 1)) & 0xFF
            elif kind == 4:
                corner = previous[i - channels] if i >= channels else 0
                estimate = left + up - corner
                pa, pb, pc = abs(estimate - left), abs(estimate - up), abs(estimate - corner)
                predictor = left if pa <= pb and pa <= pc else up if pb <= pc else corner
                line[i] = (line[i] + predictor) & 0xFF
        if row >= top and row % step == 0:
            for x in range(0, width, step):
                colors.add(bytes(line[x * channels : x * channels + min(channels, 3)]))
        previous = line
    return len(colors)


def first_problem(log: Path) -> str | None:
    if not log.is_file():
        return None
    for line in log.read_text(encoding="utf-8", errors="replace").splitlines():
        lowered = line.lower()
        if any(word in lowered for word in ("panicked", "fatal", "error", "unsatisfiedlink")):
            return line.strip()[:240]
    return None


def escape(message: str) -> str:
    """Escapes a workflow command message."""
    return message.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def build_cli() -> Path:
    subprocess.check_call(["cargo", "build", "-p", "incular-cli", "--locked"], cwd=ROOT)
    suffix = ".exe" if os.name == "nt" else ""
    target_dir = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT
        )
    )["target_directory"]
    return Path(target_dir) / "debug" / f"incular{suffix}"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("platform", choices=["android", "ios"])
    parser.add_argument("--build-only", action="store_true")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--abi", default="x86_64")
    parser.add_argument("--wait", default="10")
    parser.add_argument("--examples", help="comma-separated subset")
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()

    selected = args.examples.split(",") if args.examples else None
    names, skipped = examples(selected)
    out = (args.out or ROOT / "target" / "incular" / "screenshots" / args.platform).resolve()
    out.mkdir(parents=True, exist_ok=True)
    cli = build_cli()

    results: list[tuple[str, str, float, str]] = []
    for name in names:
        command = [str(cli), "build" if args.build_only else "run", args.platform]
        command += ["--example", name, "--features", FEATURES]
        if args.release:
            command.append("--release")
        if args.platform == "android":
            command += ["--abi", args.abi]
        if not args.build_only:
            command += ["--screenshot", str(out / f"{name}.png"), "--wait", args.wait]
        screenshot = out / f"{name}.png"
        log = screenshot.with_suffix(".log")
        started = time.monotonic()
        # A cold emulator sometimes captures before the first frame lands,
        # so a blank screenshot gets one more launch.
        for attempt in range(2):
            print(f"::group::{name}" + (" (retry)" if attempt else ""), flush=True)
            status = subprocess.call(command, cwd=ROOT)
            print("::endgroup::", flush=True)
            outcome = "ok" if status == 0 else f"failed ({status})"
            colors = -1
            if not args.build_only and screenshot.is_file():
                colors = png_content_colors(screenshot)
            if status != 0 or colors != 1:
                break
            outcome = "blank"
        problem = first_problem(log) or ""
        # A panic off the main thread can leave the process, and its last
        # frame, alive.
        if outcome == "ok" and "panicked" in problem:
            outcome = "panicked"
        if colors >= 0:
            outcome += f" ({colors} colors)"
        results.append((name, outcome, time.monotonic() - started, problem))

    failures = [name for name, outcome, _, _ in results if not outcome.startswith("ok")]
    # GitHub shows at most ten annotations of each level per step, so every
    # example goes into one annotation. It is also saved, because the Android
    # emulator action does not forward workflow commands; a later step
    # prints the file.
    report = "\n".join(
        f"{name}: {outcome}" + (f" | {problem}" if problem else "")
        for name, outcome, _, problem in results
    )
    level = "error" if failures else "notice"
    annotation = f"::{level} title={args.platform} results::{escape(report)}"
    print(annotation, flush=True)
    lines = [
        f"### {args.platform}: {len(results) - len(failures)}/{len(results)} examples ok",
        "",
        "| Example | Result | Seconds | First log problem |",
        "| --- | --- | ---: | --- |",
        *(
            f"| `{name}` | {outcome} | {elapsed:.0f} | {problem.replace('|', '/')} |"
            for name, outcome, elapsed, problem in results
        ),
    ]
    if skipped:
        lines += ["", f"Skipped (need features beyond `{FEATURES}`): {', '.join(skipped)}"]
    summary = "\n".join(lines) + "\n"
    print(summary)
    (out / "SUMMARY.md").write_text(summary, encoding="utf-8")
    (out / "annotations.txt").write_text(annotation + "\n", encoding="utf-8")
    if step_summary := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(step_summary, "a", encoding="utf-8") as file:
            file.write(summary)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
