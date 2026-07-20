#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import json

data = json.loads(open('research-ai-deep-search-2026-07-20.json', encoding='utf-8-sig').read())
r = data[0]
raw = r['raw']
j = json.loads(raw)
print('data keys:', list(j['data'].keys()))
print('messages count:', len(j['data']['messages']))
for i, m in enumerate(j['data']['messages']):
    t = m.get('type')
    role = m.get('role')
    ct = m.get('content_type')
    c = m.get('content', '')
    print(f'  msg[{i}] type={t!r} role={role!r} content_type={ct!r} content_len={len(c)}')
    if c:
        print(f'    preview: {c[:300]}')
