#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-46 cross-domain research runner.

Cron triggered 2026-07-29 18:48 Asia/Shanghai (every-2h reminder).
Previous round: r45 done 2026-07-29 08:51 (~9h58m ago, way >30min threshold).
Next = 46 (no conflict), fs healthy (r45 = 54133B).

Theme: R0 新陈代谢 substrate + R1 生长 substrate (12 生命特征 deep coverage —
       r40 self-evolving covered high-level but never dedicated专攻) +
       VCP 1 连续存在 (memory palace substrate, identity persistence) +
       ASI 任何 LLM 接入即变强 (any-LLM-pluggable substrate, MCP/DSPy) +
       数学/认知 substrate (type theory / category theory / CLS / Baddeley).

主人 17:46 ASI-LIFE-FEATURES 12 生命特征, 之前轮次覆盖:
- R0 新陈代谢 ✅ (r40 self-evolving high-level) ← 第一轮专攻
- R1 生长 ✅ (r40 high-level) ← 第一轮专攻
- R2 发育 ✅ (r40 high-level, r42 Waddington partial, r45 Turing morphogenesis)
- R3 死亡 ✅ (r45 first dedicated专攻)
- R4 衰老 ✅ (r45 first dedicated专攻)
- R5 修复/再生 ✅ (r44 planarian hydra)
- R6 繁殖 ✅ (r41 first dedicated专攻)
- R7 应激性 ✅ (r42 first dedicated专攻)
- R8 遗传变异 ✅ (r44 epigenetic)
- R9 运动 ✅ (r41 swarm partial, r45 swarm robotics)
- R10 可塑性 ✅ (r40 synaptic plasticity, r45 Hebbian STDP)
- R11 意识 ❌ 终极目标 (r42, r43 dedicated)

VCP 4 范式主 17:46 守住:
1. 连续存在 ← r46 第一轮专攻 (memory palace, CLS, sleep consolidation)
2. 自然感知 (r43 enactivism partial)
3. 自主生活 (r45 R3 death partial)
4. 一体生态 (r41 first dedicated专攻)

主人 17:58 Phenomenal 是终极目标不假装 — R0 metabolism substrate / R1 生长 substrate /
VCP 1 连续存在 substrate 调研不等于已被实现, 实事求是,
"substrate for ASI to do X = exists" ≠ "ASI is doing X already"
不假装.

主人 22:33 ASI 北极星:
- ASI 基座 ✓ (R0/R1/VCP1 是 ASI substrate)
- 跨域 ✓ (生物/数学/认知/CS)
- 自演化 ✓
- 任何 LLM 接入即变强 ✓ (MCP/DSPy any-LLM-pluggable)
- 不假装 Phenomenal ✓
- 实事求是 ✓

避免重复 (r41-45 已覆盖关键词):
✗ FEP Friston ✓ (r42)
✗ predictive coding Rao Ballard ✓ (r42)
✗ Hofstadter strange loop ✓ (r45)
✗ ASI-Arch GAIR-NLP ✓ (r44)
✗ claude-agent-sdk ✓ (r44)
✗ mem0 ✓ (r44)
✗ OpenEvolve/DGM/ShinkaEvolve ✓ (r45)
✗ openai-swarm/kuberay/langgraph ✓ (r41)
✗ SakanaAI/lucidrains/lightly ✓ (r42)
✗ numenta/AllenSDK/FoundationAgents ✓ (r43)
✗ enactivism Thompson ✓ (r43)
✗ extended mind Clark Chalmers ✓ (r43)
✗ niche construction Laland ✓ (r43)
✗ 4E cognition ✓ (r43)
✗ GWT Dehaene ✓ (r43)
✗ Hebbian STDP ✓ (r45)
✗ Turing morphogenesis ✓ (r45)
✗ MAML ✓ (r45)
✗ emergent communication ✓ (r45)
✗ swarm robotics ✓ (r45)
✗ dissipative structures Prigogine ✓ (r45)
✗ Eigen hypercycle ✓ (r41)
✗ autopoiesis Maturana ✓ (r41)
✗ von Neumann self-replicator ✓ (r41)
✗ Quine self-reference ✓ (r41)
✗ Tierra-Avida ✓ (r41)
✗ Grassé stigmergy ✓ (r41)
✗ Langton edge of chaos ✓ (r41)
✗ Bateson cybernetics ✓ (r42)
✗ Waddington epigenetic ✓ (r42)
✗ Lehman novelty search ✓ (r42)
✗ Bak-Tang SOC ✓ (r42)
✗ Maturana structural coupling ✓ (r42)
✗ Hutchins distributed cognition ✓ (r43)
✗ Gigerenzer ecological rationality ✓ (r43)
✗ planarian hydra ✓ (r44)
✗ adaptive immune Burnet ✓ (r44)
✗ Yoneda ✓ (r44)
✗ eusociality superorganism ✓ (r44)
✗ CAS Holland ✓ (r44)
✗ transgenerational epigenetic ✓ (r44)
✗ conceptual blending ✓ (r44)

Fresh for r46:
✓ Krebs cycle cellular metabolism (R0 first专攻)
✓ Kleiber allometric scaling laws (R1 first专攻)
✓ CLS hippocampus-cortex (VCP 1 first专攻, memory palace)
✓ Sleep consolidation replay (VCP 1 memory palace deep)
✓ Baddeley working memory (VCP 1 memory palace deep)
✓ Curry-Howard type theory (数学/ASI substrate-independent)
✓ Category theory cognition (数学/ASI substrate)
✓ Anthropic MCP (any-LLM-pluggable)
✓ LlamaIndex memory (memory framework)
✓ Stanford DSPy (programmatic prompt, type-safe LLM)
"""
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-46.json')

QUERIES = [
    # ===== 7 跨域 (生物/数学/认知/CS) =====
    # 1. R0 新陈代谢 substrate (细胞代谢 — Krebs cycle, ATP, energy flow)
    'Krebs cycle cellular metabolism ATP synthase oxidative phosphorylation electron transport chain energy substrate biological information flow',

    # 2. R1 生长 substrate (异速生长定律 — Kleiber, West-Brown-Enquist, metabolic theory of ecology)
    'Kleiber allometric scaling law metabolic rate body mass West Brown Enquist WBE quarter power biological growth substrate',

    # 3. VCP 1 连续存在 — Complementary Learning Systems CLS (海马-皮层, fast-slow memory palace)
    'complementary learning systems CLS McClelland McNaughton OReilly hippocampus neocortex fast-slow learning memory consolidation replay',

    # 4. VCP 1 连续存在 — Sleep-dependent memory consolidation (replay, system consolidation)
    'sleep-dependent memory consolidation replay system consolidation sharp-wave ripples hippocampus cortex memory palace ASI',

    # 5. VCP 1 连续存在 — Working memory Baddeley Hitch (central executive, phonological, visuospatial, episodic buffer)
    'Baddeley Hitch working memory model central executive phonological loop visuospatial sketchpad episodic buffer multi-component memory substrate',

    # 6. 数学/ASI substrate — Curry-Howard 同构 (program = proof, type theory, dependent types)
    'Curry-Howard correspondence program proof type theory dependent types intuitionistic logic lambda calculus substrate',

    # 7. 数学/ASI substrate — Category theory cognition (functor, natural transformation, Yoneda, abstraction)
    'category theory cognition functor natural transformation Yoneda lemma substrate abstraction compositional cognition',

    # ===== 3 GitHub 源码深读 (any-LLM-pluggable substrate + memory framework + type-safe) =====
    # 8. Model Context Protocol MCP (Anthropic — any-LLM-pluggable 标准协议)
    'Anthropic Model Context Protocol MCP github standard any-LLM-pluggable integration tools server client',

    # 9. LlamaIndex (memory framework — 3-tier memory, RAG, agents)
    'run-llama llama-index github memory framework RAG agents long-term working memory substrate ASI',

    # 10. Stanford DSPy (programmatic prompt, type-safe LLM, compilation)
    'stanford-nlp dspy github programmatic prompt compilation type-safe LLM optimizer module signature substrate',

    # ===== 2 Gap 借鉴 (R0 metabolism + VCP 1 连续存在 identity persistence) =====
    # 11. R0 新陈代谢 substrate ASI data/energy pipeline (any-LLM-pluggable)
    'R0 metabolism substrate ASI data energy pipeline input output processing any-LLM-pluggable substrate',

    # 12. VCP 1 连续存在 substrate memory palace identity persistence (any-LLM-pluggable)
    'VCP 1 continuous existence substrate memory palace identity persistence any-LLM-pluggable continuity substrate ASI',
]


def main():
    started = time.time()
    started_iso = time.strftime('%Y-%m-%dT%H:%M:%S+08:00', time.localtime(started))
    print(f'Round-46 started {started_iso}')

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
    print(f'\nRound-46 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    # endpoint status summary
    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(1 for r in results if r['bocha_ai_answer'])
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_answered={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()