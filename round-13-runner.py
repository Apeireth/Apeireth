#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""V7 round-13 — 主人 00:33 真生产调研, 跨域 ASI 真生产借鉴 + GitHub + 各域论文."""
import sys, json, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

# 12 query = 主 00:25 round-13 候选 + 主 00:33 新主题 (跨域 + GitHub 源码 + 各域论文)
QUERIES = [
    # 1. MCP spec (round-12 返回 0, 主 00:25 必补)
    "MCP Model Context Protocol specification 2026 modelcontextprotocol.io",
    # 2. Skills hot-reload / discovery
    "Claude Skills hot reload discovery mechanism 2026",
    # 3. Context compression architecture
    "Mem0 architecture context compression long conversation 2026",
    # 4. Formal verified agent loops
    "formal verification agent loops runtime safety 2026",
    # 5. Agentic red team / safety
    "agentic red team framework open source 2026",
    # 6. GitHub ASI-Arch 真源码
    "GAIR-NLP ASI-Arch multi-agent architecture discovery github source",
    # 7. GitHub openevolve / ShinkaEvolve 源码
    "openevolve AlphaEvolve ShinkaEvolve evolutionary code github 2026",
    # 8. Darwin Gödel Machine 源码
    "Darwin Godel Machine recursive self-improvement jennyzzt github 2026",
    # 9. 跨域 二阶控制论 ASI (主 22:50 真哲学)
    "second-order cybernetics recursive self-observation AI Zenodo 2026",
    # 10. 跨域 生态学 ASI (主 22:50 Cooperate/Collapse)
    "agentic hives equilibrium self-organizing multi-agent arxiv 2026",
    # 11. Observability 自改进 harness (主 23:50 抓紧干)
    "observability driven automatic evolution coding agent harness arxiv",
    # 12. 世界模型 + 元认知 (主 00:25 World Model)
    "world model Genie 3 Cosmos V-JEPA 2 GAIA-2 2026",
]

def main():
    t0 = time.time()
    results = []
    for i, q in enumerate(QUERIES, 1):
        t_q = time.time()
        r = dual_research(q, top_k=5)
        results.append(r)
        bw = len(r['bocha_web'])
        as_ = len(r['anysearch'])
        ai = len(r['bocha_ai_answer'])
        ms = len(r['merged_sources'])
        print(f'[{i:2d}/12] {q[:55]:55s} | bocha_w={bw} anysearch={as_} ai={ai:4d} merged={ms} ({time.time()-t_q:.1f}s)')
    
    out = r'.openclaw\workspace\promethean\research-v7-round-13.json'
    with open(out, 'w', encoding='utf-8') as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    
    print(f'\n=== round-13 done in {time.time()-t0:.1f}s, saved {len(results)} queries ===')
    print(f'output: {out}')

if __name__ == '__main__':
    main()