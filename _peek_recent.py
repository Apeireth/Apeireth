import json
import os
import sys
sys.stdout.reconfigure(encoding='utf-8')

# Show r40-r50 queries for context
for r in [40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50]:
    path = f'research-v7-round-{r}.json'
    if not os.path.exists(path):
        continue
    with open(path, 'r', encoding='utf-8') as f:
        data = json.load(f)
    if not isinstance(data, list):
        continue
    print(f'=== r{r} ({len(data)} querie) ===')
    for i, q in enumerate(data):
        print(f'  [{i}] {q["query"][:170]}')
    print()
