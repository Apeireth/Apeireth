import json
d = json.load(open(r'.openclaw\workspace\promethean\research-v7-round-31.json', encoding='utf-8'))
for i, r in enumerate(d):
    q = r['query']
    print(f'{i+1}. {q}')
print('---round-30---')
d2 = json.load(open(r'.openclaw\workspace\promethean\research-v7-round-30.json', encoding='utf-8'))
for i, r in enumerate(d2):
    print(f'{i+1}. {r["query"]}')