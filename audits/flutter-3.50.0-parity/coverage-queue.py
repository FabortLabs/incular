"""Reconcile case queues without inferring behavior from names or paths."""
from collections import Counter, defaultdict
import csv
import json
from pathlib import Path
import subprocess
import sys

BASE = Path(sys.argv[1])
FLUTTER = Path(sys.argv[2])
LIBS = {"animation", "foundation", "gestures", "painting", "physics", "rendering",
        "scheduler", "semantics", "services", "widget_previews", "widgets"}
FW_LIB = "packages/flutter/lib/"
FW_TEST = "packages/flutter/test/"

def tracked(cwd, *paths):
    b = subprocess.check_output(["git", "-C", str(cwd), "ls-files", "-z", "--", *paths])
    return set(filter(None, b.decode("utf-8").split("\0")))

def rows(file):
    return [json.loads(line) for line in (BASE / file).read_text(encoding="utf-8").splitlines()]

def by_file(items, prefix=""):
    counts = Counter()
    for r in items:
        counts[prefix + r["file"]] += 1
    return counts

api = rows("flutter-public-api.jsonl")
cases = rows("upstream-scenarios.jsonl")
harness = rows("existing-harness-scenarios.jsonl")
api_counts = by_file(api, FW_LIB)
case_counts = by_file(cases)
harness_counts = by_file(harness)
source = {p for p in tracked(FLUTTER, FW_LIB) if p.endswith(".dart") and (
    p.removeprefix(FW_LIB).removesuffix(".dart") in LIBS
    or p.removeprefix(FW_LIB).split("/")[:1] == ["src"]
    and p.removeprefix(FW_LIB).split("/")[1] in LIBS
)}
assert all(not p.startswith((FW_LIB+"src/material/", FW_LIB+"src/cupertino/"))
           for p in api_counts)
source |= api_counts.keys()
tests = {p for p in tracked(FLUTTER, FW_TEST, "dev/integration_tests/widget_preview_scaffold/test",
    "packages/flutter_tools/test/commands.shard/hermetic/widget_preview",
    "packages/flutter_tools/test/commands.shard/permeable/widget_preview",
    "packages/flutter_tools/test/general.shard/migrations",
    "packages/flutter_tools/test/integration.shard")
    if p.endswith(".dart") and (
        p.startswith(FW_TEST) and p.removeprefix(FW_TEST).split("/")[0] in LIBS
        or not p.startswith(FW_TEST) and "widget_preview" in p
    )}
assert case_counts.keys() <= tests
harness_files = {p for p in tracked(".", "harness/flutter/test", "harness/incular/tests")
                 if p.endswith((".dart", ".rs"))}
assert harness_counts.keys() <= harness_files

catalog = []
for p in sorted(source):
    if p.startswith(FW_LIB+"src/"):
        label = p.removeprefix(FW_LIB+"src/").split("/")[0]
    else:
        label = p.removeprefix(FW_LIB).removesuffix(".dart")
    catalog.append(("flutter-source", label, p, api_counts[p],
                    "source-contract-review-needed" if api_counts[p] else "lowering-or-support-review-needed"))
for p in sorted(tests):
    label = p.removeprefix(FW_TEST).split("/")[0] if p.startswith(FW_TEST) else "widget_previews"
    catalog.append(("flutter-test", label, p, case_counts[p],
                    "compare-registered-cases" if case_counts[p] else "helper-or-indirect-cases-review-needed"))
for p in sorted(harness_files):
    catalog.append(("existing-harness", "mixed", p, harness_counts[p],
                    "check-3.50.0-fixture-and-twin"))
with (BASE/"coverage-queue.tsv").open("w", encoding="utf-8", newline="") as f:
    out = csv.writer(f, delimiter="\t", lineterminator="\n")
    out.writerow(["kind","library","path","catalog_rows","default_disposition"])
    out.writerows(catalog)
stats = {"source_files":len(source),"public_api_rows":sum(api_counts.values()),
         "upstream_test_files_in_scope":len(tests),"upstream_registration_sites":len(cases),
         "harness_source_files":len(harness_files),"existing_harness_registration_sites":len(harness),
         "runnable_parity_verified_by_catalog":0}
assert len({(k,p) for k,l,p,n,s in catalog})==len(catalog)
assert sum(case_counts.values()) == len(cases)
assert sum(x[3] for x in catalog if x[0] == "flutter-source") == len(api)
(BASE/"coverage-queue-summary.json").write_bytes((json.dumps(stats,indent=2)+"\n").encode("utf-8"))
print(json.dumps(stats,indent=2))
