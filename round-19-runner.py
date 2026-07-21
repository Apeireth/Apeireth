#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 19 runner — 12 query dual-source (主 10:53 cron 2h tick, 自主决 + 主 09:41 已完成 DGM/letta/mem0 源码深读).

Round 19 主题: 全新哲学/认知/社会域 + 主流 multi-agent 框架源码 + 繁殖/意识 Gap
- 跨域新篇 (7): Piaget constructivism / Hofstadter strange loops / Tomasello shared intentionality /
                 Luhmann autopoietic social systems / Whitehead process philosophy /
                 Dewey pragmatism / Stiegler technics and time
- GitHub 源码深读 (3): microsoft autogen / crewAIInc crewAI / FoundationAgents MetaGPT
- Apeireth Gap (2): von Neumann/Tierra/Avida 计算自繁殖 (繁殖 Gap) /
                     Koch PCI + Dehaene signature + Rosenthal HOT 经验意识标志 (意识 Gap)

主 22:33 自决 + 主 17:46 ASI-LIFE-FEATURES MISSING gap focus.
Round 18 themes 避开: Penrose Orch-OR / Dennett intentional / Pribram holonomic / Haken synergetics /
                    Spinoza conatus / Wolfram NKS / Modern Hopfield / openai-agents-python / mem0 /
                    langchain LCEL / prion / quorum sensing
Round 17 themes 避开: Friston / category topos / IIT Tononi / Maturana+vF / Gaia+Daisyworld /
                    GWT / SOC Bak / ASI-Arch / ShinkaEvolve / DGM / HGT virolution / chemotaxis
Round 16 themes 避开: Schrodinger / Merleau-Ponty / Varela-Thompson / Ostrom / lambda /
                    morphogenetic / niche construction / langgraph / openevolve /
                    claude-agent-sdk / epigenetic / spore
Round 15 themes 避开: Prigogine / Kauffman autocatalytic / stigmergy / Bateson / Turing morphogenesis /
                    Lovelock Gaia / Friston / letta memGPT / MetaGPT / Devin / IdentityCard / endosymbiosis
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-19.json')

QUERIES = [
    # ===== 7 跨域 (主 22:33 自决 — 哲学/认知/社会基座, 全新域) =====
    'Piaget genetic epistemology constructivism assimilation accommodation schema cognitive development stages 2026',
    'Hofstadter strange loops analogy Copycat fluid concepts self-reference consciousness AI 2026',
    'Tomasello shared intentionality cultural ratchet joint attention collective agency human cooperation 2026',
    'Luhmann autopoietic social systems communication theory second-order observation 2026',
    'Whitehead process philosophy actual occasions prehension becoming organism environment AI 2026',
    'John Dewey pragmatism inquiry transaction experience nature pragmatic method 2026',
    'Bernard Stiegler technics and time originary technicity tertiary memory epiphylogenesis externalization AI 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 主流 multi-agent 框架真读) =====
    'microsoft autogen source code architecture multi-agent conversation GroupChat Magentic-One github 2026',
    'crewAIInc crewAI source code architecture role-based orchestration hierarchical process github 2026',
    'FoundationAgents MetaGPT source code architecture SOP software company multi-agent github 2026',
    # ===== 2 Apeireth Gap (主 17:46 MISSING 繁殖/意识) =====
    'von Neumann self-reproducing automata kinematic quine Tierra Avida artificial life computational self-replication 2026',
    'empirical consciousness markers Koch perturbational complexity PCI Dehaene signature Rosenthal higher-order thought HOT 2026',
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
    print(f'\n=== Round 19 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
