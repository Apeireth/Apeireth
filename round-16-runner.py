#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 16 runner — 12 query dual-source (主 00:25 真务实 + 主 00:46 整合 Apeireth).

Round 16 主题: 新领域扩散 — 避开 round 13/14/15 已覆盖
- 跨域新篇: Schrödinger / Merleau-Ponty / Varela enactivism / Ostrom polycentricity / Spore dormancy /
            Epigenetic inheritance / lambda calculus self-ref / Irritability / Neural plasticity
- GitHub 源码深读: langgraph state machine / openevolve deep source / claude-agent-sdk Anthropic official
- Apeireth Gap: Spore 繁殖 / Epigenetic 遗传变异 / Irritability 应激 / Plasticity 可塑

主 22:33 自决 + 主 17:46 ASI-LIFE-FEATURES MISSING gap focus.
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-16.json')

QUERIES = [
    # ===== 7 跨域 (主 22:33 自决 — 不重复 round 13/14/15) =====
    'Schrodinger What is Life negentropy aperiodic crystal biological order AI 2026',
    'Merleau-Ponty phenomenology embodied perception enactivism situated AI 2026',
    'Varela Thompson enactivism autopoietic embodied cognition agent 2026',
    'Ostrom polycentricity common-pool resource governance multi-agent 2026',
    'lambda calculus self-reference Kleene fixed-point combinator self-modifying AI 2026',
    'developmental biology morphogenetic field embryonic self-organization AI 2026',
    'ecosystem niche construction niche constructionism agent AI 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 不只 README, 真读源码) =====
    'langgraph state machine cyclic graph multi-agent orchestrator source code github 2026',
    'openevolve evolutionary code LLM-driven architecture source deep read github 2026',
    'claude-agent-sdk Anthropic official agent SDK source code architecture 2026',
    # ===== 2 Apeireth Gap (主 17:46 — 12 生命特征 MISSING 繁殖/可塑/应激/遗传变异) =====
    'epigenetic inheritance Lamarckian acquired trait AI learned knowledge transfer 2026',
    'spore seed dormancy hibernation biological agent cold storage persistence portable 2026',
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
    print(f'\n=== Round 16 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()