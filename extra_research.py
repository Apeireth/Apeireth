#!/usr/bin/env python3
"""Background AnySearch 跨域真生产调研 - 主人 21:00 + 21:22 并行."""
import sys
import json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

QUERIES = [
    'second-order cybernetics recursive self-observation AI',
    'klein bottle AI self-reference topology',
    'bateson ecology of mind AI agent',
    'ross ashby requisite variety cybernetics AI',
    'friston free energy principle active inference agent',
    'maturana autopoiesis living systems AI agent',
    'von bertalanffy general systems theory AI',
    'meyer physicist self-organization complexity',
]

results = []
for q in QUERIES:
    try:
        r = dual_research(q, top_k=3)
        results.append(r)
        print(f'OK: {q[:50]}')
    except Exception as e:
        print(f'ERR: {q[:50]} -- {e}')
        results.append({'query': q, 'error': str(e)})

with open('research-extra-cross-domain.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'\nsaved {len(results)} extra cross-domain queries')
