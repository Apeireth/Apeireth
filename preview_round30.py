#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import json, sys
sys.stdout.reconfigure(encoding='utf-8')
with open('research-v7-round-30.json', 'r', encoding='utf-8') as f:
    d = json.load(f)
for i, item in enumerate(d):
    q = item['query']
    print(f'--- {i+1}/12: {q[:60]} ---')
    for j, s in enumerate(item.get('merged_sources', [])[:5]):
        title = s.get('name', s.get('title', ''))[:80]
        url = s.get('url', '')[:60]
        src = s.get('source', '')
        print(f'  [{j}] [{src}] {title}')
        print(f'        {url}')