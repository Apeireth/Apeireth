import json, sys, os
sys.stdout.reconfigure(encoding='utf-8')
for r in [54, 55, 56, 57, 58]:
    p = f'research-v7-round-{r}.json'
    if os.path.exists(p):
        d = json.load(open(p, encoding='utf-8'))
        print(f'\n=== Round {r} ===')
        for q in d:
            print(f'- {q["query"][:130]}')