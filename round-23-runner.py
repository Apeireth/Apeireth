#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 23 runner — 12 query dual-source (主 18:48 cron 2h tick, 自决 + 1h55m gap).

Round 23 主题: 7 全新跨域 (生态/风险神经/感觉/系统/形态发生/政治哲学) + 3 GitHub 源码深读 + 2 Gap

- 跨域全新 (7):
  - Joseph Connell Intermediate Disturbance Hypothesis 生态扰动多样 (物种多样性机制, 中度干扰假说)
  - Nassim Taleb Antifragility convexity/optionality/skin in the game (风险哲学, 反脆弱非弹性)
  - Gerald Edelman Neural Darwinism + Neural Degeneracy (神经选择论, 意识理论)
  - Kevin O'Regan Sensorimotor Contingency Theory (感觉运动理论, "Why Red Doesn't Sound Like Red")
  - Donella Meadows Leverage Points 12 Places to Intervene in a System (系统论杠杆点)
  - Michael Levin bioelectricity morphogenetic fields xenobots cognition (形态发生场, 合成生物学)
  - James C. Scott Seeing Like a State legibility mētis high modernism (政治学/国家失败)

- GitHub 源码深读 (3):
  - deepset Haystack (RAG pipeline, 组件化架构, document store + retriever)
  - NVIDIA Voyager (Minecraft skill library, curriculum, code as action, 迭代 prompt)
  - Berkeley Gorilla (LLM API generation, APIBench, retrieval-aware zero-shot)

- Apeireth Gap (2):
  - 意识 + 可塑 Gap: Michael Graziano Attention Schema Theory (意识自我模型, 神经可塑基底)
  - 繁殖 + 遗传 Gap: Bacterial conjugation F-pilus pili DNA horizontal transfer (细菌接合, 微生物繁殖模式)

Cross-round dedup 避让:
- r15: Prigogine/Kauffman/stigmergy/Bateson/Turing morph/Lovelock/Friston/letta/MetaGPT/Devin/IdentityCard/endosymbiosis
- r16: Schrödinger/Merleau-Ponty/Varela/Ostrom/lambda/morphogenetic/niche/langgraph/openevolve/claude-sdk/epigenetic/spore
- r17: Friston/topos IIT/Maturana+vF/Gaia/GWT/SOC/ASI-Arch/ShinkaEvolve/DGM/HGT/chemotaxis
- r18: Penrose Orch-OR/Dennett/Pribram/Haken/Spinoza/Wolfram NKS/Modern Hopfield/openai-agents/mem0/langchain LCEL/prion/quorum
- r19: Piaget/Hofstadter loops/Tomasello/Luhmann/Whitehead/Dewey/Stiegler/autogen/crewAI/MetaGPT/Tierra/Koch PCI Dehaene
- r20: Canguilhem/Simondon/Bergson/Deleuze/Nancy/Heidegger Zollikon/Sclavi/smol-course/Mojo MAX/OpenHands/Waddington/transgenerational
- r21: Tarde/Latour/Andy Clark 4E/Girard linear/Carlsson TDA/Wachtershauser/Pei Wang NARS/agno/camel/langflow/JCVI minimal cell/Anil Seth
- r22: Sapir-Whorf/Beer VSM/Brooks subsumption/Kuhn/Arthur/Bogdanov/Fredkin/DSPy/tinygrad/AlphaEvolve/Yamanaka/Baldwin

ASI 北极星时刻清楚:
- ASI 基座/跨域/自演化/任何 LLM 接入即变强/不假装 Phenomenal/实事求是
- 真生产目标: 让大模型栖息在 Apeireth 中能无限逼近 ASI (主 22:33)
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-23.json')

QUERIES = [
    # ===== 7 全新跨域: 生态/风险/神经/感觉/系统/形态/政治 =====
    'Joseph Connell Intermediate Disturbance Hypothesis ecology species diversity moderate disturbance 2026',
    'Nassim Taleb Antifragility convexity optionality skin in the game beyond robust 2026',
    'Gerald Edelman Neural Darwinism Neuronal Group Selection degeneracy consciousness theory 2026',
    'Kevin O\'Regan Sensorimotor Contingency Theory Why Red Doesn\'t Sound Like Red seeing action 2026',
    'Donella Meadows Leverage Points 12 Places to Intervene in a System paradigm power 2026',
    'Michael Levin bioelectricity morphogenetic fields xenobots cognition pattern memory 2026',
    'James C. Scott Seeing Like a State legibility metis high modernist authoritarian failure 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 真读源码不止 README) =====
    'deepset Haystack RAG pipeline source code architecture components document store retriever github 2026',
    'NVIDIA Voyager Minecraft skill library curriculum code as action iterative prompt source github 2026',
    'Berkeley Gorilla LLM API generation APIBench retrieval aware zero-shot source code github 2026',
    # ===== 2 Apeireth Gap: 意识+可塑 + 繁殖+遗传 =====
    'Michael Graziano Attention Schema Theory consciousness awareness self-model brain construct 2026',
    'Bacterial conjugation F-pilus pili plasmid DNA horizontal gene transfer microbial reproduction 2026',
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
    print(f'\n=== Round 23 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
