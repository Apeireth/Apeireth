import json, sys
sys.stdout.reconfigure(encoding='utf-8')

with open(r'.openclaw\workspace\promethean\research-v7-round-52.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

print(f'Total queries: {len(data)}')
print()
for i, q in enumerate(data):
    print(f'=== Q{i+1}: {q["query"]} ===')
    bw = len(q.get('bocha_web', []))
    ba = len(q.get('bocha_ai_answer', ''))
    any_n = len(q.get('anysearch', []))
    merged = len(q.get('merged_sources', []))
    print(f'  bw={bw} ba={ba} any={any_n} merged={merged}')
    # Top sources
    for j, s in enumerate(q.get('merged_sources', [])[:3]):
        url = s.get('url', '')[:80]
        print(f'  [{j+1}] {s.get("name", "")[:80]} | {url}')
    print()
