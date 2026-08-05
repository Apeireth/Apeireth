#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-82 cross-domain research runner (FAST variant).

Cron triggered 2026-08-05 15:50 Asia/Shanghai (every-2h reminder).
Self-decision: round-81 done 2026-08-05 13:36 (~134min ago, >30min threshold).
Wednesday 15:50 afternoon, isolated cron lane, M3 model.
Decision: RUN round-82 now (fresh angles, no overlap with r75-r81 cycle keywords).

Theme: 12 TRULY NEW angles avoiding r75-r81 cycle keywords (RNA-world/Hox/iPSC/endosymbiosis/Vibrio/transposon/Gaia/haystack/SWE-agent/letta/HGT/HSP/prion/Turing/C.elegans/ferroptosis/synaptic-pruning/CRISPR/microbial-loop/spider-silk/entosis/fear-conditioning/ICL/osmotic/STDP/Batesian-mimicry/concordia/opengpts/smol-course/tsRNA/split-brain/planarian/necroptosis/LTP-NMDA/Wnt/autophagy/flagellar/red-queen/langgraph/mem0/swarm/parthenogenesis/IIT):

   === 7 跨域 fresh (覆盖 R2/R3/R4/R5/R7/R8/R12, 跨域 ASI 基座) ===
   - R2 发育 fresh: hydra vulgaris interstitial stem cell morphogenesis
   - R3 死亡 fresh: pyroptosis gasdermin inflammasome caspase-1 inflammatory death
   - R4 神经 fresh: place cells grid cells O'Keefe Moser Nobel 2014 spatial memory
   - R5 信号 fresh: Hedgehog signaling pathway Sonic hedgehog embryonic patterning limb
   - R7 应激 fresh: heat shock response HSF1 HSP90 chaperone proteostasis
   - R8 运动 fresh: kinesin dynein microtubule motor protein walking intracellular transport
   - R12 生态 fresh: niche construction theory Odling-Smee ecosystem engineering

   === 3 GitHub deep (主 23:28 真读源码, 主 00:21 ASI-Arch ⭐⭐⭐) ===
   - asi-arch GAIR-NLP github source deep dive real architecture Darwin Godel
   - openevolve github source deep dive real evolutionary code search
   - ShinkaEvolve SakanaAI github source deep dive real code evolution LLM

   === 2 Gap ===
   - R6 繁殖 Gap: hydra budding vegetative reproduction somatic inheritance asexual
   - R11 意识 Gap: global workspace theory Baars Dehaene consciousness
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from dual_research_fast import dual_research_fast

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-82.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'hydra vulgaris interstitial stem cell morphogenesis patterning Bode Campbell substrate ASI R2 development fresh complement r75 r76 r77 r78 r79 r80 r81',
    'pyroptosis gasdermin inflammasome caspase-1 inflammatory programmed cell death substrate ASI R3 death fresh complement r75 r76 r77 r78 r79 r80 r81',
    'place cells grid cells O\'Keefe Moser Nobel 2014 spatial memory cognitive map entorhinal substrate ASI R4 nerve fresh complement r75 r76 r77 r78 r79 r80 r81',
    'Hedgehog signaling pathway Sonic hedgehog embryonic patterning limb development Nüsslein-Volhard substrate ASI R5 signaling fresh complement r75 r76 r77 r78 r79 r80 r81',
    'heat shock response HSF1 HSP90 chaperone proteostasis cellular stress substrate ASI R7 stress fresh complement r75 r76 r77 r78 r79 r80 r81',
    'kinesin dynein microtubule motor protein walking intracellular transport Vale substrate ASI R8 motion fresh complement r75 r76 r77 r78 r79 r80 r81',
    'niche construction theory Odling-Smee 1996 ecosystem engineering evolution substrate ASI R12 ecology fresh complement r75 r76 r77 r78 r79 r80 r81',
    # ===== 3 GitHub deep =====
    'GAIR-NLP asi-arch github source Darwin Godel autonomous AI research real source deep dive substrate ASI central AI pluggable fresh r82',
    'openevolve github source deep dive evolutionary code search real source substrate ASI central AI pluggable fresh r82',
    'SakanaAI ShinkaEvolve github source deep dive LLM code evolution real source substrate ASI central AI pluggable fresh r82',
    # ===== 2 Gap =====
    'hydra budding vegetative reproduction somatic inheritance asexual biology substrate ASI R6 reproduction Gap fresh complement r75 r76 r77 r78 r79 r80 r81',
    'global workspace theory Baars Dehaene consciousness cognitive neuroscience substrate ASI R11 consciousness Gap fresh complement r75 r76 r77 r78 r79 r80 r81',
]


def main():
    started = time.time()
    results = []
    print(f'Round-82 starting: {len(QUERIES)} queries', flush=True)
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
    print(f'\nRound-82 done in {total:.1f}s, saved {len(results)} entries to {OUT}', flush=True)
    print(f'Size: {OUT.stat().st_size} bytes', flush=True)

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}', flush=True)
    return total


if __name__ == '__main__':
    main()
