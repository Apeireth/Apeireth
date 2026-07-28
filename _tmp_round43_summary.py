import json
d = json.load(open(r'.openclaw\workspace\promethean\research-v7-round-43.json', encoding='utf-8'))
print(f'Total queries: {len(d)}')
print(f'Total merged sources: {sum(len(q["merged_sources"]) for q in d)}')
for i, q in enumerate(d, 1):
    print(f'Q{i}: {q["query"][:65]}')
    print(f'   top: {q["merged_sources"][0]["name"][:90]}')
    print(f'   url: {q["merged_sources"][0]["url"]}')