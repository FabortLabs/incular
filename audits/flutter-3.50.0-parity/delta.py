"""Compare AST inventories by owning library, owner, member kind and name."""
import collections
import json
import pathlib
import sys

def load(file):
    items = collections.defaultdict(list)
    for line in pathlib.Path(file).read_text(encoding="utf-8").splitlines():
        row = json.loads(line)
        key = (row["file"].split("/")[1] if row["file"].startswith("src/") else row["file"],
               row.get("owner", ""), row["kind"], row["name"])
        items[key].append(row)
    return items

def summarize(row):
    return {k: row[k] for k in ("file", "line", "signature", "exportedBy")}

old, new = load(sys.argv[1]), load(sys.argv[2])
counts = collections.Counter()
with pathlib.Path(sys.argv[3]).open("w", encoding="utf-8", newline="\n") as out:
    for key in sorted(old.keys() | new.keys()):
        before, after = old.get(key, []), new.get(key, [])
        if not before:
            change = "added"
        elif not after:
            change = "removed"
        else:
            a = sorted((r["file"], r["signature"], tuple(r["exportedBy"])) for r in before)
            b = sorted((r["file"], r["signature"], tuple(r["exportedBy"])) for r in after)
            if a == b:
                continue
            change = "moved-or-signature-or-export-changed"
        counts[change] += 1
        out.write(json.dumps(dict(zip(("library", "owner", "kind", "name"), key)) | {
            "change": change, "before": list(map(summarize, before)), "after": list(map(summarize, after)),
        }, ensure_ascii=False) + "\n")
print(dict(counts))
