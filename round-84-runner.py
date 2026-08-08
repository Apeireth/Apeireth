#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-84 cross-domain research runner (FAST variant).

Cron triggered 2026-08-05 19:56 Asia/Shanghai (every-2h reminder).
Self-decision: round-83 done 2026-08-05 17:51 (~125min ago, >30min threshold).
Wednesday 19:56 evening, isolated cron lane, M3 model.
Decision: RUN round-84 now (12 fresh angles, no overlap with r71-r83 cycle).

Theme: 12 TRULY NEW angles avoiding r83 cycle keywords (phosphate / Gurdon / theta-gamma / Holliday / lipid-raft / Huxley / island-biogeography / memvid / browser-use / DGM / anastasis / Clark predictive-processing fresh; pre-keyword search showed all 12 below are FRESH except dspy/instructor/Wolbachia which were covered only once each in r22-31 — give them a fresh deep-dive angle).

   === 7 跨域 fresh (覆盖 R1/R2/R3/R4/R5/R7/R8/R12, 跨域 ASI 基座) ===
   - R1 化学 fresh: thioester world de Duve 1991 prebiotic energy currency metabolism
   - R2 发育 fresh: embryonic diapause blastocyst delay mammals suspended animation
   - R3 代谢 fresh: reverse Krebs cycle reductive tricarboxylic acid prebiotic carbon fixation
   - R4 神经 fresh: cerebellum internal model motor learning Marr Albus Ito 1982
   - R5 遗传 fresh: X chromosome inactivation Barr body Lyon dosage compensation mammal
   - R7 膜 fresh: AMPK energy sensing cellular homeostasis Hardie Hawley AMP ATP ratio
   - R8 运动 fresh: mitochondrial dynamics fission Drp1 fusion Mfn1 OPA1 quality control
   - R12 生态 fresh: legume-rhizobium nodulation symbiosis nitrogen fixation

   === 3 GitHub deep (主 23:28 真读源码, 主 00:21 ASI-Arch ⭐⭐⭐) ===
   - DSPy Stanford NLP github source deep dive program optimizer BootstrapFewShot MIPRO
   - instructor-ai github source deep dive Pydantic structured LLM output validation
   - perplexica github source deep dive open Perplexity alternative search engine

   === 2 Gap ===
   - R6 繁殖 Gap: Wolbachia cytoplasmic incompatibility endosymbiont parthenogenesis arthropod
   - R11 意识 Gap: Lamme recurrent processing theory consciousness frontal feedback
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from dual_research_fast import dual_research_fast

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-84.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'thioester world de Duve 1991 prebiotic energy currency metabolism origin substrate ASI R1 chemistry fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83',
    'embryonic diapause blastocyst delay mammals seasonal suspended animation development substrate ASI R2 development fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83',
    'reverse Krebs cycle reductive tricarboxylic acid prebiotic carbon fixation substrate ASI R3 metabolism fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83',
    'cerebellum internal model motor learning Marr Albus Ito 1982 substrate ASI R4 nerve fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83',
    'X chromosome inactivation Barr body Lyon dosage compensation mammal substrate ASI R5 genetics fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83',
    'AMPK energy sensing cellular homeostasis Hardie Hawley AMP ATP ratio substrate ASI R7 membrane fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83',
    'mitochondrial dynamics fission Drp1 fusion Mfn1 OPA1 quality control substrate ASI R8 motion fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83',
    'legume-rhizobium nodulation symbiosis nitrogen fixation substrate ASI R12 ecology fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83',
    # ===== 3 GitHub deep =====
    'DSPy Stanford NLP github source deep dive program optimizer BootstrapFewShot MIPRO substrate ASI central AI pluggable fresh r84',
    'instructor-ai github source deep dive Pydantic structured LLM output validation substrate ASI central AI pluggable fresh r84',
    'perplexica github source deep dive open Perplexity alternative search engine substrate ASI central AI pluggable fresh r84',
    # ===== 2 Gap =====
    'Wolbachia cytoplasmic incompatibility endosymbiont parthenogenesis arthropod substrate ASI R6 reproduction Gap fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83',
    'Lamme recurrent processing theory consciousness frontal feedback substrate ASI R11 consciousness Gap fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83',
]


def main():
    started = time.time()
    results = []
    print(f'Round-84 starting: {len(QUERIES)} queries', flush=True)
    for i, q in enumerate(QUERIES):
        t0 = time.time()
        r = dual_research_fast(q, top_k=5)
        dur = time.time() - t0
        bw = len(r['bocha_web'])
        ba = len(r['bocha_ai_answer'])
        any_n = len(r['anysearch'])
        merged = len(r['merged_sources'])
        print(f'[{i+1:02d}/{len(QUERIES)}] {dur:.1f}s | bw={bw} ba={ba} any={any_n} merged={merged} | {q[:80]}', flush=True)
        results.append(r)

    OUT.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - started
    print(f'\nRound-84 done in {total:.1f}s, saved {len(results)} entries to {OUT}', flush=True)
    print(f'Size: {OUT.stat().st_size} bytes', flush=True)

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}', flush=True)
    return total


if __name__ == '__main__':
    main()