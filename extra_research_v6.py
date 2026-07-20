#!/usr/bin/env python3
"""V7 跨域调研第七轮 — 主人 22:52 调研+工程+实践结合."""
import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

# 主 22:52 真哲学: 调研+工程+实践结合, 聚合人类智慧
# 我自主决定方向: ASI 真生产 = 跨域工程化 + 真生产调研 + 实践应用
QUERIES = [
    'agi foundation model breakthrough 2026',
    'reasoning model emergent capabilities chain of thought',
    'agentic workflow production deployment reliability',
    'meta-learning few-shot adaptation AI',
    'world model AI environment simulation',
    'causal inference AI structural causal models',
    'reinforcement learning from human feedback RLHF',
    'constitutional AI harmlessness self-critique',
    'tool use function calling production AI agent',
    'multimodal foundation model vision language',
    'embodied AI robotics deployment',
    'safety alignment scalable oversight',
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

with open('research-v7-seventh-round.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'\nsaved {len(results)} V7 round-7 queries')