#!/usr/bin/env python3
# -*- coding: utf-8 -*-
import json
lines = open(r'.openclaw\workspace\promethean\cron-research-runs.jsonl', encoding='utf-8').readlines()
print(f'Total log lines: {len(lines)}')
for ln in lines[-15:]:
    try:
        d = json.loads(ln)
        print(f'  {d.get("ts","?")} round={d.get("round","?")} action={d.get("action","?")}')
    except Exception as e:
        pass