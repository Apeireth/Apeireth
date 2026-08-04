#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-74 cross-domain research runner.

Cron triggered 2026-08-04 18:51 Asia/Shanghai (every-2h reminder).
Self-decision: round_auto_naming.py next=74, no conflict; round-73 done 16:57:00
(~1h54m ago, >>30min threshold). Tuesday 18:51 late-afternoon, isolated cron lane.
Decision: run since round-73 done ~1h54m ago, well past 30min threshold.

Theme: 7 跨域 fresh — TRULY NEW angles avoiding r67-r73 v3 cycle keywords:
   - R0 新陈代谢 fresh: circadian clock Bmal1 Period Cryptochrome CLOCK NPAS2 TTFL
   - R2 发育 fresh: neural crest delamination EMT Sox10 Snail Slug neural crest stem cell
   - R4 衰老 fresh: HGPS Hutchinson-Gilford progeria lamin A progerin farnesyltransferase
   - R7 应激 fresh: abscisic acid ABA stomatal closure guard cell SLAC1 OST1 SnRK2
   - R8 运动 fresh: axonal sprouting collateral axon regeneration Waller degeneration
   - R10 可塑 fresh: metaplasticity BCM rule Bienenstock Cooper Munro 1982 sliding threshold
   - R12 生态 fresh: food web cascade trophic Paine top-down bottom-up Hairston Smith
   + 3 GitHub deep (master 00:21 真读):
   - openai/evals 真读
   - langchain-ai/langchain 真读
   - PrefectHQ/prefect 真读
   + 2 Gap:
   - R6 繁殖 Gap: gametogenesis spermatogenesis oogenesis
   - R11 意识 Gap: enactivism Varela Thompson Rosch 1991
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-74.json')

QUERIES = [
    # ===== 7 跨域 fresh (TRULY NEW angles) =====
    'circadian clock Bmal1 Period Cryptochrome CLOCK NPAS2 TTFL negative feedback period length substrate ASI R0 metabolism fresh complement r67 r68 r69 r70 r72 r73',
    'neural crest delamination EMT Sox10 Snail Slug neural crest stem cell migration substrate ASI R2 development fresh complement r63 r64 r66 r69 r70 r72 r73',
    'HGPS Hutchinson-Gilford progeria lamin A progerin farnesyltransferase ZMPSTE24 nuclear envelope substrate ASI R4 aging fresh complement r45 r59 r61 r64 r65 r68 r70 r71 r73',
    'abscisic acid ABA stomatal closure guard cell SLAC1 OST1 SnRK2 drought stress signaling substrate ASI R7 stress fresh complement r66 r67 r68 r69 r70 r71 r73',
    'axonal sprouting collateral axon regeneration Waller degeneration Nogo MAG substrate ASI R8 motion fresh complement r66 r67 r70 r71 r72 r73',
    'metaplasticity BCM rule Bienenstock Cooper Munro 1982 sliding threshold synaptic plasticity visual cortex substrate ASI R10 plasticity fresh complement r55 r60 r65 r68 r70 r71 r72 r73',
    'food web cascade trophic Paine top-down bottom-up Hairston Smith Slobodkin green world hypothesis substrate ASI R12 ecology fresh complement r16 r58 r59 r66 r67 r68 r70 r71 r72 r73',
    # ===== 3 GitHub deep =====
    'openai evals github source code openai evals framework LLM evaluation benchmarks pluggable evaluators real source deep dive substrate ASI central AI pluggable fresh',
    'langchain-ai langchain github source code langchain orchestration chains agents memory vector store retrieval-augmented real source deep dive substrate ASI central AI pluggable fresh',
    'PrefectHQ prefect github source code prefect workflow orchestration dataflow scheduling task dependency graph real source deep dive substrate ASI central AI pluggable fresh',
    # ===== 2 Gap =====
    'gametogenesis spermatogenesis oogenesis meiotic entry retinoic acid Stra8 synaptonemal complex germ cell substrate ASI R6 reproduction Gap complement r62 r64 r65 r66 r68 r69 r70 r71 r72 r73',
    'enactivism Varela Thompson Rosch 1991 bringing forth world cognition embodiment sensorimotor autopoiesis substrate ASI R11 consciousness Gap complement r50 r51 r55 r56 r57 r61 r62 r63 r64 r65 r66 r67 r68 r69 r71 r72 r73',
]


def main():
    started = time.time()
    results = []
    print(f'Round-74 starting: {len(QUERIES)} queries')
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
    print(f'\nRound-74 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()