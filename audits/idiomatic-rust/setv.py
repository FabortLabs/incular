import json,sys
p='audits/idiomatic-rust/verdicts.json'; V=json.load(open(p,encoding='utf-8'))
# args: id=verdict[:note] ...
for a in sys.argv[1:]:
    i,rest=a.split('=',1); v,_,note=rest.partition(':'); V[i]={'v':v,'note':note}
json.dump(V,open(p,'w',encoding='utf-8'),indent=0,ensure_ascii=False); print(len(V))
