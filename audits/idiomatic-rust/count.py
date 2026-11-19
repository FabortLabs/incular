import os, json, collections
root='.'
skip={'target','vendor','node_modules','.tmp','.git','tests','benches'}
files=[]
for d,ds,fs in os.walk(root):
    ds[:]=[x for x in ds if x not in skip and not (d.endswith('harness') and x=='flutter')]
    for f in fs:
        if f.endswith('.rs'):
            p=os.path.join(d,f).replace(os.sep,'/')[2:]
            n=sum(1 for _ in open(p,encoding='utf-8',errors='ignore'))
            files.append((p,n))
json.dump(files,open('audits/idiomatic-rust/files.json','w'))
tot=collections.Counter()
for p,n in files:
    parts=p.split('/')
    key='/'.join(parts[:2]) if parts[0] in('crates','tools','harness','examples') else parts[0]
    tot[key]+=n
for k,v in sorted(tot.items(),key=lambda x:-x[1]): print(v,k)
print(sum(tot.values()), len(files))
