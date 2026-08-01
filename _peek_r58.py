import json, sys
sys.stdout.reconfigure(encoding='utf-8')
d = json.load(open('research-v7-round-58.json', encoding='utf-8'))
print(f'len: {len(d)}')
for i, q in enumerate(d):
    print(f'{i+1}. {q["query"]}')