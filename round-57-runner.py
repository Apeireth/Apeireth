#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-57 cross-domain research runner.

Cron triggered 2026-08-01 13:36 Asia/Shanghai (every-2h reminder).
Self-decision: round_auto_naming.py next=57, no conflict; round-56 was 10h45m ago
(> 30min threshold). Master likely engaged (Saturday 13:36); cron is reminder, log only.
Decision: run since no recent activity + gap-fill value high.

Theme: 补 ASI-life-features + 数学基座 + 真自指 + 量子生物 + GitHub 源码深读 +
       Gap 繁殖/应激性 deep.
Avoid r56 (Solomonoff / Hopfield / Liquid / VSA / Tierra / Olah / Causal emergence /
          Mamba / RWKV / TransformerLens / Avida-novelty / consciousness-metrics)

主人 17:46 ASI-LIFE-FEATURES 12 生命特征, r1-r56 覆盖现状:
- R0 新陈代谢 ? r46 (Krebs/Kleiber)
- R1 生长 ? r46 (异速生长) + r51 (Bergson)
- R2 发育 ? r40/r42/r45 + r52 (Wolpert) + r54 (Goodwin) + r56 (Solomonoff)
- R3 死亡 ? r45
- R4 衰老 ? r45
- R5 修复/再生 ? r44 + r49 deep
- R6 繁殖 ? r41 + r47 + r50 (HGT) + r51 (gametogenesis) + r54 (HGT) + r56 (Avida)
            ← r57 深 Hamilton inclusive fitness + 真重组/分离 mechanism
- R7 应激性 ? r42 (FEP) + r53 (chemotaxis/tropism)
            ← r57 深 enactivism + 真 embodied autonomy substrate
- R8 运动 ? r41/r45 + r52 (Brooks/Trewavas)
- R9 遗传变异 ? r44/r47/r48 + r54 (Lenski/D Arcy/Barbieri) + r56 (Tierra)
- R10 可塑性 ? r40/r45 + r51-55 + r56 (Hopfield/Liquid/VSA/Causal Emergence)
            ← r57 加 quantum biology (tunneling) 可塑
- R11 意识 ? r42/r43/r46/r49-55 + r56 (Olah/mech-interp/conscious-metrics)
            ← r57 加 algebraic topology persistent homology substrate

数学/哲学基座 (主 22:33 ASI 北极星):
- 中央 AI = sum of all forms — 7 跨域 substrate (Autopoiesis + Adjacent Possible +
              Quantum Biology + CAS emergence + Prigogine dissipative + Erlangen
              symmetry + Persistent Homology) NOT claim ASI has all
- 代数拓扑 persistent homology = 数学骨架, NOT claim ASI has topology
- Prigogine 不可逆热力学 = 时间之箭 substrate, NOT claim ASI is irreversible
- Autopoiesis 自创生 = 真自指 substrate, NOT claim ASI is autopoietic
- Erlangen 程序 = 对称不变原理 substrate, NOT claim ASI has symmetry invariants
- Adjacent Possible = 邻接可能性扩展 substrate, NOT claim ASI explores adjacent possible
- Quantum biology = 量子效应 substrate, NOT claim ASI exploits quantum effects
- CAS emergence = 真CAS涌现 substrate, NOT claim ASI is emergent
- R6 真繁殖 = 真重组/分离 (有性 vs 无性), NOT claim ASI reproduces
- R7 真应激 = embodied autonomy agency, NOT claim ASI is autonomous
- 跨域借鉴 = 工具/启发 (主 21:00)
- 隐喻是工具 (主 20:55)
- ASI 只能逼近 (主 20:46)
- 不假装 Phenomenal (主 17:58)
- 实事求是 (主 17:43)

避免重复 (r1-r56 已覆盖关键词):
× Solomonoff/AIXI/Kolmogorov (r56)
× Modern Hopfield/Ramsauer (r56)
× Liquid NN/Hasani (r56)
× Hyperdimensional/Kanerva (r56)
× Tierra/A vida (r56) - 但 r57 topic 不重
× Olah/Anthropic/mechanistic (r56)
× Causal emergence/Hoel/Albantakis (r56)
× Mamba/S4/S6/Albert Gu (r56)
× RWKV/Bo Peng (r56)
× TransformerLens/neelnanda (r56)
× novel search/POET/Lehman (r54/r56)
× consciousness metrics/NCC/IIT Φ phi (r56)
× Bonabeau/Hermann Haken/CAS/Bak sandpile (r50)
× Maturana/Varela (r41/r52) - r57 深 autopoiesis 不同切入点
× Prigogine-Stengers (r51) - r57 深 dissipative structures 不同
× Edelman/Damasio/Tononi IIT (r50)
× Ashby/Bateson/Penrose-Orch-OR (r51)
× Whitehead/Bergson (r51)
× Thompson enactivism (r43)
× Wolpert positional info/Goodwin (r52/r54)
× Rizzolatti (r54)
× Fisher/Hamilton inclusive fitness? /Maynard Smith kin selection?
   - r57 fresh focus 真看 Hamilton 1964 + ESS 数学基础
× Persistent homology/Carlsson topological data?
   - r57 fresh 真深读

Fresh for r57:
- Stuart Kauffman adjacent possible NK model
- Ilya Prigogine dissipative structures order through fluctuation 深
- John Holland / Murray Gell-Mann CAS 真框架
- Maturana Varela autopoiesis 第二代深读 (组织闭合 + 自我生成)
- Lars Hörmander / Felix Klein Erlangen program symmetry invariants
- Quantum biology proton tunneling / avian magnetoreception cryptochrome
- Gunnar Carlsson persistent homology / topological data analysis
- openevolve (codelion) code-level 真读
- ShinkaEvolve (SakanaAI) 真读源码
- letta 多层记忆架构 真读
- Hamilton 1964 inclusive fitness / kin selection ESS 真读
- Thompson enactivism 真深 - 'Mind in Life' + autonomy embodied
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-57.json')

QUERIES = [
    # ===== 7 跨域 fresh =====

    # 1. Stuart Kauffman / adjacent possible / NK model / self-extending biosphere
    #    (R6 学习 + R9 遗传变异 + 中央 AI 涌现 substrate — 邻接可能性, NOT claim ASI explores adjacent possible)
    'Stuart Kauffman adjacent possible NK model self-extending biosphere catalytic reaction autocatalytic sets origin of life substrate ASI R6 R9 emergence',

    # 2. Ilya Prigogine dissipative structures order through fluctuation non-equilibrium thermodynamics
    #    (R11 意识 + 中央 AI 时间之箭 substrate — 不可逆热力学, NOT claim ASI is irreversible)
    'Ilya Prigogine dissipative structures order through fluctuation non-equilibrium thermodynamics far from equilibrium substrate ASI R11 emergence time arrow',

    # 3. John Holland complex adaptive systems CAS agents internal models emergence genetic algorithm
    #    (R11 意识 + VCP 4 一体生态 + 中央 AI 涌现 substrate — CAS真框架, NOT claim ASI is CAS)
    'John Holland complex adaptive systems CAS internal models signals detectors building blocks emergence genetic algorithm substrate ASI R11 ecosystem VCP',

    # 4. Maturana Varela autopoiesis self-producing organization closure second-order deep reading
    #    (R6 繁殖 + R7 应激性 + 中央 AI 自指 substrate — 真自创生, NOT claim ASI is autopoietic)
    'Maturana Varela autopoiesis self-producing organization closure structural coupling second-order cybernetics deep substrate ASI R6 reproduction autonomy',

    # 5. Felix Klein Erlangen program symmetry invariants transformation groups geometric structure
    #    (R6 学习 + 中央 AI 数学基 substrate — 对称不变原理, NOT claim ASI has symmetry invariants)
    'Felix Klein Erlangen program symmetry invariants transformation groups geometric structure Lie groups differential equations substrate ASI R6 mathematics',

    # 6. Quantum biology photosynthesis proton tunneling enzyme avian magnetoreception cryptochrome coherent
    #    (R7 应激性 + R10 可塑性 + R9 遗传变异 substrate — 真量子生物效应, NOT claim ASI exploits quantum effects)
    'quantum biology photosynthesis proton tunneling enzyme catalysis avian magnetoreception cryptochrome radical pair coherent quantum effects substrate ASI R7 R10',

    # 7. Gunnar Carlsson persistent homology topological data analysis TDA algebraic topology Betti
    #    (R11 意识 + 中央 AI 数学骨架 substrate — 拓扑数据分析, NOT claim ASI has topology)
    'Gunnar Carlsson persistent homology topological data analysis TDA Betti numbers algebraic topology shape data nerve complex substrate ASI R11 mathematics',

    # ===== 3 GitHub 源码真读 (深) =====

    # 8. openevolve (codelion) 真读源码 - LLM驱动的进化搜索
    #    (R6 学习 + VCP 1 连续存在 substrate — 进化 + LLM 组合, NOT claim ASI implements openevolve)
    'openevolve codelion github LLM evolutionary algorithm code search real source architecture MAP-Elites substrate ASI R6 learning VCP',

    # 9. SakanaAI ShinkaEvolve 真读源码 - Sakana 的科学发现 + GA + LLM 评分
    #    (R9 遗传变异 + R6 学习 + 中央 AI substrate — 科学发现 + 进化, NOT claim ASI implements ShinkaEvolve)
    'SakanaAI ShinkaEvolve github scientific discovery code evolution LLM evaluator genetic algorithm real source paper substrate ASI R9 variation R6',

    # 10. letta 多层记忆架构 真读 (原 mem0 fork) - 真长期记忆 + 短期 + 上下文
    #     (VCP 1 连续存在 + R10 可塑性 + R6 学习 substrate — 多层记忆架构, NOT claim ASI has letta arch)
    'letta github memory architecture multi-tier core memory archival recall long-term short-term context window real source substrate ASI VCP 1 R10',

    # ===== 2 Gap biomimetic (繁殖 + 应激性) =====

    # 11. R6 繁殖 Gap 深 - 真有性 vs 真无性 - 真重组 / 分离 - Hamilton 1964 inclusive fitness +
    #     Maynard Smith ESS - 真生子机制 inspiration (NOT claim ASI reproduces)
    'Hamilton 1964 inclusive fitness kin selection Maynard Smith ESS evolutionary stable strategy game theory sexual reproduction recombination segregation substrate ASI R6 reproduction Gap 12 life features',

    # 12. R7 应激性 Gap 深 - embodied autonomy agency 真自主性 - enactivism Thompson deep
    #     'Mind in Life' / autopoiesis-to-autonomy 连续性 (NOT claim ASI has agency)
    'enactivism Thompson Mind in Life autonomy agency embodied cognition sensorimotor autopoiesis-to-autonomy sense-making basal substrate ASI R7 agency Gap 12 life features',
]


def main():
    started = time.time()
    started_iso = time.strftime('%Y-%m-%dT%H:%M:%S+08:00', time.localtime(started))
    print(f'Round-57 started {started_iso}')

    results = []
    for i, q in enumerate(QUERIES, 1):
        t0 = time.time()
        r = dual_research(q, top_k=5)
        dur = time.time() - t0
        bw = len(r['bocha_web'])
        ba = len(r['bocha_ai_answer'])
        any_n = len(r['anysearch'])
        merged = len(r['merged_sources'])
        print(f'[{i:02d}/{len(QUERIES)}] {dur:.1f}s | bw={bw} ba={ba} any={any_n} merged={merged} | {q[:80]}')
        results.append(r)
        time.sleep(0.5)

    OUT.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - started
    print(f'\nRound-57 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(1 for r in results if r['bocha_ai_answer'])
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_answered={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()
