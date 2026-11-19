import json, collections, os
files=json.load(open('audits/idiomatic-rust/files.json'))
CAP=1800
HOT={'incular-widgets','incular-rendering','incular-wgpu','incular-text','incular-scroll','incular-gestures','incular-layout','incular-runtime','incular-animation'}
def crate_of(p):
    s=p.split('/')
    if s[0]=='examples': return 'examples'
    return s[1]
# group key: crate + module dir
groups=collections.defaultdict(list)
for p,n in sorted(files):
    c=crate_of(p)
    d=os.path.dirname(p)
    groups[(c,d)].append((p,n))
units=[]
bycrate=collections.defaultdict(list)
for (c,d),fs in sorted(groups.items()):
    bycrate[c].append((d,fs))
small=[]
for c,dirs in bycrate.items():
    total=sum(n for _,fs in dirs for _,n in fs)
    cur=[];curn=0
    def flush():
        global cur,curn
        if cur: units.append({'crate':c,'files':cur,'lines':curn})
        cur=[];curn=0
    for d,fs in dirs:
        for p,n in fs:
            if curn+n>CAP and cur: flush()
            cur.append(p);curn+=n
    flush()
# merge tiny units (<600 lines) of non-hot small crates together
final=[];pool=[]
for u in units:
    if u['lines']<700 and u['crate'] not in HOT:
        pool.append(u)
    else: final.append(u)
cur={'crate':[],'files':[],'lines':0}
for u in pool:
    if cur['lines']+u['lines']>CAP and cur['files']:
        final.append({'crate':'+'.join(sorted(set(cur['crate']))),'files':cur['files'],'lines':cur['lines']}); cur={'crate':[],'files':[],'lines':0}
    cur['crate'].append(u['crate']);cur['files']+=u['files'];cur['lines']+=u['lines']
if cur['files']: final.append({'crate':'+'.join(sorted(set(cur['crate']))),'files':cur['files'],'lines':cur['lines']})
for i,u in enumerate(final,1):
    u['id']=f"U{i:03d}"
    u['hot']=any(c in HOT for c in u['crate'].split('+'))
json.dump(final,open('audits/idiomatic-rust/units.json','w'),indent=1)
with open('audits/idiomatic-rust/PLAN.md','w') as f:
    f.write('# Idiomatic Rust audit plan\n\n%d units, %d lines, %d files. Cap %d lines/unit (Haiku 5.5 kept under the 100K-token pricing tier).\n\n| Unit | Crate | Lines | Hot | Files |\n|---|---|---|---|---|\n'%(len(final),sum(u['lines'] for u in final),sum(len(u['files']) for u in final),CAP))
    for u in final:
        f.write(f"| {u['id']} | {u['crate']} | {u['lines']} | {'yes' if u['hot'] else 'no'} | {len(u['files'])} |\n")
print(len(final)); print(max(u['lines'] for u in final))
big=[(p,n) for p,n in files if n>1800]; print(sorted(big,key=lambda x:-x[1])[:15])
