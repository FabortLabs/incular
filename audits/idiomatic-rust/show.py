import json,glob,sys
F={f['id']:f for p in glob.glob('audits/idiomatic-rust/findings/*.json') for f in json.load(open(p,encoding='utf-8'))}
n=int(sys.argv[1]); 
for i in sys.argv[2:]:
    f=F[i]; L=open(f['file'],encoding='utf-8').read().split('\n'); l=f['line']
    print(f"== {i} {f['file']}:{l}")
    for j in range(max(0,l-1-n),min(len(L),l+n)): print(f"{j+1:5} {L[j][:140]}")
