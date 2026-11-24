from collections import Counter
import csv
from functools import lru_cache
import json
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
def rows(file):
    return [json.loads(line) for line in (root / file).read_text(encoding="utf-8").splitlines()]

api = rows("flutter-public-api.jsonl")
delta = rows("flutter-api-delta.jsonl")
notes = json.loads((root / "inventory-notes.json").read_text(encoding="utf-8"))
counts = json.loads((root / "review-counts.json").read_text(encoding="utf-8"))
entrypoints = set(notes["entrypoints"])
assert len(entrypoints) == 11
assert len(api) == notes["rows"] == 15791
assert len({(r["file"], r["line"], r.get("owner"), r["name"], r["kind"]) for r in api}) == len(api)
assert all(r["line"] > 0 and set(r["exportedBy"]) <= entrypoints and r["exportedBy"] for r in api)
assert not any(re.match(r"src/(material|cupertino)/", r["file"]) for r in api)
derived = Counter((entry, "members" if "owner" in r else "top_level") for r in api for entry in r["exportedBy"])
assert all(counts[e][k] == n for (e,k),n in derived.items())
assert {(r.get("owner"), r["name"]) for r in api if r["file"].endswith("/overlay.dart")} >= {
    (None, "OverlayChildLayoutInfo"), ("OverlayChildLayoutInfo", "childSize"),
    ("OverlayChildLayoutInfo", "childPaintTransform"), ("OverlayChildLayoutInfo", "overlaySize"),
}
base = pathlib.Path(".tmp/flutter/packages/flutter/lib")
@lru_cache(maxsize=None)
def source_lines(path):
    return (base / path).read_text(encoding="utf-8").splitlines()
def verify_line(path, line_no, token):
    data = source_lines(path)
    assert token in data[line_no - 1], (path, line_no, token)
for r in api:
    if r["kind"] == "operator":
        continue
    # The unnamed constructor token is the owner name. Operators are handled separately.
    verify_line(r["file"], r["line"], r["name"])
with (root / "top-level-review.tsv").open(encoding="utf-8", newline="") as f:
    review = list(csv.DictReader(f, delimiter="\t"))
assert len(review) == sum("owner" not in r for r in api) == 2110
assert len({(r["library"],r["owner"],r["kind"],r["name"]) for r in delta}) == len(delta)
assert all(r["before"] or r["after"] for r in delta)
document = (root / "README.md").read_text(encoding="utf-8")
assert all((root / p).is_file() for p in re.findall(r"\]\(([^:)]+)\)", document))
findings = (root / "findings.md").read_text(encoding="utf-8")
ids = re.findall(r"^\| ([A-Z]+\d+) \|",findings,re.M)
assert len(ids) == len(set(ids))
print(f"{len(api):,} current rows; {len(review):,} top-level; {len(delta):,} changed API keys {dict(Counter(r['change'] for r in delta))}; {len(ids)} findings")
print("Flutter source-delta paths:", len((root / "flutter-source-delta.tsv").read_text(encoding="utf-8").splitlines()))
