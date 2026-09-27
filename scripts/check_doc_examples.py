"""Fail if any docs rust block is not a runnable program or API signature.

Runnable = contains `fn main`. Signatures (`pub fn/struct/enum/...`) are
skipped (harness ignores them). Everything else (fragments referencing
`root,cx,handle,...` or bare expressions) breaks `docscheck` with E0425/E0283,
so this script enforces the Flutter-style copy-paste rule from the docs plan.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

DOCS = Path(__file__).resolve().parent.parent / "docs" / "content" / "docs"
FENCE = re.compile(r"```rust\r?\n(.*?)```", re.DOTALL)
SIG = re.compile(r"^\s*pub (fn|struct|enum|trait|type|const)\b", re.MULTILINE)


def main() -> int:
    bad: list[str] = []
    for path in sorted(DOCS.rglob("*.mdx")):
        text = path.read_text(encoding="utf-8")
        for i, m in enumerate(FENCE.findall(text)):
            body = m.strip()
            if "fn main" in body:
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
    return 0


if __name__ == "__main__":
    sys.exit(main())
