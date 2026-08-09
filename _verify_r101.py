import json
d = json.load(open(r'.openclaw\workspace\promethean\research-v7-round-101.json', encoding='utf-8'))
print(f"round={d['round']} ts={d['ts_iso']} q={d['queries_count']} ok={d['ok_count']} sec={d['total_sec']}")
for i,q in enumerate(d['queries']):
    src = '?'
    if isinstance(q.get('result'), dict):
        if 'sources' in q['result']:
            src = list(q['result']['sources'].keys())
        elif 'data' in q['result']:
            src = 'data'
        else:
            src = list(q['result'].keys())[:3]
    print(f"  {i+1:2d}. [{q['domain'][:38]:38s}] {q['elapsed_sec']:5.1f}s ok={q['ok']} src={src}")