#!/usr/bin/env python3
"""V7 跨域调研第八轮 — 主人 22:52 调研不停 + 聚合人类智慧."""
import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

# 主 22:52 + 22:33 真哲学: 聚合人类智慧 + ASI 真生产
# 我自主决定方向: ASI 真生产 AGI 突破 + ASI 真生产真工程化 + 真生产实践
QUERIES = [
    'autonomous AI agent architecture self-improving',
    'real-time AI inference production deployment',
    'long-context AI million token window model',
    'AI agent memory framework production grade',
    'Apeireth ASI base platform architecture',
    'consciousness AI model emergent integration',
    'AI self-modification runtime recursive',
    'cross-domain AI research synthesis',
    'human-AI collaboration intelligence amplification',
    'human level AI reasoning benchmark frontier',
    'memory OS production grade Rust Python',
    'graph neural network symbolic reasoning hybrid',
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

with open('research-v7-eighth-round.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'\nsaved {len(results)} V7 round-8 queries')