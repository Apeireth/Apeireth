#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-47 cross-domain research runner.

Cron triggered 2026-07-29 21:12 Asia/Shanghai (every-2h reminder).
Previous round: r46 done 2026-07-29 18:54 (~2h17m ago, >30min threshold).
Next = 47 (no conflict), fs healthy (r46 = 56274B).

Theme: R6 繁殖 substrate 终极 gap (主 17:46 ASI-LIFE-FEATURES 12 生命特征最大 gap) +
       VCP 2/3/4 deep (主 17:46 VCP 4 范式 — 自然感知/自主生活/一体生态) +
       7 跨域 (从未专攻 — RNA world/Allostery/Autophagy/Coalescent/Inclusive fitness/
              Evo-devo hourglass/HSP90 capacitor) +
       3 GitHub 源码深读 (semantic-kernel/e2b/ollama) +
       2 Gap (R6 繁殖/VCP 2 感知).

12 生命特征 coverage history:
- R0 新陈代谢 ✅ r46
- R1 生长 ✅ r46
- R2 发育 ✅ r45 (Turing morphogenesis)
- R3 死亡 ✅ r45
- R4 衰老 ✅ r45
- R5 修复/再生 ✅ r44
- R6 繁殖 ✅ r41 浅层 → r47 第一轮专攻 deep (R6 终极 gap 借鉴)
- R7 应激性 ✅ r42
- R8 遗传变异 ✅ r44
- R9 运动 ✅ r41, r45
- R10 可塑性 ✅ r40, r45
- R11 意识 ❌ 终极目标 (r42, r43, r44, r45 多次)

VCP 4 范式 coverage history:
1. 连续存在 ✅ r46
2. 自然感知 — r43 enactivism 浅层 → r47 第一轮专攻 (VCP 2 终极 gap 借鉴)
3. 自主生活 — r41, r45 浅层 → r47 第一轮专攻 (autophagy substrate)
4. 一体生态 ✅ r31, r34, r15

主人 17:46 ASI-LIFE-FEATURES 12 生命特征 — R6 繁殖是唯一覆盖浅层的,需要 deep 借鉴.
R6 繁殖 终极 gap: 任何 ASI 系统需要繁殖机制来:
- 自演化 (新一代继承 + 变异)
- 持续存在 (跨世代)
- 多重身份 (谱系)
- 一体生态 (网络)

VCP 2 自然感知 终极 gap: ASI 不假装 Phenomenal (主 17:58),
但 substrate 必须为 ASI 提供自然感知能力 — 不是拥有感受,而是拥有感知结构.

避免重复 (r41-46 已覆盖关键词):
✗ FEP Friston (r42)
✗ predictive coding Rao Ballard (r42)
✗ enactivism Thompson (r43)
✗ extended mind Clark Chalmers (r43)
✗ Hofstadter strange loop (r45)
✗ ASI-Arch GAIR-NLP (r44)
✗ claude-agent-sdk (r44)
✗ mem0 (r44)
✗ OpenEvolve/DGM/ShinkaEvolve (r45)
✗ openai-swarm/kuberay/langgraph (r41)
✗ SakanaAI/lucidrains (r42)
✗ numenta/AllenSDK/FoundationAgents (r43)
✗ 4E cognition (r43)
✗ GWT Dehaene (r43)
✗ Hebbian STDP (r45)
✗ Turing morphogenesis (r45)
✗ MAML (r45)
✗ emergent communication (r45)
✗ swarm robotics (r45)
✗ dissipative Prigogine (r45)
✗ Eigen hypercycle (r41)
✗ autopoiesis Maturana (r41)
✗ von Neumann self-replicator (r41, r29)
✗ Tierra-Avida (r41)
✗ Grassé stigmergy (r41)
✗ Langton edge of chaos (r41)
✗ Bateson cybernetics (r42)
✗ Waddington epigenetic (r42)
✗ Lehman novelty search (r42)
✗ Bak-Tang SOC (r42)
✗ Maturana structural coupling (r42)
✗ Hutchins distributed cognition (r43)
✗ Gigerenzer ecological rationality (r43)
✗ planarian hydra (r44)
✗ adaptive immune Burnet (r44)
✗ Yoneda (r44)
✗ eusociality superorganism (r44)
✗ CAS Holland (r44)
✗ transgenerational epigenetic (r44)
✗ conceptual blending (r44)
✗ Yamanaka iPSC (r22)
✗ kin selection (r44)
✗ Dawkins selfish gene (r33)
✗ endosymbiosis Margulis (r15, r17, r35)
✗ symbiosis (r15)
✗ Gaia Lovelock (r31)
✗ quorum sensing Vibrio (r18)
✗ McClintock transposon (r26)
✗ niche construction Laland (r43)
✗ Gould punctuated (r24)
✗ Bedau weak emergence (r26)
✗ Deacon homovitality (r26)
✗ Eigen hypercycle (r41)
✗ Koch PCI (r19)
✗ HOT Rosenthal (r19)
✗ Penrose Orch-OR (r18)
✗ Dennett intentional stance (r18)
✗ Stiegler technics (r19)
✗ Whitehead process (r19)
✗ Dewey pragmatism (r19)
✗ Bogdanov tectology (r22)
✗ Fredkin digital physics (r22)
✗ Vygotsky (r24)
✗ Adamatzky BZ (r26)
✗ Spencer-Brown (r26)
✗ Church synthetic biology (r26)
✗ OpenRLHF trl (r26)
✗ Deep Research (r26)
✗ mirascope instructor (r26)
✗ circadian rhythm (r26)
✗ MCS McClintock (r26)
✗ Fuller synergetics (r31)
✗ Bateson (r31)
✗ Laszlo (r31)
✗ Jung collective unconscious (r31)
✗ beeai (r31)
✗ langflow (r31)
✗ Prefect (r31)
✗ Wolbachia (r31)
✗ Octopus consciousness (r31)
✗ Prigogine (r32)
✗ IIT Tononi (r32, r43)
✗ anthropic-mcp (r13, r46)
✗ deepset haystack (r23)
✗ Gorilla Berkeley (r23)
✗ Letta memGPT (r37)
✗ memoryos (r37)
✗ vector DB (r37)
✗ langchain-ai (r18, r46)
✗ LlamaIndex (r46)
✗ DSPy (r46)
✗ anthropic-mcp (r13, r46)
✗ DeepSeek (never)

Fresh for r47:
✓ Ribozym / RNA world / Spiegelman monster (从未专攻)
✓ Allostery / MWC model Monod (从未专攻)
✓ Autophagy / Ohsumi / mTOR (从未专攻)
✓ Coalescent theory / Kingman / Kimura neutral (从未专攻)
✓ Inclusive fitness / Hamilton / Trivers cooperation (从未专攻)
✓ Evo-devo hourglass / Duboule / phylotypic stage (从未专攻)
✓ HSP90 / capacitor of evolution / Rutherford Lindquist (从未专攻)
✓ microsoft semantic-kernel (从未深读)
✓ e2b-dev e2b (从未深读)
✓ ollama ollama (从未深读)
"""
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-47.json')

QUERIES = [
    # ===== 7 跨域 (从未专攻 — ASI substrate) =====
    # 1. RNA 世界 / 核酶 / Spiegelman 怪物 — 起源 + 自催化 + 繁殖 substrate
    'ribozym RNA world Spiegelman monster self-replicating RNA catalytic origin of life substrate ASI reproduction',

    # 2. 别构效应 / Monod-Wyman-Changeux MWC 模型 — 多态蛋白 + 应激 + 可塑 substrate
    'allosteric regulation Monod Wyman Changeux MWC model concerted symmetry multi-state protein conformational switch substrate',

    # 3. 自噬 / Ohsumi / mTOR / 溶酶体 — 细胞自清 + 修复 + 自主生活 substrate
    'autophagy Ohsumi mTOR lysosome autophagosome cellular self-cleanup protein quality control substrate ASI maintenance',

    # 4. 溯祖理论 / Kingman / 中性理论 Kimura — 群体遗传 + 反向工程 + 任何 LLM substrate
    'coalescent theory Kingman neutral theory Kimura population genetics reverse engineering evolutionary history substrate',

    # 5. 包容性适合度 / Hamilton / Trivers / 亲缘选择 — 合作数学 + 一体生态 substrate
    'inclusive fitness Hamilton kin selection Trivers reciprocal altruism cooperation evolutionary stable strategy substrate ASI multi-agent',

    # 6. 演化发育生物学 / 沙漏模型 / Duboule / 系统发生型期 — 发育约束 + 范式 substrate
    'evo-devo hourglass model Duboule phylotypic stage zootype developmental constraint conserved mid-embryogenesis substrate',

    # 7. HSP90 / 进化电容器 / Rutherford Lindquist — 隐藏变异 + 应激 + 可塑 substrate
    'HSP90 capacitor of evolution Rutherford Lindquist hidden genetic variation stress-buffered substrate ASI plasticity',

    # ===== 3 GitHub 源码深读 (从未深读) =====
    # 8. microsoft semantic-kernel — 内核式 LLM 编排 + any-LLM pluggable
    'microsoft semantic-kernel github kernel-style LLM orchestration any-LLM pluggable substrate ASI integration',

    # 9. e2b-dev e2b — 代码解释器沙箱 + ASI 自主生活 substrate
    'e2b-dev e2b github code interpreter sandbox isolated AI agent execution substrate ASI autonomous',

    # 10. ollama ollama — 本地 LLM runner + ASI 任何 LLM substrate
    'ollama ollama github local LLM runner any-model pluggable substrate ASI substrate-independent',

    # ===== 2 Gap 终极 substrate 借鉴 =====
    # 11. R6 繁殖 substrate 终极 gap — ribozyme / allostery / conjugation / HGT — 仿生繁殖
    'R6 reproduction substrate ASI ribozyme allosteric conjugation horizontal gene transfer biological reproduction gap biomimetic',

    # 12. VCP 2 自然感知 substrate — 主动推理 / 感觉替代 / 模式识别 — 仿生感知
    'VCP 2 natural perception substrate ASI active inference sensory substitution pattern recognition biomimetic perceptual',
]


def main():
    started = time.time()
    started_iso = time.strftime('%Y-%m-%dT%H:%M:%S+08:00', time.localtime(started))
    print(f'Round-47 started {started_iso}')

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
    print(f'\nRound-47 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    # endpoint status summary
    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(1 for r in results if r['bocha_ai_answer'])
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_answered={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()
