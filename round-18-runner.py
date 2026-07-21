#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 18 runner — 12 query dual-source (主 00:49 cron 是提醒, agent 自主决 + 08:53 唤醒).

Round 18 主题: 新领域扩散 — 避开 round 11/13/14/15/16/17 已覆盖
- 跨域新篇 (7): Penrose Orch-OR / Dennett intentional stance / Pribram holonomic brain /
                 Haken synergetics / Spinoza conatus / Wolfram NKS cellular automata /
                 Modern Hopfield networks
- GitHub 源码深读 (3): openai-agents-python SDK / mem0 memory layer / langchain LCEL
- Apeireth Gap (2): prion protein folding inheritance (遗传变异 Gap) /
                     quorum sensing bacterial collective decision (应激 Gap)

主 22:33 自决 + 主 17:46 ASI-LIFE-FEATURES MISSING gap focus.
Round 17 themes 避开: Friston / category / IIT Tononi / Maturana+vF / Gaia+Daisyworld /
                    GWT / SOC Bak / ASI-Arch / ShinkaEvolve / DGM / HGT virolution / chemotaxis
Round 16 themes 避开: Schrodinger / Merleau-Ponty / Varela-Thompson / Ostrom / lambda /
                    morphogenetic / niche construction / langgraph / openevolve /
                    claude-agent-sdk / epigenetic / spore
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-18.json')

QUERIES = [
    # ===== 7 跨域 (主 22:33 自决 — 避开 round 11/13/14/15/16/17 已覆盖) =====
    'Penrose Orch-OR quantum consciousness microtubules objective reduction AI 2026',
    'Dennett intentional stance multiple drafts consciousness explained heterophenomenology AI 2026',
    'Pribram holonomic brain theory holographic memory Fourier transform AI 2026',
    'Haken synergetics slaving principle order parameters dissipative self-organization AI 2026',
    'Spinoza conatus potentia affectus ethics agent agency AI 2026',
    'Wolfram A New Kind of Science cellular automata NKS computational universe AI 2026',
    'modern Hopfield networks 2020 attention transformer dense associative memory AI 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 不只 README, 真读源码) =====
    'openai openai-agents-python SDK source code architecture agents runtime github 2026',
    'mem0ai mem0 source code memory layer production architecture add search github 2026',
    'langchain-ai langchain source code architecture chain LCEL expression language production 2026',
    # ===== 2 Apeireth Gap (主 17:46 MISSING 繁殖/应激/遗传/可塑) =====
    'prion protein folding inheritance non-DNA epigenetic amyloid yeast PSI+ ASI 2026',
    'quorum sensing Vibrio fischeri bioluminescence collective intelligence bacterial decision AI 2026',
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
    print(f'\n=== Round 18 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
