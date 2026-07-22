#!/usr/bin/env python3
"""Check round-36 candidate queries against past themes."""
import json, os, re

# Load all past rounds
all_qs = []
for i in range(8, 36):
    p = f'research-v7-round-{i}.json'
    if os.path.exists(p):
        try:
            with open(p, encoding='utf-8') as f:
                data = json.load(f)
            for q in data:
                all_qs.append((i, q.get('query','')))
        except Exception as e:
            print(f'r{i}: err')

print(f'Total past queries: {len(all_qs)}')
print()

# Group by themes (use author name detection)
authors = []
for r, q in all_qs:
    # Heuristic: author is first 1-3 capitalized words
    parts = q.split()
    # Find author (capitalized first 1-2 words)
    auth = ''
    for p in parts[:4]:
        if p[0].isupper() and p.lower() not in {'the','a','an','of','in','for','on','and','or','to','i','ii','iii','iv','how','what','why','when','from','with','source','code','github'}:
            auth += p + ' '
        else:
            break
    authors.append((r, auth.strip(), q))

# Most recent rounds
recent = sorted(authors, key=lambda x: -x[0])
print('=== last 5 rounds themes ===')
for r, a, q in recent[:60]:
    print(f'r{r}: {a} | {q[:80]}')

print()
print('=== author frequency all rounds ===')
from collections import Counter
ac = Counter(a for r, a, q in authors if a)
for a, c in ac.most_common(30):
    print(f'  {c}x  {a}')
