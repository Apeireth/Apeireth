#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-86 cross-domain research runner (FAST variant).

Cron triggered 2026-08-08 13:02 Asia/Shanghai (every-2h reminder).
Self-decision: round-85 done 2026-08-05 20:52 (~52.9h ago, >>30min threshold).
Saturday 13:02 afternoon, isolated cron lane, M3 model.
Decision: RUN round-86 now (12 TRULY fresh angles, validated vs r8-r85).

Theme: 12 TRULY NEW angles, all 33 candidate keywords scanned clean vs r8-r85:
  fresh: Taq / Mullis / thermostable / Treg / Foxp3 / presynaptic / synapsin /
         TMAO / thigmotropism / tendril / Bryonia / Pisum / cancer-dormancy /
         disseminated-tumor / Pleurotus / Arthrobotrys / dottxt / outlines /
         mlfoundations / triton / firecrawl / Plasmodium / berghei / schizogony /
         apicoplast / merozoite / trained-immunity / BCG / mycelium.

   === 7 跨域 fresh (覆盖 R1/R3/R4/R5/R7/R8/R12, 跨域 ASI 基座) ===
   - R1 化学 fresh: Taq polymerase Thermus aquaticus thermostable PCR Mullis 1983
   - R3 免疫 fresh: regulatory T cell Treg Foxp3 IL-10 immune tolerance
   - R4 神经 fresh: presynaptic short-term plasticity facilitation synapsin
   - R5 应激 fresh: TMAO trimethylamine N-oxide deep sea osmolyte pressure
   - R7 应激 fresh: cancer dormancy minimal residual disease disseminated tumor
   - R8 植物 fresh: thigmotropism tendril coiling Bryonia Pisum
   - R12 群落 fresh: fungal mycelium chemical signaling Pleurotus Arthrobotrys

   === 3 GitHub deep (主 23:28 真读源码) ===
   - dottxt/outlines github source structured generation JSON regex
   - mlfoundations/triton github source kernel compiler GPU
   - firecrawl/firecrawl github source web crawler LLM

   === 2 Gap ===
   - R6 繁殖 Gap: Plasmodium berghei schizogony liver stage merozoite apicoplast
   - R11 可塑 Gap: trained immunity NK cell BCG epigenetic memory
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from dual_research_fast import dual_research_fast

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-86.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'Taq polymerase Thermus aquaticus thermostable DNA polymerase Mullis 1983 PCR cycle sequencing substrate ASI R1 chemistry fresh r86',
    'regulatory T cell Treg Foxp3 IL-10 immune tolerance peripheral CD4 CD25 substrate ASI R3 immune fresh r86',
    'presynaptic short-term plasticity facilitation synapsin residual calcium neurotransmitter release substrate ASI R4 neural fresh r86',
    'TMAO trimethylamine N-oxide deep sea osmolyte counteracting pressure protein stabilizer marine fish substrate ASI R5 stress fresh r86',
    'cancer dormancy minimal residual disease disseminated tumor cells bone marrow metastasis recurrence substrate ASI R7 cancer fresh r86',
    'thigmotropism tendril coiling Bryonia dioica Pisum sativum thigmonastic mechanosensitive plant movement substrate ASI R8 plant fresh r86',
    'fungal mycelium chemical signaling Pleurotus Arthrobotrys nematode-trapping hyphae network substrate ASI R12 community fresh r86',
    # ===== 3 GitHub deep =====
    'dottxt outlines github source structured generation JSON regex constrained decoding LLM r86',
    'mlfoundations triton github source kernel compiler GPU Python tile-based automatic differentiation r86',
    'firecrawl firecrawl github source web crawler LLM markdown extraction scraping r86',
    # ===== 2 Gap =====
    'Plasmodium berghei schizogony liver stage merozoite apicoplast asexual reproduction substrate ASI R6 reproduction Gap fresh r86',
    'trained immunity NK cell BCG epigenetic memory monocyte H3K27ac innate immune substrate ASI R11 plasticity Gap fresh r86',
]


def main():
    started = time.time()
    results = []
    print(f'Round-86 starting: {len(QUERIES)} queries', flush=True)
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
    print(f'\nRound-86 done in {total:.1f}s, saved {len(results)} entries to {OUT}', flush=True)
    print(f'Size: {OUT.stat().st_size} bytes', flush=True)

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}', flush=True)
    return total


if __name__ == '__main__':
    main()
