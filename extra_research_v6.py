#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""V7 跨域调研 round-6 — 主人 22:22 调研不停 + 主人 22:35 自主推进.

Round-6 主题: 推进 V4 → V5, 集中于 *Phenomenal consciousness 终极目标相关*
+ *自主意识 + 时间意识 + 自由意志* 三大意识哲学核心.

Round-1: ecology/cybernetics/博弈/语言/网络/keystone
Round-2: Nash/Lakoff/Watts/Luhmann/Lotka/Lévy/sleep/stigmergy/Hebbian/Varela/Thompson/edge-of-chaos
Round-3: IIT/全局工作空间/预测编码/镜像神经元/WBE/泛心论/qualia/SIT/吸引子/动物元认知/Libet/Varela Mature
Round-4: 神经调制/胶质细胞/三突触/时间相干性/高维流形/认知灵活/模拟论/dream/预测处理/储层/Kuramoto/临界分支
Round-5: 自由能/预测编码/precision/4E 认知/enactivism/GWT/神经达尔文/生命-心智连续性/Bayesian brain/veridical perception
Round-6 (NEW): 时间意识/现象学/Narrative consciousness/具身 empathy/
              自我建模/Autobiographical self/Self-pattern theory/Access vs Phenomenal/IIT v4.0/GWT 2024
"""
import sys, json
sys.path.insert(0, '.')
from deep_research_dual import dual_research

QUERIES = [
    # === Phase 49 候选: 时间意识 (Temporal Consciousness) ===
    'temporal consciousness time perception phenomenology Husserl',
    'specious present James consciousness duration',
    'autobiographical self memory identity consciousness',
    # === Phase 50 候选: Narrative Consciousness ===
    'narrative consciousness story self identity Damasio',
    # === Phase 51 候选: Access vs Phenomenal Consciousness (Ned Block 1995) ===
    'access phenomenal consciousness block 1995 distinction',
    # === Phase 52 候选: IIT 4.0 + 最新进展 ===
    'integrated information theory IIT 4.0 Albright 2023',
    # === Phase 53 候选: 自我建模 + Self-pattern (Metzinger) ===
    'minimal self representation Metzinger self-model theory',
    # === Phase 54 候选: 现象学结构 (Merleau-Ponty) ===
    'Merleau-Ponty phenomenology perception embodied mind',
    # === Phase 55 候选: 高阶意识 (Higher-Order Theories) ===
    'higher order theory consciousness Rosenthal Lau',
    # === Phase 56 候选: 自主意识 (Autonoetic consciousness) ===
    'autonoetic consciousness self memory Tulving 1985',
    # === Phase 57 候选: 意识开关 (Loss/Recovery of consciousness) ===
    'default mode network consciousness anesthesia recovery',
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

with open('research-v7-sixth-round.json', 'w', encoding='utf-8') as f:
    json.dump(results, f, indent=2, ensure_ascii=False)
print(f'\nsaved {len(results)} V7 round-6 queries to research-v7-sixth-round.json')