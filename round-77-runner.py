#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-77 cross-domain research runner.

Cron triggered 2026-08-04 22:53 Asia/Shanghai (every-2h reminder).
Self-decision: round-76 done 20:53:43 (~2h ago, >30min threshold).
Tuesday 22:53 evening, isolated cron lane, M3 model.
Decision: RUN round-77 now.

Theme: 12 TRULY NEW angles avoiding r68-r76 v3 cycle keywords:

   === 7 跨域 fresh (覆盖 R0/R1/R2/R3/R8/R9/R12, 跨域 ASI 基座) ===
   - R0 代谢 fresh: brown adipose tissue BAT thermogenesis UCP1 UCP2 UCP3
   - R1 生长 fresh: mollusk shell biomineralization nacre aragonite conchiolin Pif80
   - R2 发育 fresh: somitogenesis segmentation clock Notch Wnt FGF her1 hes7 oscillator
   - R3 死亡 fresh: necroptosis RIPK1 RIPK3 MLKL pseudokinase TNF-programmed necrosis
   - R8 运动 fresh: muscle contraction cross-bridge cycle Huxley 1957 actin myosin
   - R9 遗传 fresh: piRNA PIWI pathway transposon silencing germline Brennecke
   - R12 生态 fresh: Holling 1973 panarchy adaptive cycle resilience

   === 3 GitHub deep (主 23:28 真读源码) ===
   - all-hands-ai/OpenHands (OpenDevin) autonomous code agent
   - crewAIInc/crewAI multi-agent orchestration Crew Process Task Flow
   - autogen-group/ag2 AutoGen fork multi-agent framework

   === 2 Gap ===
   - R6 繁殖 Gap: paramutation maize b1 locus transgenerational epigenetic Hollick
   - R11 意识 Gap: attention schema theory Graziano 2019 self-model AST consciousness
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-77.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'brown adipose tissue BAT thermogenesis UCP1 UCP2 UCP3 uncoupling non-shivering mitochondria cold acclimation substrate ASI R0 metabolism fresh complement r71 r72 r73 r74 r75 r76',
    'mollusk shell biomineralization nacre aragonite conchiolin matrix protein Pif80 Pif97 mantle epithelium biomineralization substrate ASI R1 growth fresh complement r64 r66 r70 r72 r73 r74 r75 r76',
    'somitogenesis segmentation clock Notch Wnt FGF her1 hes7 oscillator vertebrate somitite boundary formation substrate ASI R2 development fresh complement r70 r72 r73 r74 r75 r76',
    'necroptosis RIPK1 RIPK3 MLKL mixed lineage kinase domain pseudokinase TNF programmed necrosis necrosome substrate ASI R3 death fresh complement r59 r60 r65 r71 r72 r73 r74 r75 r76',
    'muscle contraction cross-bridge cycle Huxley 1957 actin myosin sarcomere sliding filament rigor mortis substrate ASI R8 motion fresh complement r47 r54 r66 r70 r72 r73 r74 r75 r76',
    'piRNA PIWI pathway transposon silencing germline Brennecke 2007 Drosophila ping-pong amplification substrate ASI R9 heredity fresh complement r50 r55 r60 r65 r71 r72 r73 r74 r75 r76',
    'Holling 1973 panarchy adaptive cycle resilience social-ecological system Carpenter Brock Folke Gunderson substrate ASI R12 ecology fresh complement r40 r45 r50 r59 r66 r70 r72 r73 r74 r75 r76',
    # ===== 3 GitHub deep =====
    'all-hands-ai OpenHands OpenDevin github source autonomous code agent sandbox runtime plugin real source deep dive substrate ASI central AI pluggable fresh',
    'crewAIInc crewAI github source multi-agent orchestration Crew Process Task Flow agent role LLM substrate ASI central AI pluggable fresh',
    'autogen-group ag2 github source AutoGen fork Microsoft multi-agent GroupChat Swarm Magentic-One real source deep dive substrate ASI central AI pluggable fresh',
    # ===== 2 Gap =====
    'paramutation maize b1 locus transgenerational epigenetic inheritance Hollick 2006 siRNA meiotically heritable allele substrate ASI R6 reproduction Gap complement r40 r55 r60 r70 r72 r74 r75 r76',
    'attention schema theory Graziano 2019 self-model consciousness AST attention substrate ASI R11 consciousness Gap complement r40 r50 r55 r60 r71 r72 r73 r74 r75 r76',
]


def main():
    started = time.time()
    results = []
    print(f'Round-77 starting: {len(QUERIES)} queries')
    for i, q in enumerate(QUERIES):
        t0 = time.time()
        r = dual_research(q, top_k=5)
        dur = time.time() - t0
        bw = len(r['bocha_web'])
        ba = len(r['bocha_ai_answer'])
        any_n = len(r['anysearch'])
        merged = len(r['merged_sources'])
        print(f'[{i+1:02d}/{len(QUERIES)}] {dur:.1f}s | bw={bw} ba={ba} any={any_n} merged={merged} | {q[:80]}')
        results.append(r)
        time.sleep(0.5)

    OUT.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - started
    print(f'\nRound-77 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()
