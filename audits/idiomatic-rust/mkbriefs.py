import json
R='D:/CODEBASE/PROJECTS/INCULAR/incular'
for u in json.load(open('audits/idiomatic-rust/units.json')):
    if not u['id'].startswith('A'): continue
    i=u['id']; hot=u['hot']
    files='\n'.join('- '+f for f in u['files'])
    hotline=("yes. Apply §11/§23 strictly: any allocation, clone or collect per frame, per element, per glyph or per event in build/layout/paint/input/hit-testing/frame code is a finding." if hot else "no.")
    t=f"""# Audit brief {i}

You are auditing part of a Rust UI framework (Incular, repo {R}) against the repo's style guide. This is a read-only audit: do NOT edit any source file, and do NOT run cargo, builds or shell commands.

1. Read {R}/audits/idiomatic-rust/CHECKLIST.md completely. It is the standard.
2. Unit {i}, crate {u['crate']}, about {u['lines']} lines. Hot path: {hotline}
   Read every item below completely (paths are relative to the repo root). An item written `path:START-END` means only that line range is yours: read it with offset/limit. You may peek a few lines outside the range for context, but report only lines inside it.
{files}
3. For each place the code departs from the checklist, record one finding. Re-read the surrounding code before recording; false positives are worse than misses. Don't report anything the checklist's "Do NOT report" list excludes.
4. Write findings with the Write tool to {R}/audits/idiomatic-rust/findings/{i}.json as a JSON array (write [] if the unit is clean). Each object:
{{"unit":"{i}","file":"crates/.../x.rs","line":123,"rule":"§2","severity":"high|medium|low","category":"api|alloc|iter|error|types|unsafe|concurrency|perf|other","public_api":true,"hot_path":{str(hot).lower()},"evidence":"<offending code, max 3 lines, copied verbatim>","fix":"<concrete replacement code or precise instruction>","behavior_change":false,"confidence":"high|medium|low"}}
`file` is the path without any `:START-END` suffix. `line` is the real line number of the first evidence line.
5. Your final reply must be ONE line only: "{i}: <N> findings written". No other text.
"""
    open(f'audits/idiomatic-rust/briefs/{i}.md','w',encoding='utf-8').write(t)
print('ok')
