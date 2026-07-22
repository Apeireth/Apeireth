#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 38 runner — 12 query dual-source (R4-RES-03, 调研专家, 依 R3 推荐 C1 主题).

Round 38 主题 (依 R3 reports/r3-research-round-37.md R38 建议, C1 因果推断):
- why now: 29 轮 R8-R36 共 344 queries 中无 do-calculus/Pearl/causal inference 关键词 (脚本扫验证);
  与 R17/R28 Friston 主动推断虽近但仅 Bayesian 变分框架,非实际因果 (Pearl ladder L1-L3);
  V1076 真 LLM 路由当前决策基于 cost/latency/policy, 无因果信号。
  C1 是 R1 备选, R3 转正 (R37 C3 记忆已落)。

7 跨域 (vs r8-r37 全 0 专轮):
- Pearl do-calculus causal inference LLM agent (Pearl ladder L1-L3 真正生产级)
- Causal Bayesian network CBN production agent decision 生产级贝叶斯网络
- Counterfactual reasoning agent long-horizon decision planning 反事实推理
- Structural causal models SCM AI agent production 因果结构模型
- Causal discovery neuroscience agent substrate 神经科学因果发现
- Pearl causal ladder L1 L2 L3 association intervention counterfactual 三层因果阶梯
- Actual causation Hall-Winston production LLM agent audit 实际因果归属

3 GitHub 真读 (vs r8-r37 全未深读):
- py-why/dowhy (DoWhy end-to-end causal inference, MS Research 主导)
- causal-ml/ananke (causal inference library, graph-based)
- microsoft/EconML (causal ML heterogeneous treatment effects, ATE/CATE)

2 Apeireth Gap (直喂 V1076 真 LLM 路由 + V1082 codebase audit):
- LLM agent counterfactual reasoning hallucination production audit 反事实幻觉
- Causal attribution production agent decision audit transparency 因果归因透明

Cross-round dedup 验证 (verified fresh vs r8-r37):
- r17/r28 Friston 主动推断 = Bayesian 变分框架 ≠ Pearl do-calculus 实际因果
- r14 Ashby 必要多样性 = 控制论 ≠ 因果推断
- r22 Brian Arthur increasing returns ≠ causal discovery
- r23 Meadows Leverage Points ≠ structural causal models
- r33 Hutchins distributed cognition ≠ counterfactual reasoning
- r36 Tulving episodic memory ≠ causal memory (完全不同)
- r37 memory 子工程 ≠ causal inference (C3 vs C1 不同主题)
本轮 fresh 验证:
- Pearl do-calculus ✅ fresh
- Causal Bayesian network ✅ fresh
- Counterfactual reasoning ✅ fresh
- SCM structural causal models ✅ fresh
- Causal discovery ✅ fresh
- Pearl ladder L1-L3 ✅ fresh
- Actual causation Hall-Winston ✅ fresh
- py-why/dowhy ✅ fresh
- causal-ml/ananke ✅ fresh
- microsoft/EconML ✅ fresh
- LLM hallucination 反事实 ✅ fresh (Gap 全新)
- 因果归因透明 ✅ fresh (Gap 全新)

ASI 北极星时刻清楚:
- ASI 基座 ✅ (V1076 真 LLM 路由 substrate)
- 跨域 ✅ (causal/cog-neuro/ML/decision/audit, 7 域)
- 自演化 ✅ (anysearch)
- 任何 LLM 接入即变强 ✅ (causal layer 通用)
- 不假装 Phenomenal ✅
- 实事求是 ✅
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-38.json')

QUERIES = [
    # ===== 7 全新跨域: Pearl / CBN / Counterfactual / SCM / Causal discovery / Pearl ladder / Actual causation =====
    'Pearl do-calculus causal inference LLM agent production 2026',
    'causal Bayesian network CBN production agent decision reasoning 2026',
    'counterfactual reasoning agent long-horizon decision planning production 2026',
    'structural causal models SCM AI agent production causal graph 2026',
    'causal discovery neuroscience agent substrate brain network 2026',
    'Pearl causal ladder L1 L2 L3 association intervention counterfactual hierarchy 2026',
    'actual causation Hall Winston production LLM agent audit attribution 2026',
    # ===== 3 GitHub 真读: DoWhy / ananke / EconML =====
    'py-why dowhy DoWhy end-to-end causal inference github source code Microsoft 2026',
    'causal-ml ananke causal inference library github source code graph 2026',
    'microsoft EconML causal ML heterogeneous treatment effects github source code 2026',
    # ===== 2 Apeireth Gap: 反事实幻觉 + 因果归因透明 =====
    'LLM agent counterfactual reasoning hallucination production audit 2026',
    'causal attribution production agent decision audit transparency LLM 2026',
]


def main():
    started = time.time()
    results = []
    for i, q in enumerate(QUERIES, 1):
        t0 = time.time()
        r = dual_research(q, top_k=5)
        dt = time.time() - t0
        results.append(r)
        n_web = len(r['bocha_web'])
        n_any = len(r['anysearch'])
        n_merge = len(r['merged_sources'])
        ai_chars = len(r['bocha_ai_answer'])
        print(f'[{i:2d}/12] ({dt:.1f}s) {q[:70]}')
        print(f'        bocha_web={n_web} anysearch={n_any} merged={n_merge} ai={ai_chars}')
        if r['bocha_ai_answer']:
            print(f'        ai_preview: {r["bocha_ai_answer"][:160]}')
        sys.stdout.flush()

    OUT.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - started
    print(f'\n=== Round 38 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()