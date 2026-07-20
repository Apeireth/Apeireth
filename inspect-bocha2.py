#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import json

data = json.loads(open('research-ai-deep-search-2026-07-20.json', encoding='utf-8-sig').read())
r = data[0]
raw = r['raw']
print('top-level keys:', list(json.loads(raw).keys()))
j = json.loads(raw)
# 全 print
print(json.dumps(j, indent=2, ensure_ascii=False)[:3000])
