#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-80 cross-domain research runner (FAST variant).

Cron triggered 2026-08-05 11:33 Asia/Shanghai (every-2h reminder).
Self-decision: round-79 done 2026-08-05 09:32 (~2h ago, >30min threshold).
Wednesday 11:33 morning, isolated cron lane, M3 model.
Decision: RUN round-80 now (fresh angles, no overlap with r74-r79 cycle keywords).

Theme: 12 TRULY NEW angles avoiding r74-r79 cycle keywords (RNA world/Hox/iPSC/endosymbiosis/Vibrio/transposon/Gaia/haystack/SWE-agent/letta/HGT/HSP/prion/Turing/C.elegans/ferroptosis/synaptic-pruning/CRISPR/microbial-loop):

   === 7 跨域 fresh (覆盖 R1/R3/R4/R5/R7/R10/R12, 跨域 ASI 基座) ===
   - R1 生长 fresh: spider silk spidroin Nephila MaSp repetitive block protein fiber material substrate
   - R3 死亡 fresh: entosis Overholtzer 2007 cell-in-cell cannibalism programmed death substrate
   - R4 神经 fresh: fear conditioning amygdala LeDoux 1996 emotional memory acquisition substrate
   - R5 信号 fresh: interstrand crosslink ICL Fanconi anemia DNA repair pathway network substrate
   - R7 应激 fresh: osmotic shock Hog1 MAPK glycerol yeast proteostasis stress response substrate
   - R10 神经 fresh: STDP spike-timing-dependent plasticity Bi Pribam Hebbian temporal asymmetry substrate
   - R12 生态 fresh: Batesian mimicry palatable species predator avoidance signal evolution substrate

   === 3 GitHub deep (主 23:28 真读源码) ===
   - deepmind/concordia generative agent simulation framework social emergence real source deep dive
   - jxmn/opengpts open source GPT platform chat agent alternative real source deep dive
   - huggingface/smol-course small language models alignment fine-tuning course real source deep dive

   === 2 Gap ===
   - R6 繁殖 Gap: transgenerational sperm tsRNA Chen 2016 epigenetic inheritance RNA-mediated heredity
   - R11 意识 Gap: split brain Sperry Gazzaniga 1981 corpus callosum consciousness lateralization Gap
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from dual_research_fast import dual_research_fast

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-80.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'spider silk spidroin Nephila MaSp1 MaSp2 repetitive block protein fiber material substrate ASI R1 growth fresh complement r74 r75 r76 r77 r78 r79',
    'entosis Overholtzer 2007 cell-in-cell cannibalism programmed death living cell substrate ASI R3 death fresh complement r74 r75 r76 r77 r78 r79',
    'fear conditioning amygdala LeDoux 1996 emotional memory acquisition storage substrate ASI R4 nerve fresh complement r74 r75 r76 r77 r78 r79',
    'interstrand crosslink ICL Fanconi anemia DNA repair pathway network FAAP substrate ASI R5 signaling fresh complement r74 r75 r76 r77 r78 r79',
    'osmotic shock Hog1 MAPK glycerol yeast proteostasis stress response substrate ASI R7 stress fresh complement r74 r75 r76 r77 r78 r79',
    'spike-timing-dependent plasticity STDP Bi Pribam Hebbian temporal asymmetry LTP LTD substrate ASI R10 nerve fresh complement r74 r75 r76 r77 r78 r79',
    'Batesian mimicry palatable species predator avoidance signal evolution mimic substrate ASI R12 ecology fresh complement r74 r75 r76 r77 r78 r79',
    # ===== 3 GitHub deep =====
    'deepmind concordia github source generative agent simulation framework social emergence real source deep dive substrate ASI central AI pluggable fresh',
    'jxmn opengpts github source open source GPT platform chat agent alternative real source deep dive substrate ASI central AI pluggable fresh',
    'huggingface smol-course github source small language models alignment fine-tuning course real source deep dive substrate ASI central AI pluggable fresh',
    # ===== 2 Gap =====
    'transgenerational sperm tsRNA Chen 2016 epigenetic inheritance RNA-mediated heredity substrate ASI R6 reproduction Gap complement r74 r75 r76 r77 r78 r79',
    'split brain Sperry Gazzaniga 1981 corpus callosum consciousness lateralization interpreter substrate ASI R11 consciousness Gap complement r74 r75 r76 r77 r78 r79',
]


def main():
    started = time.time()
    results = []
    print(f'Round-80 starting: {len(QUERIES)} queries', flush=True)
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
    print(f'\nRound-80 done in {total:.1f}s, saved {len(results)} entries to {OUT}', flush=True)
    print(f'Size: {OUT.stat().st_size} bytes', flush=True)

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}', flush=True)
    return total


if __name__ == '__main__':
    main()