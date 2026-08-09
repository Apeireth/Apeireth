import json
d = json.load(open(r'.openclaw\workspace\promethean\research-v7-round-100.json', encoding='utf-8'))
print(f"round={d['round']} ts={d['ts_iso']} q={d['queries_count']} ok={d['ok_count']} sec={d['total_sec']}")
for i, q in enumerate(d['queries']):
    print(f"{i+1:2d}. [{q.get('domain','?')[:35]:35s}] [{q.get('gap','?')[:10]:10s}] {q['query'][:120]}")