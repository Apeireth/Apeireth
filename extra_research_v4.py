#!/usr/bin/env python3
"""V7 跨域调研第四轮 — 主人 22:14 好好干 + 调研不停."""
import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

# 12 跨域调研第四轮(深化: 神经化学 + 嵌入 + 涌现物理)
QUERIES = [
    'neuromodulation dopamine acetylcholine cortex',
    'astrocyte glia neuron brain computation',
    'astrocyte tripartite synapse AI architecture',
    'temporal coherence binding consciousness Crick Koch',
    'high-dimensional space computation neural manifold',
    'cognitive flexibility set shifting AI',
    'simulation theory consciousness mind',
    'paradoxical sleep memory replay dream',
    'predictive processing active inference brain hierarchy',
    'recurrent neural network reservoir computing dynamics',
    'kuramoto model coupled oscillators brain synchrony',
    'critical branching dynamic brain phase transition',
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

with open('research-v7-fourth-round.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'\nsaved {len(results)} V7 round-4 queries')
