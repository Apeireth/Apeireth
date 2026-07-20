#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""V7 跨域调研 round-5 — 主人 22:22 '调研不停' (Phase 42+ 真生产新跨域).

Round-5 主题: 推进 V3 → V4, 集中于 *意识/感知/具身* 边界
(回扣 Phenomenal consciousness 终极目标 主人 17:58).

Round-1: ecology/cybernetics/博弈/语言/网络/keystone
Round-2: Nash/Lakoff/Watts/Luhmann/Lotka/Lévy/sleep/stigmergy/Hebbian/Varela/Thompson/edge-of-chaos
Round-3: IIT/全局工作空间/预测编码/镜像神经元/WBE/泛心论/qualia/SIT/吸引子/动物元认知/Libet/Varela Mature
Round-4: 神经调制/胶质细胞/三突触/时间相干性/高维流形/认知灵活/模拟论/dream/预测处理/储层/Kuramoto/临界分支
Round-5 (NEW): 自由能原理深度/Hebbian 4 法则/enactivism 真知觉/全局工作空间理论/具身 AI/4E 认知
             /生命-心智连续性/Bayesian brain 实证/分层预测/神经达尔文
"""
import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

QUERIES = [
    # === Phase 42 真生产方向: Bayesian brain / Free Energy Principle 深化 ===
    'free energy principle variational density encoding brain',
    'predictive coding hierarchical gaussian rao ballard 1999',
    'precision weighting neuromodulation attention gain',
    # === Phase 43 真生产方向: 4E 认知 (enactivism) ===
    '4E cognition embodied embedded enacted extended AI',
    'enactivism autopoiesis sensorimotor consciousness varela',
    # === Phase 44 真生产方向: Global Workspace Theory ===
    'global workspace theory consciousness baars 1988 evidence',
    'global neuronal workspace dehaene ignr 2021',
    # === Phase 45 真生产方向: 神经达尔文 + 突触选择 ===
    'neural darwinism edelman neuronal group selection',
    # === Phase 46 真生产方向: 生命-心智连续性 + 涌现意识 ===
    'mind life continuity tononi edelman consciousness evolution',
    # === Phase 47 真生产方向: Bayesian brain 实证 ===
    'bayesian brain hypothesis empirical evidence knill pouget',
    # === Phase 48 真生产方向: 真正知觉 (no hallucination) ===
    'veridical perception predictive processing predictive coding',
]

results = []
for q in QUERIES:
    try:
        r = dual_research(q, top_k=3)
        results.append(r)
        print(f'OK: {q[:60]}')
    except Exception as e:
        print(f'ERR: {q[:60]} -- {e}')
        results.append({'query': q, 'error': str(e)})

with open('research-v7-fifth-round.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'\nsaved {len(results)} V7 round-5 queries to research-v7-fifth-round.json')