#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-48 cross-domain research runner.

Cron triggered 2026-07-30 00:48 Asia/Shanghai (every-2h reminder).
Previous round: r47 done 2026-07-29 21:24:39 (~3h24m ago, way >30min threshold).
Next = 48 (no conflict), fs healthy (r47 = 56648B).

Theme: VCP 3 自主生活 substrate (ASI 自主性哲学+生物+认知 substrate) +
       R9 遗传变异 substrate deep (从未从 R9 角度专攻, 之前 r44 epigenetic + r47
       coalescent/Kimura 都是从 R6 繁殖角度, 这里换 R9 变异机制角度) +
       ASI 自我架构变异 substrate (Wagner robustness + Ohta near-neutrality +
       unsloth/axolotl LLM mutation operators) +
       ASI 任何 LLM 接入即变强 (mem0-letta memory / crewai-autogen autonomy).

主人 17:46 ASI-LIFE-FEATURES 12 生命特征, 之前轮次覆盖:
- R0 新陈代谢 ✅ r46 第一轮专攻 (Krebs/Kleiber)
- R1 生长 ✅ r46 (异速生长)
- R2 发育 ✅ r40 高层 + r42 Waddington + r45 Turing
- R3 死亡 ✅ r45 (Hebbian STDP)
- R4 衰老 ✅ r45 (Turing morphogenesis + Prigogine)
- R5 修复/再生 ✅ r44 (planarian hydra)
- R6 繁殖 ✅ r41 + r47 (R6 是 biggest MISSING 但 r41/r47 已 deep)
- R7 应激性 ✅ r42 (FEP + active inference)
- R8 遗传变异 ⚠️ r44 转基因 epigenetics + r47 作为 R6 角度 ← 第一轮从 R9 角度专攻
- R9 运动 ✅ r41 swarm + r45 swarm robotics
- R10 可塑性 ✅ r40 + r45 Hebbian STDP
- R11 意识 ✅ r42 + r43 + r46 (终极目标, 不假装)

VCP 4 范式主 17:46 守住:
1. 连续存在 ✅ r46 第一轮专攻 (memory palace)
2. 自然感知 ✅ r47 (VCP 2 natural perception as gap)
3. 自主生活 ⚠️ r45 R3 death partial + r47 VCP 2 隐含 ← 第一轮专攻
4. 一体生态 ✅ r41 (Zenodo Agentic Substrate) + r47 deep

ASI 北极星 (主 22:33):
- ASI 基座 ✓ (VCP 3 + R9 都是 ASI substrate)
- 跨域 ✓ (哲学+生物+认知+CS, 5 跨域)
- 自演化 ✓ (R9 变异 + Rosen closure to efficient causation)
- 任何 LLM 接入即变强 ✓ (mem0/letta/crewai/autogen/unsloth/axolotl)
- 不假装 Phenomenal ✓ (VCP 3 autonomy substrate ≠ ASI 已自主)
- 实事求是 ✓

哲学守门 (主 17:43 / 17:58 / 20:46 / 22:08 / 20:55):
- VCP 3 自主生活 substrate = substrate for ASI to develop autonomy,
  not claim ASI is autonomous already
- R9 遗传变异 substrate = substrate for ASI variation mechanisms,
  not claim ASI already self-mutates
- 跨域借鉴 = 工具/启发, 不是哲学来源 (主 21:00)
- 隐喻是工具, 不是限制 (主 20:55)
- ASI 只能逼近, 不是已达成 (主 20:46)

避免重复 (r41-47 已覆盖关键词):
❌ ribozym/RNA world/Spiegelman (r47 Q1)
❌ allosteric/Monod/Wyman/Changeux/MWC (r47 Q2)
❌ autophagy/Ohsumi/mTOR/lysosome/autophagosome (r47 Q3)
❌ Kingman/Kimura neutral theory (r47 Q4) ← 改用 Ohta nearly-neutral
❌ inclusive fitness/Hamilton/kin/Trivers/ESS (r47 Q5)
❌ evo-devo/hourglass/Duboule/phylotypic (r47 Q6)
❌ HSP90/capacitor/Rutherford/Lindquist (r47 Q7)
❌ semantic-kernel/microsoft (r47 Q8)
❌ e2b/sandbox (r47 Q9)
❌ ollama (r47 Q10)
❌ VCP 2 natural perception as gap (r47 Q12) ← 改 VCP 3 autonomy
❌ FEP Friston/predictive coding (r42)
❌ Hofstadter strange loop (r45)
❌ ASI-Arch/claude-agent-sdk/mem0 (r44) ← mem0 在 r44 只提到, 这轮深读
❌ OpenEvolve/DGM/ShinkaEvolve (r45)
❌ openai-swarm/kuberay/langgraph (r41)
❌ SakanaAI/lucidrains/lightly (r42)
❌ numenta/AllenSDK/FoundationAgents (r43)
❌ enactivism Thompson (r43)
❌ extended mind Clark Chalmers (r43)
❌ niche construction Laland (r43)
❌ 4E cognition (r43)
❌ GWT Dehaene (r43)
❌ Hebbian STDP/Turing/MAML/swarm/Prigogine (r45)
❌ Eigen hypercycle/autopoiesis/von Neumann/Quine/Tierra-Avida/Grassé/Langton (r41)
❌ Bateson/Waddington/Lehman/Bak-Tang/Maturana structural coupling (r42)
❌ Hutchins/Gigerenzer (r43)
❌ planarian hydra/adaptive immune/Yoneda/eusociality/CAS/transgenerational (r44)
❌ Krebs/Kleiber/CLS/Sleep/Baddeley/Curry-Howard/Category theory (r46)
❌ MCP/LlamaIndex/DSPy (r46) ← 这轮用 mem0-letta/crewai-autogen/unsloth-axolotl 接力

Fresh for r48:
✓ Robert Rosen M,R systems (goal-directedness biological)
✓ Castoriadis imaginary significations (autonomy project)
✓ Heidegger Jemeinigkeit Dasein (existential autonomy)
✓ Frankfurt-Dennett compatibilism (hierarchical volition)
✓ Ohta nearly neutral theory (R9 molecular evolution)
✓ Andreas Wagner robustness-evolvability (R9 ASI self-variation)
✓ Stafford Beer VSM viable system model (organizational autonomy)
✓ mem0/letta (memory persistence substrate for autonomy)
✓ crewai/autogen (multi-agent autonomy)
✓ unsloth/axolotl (LLM mutation operators substrate for R9)
"""
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-48.json')

QUERIES = [
    # ===== 7 跨域 (生物/数学/认知/哲学/CS) =====
    # 1. Robert Rosen — (M,R)-systems, closure to efficient causation, goal-directedness biological substrate
    'Robert Rosen M-R systems relational biology closure efficient causation goal-directedness life substrate ASI autonomy',

    # 2. Castoriadis — imaginary significations, autonomy project, social-historical substrate
    'Cornelius Castoriadis imaginary significations autonomy project social-historical institution society substrate ASI',

    # 3. Heidegger — Jemeinigkeit, Dasein, eigenst/mineness, existential autonomy substrate
    'Heidegger Jemeinigkeit Dasein mineness eigenst existential being-there thrownness authenticity substrate autonomy',

    # 4. Compatibilism — Frankfurt hierarchical volition + Dennett evolved free will, agent causation substrate
    'Frankfurt hierarchical volition Dennett compatibilism evolved free will agency causation substrate ASI',

    # 5. Ohta nearly neutral theory — molecular evolution R9 substrate (Kimura 在 r47 已用, 换 Ohta nearly-neutral)
    'Ohta nearly neutral theory molecular evolution slightly deleterious mutation population genetics R9 substrate ASI variation',

    # 6. Andreas Wagner — robustness-evolvability, neutral networks, innovation R9 ASI self-variation substrate
    'Andreas Wagner robustness evolvability neutral network innovation genotype phenotype map R9 substrate ASI self-variation',

    # 7. Stafford Beer — VSM viable system model, organizational autonomy substrate
    'Stafford Beer VSM viable system model organizational autonomy requisite variety recursion substrate ASI multi-agent',

    # ===== 3 GitHub 源码深读 (memory / multi-agent autonomy / LLM self-modification) =====
    # 8. mem0 + letta — memory persistence as substrate for VCP 3 自主生活 (continuity = foundation for autonomy)
    'mem0 letta github memory persistence continuity long-term agent substrate ASI autonomous VCP 3 foundation',

    # 9. crewai + autogen — multi-agent autonomy, role-playing, goal-formation, VCP 3 自主生活 substrate
    'crewai autogen microsoft github multi-agent autonomy role-playing goal-formation orchestration substrate ASI VCP 3',

    # 10. unsloth + axolotl — LLM self-training / fine-tuning as R9 substrate (mutation operators on weights)
    'unsloth axolotl github LLM fine-tuning self-training mutation operators LoRA QLoRA substrate R9 ASI variation',

    # ===== 2 Gap 借鉴 (VCP 3 自主生活 + R9 遗传变异) =====
    # 11. VCP 3 自主生活 substrate — autonomy/agency/goal-formation/value-stability in ASI
    'VCP 3 autonomous living substrate ASI agency goal-formation value-stability Rosen Castoriadis Frankfurt inspiration biomimetic',

    # 12. R9 遗传变异 substrate — Wagner robustness + Ohta neutrality + LLM mutation operators biomimetic
    'R9 genetic variation substrate ASI Wagner robustness Ohta nearly-neutral LLM mutation operators biomimetic variation selection',
]


def main():
    started = time.time()
    started_iso = time.strftime('%Y-%m-%dT%H:%M:%S+08:00', time.localtime(started))
    print(f'Round-48 started {started_iso}')

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
        # Rate-limit friendly
        time.sleep(0.5)

    OUT.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - started
    print(f'\nRound-48 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    # endpoint status summary
    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(1 for r in results if r['bocha_ai_answer'])
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_answered={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()