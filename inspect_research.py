"""Inspect research-v7-fourth-round.json structure."""
import json
import sys

with open('research-v7-fourth-round.json', 'r', encoding='utf-8') as f:
    d = json.load(f)

print(f'type: {type(d).__name__}, len: {len(d)}')
for i, item in enumerate(d):
    if isinstance(item, dict):
        print(f'\n--- query {i+1} ---')
        print(f'  query: {str(item.get("query", ""))[:120]}')
        print(f'  has answer: {bool(item.get("answer") or item.get("summary"))}')
        keys = list(item.keys())
        print(f'  keys: {keys[:8]}')
        if 'answer' in item:
            ans = item['answer']
            print(f'  answer len: {len(str(ans))}')
        elif 'summary' in item:
            print(f'  summary len: {len(str(item["summary"]))}')