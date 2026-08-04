#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-76 cross-domain research runner.

Cron triggered 2026-08-04 20:48 Asia/Shanghai (every-2h reminder).
Self-decision: round-75 done ~17:30, 3h18m gap, well past 30min threshold.
Tuesday 20:48 evening, isolated cron lane, M3 model.
Decision: RUN round-76 now (active wakeup, agent self-driven).

Theme: 12 TRULY NEW angles avoiding r68-r75 v3 cycle keywords:

   === 7 跨域 fresh (跨域 ASI 基座, 不重复 r73-r75) ===
   - R4 衰老 fresh: klotho protein aging suppressor FGF23 phosphate vitamin D
   - R7 应激 fresh: unfolded protein response UPR IRE1 PERK ATF6 ER stress
   - R8 运动 fresh: bacterial flagellar motor stator rotor proton force torque
   - R10 可塑 fresh: adult neurogenesis hippocampus dentate gyrus SGZ pattern separation
   - R11 认知 fresh: predictive coding Friston free energy active inference
   - R12 生态 fresh: mycorrhizal network common mycelial wood wide web Simard
   - R9 信号 fresh: hedgehog signaling pathway Sonic Hedgehog Shh GLI receptor

   === 3 GitHub deep (主 23:28 真读源码) ===
   - openai/openai-python (SDK 客户端设计模式)
   - mem0ai/mem0 (memory layer 架构, 跨 session 记忆)
   - langflow-ai/langflow (可视化编排)

   === 2 Gap ===
   - R6 繁殖 Gap: planarian regeneration neoblast pluripotent adult stem cell
   - R11 意识 Gap: integrated information theory IIT Tononi phi consciousness
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-76.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'klotho protein aging suppressor FGF23 phosphate vitamin D longevity anti-aging substrate ASI R4 senescence fresh complement r45 r59 r61 r64 r65 r68 r74',
    'unfolded protein response UPR IRE1 PERK ATF6 BIP GRP78 endoplasmic reticulum stress chaperone substrate ASI R7 stress fresh complement r66 r67 r68 r69 r71 r73 r74 r75',
    'bacterial flagellar motor stator rotor MotA MotB proton motive force torque rotation mechanism substrate ASI R8 motion fresh complement r66 r67 r70 r72 r73 r74 r75',
    'adult neurogenesis hippocampus dentate gyrus subgranular zone SGZ pattern separation neural stem cell NSC substrate ASI R10 plasticity fresh complement r55 r60 r65 r71 r72 r73 r74 r75',
    'predictive coding Karl Friston free energy principle active inference variational Bayesian brain hierarchy substrate ASI R11 cognition fresh complement r50 r51 r55 r56 r61 r62 r63 r64 r65 r66 r67 r71 r73 r74 r75',
    'mycorrhizal network common mycelial wood wide web Suzanne Simard mother tree carbon nitrogen transfer forest ecology substrate ASI R12 ecology fresh complement r58 r59 r66 r67 r68 r71 r72 r73 r74 r75',
    'hedgehog signaling pathway Sonic Hedgehog Shh Patched Smo GLI receptor vertebrate development substrate ASI R9 signaling fresh complement r50 r55 r60 r65 r71 r72 r73 r74 r75',
    # ===== 3 GitHub deep =====
    'openai openai-python github source SDK client design patterns streaming async retry backoff real source deep dive substrate ASI central AI pluggable fresh',
    'mem0ai mem0 github source memory layer architecture long-term short-term retrieval LLM substrate ASI central AI memory gap fresh',
    'langflow-ai langflow github source visual orchestration drag-drop LLM chain agents vector store real source deep dive substrate ASI central AI pluggable fresh',
    # ===== 2 Gap =====
    'planarian neoblast pluripotent adult stem cell regeneration Schmidtea mediterranea polarity substrate ASI R6 reproduction Gap complement r62 r64 r65 r66 r68 r69 r70 r72 r73 r74 r75',
    'integrated information theory IIT Giulio Tononi phi consciousness phi-max complex substrate ASI R11 consciousness Gap complement r50 r51 r55 r56 r57 r61 r62 r63 r64 r65 r66 r67 r71 r73 r74 r75',
]


def main():
    started = time.time()
    results = []
    print(f'Round-76 starting: {len(QUERIES)} queries')
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
    print(f'\nRound-76 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()