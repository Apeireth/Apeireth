import json, sys
with open(r'.openclaw\workspace\promethean\research-v7-round-89.json', encoding='utf-8') as f:
    data = json.load(f)
print('Round 89 queries:')
for q in data['queries']:
    print(f"  [{q['id']}] domain={q['domain']} gap={q['gap']}")
    print(f"    Q: {q['query'][:200]}")
print()
print(f"Total: {len(data['queries'])}, ok: {data.get('ok_count')}, sec: {data.get('total_sec')}")
