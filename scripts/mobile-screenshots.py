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
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FEATURES = "mobile,controls,material"


def examples(selected: list[str] | None) -> tuple[list[str], list[str]]:
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT
        )
    )
    package = next(p for p in metadata["packages"] if p["name"] == "incular")
    enabled = set(FEATURES.split(","))
    runnable, skipped = [], []
    for target in package["targets"]:
        if "example" not in target["kind"]:
            continue
        name = target["name"]
        if selected and name not in selected:
            continue
        required = set(target.get("required-features") or [])
        (runnable if required <= enabled else skipped).append(name)
    return sorted(runnable), sorted(skipped)


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

    results: list[tuple[str, str, float]] = []
    for name in names:
        command = [str(cli), "build" if args.build_only else "run", args.platform]
        command += ["--example", name, "--features", FEATURES]
        if args.release:
            command.append("--release")
        if args.platform == "android":
            command += ["--abi", args.abi]
        if not args.build_only:
            command += ["--screenshot", str(out / f"{name}.png"), "--wait", args.wait]
        print(f"::group::{name}", flush=True)
        started = time.monotonic()
        status = subprocess.call(command, cwd=ROOT)
        elapsed = time.monotonic() - started
        print("::endgroup::", flush=True)
        outcome = "ok" if status == 0 else f"failed ({status})"
        if status != 0:
            print(f"::error title={name}::{args.platform} {name} {outcome}", flush=True)
        results.append((name, outcome, elapsed))

    failures = [name for name, outcome, _ in results if outcome != "ok"]
    lines = [
        f"### {args.platform}: {len(results) - len(failures)}/{len(results)} examples ok",
        "",
        "| Example | Result | Seconds |",
        "| --- | --- | ---: |",
        *(f"| `{name}` | {outcome} | {elapsed:.0f} |" for name, outcome, elapsed in results),
    ]
    if skipped:
        lines += ["", f"Skipped (need features beyond `{FEATURES}`): {', '.join(skipped)}"]
    summary = "\n".join(lines) + "\n"
    print(summary)
    (out / "SUMMARY.md").write_text(summary, encoding="utf-8")
    if step_summary := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(step_summary, "a", encoding="utf-8") as file:
            file.write(summary)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
