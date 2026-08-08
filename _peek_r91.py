import json, sys
sys.stdout.reconfigure(encoding='utf-8')
with open('research-v7-round-91.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
print('round:', data['round'])
print('queries_count:', data['queries_count'])
print('ok_count:', data['ok_count'])
print('total_sec:', data['total_sec'])
print('ts_iso:', data['ts_iso'])
print()
print('Sample Q1:')
q1 = data['queries'][0]
print('  domain:', q1['domain'])
print('  ok:', q1['ok'])
res = q1.get('result', {})
print('  result keys:', list(res.keys())[:10])
items = res.get('items') or res.get('results') or res.get('data') or []
if isinstance(items, list):
    print(f'  item count: {len(items)}')
    if items:
        first = items[0]
        if isinstance(first, dict):
            title = first.get('title') or first.get('name') or str(first)[:100]
            url = first.get('url') or first.get('link') or '?'
            print(f'  first title: {title}')
            print(f'  first url: {url}')
print()
print('All domains:')
for q in data['queries']:
    print(f'  {q["id"]:8s} {q["domain"]:35s} {q["elapsed_sec"]}s ok={q["ok"]}')