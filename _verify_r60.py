import json
d = json.load(open('research-v7-round-60.json', encoding='utf-8'))
print(f'Round-60 entries: {len(d)}')
print(f'Total sources: {sum(len(q.get("merged_sources",[])) for q in d)}')
print(f'Bocha web: {sum(len(q.get("bocha_web",[])) for q in d)}')
print(f'Bocha ai: {sum(1 for q in d if q.get("bocha_ai_answer"))}')
print(f'AnySearch: {sum(len(q.get("anysearch",[])) for q in d)}')
print('Queries:')
for i,q in enumerate(d, 1):
    print(f'  {i}. {q["query"][:90]}')