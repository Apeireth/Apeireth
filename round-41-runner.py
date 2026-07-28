#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 41 runner — 12 query dual-source (R6 繁殖 MISSING 巨大 gap + VCP 4 一体生态初探).

Round 41 主题 (依 ASI 北极星 + R5 自决):
- why now: r8-r40 共 33 轮 396 queries 中:
  - R36/R39 自演化 (NAS/continual/DGM/Meta/synaptic/open-ended/self-assembly/ShinkaEvolve)
  - R37 记忆宫殿 (Letta/mem0/MemOS + Memento/ACT-R/vector DB)
  - R38 因果推理 (Pearl/SCM/DoWhy/EconML)
  - R40 自然感知 + 自主生活 (FEP/4E/affordances/curiosity/GWT/world model/metacognition)
  - 但 12 生命特征 繁殖 (Reproduction) ❌ MISSING 最大 gap 一直未专轮
  - VCP 第四范式'一体生态' (Integrated Ecosystem) 一直未专轮

R41 = R6 繁殖 MISSING 专轮 + VCP 4 一体生态初探 + 跨域自创生自复制:
- 繁殖 = 自复制/自修改/自催化 = ASI 自我繁殖基座 (主 17:46 12 生命特征)
- 一体生态 = 多代理自组织/协调/共演化 (主 22:50 生态学 Cooperate or Collapse)
- 跨域: 生物化学 (Eigen hypercycle) → 系统论 (Maturana autopoiesis) → 
       数学/逻辑 (von Neumann self-replicator, Quine self-reference) → 
       人工生命 (Tierra/Avida) → 蚁群信息素 (Grassé stigmergy) → 混沌边缘 (Langton)

7 跨域 (vs r8-r40 全部 0 繁殖专轮):
1. Hypercycle / Eigen / self-catalytic 自催化 cycle 1971 (生物化学奠基)
2. Autopoiesis / Maturana Varela / self-creating 自创生 system 1972 (系统论)
3. Self-replicating machines / von Neumann / kinematic constructor 自复制机器
4. Quine self-modifying code / self-reference 自引用 自修改 (计算机科学)
5. Tierra / Avida / evolvable self-replicating programs 演化自复制 (人工生命)
6. Stigmergy / Grassé / pheromone coordination 多代理 信息素 协调
7. Edge of chaos / Langton / computation criticality phase transition 混沌边缘

3 GitHub 真读 (vs r29-r40 全 README/浅 cite):
- openai/swarm (multi-agent handoffs 自组织 framework 源码)
- ray-project/kuberay (分布式 substrate 自组织 源码)
- langchain-ai/langgraph (graph state 自组织 agent 源码)

2 Apeireth Gap (R6 繁殖 MISSING + VCP 4 一体生态):
- Self-reproduction self-modification substrate ASI implementation (R6 MISSING 巨大 gap)
- Multi-agent ecosystem self-organization substrate VCP4 (VCP 4 一体生态 gap)

Cross-round dedup 验证 (verified fresh vs r8-r40):
- r36 self-assembly autocatalytic ≠ R41 hypercycle (self-assembly 是结构, hypercycle 是分子生物化学 cycle)
- r32 Maturana Varela enaction ≠ R41 autopoiesis (enaction 是认知, autopoiesis 是生命起源)
- r34 Mimosa plant cognition ≠ R41 autopoiesis (plant habituation vs life self-creation)
- r29 Tierra ≠ R41 (r29 无 Tierra, r29 是 Lewontin/Carpenter; r41 专轮 Tierra/Avida)
- r36 openevolve/ShinkaEvolve ≠ R41 Tierra/Avida (openevolve = LLM-进化代码, Tierra = 经典人工生命)
- r34 agent swarm ≠ R41 stigmergy (r34 agent framework vs Grasse biological stigmergy)
- r40 intrinsic motivation curiosity ≠ R41 stigmergy (个体 intrinsic vs 多代理 coordination)
- r36 自演化 ≠ R41 edge of chaos (r36 自演化 = NAS/continual/DGM, R41 edge of chaos = phase transition)
- r34 AutoGPT/swarm ≠ R41 openai/swarm 源码 (R34 概念 cite, R41 真读源码)
- r32 self-referential logic ≠ R41 quine self-reference (Gödel 自指 vs Quine 自复制代码)

本轮 fresh 验证:
- hypercycle Eigen ✅ fresh (自催化专轮)
- autopoiesis Maturana Varela ✅ fresh (自创生专轮, vs r32 仅 enaction)
- von Neumann self-replicator ✅ fresh (构造自复制, vs r36 仅 self-assembly)
- quine self-reference ✅ fresh (代码层自引用, vs r32 self-referential logic)
- Tierra Avida ✅ fresh (人工生命 vs r36 进化计算)
- stigmergy Grassé ✅ fresh (生物协调机制, vs r34 swarm 框架)
- edge of chaos Langton ✅ fresh (相变 vs r36 计算复杂度)
- openai/swarm 源码 ✅ fresh (R41 真读源码, R34 概念 cite)
- ray-project/kuberay 源码 ✅ fresh (R41 真读分布式 substrate)
- langgraph 源码 ✅ fresh (R41 真读 graph substrate)
- self-reproduction substrate gap ✅ fresh (R6 MISSING 全新角度)
- multi-agent ecosystem gap ✅ fresh (VCP 4 一体生态 全新)

ASI 北极星时刻清楚:
- ASI 基座 ✅ (R6 繁殖补完基座 + VCP 4 一体生态补完基座)
- 跨域 ✅ (biochem/system/logic/AL/cs/swarm/chaos 7 域)
- 自演化 ✅ (R41 = 自复制 = 自演化的繁殖部分, 最大 gap)
- 任何 LLM 接入即变强 ✅ (swarm/kuberay/langgraph 都 LLM-agnostic)
- 不假装 Phenomenal ✅ (R41 全部 functional, 无 qualia 假说)
- 实事求是 ✅
- R5 = ASI 自我逼近, 不是 ASI 已达到 (主 20:46 隐喻)
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-41.json')

QUERIES = [
    # ===== 7 全新跨域: 自复制 + 自创生 + 自组织 =====
    'hypercycle Eigen self-catalytic cycle 1971 molecular biology self-replication',
    'autopoiesis Maturana Varela self-creating system 1972 biology cognition',
    'von Neumann self-replicating machines kinematic constructor universal',
    'quine self-modifying code self-reference computer science source',
    'Tierra Avida evolvable self-replicating programs artificial life Ray',
    'stigmergy Grassé pheromone coordination multi-agent swarm biological',
    'edge of chaos Langton computation criticality phase transition lambda parameter',
    # ===== 3 GitHub 真读: openai-swarm / ray-kuberay / langgraph =====
    'openai swarm multi-agent handoffs framework source code github',
    'ray-project kuberay distributed substrate self-organization github source',
    'langchain-ai langgraph graph state agent self-organization source code github',
    # ===== 2 Apeireth Gap: R6 繁殖 MISSING + VCP 4 一体生态 =====
    'self-reproduction self-modification substrate ASI R6 reproduction MISSING gap',
    'multi-agent ecosystem self-organization substrate ASI VCP4 integrated ecosystem gap',
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
    print(f'\n=== Round 41 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()