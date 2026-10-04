"""Check MDX Rust snippets; use --compile to type-check runnable examples."""
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path
from tempfile import TemporaryDirectory

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs" / "content" / "docs"
FENCE = re.compile(r"```rust\r?\n(.*?)```", re.DOTALL)
SIG = re.compile(r"^\s*pub (fn|struct|enum|trait|type|const)\b", re.MULTILINE)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compile", action="store_true", help="Compile every fn main snippet with the default facade features and DevTools")
    args = parser.parse_args()
    bad: list[str] = []
    examples: list[tuple[str, str]] = []
    for path in sorted(DOCS.rglob("*.mdx")):
        text = path.read_text(encoding="utf-8")
        for i, m in enumerate(FENCE.findall(text)):
            body = m.strip()
            if "fn main" in body:
                name = re.sub(r"[^a-zA-Z0-9_]", "_", path.relative_to(DOCS).with_suffix("").as_posix()) + f"_{i + 1}"
                examples.append((name, body))
                continue
            if SIG.search(body):
                continue
            bad.append(f"{path.relative_to(DOCS).as_posix()}#{i}")
    if bad:
        print("Non-runnable docs rust blocks (must contain `fn main`):")
        for item in bad:
            print(f"  - {item}")
        return 1
    print(f"docs examples ok ({len(list(DOCS.rglob('*.mdx')))} files)")
    if args.compile:
        output = ROOT / "target"
        output.mkdir(exist_ok=True)
        with TemporaryDirectory(prefix="docscheck-", dir=output) as directory:
            harness = Path(directory)
            (harness / "examples").mkdir()
            for name, body in examples:
                (harness / "examples" / f"{name}.rs").write_text(
                    "#![deny(warnings)]\n" + body + "\n", encoding="utf-8"
                )
            # Preserve the release's locked dependency versions while Cargo adds
            # this temporary consumer to the lockfile outside the workspace.
            shutil.copyfile(ROOT / "Cargo.lock", harness / "Cargo.lock")
            (harness / "Cargo.toml").write_text(
                '[package]\nname = "incular-docs-examples"\nversion = "0.0.0"\n'
                'edition = "2024"\npublish = false\n\n[workspace]\n\n[dependencies]\n'
                'incular = { path = ' + json.dumps((ROOT / "crates/incular").as_posix())
                + ', features = ["devtools"] }\n', encoding="utf-8"
            )
            print(f"Compiling {len(examples)} runnable snippets (names map to MDX paths)", flush=True)
            result = subprocess.run([
                "cargo", "check", "--manifest-path", str(harness / "Cargo.toml"),
                "--examples", "--keep-going", "--offline", "--target-dir", str(output),
            ], cwd=ROOT)
            if result.returncode:
                return result.returncode
            print(f"Compiled all {len(examples)} runnable snippets")
    return 0


if __name__ == "__main__":
    sys.exit(main())
