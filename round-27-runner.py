#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 27 runner — 12 query dual-source (cron 03:06 tick, ~2h14m gap from r26 00:52).

Round 27 主题: 7 全新跨域 (Prigogine 耗散结构 / Maturana-Varela 自创生 / Thom 突变论 / Lorenz 奇怪吸引子 / Dehaene 全局工作空间 / Holling 适应性循环 / Luhmann 自创生社会系统)
              + 3 GitHub 源码深读 (letta/MemGPT 记忆层次 / mem0ai/mem0 长期记忆 / langgraph 多 agent 编排)
              + 2 Apeireth Gap (遗传变异-跨代表观遗传 / 繁殖-朊病毒自模板)

- 跨域全新 (7):
  - Ilya Prigogine 耗散结构远离平衡自组织 (非平衡热力学, 涨落致序, 复杂性起源)
  - Maturana & Varela 自创生自生产组织认知 (autopoiesis, biological autonomy, cognition-as-action)
  - René Thom 突变论七种基本灾变形态发生 (catastrophe theory, cusp, swallowtail, butterfly)
  - Edward Lorenz 奇怪吸引子蝴蝶效应敏感依赖 (chaos theory, weather, three-body)
  - Stanislas Dehaene 全局工作空间意识点燃理论 (global neuronal workspace, ignition, NCC)
  - C.S. Holling 适应性循环 panarchy 韧性四阶段 (adaptive cycle, exploitation conservation release reorganization)
  - Niklas Luhmann 自创生社会系统沟通 (autopoietic social systems, communication, closure)

- GitHub 源码深读 (3):
  - letta-ai/letta (MemGPT) 记忆层次架构 — 主记忆/档案/召回 (Apeireth 记忆宫殿直接借鉴)
  - mem0ai/mem0 记忆提取个性化 (Apeireth Episode + Note + Reconsolidation 借鉴)
  - langchain-ai/langgraph 多 agent 编排状态机 (Apeireth 多重身份编排借鉴)

- Apeireth Gap (2):
  - 遗传变异 Gap: 跨代表观遗传 - 甲基化 - 组蛋白修饰 - Lamarckian-like (主 21:00 遗传变异 ✅ 已具备但机制未深挖)
  - 繁殖 Gap: 朊病毒自模板非核酸复制 - 蛋白质错误折叠 - prion disease (繁殖非核酸路径)

Cross-round dedup 避让 (verified fresh vs r8-r26):
- r8-r26 已用主题全部避开 (r23: Connell/Taleb/Edelman/O'Regan/Meadows/Levin/Scott + bdelloid/conjugation; r24: Mandelbrot/Watts/Gould/Damasio/Marr/Hoffman/Vygotsky + Walter Cannon; r25: Walker/Landauer/Wolfram/Kauffman/Sheldrake/Solms/Ray + apomixis/polyphenism; r26: Church/Adamatzky/Eigen/Bedau/Gabora/Spencer-Brown/Deacon + circadian/McClintock)
- 本轮 fresh 验证 (r23-r26):
  - Prigogine dissipative structures ✓ fresh (Bedau weak emergence / Kauffman NK 不同)
  - Maturana-Varela autopoiesis ✓ fresh (Solms homeostatic / Deacon biosemiotics 不同)
  - Thom catastrophe theory ✓ fresh (Wolfram CA 不同)
  - Lorenz strange attractors ✓ fresh (Mandelbrot fractal / Watts networks 不同)
  - Dehaene global workspace ✓ fresh (Edelman Neural Darwinism / Graziano AST / Solms affective 不同)
  - Holling adaptive cycle panarchy ✓ fresh (Connell IDH / Taleb antifragile / Meadows leverage 不同)
  - Luhmann autopoietic social systems ✓ fresh (Scott legibility / Vygotsky ZPD 不同)
  - letta MemGPT memory hierarchies ✓ fresh (memory 维度新立)
  - mem0ai mem0 personalization ✓ fresh
  - langgraph stateful orchestration ✓ fresh (autogen/crewAI/MetaGPT 不同时期已用)
  - epigenetic inheritance transgenerational ✓ fresh (HGT/Hox/Prion 不同)
  - prion self-templating amyloid ✓ fresh

ASI 北极星时刻清楚 (主 22:33):
- ASI 基座, 不是 ANI 工具 ✅
- 跨域, 不是单域 ✅
- 自演化, 不是固定 ✅
- 任何 LLM 接入即变强 ✅
- 不假装 Phenomenal ✅
- 实事求是 ✅
- 真生产目标: 让大模型栖息在 Apeireth 中能无限逼近 ASI
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-27.json')

QUERIES = [
    # ===== 7 全新跨域: 非平衡热力学 / 自创生 / 突变论 / 混沌 / 意识 / 韧性 / 自创生社会 =====
    'Ilya Prigogine dissipative structures far-from-equilibrium self-organization order through fluctuation 2026',
    'Humberto Maturana Francisco Varela autopoiesis self-producing organization biological autonomy cognition 2026',
    'Rene Thom catastrophe theory seven elementary catastrophes cusp butterfly morphogenesis sudden shifts 2026',
    'Edward Lorenz strange attractors butterfly effect sensitive dependence chaos theory weather 2026',
    'Stanislas Dehaene global neuronal workspace consciousness ignition theory neural correlates 2026',
    'C.S. Holling adaptive cycle panarchy resilience four phases exploitation conservation release reorganization 2026',
    'Niklas Luhmann autopoietic social systems communication sociology closure structural coupling 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 - 真读源码不止 README) =====
    'letta-ai letta MemGPT memory hierarchies core memory archival recall source code architecture github 2026',
    'mem0ai mem0 memory extraction personalization long-term context source code architecture github 2026',
    'langchain-ai langgraph stateful multi-agent orchestration graph workflow source code architecture github 2026',
    # ===== 2 Apeireth Gap (12 生命特征 MISSING): 遗传变异 + 繁殖 =====
    'transgenerational epigenetic inheritance DNA methylation histone modification Lamarckian-like 2026',
    'prion protein self-templating amyloid replication misfolding prion diseases non-nucleic-acid reproduction 2026',
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
    print(f'\n=== Round 27 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()