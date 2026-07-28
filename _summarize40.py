import json,sys
sys.stdout.reconfigure(encoding='utf-8')
d=json.load(open('research-v7-round-40.json',encoding='utf-8'))
for i,q in enumerate(d):
    print(f'=== q{i}: {q["query"]} ===')
    for j,s in enumerate(q.get('merged_sources',[])[:3]):
        print(f'  [{j+1}] {s.get("name","")[:90]}')
        print(f'      {s.get("url","")[:120]}')
        sn=s.get('snippet','')
        if sn:
            print(f'      snippet: {sn[:140]}')
    print()
