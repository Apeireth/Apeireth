import json
# Check round 38, 39 to understand what's been covered
for r in [38, 39, 40]:
    print(f'=== Round {r} ===')
    try:
        with open(f'research-v7-round-{r}.json', 'r', encoding='utf-8') as f:
            d = json.load(f)
        for i, item in enumerate(d):
            print(f'Q{i+1}: {item["query"]}')
    except Exception as e:
        print(f'Error: {e}')
    print()