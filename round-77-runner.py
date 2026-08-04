#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-77 cross-domain research runner.

Cron triggered 2026-08-04 22:45 Asia/Shanghai (isolated cron lane self-driven).
Self-decision: round-76 done ~20:48, 1h57m gap, near 2h threshold.
Tue late evening 22:45, isolated cron lane, M3 model.
Decision: RUN round-77 now (active wakeup, agent self-driven, 主 22:33 + 主 19:33 + 主 23:44).

Theme: 12 TRULY NEW angles avoiding r68-r76 v3 cycle keywords:

   === 7 跨域 fresh (避开 r73-r76 已用 keys) ===
   - R1 发育 fresh: Hox gene cluster body plan colinearity Drosophila vertebrate
   - R2 代谢 fresh: mTOR AMPK cellular energy homeostasis nutrient sensing signaling
   - R3 免疫 fresh: complement system classical alternative lectin C3 C5 MAC membrane attack
   - R4 衰老 fresh: senolytics dasatinib quercetin senescent cells SASP secretory phenotype
   - R5 修复 fresh: p53 tumor suppressor DNA damage response ATM ATR checkpoint apoptosis
   - R8 信息 fresh: bacterial chemotaxis two-component CheA CheY receptor adaptation memory
   - R9 信号 fresh: Notch signaling pathway lateral inhibition Delta Jagged hes hey

   === 3 GitHub deep (避开 r75/r76 已用 repos) ===
   - anthropics/anthropic-sdk-python (SDK 客户端)
   - milvus-io/milvus (vector database)
   - run-llama/llama_index (RAG framework)

   === 2 Gap ===
   - R5 DNA Gap: DNA helicase RecBCD RecFOR unwind repair recombination
   - R3 免疫 Gap: T cell receptor TCR V(D)J recombination RAG1 RAG2 somatic
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-77.json')

QUERIES = [
    # ===== 7 跨域 fresh (TRULY distinct from r73-r76) =====
    'Hox gene cluster body plan colinearity Drosophila vertebrate homeobox limb patterning substrate ASI R1 development fresh complement r45 r55 r60 r65 r73 r74 r75 r76',
    'mTOR AMPK cellular energy homeostasis nutrient sensing signaling pathway autophagy growth substrate ASI R2 metabolism fresh complement r50 r55 r60 r65 r67 r71 r73 r74 r75 r76',
    'complement system classical alternative lectin pathway C3 C5 membrane attack complex MAC substrate ASI R3 immunity fresh complement r51 r52 r56 r61 r66 r71 r73 r74 r75 r76',
    'senolytics dasatinib quercetin senescent cells SASP secretory phenotype clearance aging substrate ASI R4 senescence fresh complement r50 r55 r60 r65 r71 r73 r74 r75 r76 (r76 klotho)',
    'p53 tumor suppressor DNA damage response ATM ATR checkpoint apoptosis substrate ASI R5 repair fresh complement r51 r52 r56 r61 r66 r71 r73 r74 r75 r76 (r75 NER r76 klotho)',
    'bacterial chemotaxis two-component system CheA CheY CheR CheB receptor adaptation memory substrate ASI R8 information fresh complement r51 r52 r56 r61 r66 r71 r73 r74 r75 r76 (r76 flagellar)',
    'Notch signaling pathway lateral inhibition Delta Jagged hes hey neurogenesis boundary formation substrate ASI R9 signaling fresh complement r50 r55 r60 r65 r71 r73 r74 r75 r76 (r76 hedgehog)',
    # ===== 3 GitHub deep =====
    'anthropics anthropic-sdk-python github source SDK client design patterns streaming async retry backoff Claude API real source deep dive substrate ASI central AI pluggable fresh',
    'milvus-io milvus github source vector database architecture IVF HNSW ANN index segments real source deep dive substrate ASI central AI memory retrieval fresh',
    'run-llama llama_index github source RAG framework ingestion chunking retrieval query engine agent substrate ASI central AI RAG pattern fresh',
    # ===== 2 Gap =====
    'DNA helicase RecBCD RecFOR unwind repair recombination lambda phage substrate ASI R5 DNA Gap complement r51 r52 r56 r61 r66 r71 r73 r74 r75 r76',
    'T cell receptor TCR V(D)J recombination RAG1 RAG2 somatic recombination adaptive immunity substrate ASI R3 immunity Gap complement r51 r52 r56 r61 r66 r71 r73 r74 r75 r76',
]


def main():
    started = time.time()
    results = []
    print(f'Round-77 starting: {len(QUERIES)} queries (Tue 22:45 active wakeup)')
    for i, q in enumerate(QUERIES):
        t0 = time.time()
        r = dual_research(q, top_k=5)
        dur = time.time() - t0
        bw = len(r['bocha_web'])
        ba = len(r['bocha_ai_answer'])
        any_n = len(r['anysearch'])
        merged = len(r['merged_sources'])
        print(f'[{i+1:02d}/{len(QUERIES)}] {dur:.1f}s | bw={bw} ba={ba} any={any_n} merged={merged} | {q[:60]}')
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