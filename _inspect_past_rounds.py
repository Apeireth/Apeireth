import json
for r in [60, 61, 62]:
    d = json.load(open(rf'.openclaw\workspace\promethean\research-v7-round-{r}.json', encoding='utf-8'))
    print(f'=== Round {r} queries ===')
    for i, q in enumerate(d):
        qq = q.get('query', '?')
        print(f'  {i+1}. {qq[:130]}')
    print()
