#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-79 cross-domain research runner (FAST variant).

Cron triggered 2026-08-05 09:32 Asia/Shanghai (every-2h reminder).
Self-decision: round-78 done 2026-08-05 07:34 (~2h ago, >30min threshold).
Wednesday 09:32 morning, isolated cron lane, M3 model.
Decision: RUN round-79 now (fresh angles, no overlap with r78's prion/Turing/C.elegans/ferroptosis/synaptic-pruning/CRISPR/microbial-loop cycle).

Theme: 12 TRULY NEW angles avoiding r74-r78 cycle keywords:

   === 7 跨域 fresh (覆盖 R0/R1/R2/R3/R5/R9/R12, 跨域 ASI 基座) ===
   - R0 代谢 fresh: RNA world hypothesis Gilbert 1986 self-replicating ribozyme origin of life autocatalytic
   - R1 生长 fresh: Hox gene colinearity Lewis 1978 Drosophila bithorax homeotic body plan patterning
   - R2 发育 fresh: Yamanaka 2006 iPSC Oct4 Sox2 Klf4 cMyc pluripotency reprogramming somatic cell
   - R3 死亡 fresh: endosymbiosis Margulis 1970 mitochondria chloroplast serial eukaryotic origin
   - R5 神经 fresh: Vibrio fischeri bioluminescence quorum sensing luxR luxI autoinducer density-dependent
   - R9 遗传 fresh: transposon McClintock 1951 Ac Ds Activator Dissociation maize mobile genetic element
   - R12 生态 fresh: Gaia hypothesis Lovelock Margulis 1974 earth homeostasis biosphere feedback

   === 3 GitHub deep (主 23:28 真读源码) ===
   - deepset-ai/haystack pipeline RAG retrieval augmented agent real source deep dive
   - SWE-agent/SWE-bench autonomous code agent benchmark real source deep dive
   - letta-ai/letta memGPT memory management LLM agent self-editing context window real source deep dive

   === 2 Gap ===
   - R6 繁殖 Gap: horizontal gene transfer HGT conjugation transformation transduction antibiotic resistance biofilm
   - R7 意识 Gap: heat shock protein HSP Hsp70 Hsp90 HSF chaperone stress response misfolded protein
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from dual_research_fast import dual_research_fast

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-79.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'RNA world hypothesis Gilbert 1986 self-replicating ribozyme autocatalytic network origin of life substrate ASI R0 metabolism fresh complement r74 r75 r76 r77 r78',
    'Hox gene colinearity Lewis 1978 Drosophila bithorax homeotic body plan patterning segmentation substrate ASI R1 growth fresh complement r74 r75 r76 r77 r78',
    'Yamanaka 2006 iPSC Oct4 Sox2 Klf4 cMyc pluripotency reprogramming somatic cell epigenetic substrate ASI R2 development fresh complement r74 r75 r76 r77 r78',
    'endosymbiosis Margulis 1970 mitochondria chloroplast serial eukaryotic origin symbiosis substrate ASI R3 emergence fresh complement r74 r75 r76 r77 r78',
    'Vibrio fischeri bioluminescence quorum sensing luxR luxI autoinducer density-dependent communication substrate ASI R5 signaling fresh complement r74 r75 r76 r77 r78',
    'transposon McClintock 1951 Ac Ds Activator Dissociation maize mobile genetic element jumping gene substrate ASI R9 heredity fresh complement r74 r75 r76 r77 r78',
    'Gaia hypothesis Lovelock Margulis 1974 earth homeostasis biosphere feedback regulation daisyworld substrate ASI R12 ecology fresh complement r74 r75 r76 r77 r78',
    # ===== 3 GitHub deep =====
    'deepset-ai haystack github source pipeline RAG retrieval augmented generation agent real source deep dive substrate ASI central AI pluggable fresh',
    'SWE-agent SWE-bench github source autonomous code agent benchmark software engineering real source deep dive substrate ASI central AI pluggable fresh',
    'letta-ai letta memGPT github source memory management LLM agent self-editing context window hierarchical real source deep dive substrate ASI central AI pluggable fresh',
    # ===== 2 Gap =====
    'horizontal gene transfer HGT conjugation transformation transduction antibiotic resistance biofilm bacterial evolution substrate ASI R6 reproduction Gap complement r74 r75 r76 r77 r78',
    'heat shock protein HSP Hsp70 Hsp90 HSF1 chaperone stress response misfolded protein proteostasis substrate ASI R7 stress Gap complement r74 r75 r76 r77 r78',
]


def main():
    started = time.time()
    results = []
    print(f'Round-79 starting: {len(QUERIES)} queries', flush=True)
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
    print(f'\nRound-79 done in {total:.1f}s, saved {len(results)} entries to {OUT}', flush=True)
    print(f'Size: {OUT.stat().st_size} bytes', flush=True)

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}', flush=True)
    return total


if __name__ == '__main__':
    main()