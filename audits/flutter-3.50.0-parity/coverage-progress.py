"""Validate audit disposition overlays and report measured denominators."""
import argparse
import csv
import json
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent

def jsonl(name):
    with (ROOT / name).open(encoding="utf-8") as file:
        return [json.loads(row) for row in file if row.strip()]

def tsv(name):
    with (ROOT / name).open(encoding="utf-8", newline="") as file:
        return list(csv.DictReader(file, delimiter="\t"))

def evidence_exists(row):
    rel = Path(row["evidence"].split("#", 1)[0])
    assert not rel.is_absolute() and ".." not in rel.parts, rel
    assert (ROOT / rel).is_file(), rel

def percent(n, d):
    return round(100 * n / d, 4)

upstream = jsonl("upstream-scenarios.jsonl")
frame_rows = [r for r in jsonl("existing-harness-scenarios.jsonl") if r["registration"] == "frameScenario"]
ukeys = {(r["file"], r["line"], r["description"]): r["library"] for r in upstream}
fkeys = {(r["file"], r["line"], r["description"]) for r in frame_rows}
known = set()
u = Counter()
library_outcomes = defaultdict(Counter)
for r in tsv("upstream-dispositions.tsv"):
    key = r["file"], int(r["line"]), r["scenario"]
    assert key in ukeys and key not in known, key
    assert r["disposition"] in {
        "paired-match",
        "paired-mismatch",
        "reference-ran-incular-open",
        "reference-skipped",
        "partial-match",
        "partial-mismatch",
    }, r
    evidence_exists(r)
    known.add(key)
    u[r["disposition"]] += 1
    library_outcomes[ukeys[key]][r["disposition"]] += 1

known.clear()
f = Counter()
for r in tsv("harness-dispositions.tsv"):
    key = r["file"], int(r["line"]), r["scenario"]
    assert key in fkeys and key not in known, key
    assert r["status"] in {"match", "mismatch", "fixture-not-equivalent"}, r
    assert int(r["frames"]) > 0, r
    evidence_exists(r)
    known.add(key)
    f[r["status"]] += 1

paired = u["paired-match"] + u["paired-mismatch"]
checked = f["match"] + f["mismatch"]
result = {
    "upstream_registration_sites": len(upstream),
    "upstream_paired": paired,
    "upstream_paired_percent": percent(paired, len(upstream)),
    "upstream_outcomes": dict(u),
    "frame_registration_sites": len(frame_rows),
    "frame_checked": checked,
    "frame_checked_percent": percent(checked, len(frame_rows)),
    "frame_outcomes": dict(f),
    "note": "Registration sites, not dynamic scenario expansions. Custom C-series probes are separate."
}
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument(
    "--by-library", action="store_true", help="separate upstream totals and outcomes by library"
)
if parser.parse_args().by_library:
    registered = Counter(row["library"] for row in upstream)
    result["upstream_by_library"] = {}
    for library, total in sorted(registered.items()):
        outcomes = library_outcomes[library]
        paired = outcomes["paired-match"] + outcomes["paired-mismatch"]
        result["upstream_by_library"][library] = {
            "registration_sites": total,
            "paired": paired,
            "paired_percent": percent(paired, total),
            "unreviewed": total - sum(outcomes.values()),
            "outcomes": dict(outcomes),
        }
print(json.dumps(result, indent=2))
