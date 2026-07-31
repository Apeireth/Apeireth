import json
with open(r'.openclaw\workspace\promethean\research-v7-round-53.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
print(f'Round 53 total queries: {len(data)}')
for i, q in enumerate(data, 1):
    print(f'{i}. {q["query"][:140]}')