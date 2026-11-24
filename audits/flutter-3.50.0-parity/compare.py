"""Produce lexical leads, never parity verdicts. Run from the Incular root."""
import collections
import csv
import json
import pathlib
import re
import subprocess
import sys

out = pathlib.Path(sys.argv[1])
def read_rows(p):
    return [json.loads(line) for line in p.read_text(encoding="utf-8").splitlines() if line.strip()]

def tracked(*patterns):
    b = subprocess.check_output(["git", "ls-files", "-z", "--", *patterns])
    return sorted(filter(None, b.decode("utf-8").split("\0")))

def snake(s):
    s = re.sub(r"([A-Z]+)([A-Z][a-z])", r"\1_\2", s)
    return re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s).lower()

def decl_key(row):
    f = row["file"].removeprefix("src/")
    return f, row["name"]

root_only = []
rows = read_rows(out / "flutter-public-api.jsonl")
for row in rows:
    if "owner" not in row:
        root_only.append(row)

public = re.compile(r"\bpub\s+(?:(?:unsafe|async|const|default|extern)\s+)*(?:struct|enum|trait|union|type|mod|const|static|fn)\s+(?:r#)?([A-Za-z_]\w*)")
alias = re.compile(r"\bas\s+([A-Za-z_]\w*)\b")
index = collections.defaultdict(list)
for f in tracked("crates", "tools"):
    if not f.endswith(".rs") or "/src/" not in f or re.search(r"incular-(material|cupertino)/", f):
        continue
    for n, line in enumerate(pathlib.Path(f).read_text(encoding="utf-8").splitlines(), 1):
        m = public.search(line) if not line.lstrip().startswith(("//", "*")) else None
        if m:
            index[m[1]].append(f"{f}:{n}")
        if line.lstrip().startswith("pub use "):
            for name in alias.findall(line):
                index[name].append(f"{f}:{n}")

history = collections.defaultdict(set)
old_coverage = pathlib.Path("audits/flutter-parity/coverage")
for file in old_coverage.glob("*.jsonl"):
    for r in read_rows(file):
        history[(r.get("file"),r.get("name"))].add((r.get("status") or "?",r.get("counterpart") or ""))

old = read_rows(old_coverage.parent / "declarations.jsonl")
old_names = {(r["file"],r["name"]) for r in old if "." not in r["name"]}
old_depts = {r["file"].split("/")[0] for r in old}
stats = {name: collections.Counter() for name in json.loads((out / "inventory-notes.json").read_text(encoding="utf-8"))["entrypoints"]}
for row in rows:
    for e in row["exportedBy"]:
        stats[e]["members" if "owner" in row else "top_level"] += 1

with (out / "top-level-review.tsv").open("w", encoding="utf-8", newline="") as f:
    writer = csv.writer(f, delimiter="\t", lineterminator="\n")
    writer.writerow(["flutter_file","line","name","kind","exported_by","historical_inventory","candidate_status","rust_candidates","historical_coverage_3_41_7"])
    for row in root_only:
        loc, name = decl_key(row)
        hits = list(dict.fromkeys(index[name] + index[snake(name)]))
        prior = sorted(history[(loc,name)])
        mark = "lexical-candidate" if hits else "historical-lead-only" if prior else "no-lexical-lead"
        old_status = ("seen" if (loc,name) in old_names else "not-seen") if loc.split("/")[0] in old_depts else "not-scanned"
        for e in row["exportedBy"]:
            stats[e][mark] += 1
        writer.writerow([row["file"],row["line"],name,row["kind"],",".join(row["exportedBy"]),old_status,mark,
                         ("; ".join(hits[:12]) + (f"; +{len(hits)-12} more" if len(hits)>12 else "")) or "-",
                         "; ".join(f"{s}: {p}" for s,p in prior) or "-"])

(out / "review-counts.json").write_text(json.dumps(stats,indent=2)+"\n",encoding="utf-8")
print(json.dumps(stats,indent=2))
