import json
with open('research-v7-round-50.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
print('Round 50 queries:')
for i, q in enumerate(data):
    print(f'  [{i}] {q["query"][:200]}')
print()
print('AnySearch sources per query:')
for i, q in enumerate(data):
    items = q.get('anysearch', [])
    print(f'  [{i}] {len(items)} hits')
