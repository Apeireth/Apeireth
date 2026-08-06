#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-75 cross-domain research runner.

Cron triggered 2026-08-04 20:44 Asia/Shanghai (every-2h reminder).
Self-decision: 19:10 SKIPPED (round-74 done 18:59, only 11min < 30min master 00:49 rule).
Now at 20:44 = 1h45m after round-74 done, well past 30min threshold.
Tuesday 20:44 evening, isolated cron lane, M3 model.
Decision: RUN round-75 now (active wakeup, agent self-driven).

Theme: 7 跨域 fresh — TRULY NEW angles avoiding r68-r74 v3 cycle keywords:
   - R1 生长 fresh: feather keratin beta-keratin birds sauropsid vs mammalian alpha
   - R3 死亡 fresh: systemic lupus erythematosus SLE dsDNA anti-dsDNA autoantibody immune complex
   - R5 修复 fresh: nucleotide excision repair NER xeroderma pigmentosum XPA XPC TFIIH UV-DDB
   - R7 应激 fresh: jasmonic acid JA wounding plant defense COI1 JAZ MYC2
   - R8 运动 fresh: vestibular otolith saccule utricle balance gravity hair cell
   - R10 可塑 fresh: homeostatic plasticity Turrigiano synaptic scaling TTX bicuculline
   - R12 生态 fresh: metapopulation Hanski 1991 incidence function Levins patch occupancy
   + 3 GitHub deep (master 00:21 真读):
   - SakanaAI/AI-CUDA-Engineer (LLM 自动写 CUDA kernel 加速)
   - camel-ai/camel (role-playing communicative agents 早期 multi-agent)
   - microsoft/TypeChat (类型化 LLM 交互 schema 驱动)
   + 2 Gap:
   - R6 繁殖 Gap: parthenogenesis apomixis clonal reproduction (无性繁殖 真模式)
   - R11 意识 Gap: neurophenomenology Varela 1996 first-person methods 现象学 + 神经科学
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-75.json')

QUERIES = [
    # ===== 7 跨域 fresh (TRULY NEW angles) =====
    'feather keratin beta-keratin birds sauropsid vs mammalian alpha-keratin intermediate filament scale reptilian claw substrate ASI R1 growth fresh complement r73 skin keratin KRT',
    'systemic lupus erythematosus SLE anti-dsDNA autoantibody immune complex type III hypersensitivity glomerulonephritis complement classical pathway substrate ASI R3 death fresh complement r66 r67 r68 r69 r70 r72 r73',
    'nucleotide excision repair NER xeroderma pigmentosum XPA XPC TFIIH UV-DDB global genomic transcription coupled substrate ASI R5 repair fresh complement r50 r55 r60 r65 r68 r70 r71 r73',
    'jasmonic acid JA wounding plant defense COI1 JAZ MYC2 OPDA lipid peroxidation methyl jasmonate systemic acquired substrate ASI R7 stress fresh complement r66 r67 r68 r69 r71 r73 r74',
    'vestibular otolith saccule utricle hair cell otoconcalcin otogelin gravity linear acceleration semicircular canal ampulla substrate ASI R8 motion fresh complement r66 r67 r70 r72 r73 r74',
    'homeostatic plasticity Turrigiano synaptic scaling TTX bicuculline multiplicative firing rate set point neuron intrinsic excitability substrate ASI R10 plasticity fresh complement r55 r60 r65 r71 r72 r73 r74',
    'metapopulation Hanski 1991 incidence function Levins 1969 patch occupancy rescue effect extinction threshold core satellite species substrate ASI R12 ecology fresh complement r58 r59 r66 r67 r68 r71 r72 r73 r74',
    # ===== 3 GitHub deep =====
    'SakanaAI AI-CUDA-Engineer github source LLM automated CUDA kernel generation GPU acceleration deep learning framework real source deep dive substrate ASI central AI pluggable fresh',
    'camel-ai camel github source communicative agents role-playing inception prompting chat chain early multi-agent framework real source deep dive substrate ASI central AI pluggable fresh',
    'microsoft TypeChat github source schema-driven LLM interaction typescript type-safe structured response validator real source deep dive substrate ASI central AI pluggable fresh',
    # ===== 2 Gap =====
    'parthenogenesis apomixis clonal reproduction daphnia rotifer aphid sperm-dependent asexual mitotic meiotic substrate ASI R6 reproduction Gap complement r62 r64 r65 r66 r68 r69 r70 r72 r73 r74',
    'neurophenomenology Varela 1996 first-person methods phenomenological reduction neurophenomenological interview Husserl Merleau-Ponty mutual constraint substrate ASI R11 consciousness Gap complement r50 r51 r55 r56 r57 r61 r62 r63 r64 r65 r66 r67 r71 r73 r74',
]


def main():
    started = time.time()
    results = []
    print(f'Round-75 starting: {len(QUERIES)} queries')
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
    print(f'\nRound-75 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()
