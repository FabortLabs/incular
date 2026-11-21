#!/usr/bin/env python3
"""Build every `incular` example for a platform, run it and screenshot it.

Usage:
    python scripts/screenshots.py android [--build-only] [--abi x86_64]
    python scripts/screenshots.py ios [--build-only]
    python scripts/screenshots.py linux|windows|macos [--build-only]
    python scripts/screenshots.py android --examples counter,scroll

Android and iOS go through the `incular` CLI (tools/incular-cli): `incular
build|run <platform> --example <name>`, which needs a booted emulator or
simulator to run. Desktop platforms build the examples with Cargo, run each on
this machine and capture only that example's window: `xdotool` and
ImageMagick's `import` on the X display in `$DISPLAY` (Xvfb in CI) for Linux,
`PrintWindow` from PowerShell on Windows, `screencapture -l` on macOS (which
needs `swiftc` and the Screen Recording permission). Browser screenshots come
from scripts/web-smoke.py.

Only examples whose required features fit the platform's feature set run.
Screenshots and logs land in `--out` (default
target/incular/screenshots/<platform>). Every example is attempted; the exit
status is non-zero when any failed, and a Markdown summary is appended to
$GITHUB_STEP_SUMMARY when it is set.
"""

from __future__ import annotations

import argparse
import functools
import json
import os
import subprocess
import sys
import tempfile
import time
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# Facade features per platform, plus the features they enable, which decide
# which examples can run.
FEATURES = {
    "android": ("mobile,controls,material", {"mobile-host", "runner"}),
    "ios": ("mobile,controls,material", {"mobile-host", "runner"}),
    "linux": ("desktop,controls,material", {"desktop-host", "runner"}),
    "windows": ("desktop,controls,material", {"desktop-host", "runner"}),
    "macos": ("desktop,controls,material", {"desktop-host", "runner"}),
}
DESKTOP = {"linux", "windows", "macos"}
# Desktop captures cover only the example's own window, never the rest of the
# screen: run locally, a whole-screen grab would record whatever else is open.
#
# PowerShell that saves the main window of process $env:APP_PID to $env:PNG.
# PrintWindow with PW_RENDERFULLCONTENT reads the window's own surface, so
# windows covering it do not leak into the image.
WINDOWS_CAPTURE = r"""
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class IncularCapture {
    public struct Rect { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr window, IntPtr dc, uint flags);
}
'@
[IncularCapture]::SetProcessDPIAware() | Out-Null
$window = (Get-Process -Id $env:APP_PID).MainWindowHandle
if ($window -eq [IntPtr]::Zero) { exit 2 }
$rect = New-Object IncularCapture+Rect
[IncularCapture]::GetWindowRect($window, [ref]$rect) | Out-Null
$image = New-Object System.Drawing.Bitmap ($rect.Right - $rect.Left), ($rect.Bottom - $rect.Top)
$graphics = [System.Drawing.Graphics]::FromImage($image)
$dc = $graphics.GetHdc()
$ok = [IncularCapture]::PrintWindow($window, $dc, 2)
$graphics.ReleaseHdc($dc)
if (-not $ok) { exit 3 }
$image.Save($env:PNG, [System.Drawing.Imaging.ImageFormat]::Png)
"""
# Swift that prints the on-screen window number of process argv[1].
MACOS_WINDOW_ID = r"""
import CoreGraphics
import Foundation
let pid = Int32(CommandLine.arguments[1])!
let windows = (CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as! [[String: Any]])
    .filter { ($0[kCGWindowOwnerPID as String] as? Int32) == pid }
// One line per window on stderr, for reports on blank captures.
for window in windows {
    let bounds = window[kCGWindowBounds as String] as? [String: Any] ?? [:]
    let size = "\(bounds["Width"] ?? "?")x\(bounds["Height"] ?? "?")"
    let alpha = window[kCGWindowAlpha as String] ?? "?"
    let layer = window[kCGWindowLayer as String] ?? "?"
    FileHandle.standardError.write(
        "window \(window[kCGWindowNumber as String]!) layer \(layer) \(size) alpha \(alpha)\n"
            .data(using: .utf8)!)
}
guard let window = windows.first(where: { ($0[kCGWindowLayer as String] as? Int) == 0 })
else { exit(1) }
print(window[kCGWindowNumber as String]!)
"""
# Log lines that name a failure. "terminating app" is AppKit's line with an
# uncaught NSException's name and reason.
PROBLEM_WORDS = ("panicked", "fatal", "error", "unsatisfiedlink", "terminating app")
# Prefixes the lines run_desktop appends to an example's log.
RUNNER_NOTE = "screenshots.py:"
# Prefixes the line describing the captured windows, reported for blank
# captures.
WINDOW_NOTE = "screenshots.py windows:"
PNG_SIGNATURE = bytes([0x89]) + b"PNG" + bytes([13, 10, 26, 10])


def examples(platform: str, selected: list[str] | None) -> tuple[list[str], list[str]]:
    features, implied = FEATURES[platform]
    enabled = {*features.split(","), *implied}
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
        (runnable if required <= enabled else skipped).append(name)
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
    """The first error-like log line, else the runner's own note (written
    last, when the example exited or showed no window)."""
    if not log.is_file():
        return None
    lines = log.read_text(encoding="utf-8", errors="replace").splitlines()
    for line in lines:
        lowered = line.lower()
        if any(word in lowered for word in PROBLEM_WORDS):
            return line.strip()[:240]
    notes = [line for line in lines if line.startswith(RUNNER_NOTE)]
    return notes[-1][:240] if notes else None


def escape(message: str) -> str:
    """Escapes a workflow command message."""
    return message.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def target_directory() -> str:
    return json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT
        )
    )["target_directory"]


def build_cli() -> Path:
    subprocess.check_call(["cargo", "build", "-p", "incular-cli", "--locked"], cwd=ROOT)
    suffix = ".exe" if os.name == "nt" else ""
    return Path(target_directory()) / "debug" / f"incular{suffix}"


@functools.cache
def macos_window_finder() -> Path:
    """Compiles MACOS_WINDOW_ID once per run."""
    directory = Path(tempfile.mkdtemp(prefix="incular-screenshots-"))
    source = directory / "window_id.swift"
    source.write_text(MACOS_WINDOW_ID, encoding="utf-8")
    subprocess.check_call(["swiftc", "-O", str(source), "-o", str(directory / "window_id")])
    return directory / "window_id"


def capture_window(platform: str, pid: int, png: Path) -> tuple[bool, str]:
    """Saves the main window of process `pid` to `png`; false when the
    process has no visible window. Also returns a description of the
    process's windows where the platform gives one."""
    if platform == "windows":
        command = ["powershell", "-NoProfile", "-NonInteractive", "-Command", WINDOWS_CAPTURE]
        env = {**os.environ, "APP_PID": str(pid), "PNG": str(png)}
        return subprocess.call(command, env=env) == 0, ""
    if platform == "macos":
        finder = subprocess.run([macos_window_finder(), str(pid)], capture_output=True, text=True)
        window = finder.stdout.strip()
        windows = " / ".join(finder.stderr.strip().splitlines())
        command = ["screencapture", "-x", "-o", "-l", window, str(png)]
        return bool(window) and subprocess.call(command) == 0, windows
    else:
        finder = subprocess.run(
            ["xdotool", "search", "--onlyvisible", "--pid", str(pid)], capture_output=True, text=True
        )
        window = finder.stdout.split()[-1] if finder.stdout.split() else ""
        command = ["import", "-window", window, str(png)]
    return bool(window) and subprocess.call(command) == 0, ""


def run_desktop(
    platform: str, binary: Path, screenshot: Path, log: Path, wait: float
) -> int:
    """Runs a desktop example and captures its window; returns non-zero
    when the example exited early or showed no window."""
    screenshot.unlink(missing_ok=True)
    with log.open("wb") as output:
        process = subprocess.Popen([binary], cwd=ROOT, stdout=output, stderr=subprocess.STDOUT)
        time.sleep(wait)
        running = process.poll() is None
        captured, windows = False, ""
        if running:
            captured, windows = capture_window(platform, process.pid, screenshot)
        if windows:
            output.write(f"\n{WINDOW_NOTE} {windows}\n".encode())
        process.kill()
        process.wait()
        if not running:
            note = f"exited with code {process.returncode} before the screenshot"
        elif not captured:
            note = "showed no window to capture"
        else:
            return 0
        tail = log.read_text(encoding="utf-8", errors="replace").strip().splitlines()[-3:]
        output.write(f"\n{RUNNER_NOTE} {binary.name} {note}; last output: {' / '.join(tail)}\n".encode())
    print(f"{binary.name} {note}; see {log}")
    return 1 if not running else 2


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("platform", choices=sorted(FEATURES))
    parser.add_argument("--build-only", action="store_true")
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--abi", default="x86_64")
    parser.add_argument("--wait", type=float, default=10)
    parser.add_argument("--examples", help="comma-separated subset")
    parser.add_argument("--out", type=Path)
    args = parser.parse_args()

    selected = args.examples.split(",") if args.examples else None
    names, skipped = examples(args.platform, selected)
    features = FEATURES[args.platform][0]
    out = (args.out or ROOT / "target" / "incular" / "screenshots" / args.platform).resolve()
    out.mkdir(parents=True, exist_ok=True)
    if args.platform in DESKTOP:
        # Desktop examples build in one Cargo invocation and run directly.
        profile = "release" if args.release else "debug"
        build = ["cargo", "build", "-p", "incular", "--locked", "--features", features]
        build += ["--release"] if args.release else []
        build += [f"--example={name}" for name in names]
        subprocess.check_call(build, cwd=ROOT)
        if args.build_only:
            return 0
        examples_dir = Path(target_directory()) / profile / "examples"
    else:
        cli = build_cli()

    results: list[tuple[str, str, float, str]] = []
    for name in names:
        screenshot = out / f"{name}.png"
        log = screenshot.with_suffix(".log")
        if args.platform in DESKTOP:
            command = None
        else:
            command = [str(cli), "build" if args.build_only else "run", args.platform]
            command += ["--example", name, "--features", features]
            if args.release:
                command.append("--release")
            if args.platform == "android":
                command += ["--abi", args.abi]
            if not args.build_only:
                command += ["--screenshot", str(screenshot), "--wait", str(args.wait)]
        started = time.monotonic()
        # A cold emulator sometimes captures before the first frame lands,
        # so a blank screenshot gets one more launch.
        for attempt in range(2):
            print(f"::group::{name}" + (" (retry)" if attempt else ""), flush=True)
            if command is None:
                binary = examples_dir / (name + (".exe" if args.platform == "windows" else ""))
                status = run_desktop(args.platform, binary, screenshot, log, args.wait)
            else:
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
        if outcome == "blank" and log.is_file():
            # A blank capture says nothing about the window it came from.
            notes = [
                line for line in log.read_text(encoding="utf-8", errors="replace").splitlines()
                if line.startswith(WINDOW_NOTE)
            ]
            problem = problem or (notes[-1][:240] if notes else "")
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
        lines += ["", f"Skipped (need features beyond `{features}`): {', '.join(skipped)}"]
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
