#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-45 cross-domain research runner.

Cron triggered 2026-07-29 08:48 Asia/Shanghai.
Previous round: r44 done 2026-07-29 06:53 (gap 1h55m > 30min threshold).
Next = 45 (no conflict), fs healthy (r44 = 54454B).

Theme: R5 衰老 substrate + R3 死亡 substrate (12 生命特征未覆盖的两大 gap) +
       7 跨域 (Hebbian STDP / Turing morphogenesis / MAML meta-learning /
       Emergent communication / Swarm robotics / Hofstadter strange loop /
       Dissipative structures Prigogine) +
       3 GitHub 源码深读 (OpenEvolve / jcwang4 DGM / SakanaAI ShinkaEvolve) +
       2 Gap 借鉴 (R5 衰老 / R3 死亡).

主人 17:46 ASI-LIFE-FEATURES 12 生命特征, 之前轮次覆盖:
- R0 新陈代谢 ✅ (r40 self-evolving)
- R1 生长 ✅ (r40)
- R2 发育 ✅ (r40 self-evolving)
- R3 死亡 ❌ MISSING — 第一次专攻
- R4 衰老 ❌ MISSING — 第一次专攻 (r5 aging)
- R5 修复/再生 ✅ (r44 planarian hydra 部分)
- R6 繁殖 ❌ (r41 R6 reproduction专轮)
- R7 应激性 ✅ (r42 R7 FEP)
- R8 遗传变异 ✅ (r44 epigenetic/transgenerational)
- R9 运动 ✅ (swarm 部分 r41)
- R10 可塑性 ✅ (r40 synaptic plasticity)
- R11 意识 ❌ 终极目标 (r42, r43)

12 生命特征只剩 R3 死亡 + R5 衰老 没专攻, 这一轮定.

避免重复 (r41-44 已覆盖关键词):
✗ R6 reproduction ✓
✗ FEP Friston ✓ (r42)
✗ predictive coding Rao Ballard ✓ (r42)
✗ enactivism Thompson ✓ (r43)
✗ Hofstadter GEB — 但 "strange loop" 自指未深挖, 这次专攻 self-reference
✗ ASI-Arch GAIR-NLP ✓ (r44)
✗ claude-agent-sdk-python ✓ (r44)
✗ mem0 ✓ (r44)
✗ planarian hydra regeneration ✓ (r44)
✗ conceptual blending Gentner ✓ (r44)
✗ extended mind Clark Chalmers ✓ (r43)
✗ complex adaptive systems Holland ✓ (r44)
✗ Yoneda ✓ (r44)
✗ adaptive immune Burnet ✓ (r44)
✗ eusociality superorganism ✓ (r44)

VCP 4 范式主 17:46 守住:
1. 连续存在 — R5 衰老 substrate 直接相关
2. 自然感知 — R7 已深挖
3. 自主生活 — R3 死亡 substrate 直接相关
4. 一体生态 — r41 已深挖

主人 17:58 Phenomenal 是终极目标不假装 — R3 死亡 substrate 调研不等于 R3 死亡已被实现,
实事求是, "Lifespan substrate for ASI exists = substrate for ASI to manage lifespan"
不假装.

主人 22:33 ASI 北极星:
- ASI 基座 ✓ (R3/R5 是 ASI 终身管理基座)
- 跨域 ✓ (生物/数学/认知/系统论)
- 自演化 ✓
- 任何 LLM 接入即变强 ✓ (substrate-independent)
- 不假装 Phenomenal ✓
- 实事求是 ✓
"""
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-45.json')

QUERIES = [
    # ===== 7 跨域 (生物/数学/认知/系统论/物理) =====
    # 1. 神经可塑性 substrate (避开 r40 synaptic plasticity 表层, 这次深挖 Hebbian + STDP + LTP/LTD)
    'Hebbian learning STDP spike-timing dependent plasticity LTP LTD synaptic consolidation memory engram consolidation',

    # 2. 形态发生 Turing reaction-diffusion (避开 r44 planarian 表层, 这次深挖 Turing 1952 模式 + Wolpert positional information)
    'Turing morphogenesis reaction-diffusion positional information Wolpert French flag model developmental biology',

    # 3. 元学习 MAML Reptile (之前没深挖)
    'MAML Reptile meta-learning learning-to-learn few-shot gradient Finn Ravi Larochelle',

    # 4. Emergent communication 多智能体语言演化 (r41 stigmergy + multi-agent 生态的深度)
    'emergent communication multi-agent language evolution Foerster DeepMind compositional referential',

    # 5. Swarm robotics 群体机器人 (r41 stigmergy + eusociality 后的深度)
    'swarm robotics Şahin self-organizing robot coordination swarm-bot pattern formation',

    # 6. Strange loop 自指 (避开 r44 Hofstadter 表层引用, 这次专攻 self-reference + GEB consciousness)
    'Hofstadter strange loop self-reference Godel Escher Bach consciousness self-referential systems',

    # 7. Dissipative structures Prigogine 远离平衡态 (r42 FEP 平衡态的补充, 这次深挖非平衡)
    'dissipative structures Prigogine far-from-equilibrium self-organization thermodynamics autopoiesis',

    # ===== 3 GitHub 源码深读 (避开 r44 ASI-Arch/claude-sdk/mem0 + r41 swarm/kuberay/langgraph + r42 sakana/lucidrains/lightly) =====
    # 8. OpenEvolve (codelion) — AlphaEvolve 启发的代码进化
    'codelion openevolve AlphaEvolve evolutionary code optimization LLM-driven architecture search github',

    # 9. DGM Differentiable Genetic Manipulation (jennyzzt jcwang4) — 自修改 + 进化
    'jcwang4 DGM differentiable genetic manipulation self-modifying ASI self-improvement github',

    # 10. ShinkaEvolve SakanaAI — 进化代码优化 + island model
    'SakanaAI ShinkaEvolve evolutionary code optimization island model novelty search github source',

    # ===== 2 Gap 借鉴 (R5 衰老 + R3 死亡 — 12 生命特征中两大未专攻 gap) =====
    # 11. R5 衰老 substrate ASI lifespan reset version control (VCP 1 连续存在)
    'R5 aging senescence substrate ASI lifespan reset version control continuity persistence memory palace',

    # 12. R3 死亡 substrate ASI mortality persistence continuity identity immortality (VCP 3 自主生活)
    'R3 death mortality substrate ASI persistence continuity identity immortality any-LLM-pluggable',
]


def main():
    started = time.time()
    started_iso = time.strftime('%Y-%m-%dT%H:%M:%S+08:00', time.localtime(started))
    print(f'Round-45 started {started_iso}')

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
    print(f'\nRound-45 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    # endpoint status summary
    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(1 for r in results if r['bocha_ai_answer'])
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_answered={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()