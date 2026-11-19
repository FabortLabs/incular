import json, collections, os
files=json.load(open('audits/idiomatic-rust/files.json'))
old=json.load(open('audits/idiomatic-rust/units.json'))
PILOT={'U025','U026','U017','U084','U088','U102','U123','U106'}
done=set(p for u in old if u['id'] in PILOT for p in u['files'])
CAP=1100
HOT={'incular-widgets','incular-rendering','incular-wgpu','incular-text','incular-scroll','incular-gestures','incular-layout','incular-runtime','incular-animation','incular-core'}
def crate_of(p):
    s=p.split('/'); return 'examples' if s[0]=='examples' else s[1]
items=collections.defaultdict(list)  # crate -> list of (path, start,end, n)
for p,n in sorted(files):
    if p in done: continue
    c=crate_of(p)
    if n>CAP:
        k=-(-n//CAP); step=-(-n//k)
        for s in range(0,n,step): items[c].append((f"{p}:{s+1}-{min(n,s+step)}",min(n,s+step)-s))
    else: items[c].append((p,n))
units=[{'id':u['id'],'crate':u['crate'],'files':u['files'],'lines':u['lines'],'hot':u['hot'],'status':'pilot'} for u in old if u['id'] in PILOT]
seq=1
def add(c,fs,n):
    global seq
    units.append({'id':f"A{seq:03d}",'crate':c,'files':fs,'lines':n,'hot':any(x in HOT for x in c.split('+'))}); seq+=1
small=[]
for c,its in items.items():
    cur=[];n=0
    for p,k in its:
        if n+k>CAP and cur: 
            if n<500 and c not in HOT: small.append((c,cur,n))
            else: add(c,cur,n)
            cur=[];n=0
        cur.append(p);n+=k
    if cur:
        if n<500 and c not in HOT: small.append((c,cur,n))
        else: add(c,cur,n)
cc=[];cf=[];cn=0
for c,fs,n in small:
    if cn+n>CAP and cf: add('+'.join(sorted(set(cc))),cf,cn); cc=[];cf=[];cn=0
    cc.append(c);cf+=fs;cn+=n
if cf: add('+'.join(sorted(set(cc))),cf,cn)
json.dump(units,open('audits/idiomatic-rust/units.json','w'),indent=1)
with open('audits/idiomatic-rust/PLAN.md','w') as f:
    new=[u for u in units if u['id'].startswith('A')]
    f.write(f"# Idiomatic Rust audit plan\n\n8 pilot units (U*) at <=1,800 lines, then {len(new)} units (A*) at <={CAP} lines; files over the cap are split into line ranges (`path:start-end`). Cap chosen to keep each Haiku 5.5 agent under the 100K-token prompt pricing tier.\n\nTotal {sum(u['lines'] for u in units)} source lines.\n\n| Unit | Crate | Lines | Hot | Files |\n|---|---|---|---|---|\n")
    for u in units: f.write(f"| {u['id']} | {u['crate']} | {u['lines']} | {'yes' if u['hot'] else 'no'} | {' '.join(u['files'])} |\n")
print(len(units), sum(u['lines'] for u in units))
