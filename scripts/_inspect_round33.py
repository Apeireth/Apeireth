import json
d = json.load(open('research-v7-round-33.json', encoding='utf-8'))
print('round-33 queries:')
for i, q in enumerate(d, 1):
    print(f'{i:2d}. {q["query"]}')