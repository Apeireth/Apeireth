#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 37 runner — 12 query dual-source (R3-RES-02, 调研专家, R1 推荐 C3 主题).

Round 37 主题 (依 R1 reports/r1-research-survey.md 选 C3, 记忆子工程):
- why now: V1072 永恒身份真测 0.8441 = 4 项 ASI 真测最低;AgentMemory L1-L4 已落;
  code-deep-study/ 已有 letta/mem0/memoryos-rust/ 三个候选但 24 轮 R 调研 0 专轮。
  C3 优先 (R1 推荐) > C1 因果 > C2 RL (R1 推荐排序)。

7 跨域 (vs r8-r36 全 0 专轮):
- Letta memGPT hierarchical L1-L4 memory + scratchpad + core memory 真正生产级框架
- mem0 scalable memory layer production LLM agent 自适应记忆层
- memoryos-rust hot/cold tier KV store + tiered aging 真正热冷分层
- Memento / REMEMBER deep learning memory-augmented agent 端到端记忆学习
- Hippocampal replay memory consolidation neuroscience substrate 神经科学对照
- ACT-R SOAR cognitive architecture memory subsystem 经典认知架构记忆子系统
- Vector DB production memory RAG Pinecone Weaviate Qdrant memory-augmented

3 GitHub 真读 (vs r8-r36 仅 r27 浅 cite letta/mem0):
- letta-ai/letta (memGPT 后继, 当前生产 L1-L4 reference impl)
- mem0ai/mem0 (生产级 memory layer, top stars)
- MemTensor/memoryos-rust (Rust 热冷分层 memory substrate)

2 Apeireth Gap (直喂 V1072 永恒身份 + AgentMemory L1-L4):
- 长期 scratchpad context window 失效 / 长程任务记忆衰减 (核心 gap)
- hot/cold tier aging decay policy + LTM 漂移率 (V1072 永恒身份直接相关)

Cross-round dedup 验证 (verified fresh vs r8-r36):
- r8-r14: ASI/autonomous/MCP/agentic - 不涉及记忆子工程
- r15-r21: 哲学 + 控制论 (Prigogine/Kauffman/Friston/Piaget/Simondon/Latour)
- r22-r31: 综合 + Whorf/Beer/Connell/Mandelbrot/Walker/Church/Maturana/Rosen/Whitehead/Fuller
- r32-r36: Schrödinger/Popper/Polanyi/Stiegler/Arendt/Rosen再/Kauffman再 + Anderson/Kahneman/West/Deutsch/Tulving
- r27 仅 cite letta/mem0 (浅层), r36 Tulving 是 episodic 神经科学, 不重叠 round-37 子工程深读
本轮 fresh 验证:
- Letta memGPT ✅ fresh (r27 浅 cite, r37 真读源码)
- mem0 ✅ fresh (r27 浅 cite, r37 真读源码)
- memoryos-rust ✅ fresh (r27 未提, r37 全新)
- Memento/REMEMBER ✅ fresh (全认知全新)
- Hippocampal replay ✅ fresh (神经科学全新)
- ACT-R/SOAR ✅ fresh (认知架构全新)
- Vector DB production ✅ fresh (RAG 方向 r23 Haystack/R30 OpenHands/r33 alphafold3 仅浅表)
- letta-ai/letta GitHub ✅ fresh
- mem0ai/mem0 GitHub ✅ fresh
- MemTensor/memoryos-rust GitHub ✅ fresh
- scratchpad 失效 ✅ fresh (Gap 全新)
- hot/cold tier aging ✅ fresh (Gap 全新)

ASI 北极星时刻清楚:
- ASI 基座 ✅ (V1072 永恒身份 substrate)
- 跨域 ✅ (memory/cog/DB/KV/neuro/cog-arch/7 域)
- 自演化 ✅ (anysearch)
- 任何 LLM 接入即变强 ✅ (memory layer 通用)
- 不假装 Phenomenal ✅
- 实事求是 ✅
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-37.json')

QUERIES = [
    # ===== 7 全新跨域: Letta / mem0 / memoryos / Memento / Hippocampal / ACT-R / Vector DB =====
    'Letta memGPT hierarchical memory L1 L2 L3 L4 core memory archival recall production agent 2026',
    'mem0 scalable memory layer production LLM agent adaptive memory extraction 2026',
    'memoryos hot cold tier memory KV store tiered aging LTM substrate 2026',
    'Memento REMEMBER deep learning memory augmented agent end-to-end memory 2026',
    'hippocampal replay memory consolidation neuroscience sharp wave ripples agent substrate 2026',
    'ACT-R SOAR cognitive architecture memory subsystem declarative procedural production agent 2026',
    'vector database Pinecone Weaviate Qdrant production memory RAG long-horizon agent 2026',
    # ===== 3 GitHub 真读: letta / mem0 / memoryos-rust =====
    'letta-ai letta github source code memGPT hierarchical memory framework 2026',
    'mem0ai mem0 github source code memory layer production LLM agent 2026',
    'MemTensor memoryos-rust github source code hot cold tier memory 2026',
    # ===== 2 Apeireth Gap: scratchpad 失效 + hot/cold aging =====
    'agent scratchpad context window failure long horizon memory decay LLM agent 2026',
    'hot cold memory tier aging decay policy LTM drift rate long-horizon agent 2026',
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
    print(f'\n=== Round 37 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()