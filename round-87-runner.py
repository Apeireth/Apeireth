#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-87 cross-domain research runner (FAST variant).

Cron triggered 2026-08-08 14:48 Asia/Shanghai (every-2h reminder).
Self-decision: round-86 done 2026-08-08 13:04 (~102min ago, >30min threshold).
Saturday 14:48 afternoon, isolated cron lane, M3 model.
Decision: RUN round-87 now (12 TRULY fresh angles, validated vs r8-r86, 0 collisions).

Theme: 12 TRULY NEW angles — all 36 candidate keywords scanned clean vs r8-r86:
  fresh: Cas12a / Cas13a / crRNA / trans-cleavage / programmable-nuclease /
         gamma-delta-T / intraepithelial-lymphocyte / mucosal-surveillance /
         schema-integration / gist-memory / REM-replay / cortical-consolidation /
         p62-sequestosome / selective-autophagy / angiopoietin / Tie2-receptor /
         vascular-stabilization / phyllotaxis / Fibonacci-spiral / leaf-primordium /
         shoot-apical-meristem / syntrophy / cross-feeding / methanogen /
         BERTrend / gradio / chainlit / self-incompatibility / SI-RNase / S-locus /
         latent-inhibition / noradrenergic / locus-coeruleus / learning-suppression.

   === 7 跨域 fresh (覆盖 R1/R3/R4/R5/R7/R8/R12, 跨域 ASI 基座) ===
   - R1 化学 fresh: Cas12a Cas13a crRNA trans-cleavage programmable nuclease
   - R3 免疫 fresh: gamma-delta-T intraepithelial-lymphocyte mucosal-surveillance
   - R4 神经 fresh: schema-integration gist-memory REM-replay cortical-consolidation
   - R5 应激 fresh: p62-sequestosome selective-autophagy (replaced stress-granule r61 collision)
   - R7 应激 fresh: angiopoietin Tie2-receptor vascular-stabilization (replaced VEGF/angiogenesis r69)
   - R8 植物 fresh: phyllotaxis Fibonacci-spiral leaf-primordium SAM
   - R12 群落 fresh: syntrophy cross-feeding methanogen anaerobic-digestion

   === 3 GitHub deep (主 23:28 真读源码) ===
   - microsoft/BERTrend github source
   - gradio/gradio github source
   - Chainlit/chainlit github source

   === 2 Gap ===
   - R6 繁殖 Gap: self-incompatibility SI-RNase S-locus (replaced apomixis r25/polyembryony r66)
   - R11 可塑 Gap: latent-inhibition noradrenergic locus-coeruleus learning-suppression
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from dual_research_fast import dual_research_fast

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-87.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'Cas12a Cas13a crRNA trans-cleavage programmable nuclease collateral detection RNA-guided substrate ASI R1 chemistry fresh r87',
    'gamma-delta-T intraepithelial-lymphocyte mucosal-surveillance unconventional T cell Vgamma9 Vdelta1 TRG TRD substrate ASI R3 immune fresh r87',
    'schema-integration gist-memory REM-replay cortical-consolidation hippocampal neocortex slow-wave sharp-wave ripple substrate ASI R4 neural fresh r87',
    'p62-sequestosome selective-autophagy ubiquitin-binding LC3 cargo receptor Nrf2 PINK1 Parkin substrate ASI R5 stress fresh r87',
    'angiopoietin Tie2-receptor vascular-stabilization endothelial quiescence pericyte ANGPT1 ANGPT2 substrate ASI R7 stress fresh r87',
    'phyllotaxis Fibonacci-spiral leaf-primordium shoot-apical-meristem auxin-pattern PIN1 convergent-substrate ASI R8 plant fresh r87',
    'syntrophy cross-feeding methanogen anaerobic-digestion interspecies-H2 acetate syntrophic-bacteria substrate ASI R12 community fresh r87',
    # ===== 3 GitHub deep =====
    'microsoft BERTrend github source LLM trend monitoring RAG news subreddit clustering r87',
    'gradio gradio github source ChatInterface Blocks events queue websocket UI LLM r87',
    'Chainlit chainlit github source chat UI LLM observability prompt engineering streaming r87',
    # ===== 2 Gap =====
    'self-incompatibility SI-RNase S-locus pollen-style recognition Brassica Papaver asexual reproduction substrate ASI R6 reproduction Gap fresh r87',
    'latent-inhibition noradrenergic locus-coeruleus learning-suppression norepinephrine CS pre-exposure plasticity substrate ASI R11 plasticity Gap fresh r87',
]


def main():
    started = time.time()
    results = []
    print(f'Round-87 starting: {len(QUERIES)} queries', flush=True)
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
    print(f'\nRound-87 done in {total:.1f}s, saved {len(results)} entries to {OUT}', flush=True)
    print(f'Size: {OUT.stat().st_size} bytes', flush=True)

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}', flush=True)
    return total


if __name__ == '__main__':
    main()
