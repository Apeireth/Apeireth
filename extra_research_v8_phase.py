#!/usr/bin/env python3
"""V7 round-8 background — 主 23:50 抓紧干."""
import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

# 主 22:52 调研不停 + 主 22:33 ASI 北极星 + 干到底
QUERIES = [
    'ASI artificial super intelligence foundation model breakthrough 2026',
    'long horizon agentic reasoning planning 2026',
    'LLM memory system stateful agent architecture',
    'knowledge graph construction LLM real production',
    'agentic workflow LangGraph 2026 production',
    'code agent SWE-bench benchmark 2026 state of the art',
    'constitutional AI harmlessness RLHF 2026',
    'multi-modal foundation model 2026',
    'world model generation agent 2026',
    'tool use function calling production agent reliability',
    'human in the loop AI agent feedback learning',
    'emergent capabilities large language model scaling',
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

with open('research-v7-round-8.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'saved {len(results)} V7 round-8 queries')