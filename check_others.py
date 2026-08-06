import json
for r in [41, 40, 38, 36, 33, 30, 25]:
    print(f'\n=== Round {r} ===')
    try:
        with open(f'research-v7-round-{r}.json', 'r', encoding='utf-8') as f:
            data = json.load(f)
        for i, q in enumerate(data):
            print(f'Q{i+1}: {q["query"][:140]}')
    except Exception as e:
        print(f'err: {e}')
