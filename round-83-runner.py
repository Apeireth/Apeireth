#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-83 cross-domain research runner (FAST variant).

Cron triggered 2026-08-05 17:50 Asia/Shanghai (every-2h reminder).
Self-decision: round-82 done 2026-08-05 15:52 (~118min ago, >30min threshold).
Wednesday 17:50 afternoon, isolated cron lane, M3 model.
Decision: RUN round-83 now (12 fresh angles, no overlap with r75-r82 cycle).

Theme: 12 TRULY NEW angles avoiding r75-r82 cycle keywords (phosphate-origin / Gurdon / Holliday / Huxley / island-biogeography / lipid-raft / memvid / browser-use / DGM / anastasis / predictive-processing fresh; r82 used hydra-vulgaris / pyroptosis / place-grid / Hedgehog / heat-shock / kinesin-dynein / niche-construction / asi-arch / openevolve / ShinkaEvolve / hydra-budding / global-workspace):

   === 7 跨域 fresh (覆盖 R1/R2/R4/R5/R7/R8/R12, 跨域 ASI 基座) ===
   - R1 化学 fresh: phosphate origin of life prebiotic chemistry Westheimer
   - R2 发育 fresh: nuclear transfer Gurdon amphibian somatic cell cloning totipotent
   - R4 神经 fresh: theta-gamma coupling memory consolidation Buzsáki hippocampus
   - R5 遗传 fresh: Holliday junction 1964 homologous recombination DNA repair
   - R7 膜 fresh: lipid raft membrane microdomain Singer-Nicolson fluid mosaic
   - R8 运动 fresh: cardiac contraction Huxley striated muscle sarcomere sliding filament
   - R12 生态 fresh: island biogeography MacArthur-Wilson 1967 equilibrium turnover

   === 3 GitHub deep (主 23:28 真读源码, 主 00:21 ASI-Arch ⭐⭐⭐) ===
   - memvid video-memory github source deep dive
   - browser-use Browser-Use-Inc github source deep dive
   - DGM jennyzzt Darwin Godel Machine github source deep dive

   === 2 Gap ===
   - R6 繁殖 Gap: anastasis cell death recovery reversal apoptosis
   - R11 意识 Gap: predictive processing Clark action-oriented hierarchical Bayesian brain
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from dual_research_fast import dual_research_fast

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-83.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'phosphate origin of life prebiotic chemistry Westheimer phosphorus biology substrate ASI R1 chemistry fresh complement r75 r76 r77 r78 r79 r80 r81 r82',
    'nuclear transfer Gurdon amphibian Xenopus somatic cell cloning totipotent differentiated genome equivalence substrate ASI R2 development fresh complement r75 r76 r77 r78 r79 r80 r81 r82',
    'theta-gamma coupling memory consolidation Buzsáki hippocampal ripple replay cross-frequency substrate ASI R4 nerve fresh complement r75 r76 r77 r78 r79 r80 r81 r82',
    'Holliday junction 1964 homologous recombination DNA repair Holliday double-strand break Meselson substrate ASI R5 genetics fresh complement r75 r76 r77 r78 r79 r80 r81 r82',
    'lipid raft membrane microdomain Singer-Nicolson fluid mosaic cholesterol sphingolipid platform substrate ASI R7 membrane fresh complement r75 r76 r77 r78 r79 r80 r81 r82',
    'cardiac contraction Huxley striated muscle sarcomere sliding filament cross-bridge troponin substrate ASI R8 motion fresh complement r75 r76 r77 r78 r79 r80 r81 r82',
    'island biogeography MacArthur-Wilson 1967 equilibrium species-area turnover immigration extinction substrate ASI R12 ecology fresh complement r75 r76 r77 r78 r79 r80 r81 r82',
    # ===== 3 GitHub deep =====
    'memvid video-memory github source deep dive real architecture frame encoding substrate ASI central AI pluggable fresh r83',
    'browser-use Browser-Use-Inc github source deep dive real browser agent playwright substrate ASI central AI pluggable fresh r83',
    'DGM jennyzzt Darwin Godel Machine github source deep dive self-improving AI research substrate ASI central AI pluggable fresh r83',
    # ===== 2 Gap =====
    'anastasis cell death recovery reversal apoptosis dying cell resurrection Tang substrate ASI R6 reproduction Gap fresh complement r75 r76 r77 r78 r79 r80 r81 r82',
    'predictive processing Clark action-oriented hierarchical Bayesian brain free-energy Friston substrate ASI R11 consciousness Gap fresh complement r75 r76 r77 r78 r79 r80 r81 r82',
]


def main():
    started = time.time()
    results = []
    print(f'Round-83 starting: {len(QUERIES)} queries', flush=True)
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
    print(f'\nRound-83 done in {total:.1f}s, saved {len(results)} entries to {OUT}', flush=True)
    print(f'Size: {OUT.stat().st_size} bytes', flush=True)

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}', flush=True)
    return total


if __name__ == '__main__':
    main()