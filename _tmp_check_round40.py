import json
with open('research-v7-round-40.json', 'r', encoding='utf-8') as f:
    d = json.load(f)
for i, item in enumerate(d):
    print(f'Q{i+1}: {item["query"]}')