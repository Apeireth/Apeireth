import json
with open(r'.openclaw\workspace\promethean\research-v7-round-54.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
print(f'Total entries: {len(data)}')
print(f'First query: {data[0]["query"][:80]}')
print(f'Last query: {data[-1]["query"][:80]}')
print(f'Sources per query: {[len(d["merged_sources"]) for d in data]}')
print(f'Bocha web total: {sum(len(d["bocha_web"]) for d in data)}')
print(f'AnySearch total: {sum(len(d["anysearch"]) for d in data)}')
print(f'Merged total: {sum(len(d["merged_sources"]) for d in data)}')