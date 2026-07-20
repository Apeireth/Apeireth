#!/usr/bin/env python3
"""V7 跨域调研第二轮 — 主人 22:01 '调研少不了'."""
import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

# 12 新跨域调研(避开 24-37 已做)
QUERIES = [
    'nash equilibrium incentive mechanism AI agents game theory',
    'lakoff embodied cognition metaphor AI cognitive linguistics',
    'watts small-world network collective intelligence',
    'niklas luhmann social systems autopoietic communication',
    'lotka-volterra predator-prey population dynamics AI',
    'levy flight search strategy optimization ai',
    'sleep memory consolidation hippocampus replay neuroscience',
    'stigmergy ant colony biological coordination swarm',
    'hebbian learning neurons fire together wire together',
    'varela neurophenomenology consciousness brain life',
    'thompson mind in life enactivism cognitive',
    'edge of chaos cellular automata computation',
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

with open('research-v7-second-round.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'\nsaved {len(results)} V7 round-2 queries')
