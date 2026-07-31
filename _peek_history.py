import json
import os

# Quick scan of round 40-50 queries for dedup
rounds = [40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50]
all_queries = []
for r in rounds:
    path = f'research-v7-round-{r}.json'
    if not os.path.exists(path):
        continue
    with open(path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    if isinstance(data, list):
        for q in data:
            all_queries.append((r, q.get('query', '')[:100]))

print(f'{len(all_queries)} total queries across r40-r50')
print()
print('All keywords (sorted, deduplicated):')
keywords = set()
for r, q in all_queries:
    for w in q.split():
        w = w.strip('.,:;()[]').lower()
        if len(w) > 4:
            keywords.add(w)
print(f'  {len(keywords)} unique words > 4 chars')
