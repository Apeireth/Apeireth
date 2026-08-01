import json
d=json.load(open('research-v7-round-59.json',encoding='utf-8'))
for i,q in enumerate(d):
    print(f"{i+1}. {q['query'][:200]}")