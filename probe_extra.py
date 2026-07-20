#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Display AnySearch extra cross-domain results."""
import sys, json
sys.stdout.reconfigure(encoding='utf-8')

data = json.load(open('research-extra-cross-domain.json', encoding='utf-8'))
print(f'Total queries: {len(data)}\n')
for i, r in enumerate(data):
    q = r.get('query', '')
    anysearch = r.get('anysearch', [])
    ai_answer = r.get('bocha_ai_answer', '')
    print(f'[{i+1}] Q: {q}')
    print(f'  AnySearch: {len(anysearch)} hits')
    for s in anysearch[:2]:
        print(f'    - {s["name"][:80]}')
        print(f'      {s["url"][:80]}')
    if ai_answer:
        print(f'  AI answer: {len(ai_answer)} chars')
        print(f'    preview: {ai_answer[:300]}')
    print()
