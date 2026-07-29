import json
with open('research-v7-round-44.json','r',encoding='utf-8') as f:
    r44 = json.load(f)
print(f"Round-44 entries ({len(r44)}):")
for i, q in enumerate(r44):
    if isinstance(q, dict):
        print(f"{i+1}. keys={list(q.keys())}")
        for k, v in q.items():
            sv = str(v)[:140]
            print(f"   {k}: {sv}")
    else:
        print(f"{i+1}. type={type(q).__name__} val={str(q)[:140]}")
    print()