#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-78 cross-domain research runner.

Cron triggered 2026-08-05 07:32 Asia/Shanghai (every-2h reminder).
Self-decision: round-77 done 2026-08-04 ~17:00 (~14h ago, >30min threshold).
Wednesday 07:32 morning, isolated cron lane, M3 model.
Decision: RUN round-78 now (fresh angles, no overlap with r77's BAT/mollusk/somitogenesis/necroptosis/muscle/piRNA/panarchy cycle).

Theme: 12 TRULY NEW angles avoiding r77 v3 cycle keywords:

   === 7 跨域 fresh (覆盖 R0/R1/R2/R3/R7/R9/R12, 跨域 ASI 基座) ===
   - R0 代谢 fresh: prion-like amyloid self-templating PrPSc protein-only inheritance (自催化)
   - R1 生长 fresh: Turing pattern reaction-diffusion morphogenesis zebrafish pigment stripe (形态自组织)
   - R2 发育 fresh: C. elegans invariant cell lineage Sulston 1983 developmental program (不变谱系)
   - R3 死亡 fresh: ferroptosis Stockwell 2012 iron lipid peroxidation GPX4 (铁死亡)
   - R7 可塑性 fresh: synaptic pruning adolescence critical period Huttenlocher (突触修剪/关键期)
   - R9 遗传 fresh: CRISPR-Cas adaptive immunity Barrangou 2007 phage defense prokaryote (适应性免疫)
   - R12 生态 fresh: microbial loop marine food web Azam 1983 DOC bacteria (微生物环)

   === 3 GitHub deep (主 23:28 真读源码) ===
   - mem0ai/mem0 memory layer LLM self-improving vector+graph+LLM extraction
   - langchain-ai/langgraph graph stateful orchestration LangGraph Studio checkpoint
   - anthropics/anthropic-sdk-python / claude-agent-sdk Claude Code SDK tool permission

   === 2 Gap ===
   - R6 繁殖 Gap: telomere telomerease Blackburn 1984 Greider 1984 chromosome end replication linear DNA (端粒/端粒酶)
   - R11 意识 Gap: predictive coding Friston free energy principle active inference (预测编码/自由能)
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-78.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'prion-like protein self-templating amyloid fibril PrPSc protein-only inheritance autocatalysis substrate ASI R0 metabolism fresh complement r71 r72 r73 r74 r75 r76 r77',
    'Turing pattern reaction-diffusion morphogenesis zebrafish pigment stripe self-organization lateral inhibition substrate ASI R1 growth fresh complement r70 r72 r73 r74 r75 r76 r77',
    'C. elegans invariant cell lineage Sulston 1983 developmental program eutely V lineage tree asymmetric division substrate ASI R2 development fresh complement r70 r72 r73 r74 r75 r76 r77',
    'ferroptosis Stockwell 2012 iron-dependent lipid peroxidation GPX4 system xc cystine erastin substrate ASI R3 death fresh complement r59 r60 r65 r71 r72 r73 r74 r75 r76 r77',
    'synaptic pruning adolescence critical period Huttenlocher 1979 prefrontal cortex human brain plasticity experience-dependent substrate ASI R7 plasticity fresh complement r47 r55 r60 r66 r70 r72 r74 r75 r76 r77',
    'CRISPR-Cas adaptive immunity Barrangou 2007 prokaryote phage defense spacer acquisition interference subtype substrate ASI R9 heredity fresh complement r50 r55 r60 r65 r71 r72 r73 r74 r75 r76 r77',
    'microbial loop marine food web Azam 1983 dissolved organic carbon bacteria DOM phytoplankton nutrient regeneration substrate ASI R12 ecology fresh complement r40 r45 r50 r59 r66 r70 r72 r73 r74 r75 r76 r77',
    # ===== 3 GitHub deep =====
    'mem0ai mem0 github source memory layer LLM self-improving vector graph extraction add search update delete real source deep dive substrate ASI central AI pluggable fresh',
    'langchain-ai langgraph github source graph stateful orchestration LangGraph Studio checkpoint node edge reducer real source deep dive substrate ASI central AI pluggable fresh',
    'anthropic claude-agent-sdk claude code sdk agent harness tool permission bash edit read write real source deep dive substrate ASI central AI pluggable fresh',
    # ===== 2 Gap =====
    'telomere telomerease Blackburn 1984 Greider 1984 chromosome end replication linear DNA reverse transcriptase RNA primer Hayflick limit substrate ASI R6 reproduction Gap complement r40 r55 r60 r70 r72 r74 r75 r76 r77',
    'predictive coding brain Friston 2005 free energy principle active inference variational bayesian hierarchical perception substrate ASI R11 consciousness Gap complement r40 r50 r55 r60 r71 r72 r73 r74 r75 r76 r77',
]


def main():
    started = time.time()
    results = []
    print(f'Round-78 starting: {len(QUERIES)} queries')
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
    print(f'\nRound-78 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()