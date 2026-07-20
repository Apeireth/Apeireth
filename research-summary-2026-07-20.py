#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Display AI deep search results."""
import sys
import json
from pathlib import Path

# Force UTF-8 output
if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8')

p = Path(r".openclaw\workspace\promethean\research-ai-deep-search-2026-07-20.json")
data = json.loads(p.read_text(encoding="utf-8-sig"))
print(f"Total: {len(data)} queries")
for i, r in enumerate(data):
    q = r.get("query", "")
    if r.get("answer"):
        print(f"\n=== [{i+1}] {q[:100]} ===")
        ans = r["answer"][:2000]
        print(ans)
        print(f"   ... [{len(r['answer'])} chars total]")
    elif r.get("raw"):
        print(f"\n=== [{i+1}] {q[:100]} === RAW (no assistant msg)")
        print(r["raw"][:500])
    elif r.get("err"):
        print(f"\n=== [{i+1}] {q[:100]} === ERR: {r['err']}")
    else:
        print(f"\n=== [{i+1}] {q[:100]} === (no data)")
