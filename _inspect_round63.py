import json
d = json.load(open(r'.openclaw\workspace\promethean\research-v7-round-63.json', encoding='utf-8'))
print('Round 63 queries:')
for i, item in enumerate(d):
    q = item.get('query', '?')
    print(f'  {i+1}. {q}')
print()
print('Total:', len(d))
