"""Build REPORT.md from findings/*.json + verdicts.json (id -> {"v": confirmed|rejected|needs-design|systemic:<name>, "note": str})."""
import json, glob, os, collections
D='audits/idiomatic-rust'
V=json.load(open(f'{D}/verdicts.json',encoding='utf-8')) if os.path.exists(f'{D}/verdicts.json') else {}
S=json.load(open(f'{D}/systemic.json',encoding='utf-8')) if os.path.exists(f'{D}/systemic.json') else []
units=json.load(open(f'{D}/units.json'))
done=[u['id'] for u in units if os.path.exists(f"{D}/findings/{u['id']}.json")]
F=[]
for p in sorted(glob.glob(f'{D}/findings/*.json')):
    for f in json.load(open(p,encoding='utf-8')): F.append(f)
def crate(f):
    s=f['file'].split('/'); return 'examples' if s[0]=='examples' else s[1]
def verdict(f):
    v=V.get(f['id'],{}).get('v')
    if v: return v
    if f.get('mech') in ('notfound','nofile','noevidence'): return 'rejected'
    return 'unverified'
sev=['high','medium','low']
tab=collections.defaultdict(collections.Counter)
vc=collections.Counter()
for f in F:
    tab[crate(f)][f.get('severity','low')]+=1; vc[verdict(f).split(':')[0]]+=1
checked=vc['confirmed']+vc['rejected']+vc['needs-design']+vc['systemic']
out=[f"# Idiomatic Rust audit report\n\nUnits audited: {len(done)}/{len(units)} ({sum(u['lines'] for u in units if u['id'] in done)} of {sum(u['lines'] for u in units)} lines). Findings: {len(F)}.\n",
 f"Verdicts: {dict(vc)}. Haiku rejection rate among checked findings: {vc['rejected']}/{checked} = {100*vc['rejected']/max(1,checked):.0f}%.\n",
 open(f'{D}/METHOD.md',encoding='utf-8').read(),
 "\n## Findings by crate\n\n| Crate | high | medium | low |\n|---|---|---|---|\n"]
for c in sorted(tab, key=lambda c:-sum(tab[c].values())): out.append(f"| {c} | {tab[c]['high']} | {tab[c]['medium']} | {tab[c]['low']} |\n")
out.append("\n## Systemic issues\n\n")
for s in S: out.append(f"### {s['name']}\n\n{s['text']}\n\n")
def line(f):
    n=V.get(f['id'],{}).get('note','')
    ev=f['evidence'].replace('\n',' ⏎ ')[:160]
    return f"- `{f['id']}` [{f['file']}:{f['line']}]({f['file']}#L{f['line']}) {f['rule']} {f.get('severity')} {'**public**' if f.get('public_api') else ''} — `{ev}` → {f['fix'][:300]}{(' — *'+n+'*') if n else ''}\n"
for title,keys in [("Confirmed findings",('confirmed',)),("Public API changes",None),("Design follow-ups",('needs-design',)),("Unverified findings",('unverified',)),("Rejected findings",('rejected',))]:
    out.append(f"\n## {title}\n\n")
    if keys is None:
        sel=[f for f in F if f.get('public_api') and verdict(f)=='confirmed']
    else: sel=[f for f in F if verdict(f).split(':')[0] in keys]
    by=collections.defaultdict(list)
    for f in sel: by[crate(f)].append(f)
    for c in sorted(by):
        out.append(f"\n### {c}\n\n")
        for f in sorted(by[c],key=lambda f:sev.index(f.get('severity','low')) if f.get('severity') in sev else 3): out.append(line(f))
open(f'{D}/REPORT.md','w',encoding='utf-8').write(''.join(out))
print(len(F), dict(vc))
