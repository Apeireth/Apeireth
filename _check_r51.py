import json
import sys
sys.stdout.reconfigure(encoding='utf-8')

with open('research-v7-round-51.json', 'r', encoding='utf-8') as f:
    data = json.load(f)
print(f'Type: {type(data).__name__}, length: {len(data)}')
print()
print('AnySearch sources per query:')
for i, q in enumerate(data):
    items = q.get('anysearch', [])
    print(f'  [{i:02d}] {len(items)} hits')
    for j, src in enumerate(items[:3]):
        name = src.get('name', '')[:80]
        url = src.get('url', '')[:80]
        print(f'        [{j}] {name}')
        print(f'             {url}')
