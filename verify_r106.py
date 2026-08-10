import json
r = json.load(open('.openclaw/workspace/promethean/research-v7-round-106.json', encoding='utf-8'))
print('round:', r['round'])
print('ts_iso:', r['ts_iso'])
print('queries_count:', r['queries_count'])
print('ok_count:', r['ok_count'])
print('total_sec:', r['total_sec'])
print()
print('12 queries:')
for q in r['queries']:
    res = q.get('result', {})
    items = []
    if isinstance(res, dict):
        items = res.get('items', res.get('results', []))
    n = len(items) if isinstance(items, list) else 0
    title_chains = []
    if isinstance(items, list):
        for it in items[:3]:
            if isinstance(it, dict):
                src = it.get('source') or it.get('site') or ''
                title = (it.get('title') or it.get('name') or '')[:60]
                if title:
                    title_chains.append('[{}] {}'.format(src[:18], title))
    print('  {} [{}] {} items, {:.1f}s'.format(q['id'], q['gap'][:25], n, q['elapsed_sec']))
    for s in title_chains:
        print('      -', s)
