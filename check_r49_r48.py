import json
for r in [49, 48, 47, 46]:
    print(f'\n=== Round {r} ===')
    with open(f'research-v7-round-{r}.json', 'r', encoding='utf-8') as f:
        data = json.load(f)
    for i, q in enumerate(data):
        print(f'Q{i+1}: {q["query"][:180]}')
    print(f'Total: {len(data)}')
