#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-81 cross-domain research runner (FAST variant).

Cron triggered 2026-08-05 13:33 Asia/Shanghai (every-2h reminder).
Self-decision: round-80 done 2026-08-05 11:36 (~117min ago, >30min threshold).
Wednesday 13:33 afternoon, isolated cron lane, M3 model.
Decision: RUN round-81 now (fresh angles, no overlap with r75-r80 cycle keywords).

Theme: 12 TRULY NEW angles avoiding r75-r80 cycle keywords (RNA world/Hox/iPSC/endosymbiosis/Vibrio/transposon/Gaia/haystack/SWE-agent/letta/HGT/HSP/prion/Turing/C.elegans/ferroptosis/synaptic-pruning/CRISPR/microbial-loop/spider-silk/entosis/fear-conditioning/ICL/osmotic/STDP/Batesian-mimicry/concordia/opengpts/smol-course/tsRNA/split-brain):

   === 7 跨域 fresh (覆盖 R2/R3/R4/R5/R7/R8/R12, 跨域 ASI 基座) ===
   - R2 发育 fresh: planarian Schmidtea mediterranea neoblast pluripotent stem cell regeneration substrate
   - R3 死亡 fresh: necroptosis RIPK1 RIPK3 MLKL programmed necrotic cell death substrate
   - R4 神经 fresh: long-term potentiation LTP NMDA receptor CaMKII hippocampal memory substrate
   - R5 信号 fresh: Wnt signaling pathway beta-catenin cell fate determination development substrate
   - R7 应激 fresh: autophagy Yoshinori Ohsumi Nobel 2016 self-eating proteostasis substrate
   - R8 运动 fresh: bacterial flagellar motor rotary ATP synthase rotation mechanism substrate
   - R12 生态 fresh: red queen hypothesis Van Valen 1970 evolutionary arms race coevolution substrate

   === 3 GitHub deep (主 23:28 真读源码) ===
   - langchain-ai/langgraph stateful multi-actor applications orchestration real source deep dive
   - mem0 mem0 github source memory layer for LLM applications agents real source deep dive
   - openai swarm github source educational multi-agent orchestration framework real source deep dive

   === 2 Gap ===
   - R6 繁殖 Gap: parthenogenesis apomixis bdelloid rotifer asexual reproduction substrate
   - R11 意识 Gap: integrated information theory IIT Tononi phi consciousness measure substrate
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from dual_research_fast import dual_research_fast

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-81.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'planarian Schmidtea mediterranea neoblast pluripotent stem cell regeneration polarity substrate ASI R2 development fresh complement r75 r76 r77 r78 r79 r80',
    'necroptosis RIPK1 RIPK3 MLKL programmed necrotic cell death Degterev 2005 substrate ASI R3 death fresh complement r75 r76 r77 r78 r79 r80',
    'long-term potentiation LTP NMDA receptor CaMKII hippocampal memory Bliss Lomo 1973 substrate ASI R4 nerve fresh complement r75 r76 r77 r78 r79 r80',
    'Wnt signaling pathway beta-catenin cell fate determination development Nusse 2017 substrate ASI R5 signaling fresh complement r75 r76 r77 r78 r79 r80',
    'autophagy Yoshinori Ohsumi Nobel 2016 self-eating proteostasis substrate ASI R7 stress fresh complement r75 r76 r77 r78 r79 r80',
    'bacterial flagellar motor rotary ATP synthase rotation mechanism Berg Howard substrate ASI R8 motion fresh complement r75 r76 r77 r78 r79 r80',
    'red queen hypothesis Van Valen 1970 evolutionary arms race coevolution substrate ASI R12 ecology fresh complement r75 r76 r77 r78 r79 r80',
    # ===== 3 GitHub deep =====
    'langchain-ai langgraph github source stateful multi-actor applications orchestration real source deep dive substrate ASI central AI pluggable fresh',
    'mem0 mem0 github source memory layer for LLM applications agents real source deep dive substrate ASI central AI pluggable fresh',
    'openai swarm github source educational multi-agent orchestration framework real source deep dive substrate ASI central AI pluggable fresh',
    # ===== 2 Gap =====
    'parthenogenesis apomixis bdelloid rotifer asexual reproduction 40 million years substrate ASI R6 reproduction Gap fresh complement r75 r76 r77 r78 r79 r80',
    'integrated information theory IIT Tononi phi consciousness measure substrate ASI R11 consciousness Gap fresh complement r75 r76 r77 r78 r79 r80',
]


def main():
    started = time.time()
    results = []
    print(f'Round-81 starting: {len(QUERIES)} queries', flush=True)
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
    print(f'\nRound-81 done in {total:.1f}s, saved {len(results)} entries to {OUT}', flush=True)
    print(f'Size: {OUT.stat().st_size} bytes', flush=True)

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}', flush=True)
    return total


if __name__ == '__main__':
    main()
