import json
d = json.load(open(r'.openclaw\workspace\promethean\research-v7-round-90.json', encoding='utf-8'))
print(f"round={d['round']} ts={d['ts_iso']} ok={d['ok_count']}/{d['queries_count']} sec={d['total_sec']}")
