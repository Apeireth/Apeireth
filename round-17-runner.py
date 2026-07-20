#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 17 runner — 12 query dual-source (主 00:49 cron 是提醒, agent 自主决 + 06:52 唤醒).

Round 17 主题: 新领域扩散 — 避开 round 13/14/15/16 已覆盖
- 跨域新篇 (7): Free Energy Principle / Category theory topos / IIT Tononi Φ / Maturana autopoiesis /
                 Gaia Daisyworld / Global Workspace Theory / Self-organized criticality
- GitHub 源码深读 (3): ASI-Arch (GAIR-NLP) / ShinkaEvolve (SakanaAI) / DGM (jennyzzt)
- Apeireth Gap (2): HGT horizontal gene transfer + virolution (繁殖 Gap) /
                     bacterial chemotaxis signal amplification (应激 Gap)

主 22:33 自决 + 主 17:46 ASI-LIFE-FEATURES MISSING gap focus.
Round 16 themes 避开: Schrödinger / Merleau-Ponty / Varela / Ostrom / lambda / morphogenetic / niche
                    construction / langgraph / openevolve / claude-agent-sdk / epigenetic / spore
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-17.json')

QUERIES = [
    # ===== 7 跨域 (主 22:33 自决 — 避开 round 13/14/15/16 已覆盖) =====
    'Friston free energy principle active inference variational Bayesian agent AI 2026',
    'category theory topos Yoneda functor agent composition AI 2026',
    'Tononi integrated information theory IIT Phi consciousness measure AI 2026',
    'Maturana autopoiesis second-order cybernetics Heinz von Foerster self-referential 2026',
    'Gaia hypothesis Daisyworld Lovelock biosphere homeostasis planetary AI 2026',
    'Global Workspace Theory Baars Dehaene neural workspace consciousness AI 2026',
    'self-organized criticality edge of chaos Bak sandpile phase transition AI 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 不只 README, 真读源码) =====
    'ASI-Arch GAIR-NLP AlphaEvolve autonomous discovery research code github source 2026',
    'ShinkaEvolve SakanaAI evolutionary code LLM-driven algorithm discovery source github 2026',
    'DGM Darwin Godel Machine jennyzzt self-improvement agent recursive source github 2026',
    # ===== 2 Apeireth Gap (主 17:46 MISSING 繁殖/应激 终极目标: 意识) =====
    'horizontal gene transfer endosymbiosis virolution non-parental reproduction AI lineage sharing 2026',
    'bacterial chemotaxis receptor signal amplification noise filter stimulus response agent 2026',
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
    print(f'\n=== Round 17 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
