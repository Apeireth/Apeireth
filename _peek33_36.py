import json,sys
sys.stdout.reconfigure(encoding='utf-8')
for r in [33,34,35,36]:
    d=json.load(open(f'research-v7-round-{r}.json',encoding='utf-8'))
    print(f'=== round {r} ===')
    for i,q in enumerate(d):
        print(f'  {i:2d} - {q["query"][:130]}')
    print()
