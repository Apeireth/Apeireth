import json
with open('research-v7-round-80.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
# data may be a list directly
if isinstance(data, list):
    queries = data
    print('Type: list, items count:', len(queries))
else:
    print('Keys:', list(data.keys()))
    queries = data.get('queries', data.get('results', data.get('items', [])))
    if not queries:
        print('No queries key, dumping top-level structure (first 1000 chars)')
        print(json.dumps(data, indent=2, ensure_ascii=False)[:1000])
        raise SystemExit

print()
print('Round-80 queries/items count:', len(queries))
for i, q in enumerate(queries[:15]):
    if isinstance(q, dict):
        text = q.get('q', q.get('query', q.get('text', q.get('title', str(q)[:200]))))
    else:
        text = str(q)[:200]
    print(f'  {i+1}. {text}')
