"""Check formatting one package at a time.

`cargo fmt --all` hands every target of the workspace members to a single
rustfmt command, which outgrows the Windows command line limit. This runs
rustfmt on the same targets, one package per command, as `cargo fmt` would.
"""
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main():
    metadata = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
            cwd=ROOT,
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    members = set(metadata["workspace_members"])
    local = sorted(
        (package for package in metadata["packages"] if package["id"] in members),
        key=lambda package: package["name"],
    )
    failed = []
    for package in local:
        print("Checking formatting: " + package["name"], flush=True)
        sources = sorted({target["src_path"] for target in package["targets"]})
        command = ["rustfmt", "--check", "--edition", package["edition"], *sources]
        if subprocess.run(command, cwd=ROOT).returncode != 0:
            failed.append(package["name"])
    if failed:
        raise SystemExit("Unformatted packages: " + ", ".join(failed))


if __name__ == "__main__":
    main()
