#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Probe dual-source results."""
import sys, json
sys.stdout.reconfigure(encoding='utf-8')
data = json.load(open('research-dual-source.json', encoding='utf-8'))
print('=== AnySearch 真生产内容 ===')
for r in data:
    print(f'\n--- Q: {r["query"]} ---')
    for s in r['anysearch'][:3]:
        print(f'  - {s["name"][:80]}')
        print(f'    {s["url"]}')
