import json
d = json.load(open('research-v7-round-49.json', encoding='utf-8'))
for i, q in enumerate(d, 1):
    qstr = q['query']
    sources = q.get('merged_sources', [])
    print(f'Q{i}: {qstr[:70]} | sources={len(sources)}')
    for s in sources[:2]:
        title = (s.get('title') or '?')[:80]
        url = (s.get('url') or '?')[:80]
        print(f'  - {title} ({url})')