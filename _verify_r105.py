import json, os
size = os.path.getsize('research-v7-round-105.json')
print(f"Size: {size} bytes ({size/1024:.1f} KB)")
with open('research-v7-round-105.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
print(f"Round: {data['round']}")
print(f"TS: {data['ts_iso']}")
print(f"Queries: {data['queries_count']}, OK: {data['ok_count']}, Total sec: {data['total_sec']}")
print()
for q in data['queries']:
    res = q.get('result', {})
    sources = res.get('sources', {}) if isinstance(res, dict) else {}
    bw_ok = sources.get('bocha_web', {}).get('status', 'n/a') if isinstance(sources, dict) else 'n/a'
    ba_ok = sources.get('bocha_ai', {}).get('status', 'n/a') if isinstance(sources, dict) else 'n/a'
    gap = q['gap']
    dom = q['domain']
    elapsed = q['elapsed_sec']
    qid = q['id']
    print(f"{qid} [{gap[:10]}] {elapsed}s bw={bw_ok} ba={ba_ok} dom={dom[:40]}")