import json
for n in [100, 101, 102, 103, 104]:
    try:
        d = json.load(open(f'research-v7-round-{n}.json', encoding='utf-8'))
        print(f'\n=== round-{n} ===')
        for q in d.get('queries', []):
            print(f"  {q['id']}: {q['domain']}")
    except Exception as e:
        print(f'{n}: error {e}')
