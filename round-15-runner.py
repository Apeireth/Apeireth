#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 15 runner — 12 query dual-source (主 00:25 真务实修 + 主 00:46 整合 Apeireth).

Round 15 主题: 新领域扩散 (避开 round 13/14 已覆盖)
- 跨域新篇: Prigogine / Kauffman / Stigmergy / Bateson / Turing / Lovelock / Friston
- GitHub 源码深读: letta memGPT / metagpt SOP / Devin SWE agent
- Apeireth Gap: IdentityCard reproduction portable seed / endosymbiosis symbiogenesis merger

主 22:33 自决 + 主 00:33 主题.
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-15.json')

QUERIES = [
    # ===== 7 跨域 (主 22:33 自决 — 不重复 round 13/14) =====
    'Prigogine dissipative structures far-from-equilibrium self-organization ASI 2026',
    'Kauffman autocatalytic sets origin of life emergence ASI 2026',
    'stigmergy ant colony pheromone multi-agent coordination 2026',
    'Bateson ecology of mind levels of learning cybernetics 2026',
    'Turing morphogenesis reaction-diffusion pattern formation AI 2026',
    'Lovelock Gaia hypothesis self-regulation feedback AI ecosystem 2026',
    'Friston free energy principle active inference predictive processing 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 不只 README, 真读源码) =====
    'letta memGPT memory hierarchy archival recall source code github 2026',
    'metagpt software company SOP multi-agent framework source code github 2026',
    'Devin Cognition Labs SWE agent architecture sandbox source code analysis 2026',
    # ===== 2 Apeireth Gap (主 17:46 — 12 生命特征 MISSING) =====
    'IdentityCard export portable seed cross-platform agent reproduction 2026',
    'endosymbiosis symbiogenesis multi-agent merger fusion reproduction 2026',
]


def main():
    started = time.time()
    results = []
    for i, q in enumerate(QUERIES, 1):
        t0 = time.time()
        r = dual_research(q, top_k=5)
        dt = time.time() - t0
        results.append(r)
        n_web = len(r['bocha_web'])
        n_any = len(r['anysearch'])
        n_merge = len(r['merged_sources'])
        ai_chars = len(r['bocha_ai_answer'])
        print(f'[{i:2d}/12] ({dt:.1f}s) {q[:70]}')
        print(f'        bocha_web={n_web} anysearch={n_any} merged={n_merge} ai={ai_chars}')
        if r['bocha_ai_answer']:
            print(f'        ai_preview: {r["bocha_ai_answer"][:160]}')
        sys.stdout.flush()

    OUT.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - started
    print(f'\n=== Round 15 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()