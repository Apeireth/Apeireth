import json
with open('research-v7-round-104.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
for i, q in enumerate(data['queries']):
    gap = q['gap']
    dom = q['domain']
    qq = q['query']
    print(f"{i+1}. [{gap}] {dom[:60]}")
    print(f"    Q: {qq}")