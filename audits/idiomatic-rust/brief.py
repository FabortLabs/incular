import json,sys
u={x['id']:x for x in json.load(open('audits/idiomatic-rust/units.json'))}
for i in sys.argv[1:]:
    x=u[i]
    print(f"=== {i} {x['crate']} {x['lines']} hot={x['hot']}")
    print(' '.join(x['files']))
