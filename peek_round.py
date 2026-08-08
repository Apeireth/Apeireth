import json
with open('research-v7-round-87.json','r',encoding='utf-8') as f:
    d = json.load(f)
# d is a list
print('type:', type(d).__name__, 'len:', len(d))
if isinstance(d, list):
    for i, q in enumerate(d):
        s = q.get('query', str(q)) if isinstance(q, dict) else str(q)
        print('  Q' + str(i+1) + ': ' + s[:160])