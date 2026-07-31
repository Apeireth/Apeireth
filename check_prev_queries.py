import json
with open('research-v7-round-51.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
for i, q in enumerate(data):
    print(f'Q{i+1}: {q["query"][:200]}')
print('---')
print(f'Total queries: {len(data)}')
