import json, re
from pathlib import Path

base = Path(r'.openclaw\workspace\promethean')
all_keywords = {}
for jf in sorted(base.glob('research-v7-round-*.json')):
    with open(jf, 'r', encoding='utf-8') as f:
        data = json.load(f)
    rn = jf.stem.replace('research-v7-round-', '')
    queries = [q['query'] for q in data]
    text = ' '.join(queries).lower()
    # 抽取可能的姓名/概念
    tokens = re.findall(r'\b[A-Z][a-z]+(?:\s+[A-Z][a-z]+){0,3}\b', ' '.join(queries))
    for t in tokens:
        all_keywords.setdefault(t, []).append(rn)

# 打印出现过的 tokens
seen = sorted(all_keywords.items(), key=lambda x: -len(set(x[1])))
for t, rounds in seen[:80]:
    rounds_uniq = sorted(set(rounds), key=lambda r: int(r) if r.isdigit() else 0)
    print(f'  {t}: {len(rounds_uniq)} rounds (e.g. {rounds_uniq[-3:]})')