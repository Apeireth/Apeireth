import json, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
with open('research-v7-round-45.json','r',encoding='utf-8') as f:
    r = json.load(f)

print('# Round 45 raw summary (for memory append)')
print()
for i, q in enumerate(r, 1):
    q_str = q.get('query', '?')[:140]
    merged = q.get('merged_sources', [])
    print(f'## Q{i:02d}: {q_str}')
    for j, s in enumerate(merged[:5], 1):
        n = s.get('name','')[:120]
        u = s.get('url','')[:120]
        snip = s.get('snippet','')[:160]
        src = s.get('source','?')
        print(f'  [{j}] ({src}) {n}')
        print(f'      url: {u}')
        if snip:
            print(f'      snip: {snip}')
    print()