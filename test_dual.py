#!/usr/bin/env python3
"""Test dual research."""
import sys
sys.path.insert(0, '.')
from deep_research_dual import dual_research

print('--- testing dual_research ---')
r = dual_research('ecosystem engineering ASI base', top_k=3)
print(f'bocha_web: {len(r["bocha_web"])} hits')
print(f'anysearch: {len(r["anysearch"])} hits')
print(f'AI answer len: {len(r["bocha_ai_answer"])}')
print(f'AI answer preview: {r["bocha_ai_answer"][:300]}')
print(f'merged_sources: {len(r["merged_sources"])}')
for s in r["merged_sources"][:5]:
    print(f'  - {s[:100]}')
