#!/usr/bin/env python3
"""V7 跨域调研第三轮 — 主人 22:11 调研不停 + 继续工程化."""
import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

# 12 跨域调研第三轮(认知神经/信息论/意识哲学)
QUERIES = [
    'tononi integrated information theory IIT consciousness phi',
    'global workspace theory Baars consciousness',
    'predictive coding Clark free energy mind',
    'mirror neurons empathy social cognition',
    'whole brain emulation substrate independent mind',
    'panpsychism philosophy of mind consciousness',
    'qualia hard problem consciousness Chalmers',
    'structural information theory Gell-Mann Lloyd',
    'dynamical systems attractor neural computation',
    'metacognition animal consciousness evidence',
    'libet free will neuroscience readiness potential',
    'autopoiesis varela mature consciousness embodied',
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

with open('research-v7-third-round.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'\nsaved {len(results)} V7 round-3 queries')
