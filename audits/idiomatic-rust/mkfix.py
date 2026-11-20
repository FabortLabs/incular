"""Generate Phase 4 fix briefs: fixes/<crate>-NN.md, one per batch of files.

Selects confirmed and unverified findings that are not public-API changes
(those ripple across crates and get their own pass) and not already fixed.
Batches stay inside one crate, never split a file, and hold at most CAP
findings. Run from the repo root: python audits/idiomatic-rust/mkfix.py
"""
import collections
import glob
import json
import os

D = 'audits/idiomatic-rust'
CAP = 12
DONE = set(json.load(open(f'{D}/fixed.json'))) if os.path.exists(f'{D}/fixed.json') else set()
V = json.load(open(f'{D}/verdicts.json', encoding='utf-8'))
F = [f for p in sorted(glob.glob(f'{D}/findings/*.json')) for f in json.load(open(p, encoding='utf-8'))]


def verdict(f):
    v = V.get(f['id'], {}).get('v')
    if v:
        return v
    return 'rejected' if f.get('mech') in ('notfound', 'nofile', 'noevidence') else 'unverified'


def crate(f):
    s = f['file'].split('/')
    return 'examples' if s[0] == 'examples' else s[1]


by = collections.defaultdict(lambda: collections.defaultdict(list))
for f in F:
    if f['id'] in DONE or f.get('public_api') or verdict(f) not in ('confirmed', 'unverified'):
        continue
    by[crate(f)][f['file']].append(f)

TEMPLATE = """# Fix brief {bid}

Repo: D:/CODEBASE/PROJECTS/INCULAR/incular (Rust UI framework). You are
applying audit findings from an idiomatic-Rust review. The guide is
`IDIOMATIC-RUST.md` at the repo root; read a section only if a finding needs it.

## Rules
- Edit ONLY these files: {files}. Other agents are editing other files at
  the same time.
- No behavior change. Flutter-parity constants and math stay exactly as they are.
- Do not change any `pub` signature, public type or public field. If a fix
  needs that, skip it.
- Re-read the code around each finding first. The line numbers may have
  drifted a little. Apply the fix only if the finding is still correct and
  the fix is clearly behavior-preserving. Otherwise skip it with a reason.
  Skipping is better than a risky edit.
- Keep fixes minimal and idiomatic. Match the surrounding style. For hot-path
  scratch buffers, prefer a reusable field that is cleared and reused. Keep
  comments short.
- Never add `#[cfg(test)]` code under src/. Do not run cargo; the
  orchestrator builds and tests after the wave. Do not commit.

## Findings
{findings}

## Reply
One line per finding: `<id>: fixed` or `<id>: skipped - <reason>`. Nothing else.
"""


def render(f):
    return (f"### {f['id']} - {f['file']}:{f['line']} ({f['rule']}, {f.get('severity')})\n"
            f"Evidence:\n```\n{f['evidence']}\n```\nSuggested fix: {f['fix']}\n")


os.makedirs(f'{D}/fixes', exist_ok=True)
index = []
for c in sorted(by):
    batches, cur, n = [], [], 0
    for path in sorted(by[c], key=lambda p: -len(by[c][p])):
        items = by[c][path]
        if cur and n + len(items) > CAP:
            batches.append(cur)
            cur, n = [], 0
        cur.append(path)
        n += len(items)
    if cur:
        batches.append(cur)
    for i, paths in enumerate(batches, 1):
        bid = f'{c}-{i:02d}'
        items = [f for p in paths for f in sorted(by[c][p], key=lambda f: f['line'])]
        text = TEMPLATE.format(bid=bid, files=', '.join(f'`{p}`' for p in paths),
                               findings='\n'.join(render(f) for f in items))
        open(f'{D}/fixes/{bid}.md', 'w', encoding='utf-8').write(text)
        index.append({'id': bid, 'crate': c, 'files': paths, 'findings': [f['id'] for f in items]})
json.dump(index, open(f'{D}/fixes/index.json', 'w'), indent=1)
print(len(index), 'batches', sum(len(b['findings']) for b in index), 'findings')
for c in sorted({b['crate'] for b in index}):
    print(c, sum(1 for b in index if b['crate'] == c))
