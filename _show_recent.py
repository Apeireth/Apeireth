import json, os
WORKDIR = r".openclaw\workspace\promethean"
print("Scanning rounds 80-89 for domain/query rotation awareness...")
for r in [80, 82, 84, 85, 86, 87, 88, 89]:
    path = os.path.join(WORKDIR, f"research-v7-round-{r}.json")
    if not os.path.exists(path):
        continue
    with open(path, encoding='utf-8') as f:
        data = json.load(f)
    queries = data if isinstance(data, list) else data.get('queries', [])
    ok_count = sum(1 for q in queries if q.get('ok', False)) if isinstance(queries, list) else 0
    print(f"\n=== Round {r} (ok={ok_count}/{len(queries) if isinstance(queries,list) else '?'}) ===")
    if isinstance(queries, list):
        for q in queries:
            print(f"  [{q.get('id','?')}] {q.get('domain','?'):35s} gap={q.get('gap','?'):25s} Q={q.get('query','')[:90]}")
