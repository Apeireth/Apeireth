#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import json
d = json.load(open(r'.openclaw\workspace\promethean\research-v7-round-47.json', encoding='utf-8'))
print('Type:', type(d).__name__, 'len:', len(d))
print('Keys of d[0]:', list(d[0].keys()))
print()
print('=== Round-47 queries ===')
for i, q in enumerate(d, 1):
    print(f'  Q{i}: {q.get("query", "")[:140]}')
    src = q.get('merged_sources', [])
    print(f'      sources={len(src)}, ai_answer_len={len(q.get("bocha_ai_answer",""))}')