#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""V7 round-11 dual-source research — 跨域 ASI 真生产借鉴 (主 00:21 抓紧干).

12 queries focused on:
- GitHub/知网 优质项目 + 源码深入
- Cross-domain 真哲学借鉴 (ecology, cybernetics, active inference, autopoiesis)
- 真生产 patterns (real production agents, frameworks)
"""
from __future__ import annotations
import json, sys, time
from concurrent.futures import ThreadPoolExecutor, as_completed
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

QUERIES = [
    # === GitHub 优质项目 (主 00:21 源码深入) ===
    'Darwin Godel Machine recursive self-improvement github',
    'AlphaEvolve DeepMind code github implementation',
    'ShinkaEvolve self-evolving code agent github',
    'mem0 Letta production memory layer github',
    'Anthropic Claude Agent SDK architecture github',
    'LangGraph LangChain production agent patterns 2026',
    
    # === 跨域真哲学借鉴 (主 23:28) ===
    'ecology keystone species multi-agent ASI production',
    'second-order cybernetics self-aware AI implementation',
    'active inference free energy principle LLM agent production',
    'autopoiesis Maturana Varela self-producing system ASI',
    
    # === 真生产 patterns ===
    'ASI architecture open source github 2026',
    'agent harness production observability 2026',
]

def run_one(q):
    t0 = time.time()
    r = dual_research(q, top_k=4)
    dt = time.time() - t0
    bw = len(r['bocha_web'])
    as_ = len(r['anysearch'])
    mg = len(r['merged_sources'])
    ai = len(r['bocha_ai_answer'])
    print(f'[{dt:5.1f}s] {q[:55]:<55} | bw={bw} any={as_} merged={mg} ai={ai}', flush=True)
    return r

def main():
    out_path = Path(r'.openclaw\workspace\promethean\research-v7-round-11.json')
    print(f'=== V7 round-11 双端点调研 (12 query) ===', flush=True)
    print(f'output: {out_path}', flush=True)
    
    # 并行跑 (主 21:22 任何时候都跑得起 = AnySearch 8x 并行)
    results = [None] * len(QUERIES)
    with ThreadPoolExecutor(max_workers=6) as ex:
        futs = {ex.submit(run_one, q): i for i, q in enumerate(QUERIES)}
        for fut in as_completed(futs):
            i = futs[fut]
            try:
                results[i] = fut.result()
            except Exception as e:
                print(f'Q{i} failed: {e}', flush=True)
                results[i] = {'query': QUERIES[i], 'error': str(e)}
    
    out_path.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    print(f'\n=== saved {len(results)} queries → {out_path.name} ===')
    
    # 摘要
    total_sources = sum(len(r.get('merged_sources', [])) for r in results)
    total_ai_chars = sum(len(r.get('bocha_ai_answer', '')) for r in results)
    print(f'total merged sources: {total_sources}')
    print(f'total AI answer chars: {total_ai_chars}')

if __name__ == '__main__':
    main()