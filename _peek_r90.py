import json, sys
sys.stdout.reconfigure(encoding='utf-8')
with open('research-v7-round-90.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
print('round:', data.get('round'))
print('queries_count:', data.get('queries_count'))
print('ok_count:', data.get('ok_count'))
print('total_sec:', data.get('total_sec'))
print()
print('queries:')
for i, q in enumerate(data['queries']):
    if isinstance(q, dict):
        qstr = q.get('query', str(q))
        qmode = q.get('mode', '?')
        qok = q.get('ok', '?')
        print(f'  {i+1:2d}. [{qmode}/ok={qok}] {qstr}')
    else:
        print(f'  {i+1:2d}. {q}')