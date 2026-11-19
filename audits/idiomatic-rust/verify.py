"""Mechanical verification of Haiku findings: evidence must exist near the cited line."""
import json, sys, os, glob, re
def norm(s): return re.sub(r'\s+','',s)
out=[]
paths=sorted(glob.glob('audits/idiomatic-rust/findings/*.json'))
if len(sys.argv)>1: paths=[f'audits/idiomatic-rust/findings/{u}.json' for u in sys.argv[1:]]
tot=bad=0
for p in paths:
    try: fs=json.load(open(p,encoding='utf-8'))
    except Exception as e: print('INVALID JSON',p,e); continue
    for i,f in enumerate(fs):
        tot+=1
        f.setdefault('id',f"{os.path.basename(p)[:-5]}-{i+1:02d}")
        file=f.get('file','')
        if not os.path.exists(file): f['mech']='nofile'; bad+=1; continue
        lines=open(file,encoding='utf-8',errors='ignore').read().split('\n')
        ln=int(f.get('line') or 0)
        ev=[norm(x) for x in str(f.get('evidence','')).split('\n') if norm(x) and not norm(x).startswith('//')]
        ev=[e for e in ev if len(e)>3 and e not in('...','…')]
        if not ev: f['mech']='noevidence'; bad+=1; continue
        first=ev[0][:60]
        win=range(max(0,ln-8),min(len(lines),ln+8))
        if any(first in norm(lines[j]) for j in win): f['mech']='ok'
        elif any(first in norm(l) for l in lines): f['mech']='lineoff'
        else: f['mech']='notfound'; bad+=1
    json.dump(fs,open(p,'w',encoding='utf-8'),indent=1,ensure_ascii=False)
    c={}
    for f in fs: c[f['mech']]=c.get(f['mech'],0)+1
    print(os.path.basename(p),len(fs),c)
print('total',tot,'unverifiable',bad)
