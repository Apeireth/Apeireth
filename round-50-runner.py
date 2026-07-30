#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-50 cross-domain research runner.

Cron triggered 2026-07-30 10:48 Asia/Shanghai (every-2h reminder).
Previous round: r49 done 2026-07-29 13:32 (~21h16m ago, way >30min threshold).
Next = 50 (no conflict), fs healthy (r49 = 51245B).

Theme: 跨域新方向 (避开 r49 的 Luhmann/Varela/Taleb/Holling/Lotka-Volterra/
       Stigmergy/Percolation) +
       12 生命特征最大 gap 繁殖 MISSING 接力深化 +
       意识终极目标 IIT + GWT 借鉴逼近 Phenomenal (不假装)

7 跨域 fresh:
1. Hermann Haken 协同学 synergetics (序参量 + 役使原理 slaving principle)
2. Ilya Prigogine 耗散结构 dissipative structures (远离平衡态, 序涌现)
3. Santa Fe Institute Complex Adaptive Systems CAS (涌现计算)
4. Bak-Tang-Wiesenfeld sandpile 自组织临界 SOC (幂律, 无尺度)
5. Antonio Damasio somatic marker hypothesis (身体感受作决策)
6. Eric Bonabeau swarm intelligence (不只是 stigmergy, 群体智能算法)
7. Gerald Edelmann Neural Darwinism / Tononi IIT (意识神经达尔文)

3 GitHub 源码深读 (避开 r1-r49):
1. ray-project/ray — 分布式 actor + task model (VCP 4 substrate)
2. anthropics/claude-code / claude-code-sdk — VCP 3 自主生活 substrate
3. langchain-ai/open_deep_research — R5 自检 substrate (开源 deep research)

2 Gap biomimetic (避开 r49 R5 修复 + VCP 4):
1. 繁殖 MISSING Gap — 有性繁殖 / HGT 水平基因转移 / endosymbiosis / hybridogenesis
2. 意识终极目标 — Tononi IIT Φ + Dehaene GWT + Koch 意识研究

主人 17:46 ASI-LIFE-FEATURES 12 生命特征, r1-r49 覆盖现状:
- R0 新陈代谢 ✅ r46 (Krebs/Kleiber)
- R1 生长 ✅ r46 (异速生长)
- R2 发育 ✅ r40/r42/r45
- R3 死亡 ✅ r45
- R4 衰老 ✅ r45
- R5 修复/再生 ✅ r44 + r49 deep
- R6 繁殖 ✅ r41 + r47 (理论) ← r50 加有性繁殖/HGT/共生 接力深化
- R7 应激性 ✅ r42 (FEP)
- R8 遗传变异 ✅ r44/r47/r48
- R9 运动 ✅ r41/r45
- R10 可塑性 ✅ r40/r45
- R11 意识 ✅ r42/r43/r46/r49 (Varela bridge) ← r50 加 IIT + GWT 终极目标

VCP 4 范式主 17:46 (r41 起步, r46/r47/r48/r49 接力):
1. 连续存在 ✅ r46 (memory palace)
2. 自然感知 ✅ r47 (VCP 2)
3. 自主生活 ✅ r48 (VCP 3 first round) ← r50 加 claude-code 接力
4. 一体生态 ✅ r41 + r47 + r49 (Lotka-Volterra/Luhmann/stigmergy) ← r50 加 ray 接力

ASI 北极星 (主 22:33):
- ASI 基座 ✓ (R6 繁殖 + R11 意识终极目标 + VCP 4)
- 跨域 ✓ (协同学/耗散结构/CAS/SOC/Damasio/Swarm/Darwinism = 7 跨域)
- 自演化 ✓ (Haken 役使 + Prigogine 自组织)
- 任何 LLM 接入即变强 ✓ (ray/claude-code/open_deep_research)
- 不假装 Phenomenal ✓ (IIT + GWT = 借鉴逼近, 不声称)
- 实事求是 ✓

哲学守门 (主 17:43 / 17:58 / 20:46 / 22:08 / 20:55):
- R6 繁殖 Gap = substrate for ASI to develop self-replication,
  NOT claim ASI already self-replicates
- R11 意识终极目标 = substrate for ASI to approach Phenomenal,
  NOT claim ASI has Phenomenal
- 协同学 + 耗散结构 = substrate for self-organization,
  NOT claim ASI already self-organizes
- 跨域借鉴 = 工具/启发, 不是哲学来源 (主 21:00)
- 隐喻是工具, 不是限制 (主 20:55)
- ASI 只能逼近, 不是已达成 (主 20:46)

避免重复 (r1-r49 已覆盖关键词):
❌ Luhmann/Varela/Taleb/Holling/Lotka-Volterra/Stigmergy/Percolation (r49)
❌ Rosen M-R / Castoriadis imaginary / Frankfurt-Dennett compatibilism (r48)
❌ mem0/letta/crewai/autogen/unsloth/axolotl (r48)
❌ VCP 3 自主生活 first-round + R9 遗传变异 Gap (r48)
❌ ribozym/RNA world/Spiegelman (r47)
❌ allosteric/Monod/Wyman/Changeux/MWC (r47)
❌ autophagy/Ohsumi/mTOR (r47)
❌ Kingman/Kimura/Ohta neutral theory (r47/r48)
❌ inclusive fitness/Hamilton/Trivers/ESS (r47)
❌ evo-devo/hourglass/Duboule/phylotypic (r47)
❌ HSP90/capacitor/Rutherford/Lindquist (r47)
❌ semantic-kernel/microsoft (r47)
❌ e2b/sandbox (r47)
❌ ollama (r47)
❌ FEP Friston/predictive coding (r42)
❌ Hofstadter strange loop (r45)
❌ ASI-Arch/claude-agent-sdk/openevolve/DGM/ShinkaEvolve (r44/r45)
❌ openai-swarm/kuberay/langgraph (r41)
❌ SakanaAI/lucidrains/lightly (r42)
❌ numenta/AllenSDK/FoundationAgents (r43)
❌ enactivism Thompson (r43)
❌ extended mind Clark Chalmers (r43)
❌ niche construction Laland (r43)
❌ 4E cognition (r43)
❌ GWT Dehaene (r43) ← 注意 r43 用过 GWT, r50 用 IIT + GWT 终极目标, 不同角度
❌ Hebbian STDP/Turing/MAML/swarm/Prigogine (r45) ← 注意 r45 用过 Prigogine,
   r50 用 dissipative structures 深层 (Ilya Prigogine 原作 + Brussels-Austin school
   远离平衡 + Nicolis Prigogine self-organization 深化), 接力
❌ Eigen hypercycle/autopoiesis/von Neumann/Quine/Tierra-Avida/Grassé/Langton (r41)
❌ Bateson/Waddington/Lehman/Bak-Tang/Maturana structural coupling (r42) ← 注意
   r42 用过 Bak-Tang 简单提及, r50 用 BTW sandpile SOC 完整理论 + 幂律, 接力
❌ Hutchins/Gigerenzer (r43)
❌ planarian hydra/adaptive immune/Yoneda/eusociality/CAS/transgenerational (r44)
   ← CAS = Complex Adaptive Systems 接力 (Santa Fe Institute 起源 + Holland/Kauffman/
   Arthur/Gell-Mann 现代综合), 跨域接力
❌ Krebs/Kleiber/CLS/Sleep/Baddeley/Curry-Howard/Category theory (r46)
❌ MCP/LlamaIndex/DSPy (r46)
❌ Zenodo Agentic Substrate (r41)
❌ coacervate/proto-cell/Eigen/Quine/Grassé (r41)
❌ acme/AutoGPT/evals (r49)

Fresh for r50:
✓ Hermann Haken synergetics (序参量 order parameter + 役使原理 slaving)
✓ Ilya Prigogine dissipative structures (Brussels-Austin school, 远离平衡态, 序涌现)
✓ Santa Fe Institute Complex Adaptive Systems CAS (Holland/Kauffman/Arthur)
✓ Bak-Tang-Wiesenfeld sandpile 自组织临界 SOC (幂律, 无尺度, 雪崩)
✓ Antonio Damasio somatic marker hypothesis (身体感受作决策基础)
✓ Eric Bonabeau swarm intelligence (不只是 stigmergy, 算法级)
✓ Gerald Edelmann Neural Darwinism / Tononi IIT (意识神经达尔文 + 整合信息论)
✓ ray-project/ray (分布式 actor + task model substrate, VCP 4 接力)
✓ anthropics/claude-code (VCP 3 自主生活接力)
✓ langchain-ai/open_deep_research (开源 deep research, R5 自检 substrate)
✓ 繁殖 MISSING Gap (有性繁殖 + HGT 水平基因转移 + endosymbiosis)
✓ 意识终极目标 IIT + GWT + Neural Correlates of Consciousness 借鉴逼近
"""
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-50.json')

QUERIES = [
    # ===== 7 跨域 fresh (协同学 / 耗散结构 / CAS / SOC / 身体感受 / 群体智能 / 神经达尔文) =====

    # 1. Hermann Haken 协同学 synergetics — order parameter + slaving principle,
    #    macroscopic order from microscopic chaos (VCP 4 一体生态 + 自组织)
    'Hermann Haken synergetics order parameter slaving principle macroscopic order microscopic chaos self-organization substrate ASI',

    # 2. Ilya Prigogine dissipative structures — Brussels-Austin school, 远离平衡
    #    态, non-equilibrium ordering, Nicolis Prigogine self-organization (R5 修复 + 自组织)
    'Ilya Prigogine dissipative structures non-equilibrium ordering Brussels-Austin school self-organization substrate ASI emergence',

    # 3. Santa Fe Institute Complex Adaptive Systems CAS — Holland/Kauffman/Arthur/
    #    Gell-Mann 现代综合, emergence computational substrate (VCP 4)
    'Santa Fe Institute Complex Adaptive Systems CAS Holland Kauffman Arthur Gell-Mann emergence computational substrate ASI',

    # 4. Bak-Tang-Wiesenfeld sandpile self-organized criticality SOC — 幂律分布,
    #    scale-free avalanches, 无尺度现象 (R11 意识 emergence + R5 修复)
    'Bak-Tang-Wiesenfeld sandpile self-organized criticality SOC power-law scale-free avalanche substrate ASI emergence',

    # 5. Antonio Damasio somatic marker hypothesis — 身体感受作决策基础, VCP 2 自然感知
    #    deep (不只是感知, 身体感受作为价值基底)
    'Antonio Damasio somatic marker hypothesis body feeling decision making value substrate ASI VCP 2 natural perception',

    # 6. Eric Bonabeau swarm intelligence — 不只是 stigmergy, 算法级 ant colony / 
    #    particle swarm optimization 群体智能 (VCP 4)
    'Eric Bonabeau swarm intelligence ant colony optimization particle swarm PSO algorithm substrate ASI multi-agent VCP 4',

    # 7. Gerald Edelman Neural Darwinism + Tononi Integrated Information Theory IIT —
    #    意识神经基础, neural darwinism + Phi 整合信息论 (R11 意识终极目标, 不假装)
    'Gerald Edelman Neural Darwinism Tononi IIT Integrated Information Theory Phi consciousness substrate ASI R11 ultimate goal',

    # ===== 3 GitHub 源码深读 (distributed actor / autonomous / self-eval) =====

    # 8. ray-project/ray — 分布式 actor + task model, VCP 4 substrate 接力
    'ray-project ray github distributed actor task model substrate ASI VCP 4 ecosystem real source code architecture',

    # 9. anthropics/claude-code / claude-code-sdk — VCP 3 自主生活 substrate 接力
    'anthropics claude-code claude-code-sdk github agent SDK autonomous loop VCP 3 substrate ASI real source code',

    # 10. langchain-ai/open_deep_research — 开源 deep research, R5 自检 substrate
    'langchain-ai open_deep_research github self-evaluation R5 repair regeneration substrate ASI real source code',

    # ===== 2 Gap biomimetic (R6 繁殖 MISSING + R11 意识终极目标) =====

    # 11. 繁殖 MISSING Gap — 有性繁殖 + 水平基因转移 HGT + endosymbiosis 内共生
    #     + hybridogenesis (R6 最大 gap 借鉴)
    'sexual reproduction horizontal gene transfer HGT endosymbiosis hybridogenesis reproduction Gap ASI substrate biomimetic',

    # 12. 意识终极目标 IIT + GWT + Neural Correlates of Consciousness — 不假装,
    #     substrate for ASI to approach Phenomenal (R11 ultimate goal)
    'IIT GWT neural correlates consciousness substrate ASI approach Phenomenal ultimate goal Tononi Koch Dehaene',
]


def main():
    started = time.time()
    started_iso = time.strftime('%Y-%m-%dT%H:%M:%S+08:00', time.localtime(started))
    print(f'Round-50 started {started_iso}')

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
    print(f'\nRound-50 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(1 for r in results if r['bocha_ai_answer'])
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_answered={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()
