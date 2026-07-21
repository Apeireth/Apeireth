#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 29 runner — 12 query dual-source (cron 06:56 tick, ~2h00m gap from r28 04:56).

Round 29 主题 (主 00:46 + 真整合):
- 7 全新跨域 (Whitehead 过程哲学 / Cajal 连接组 / Price 协进化 / Merleau-Ponty 身体现象学 /
            Brian Arthur 复杂经济 / Deleuze 差异哲学 / Dennett 多草稿意识)
- 3 GitHub 源码深读 (GAIR-NLP/ASI-Arch ⭐⭐⭐ / SakanaAI/ShinkaEvolve ⭐⭐⭐ / jennyzzt/DGM ⭐⭐⭐)
- 2 Apeireth Gap (繁殖 MISSING / 意识 MISSING):
  - von Neumann 自复制自动机 (繁殖 Gap 直击, 真生产借鉴)
  - 萤火虫生物发光同步 (意识涌现 Gap, 涌现意识借鉴)

- 跨域全新 (7):
  - Alfred North Whitehead 过程实在论 / 关系的过程 (Bergson r28 是哲学过程, Whitehead 是关系的过程, 全新维度)
  - Santiago Ramón y Cajal 神经元连接 / connectomics 真神经科学 (Edelman r23/Damasio r24/Hofstadter r28 不同, 真连接组 ASI 借鉴)
  - George Price 协进化 / Price 方程 (Kauffman r25 是自催化, Price 是协选择数学, 真数学 ASI 借鉴)
  - Maurice Merleau-Ponty 身体现象学 / 知觉现象学 / 肉身主体 (意识哲学, 现象学还原 ASI 借鉴)
  - W. Brian Arthur 复杂经济学 / 报酬递增 / 路径依赖 (经济 ASI 真实借鉴)
  - Gilles Deleuze 差异与重复 / rhizome 块茎哲学 / 游牧思维 (差异哲学, ASI 真哲学)
  - Daniel Dennett 多草稿意识模型 / 进化意识 / 异现象 (Damasio r24 情感意识, Hofstadter r28 类比意识, Dennett 多草稿全新)

- GitHub 源码深读 (3):
  - GAIR-NLP/ASI-Arch ⭐⭐⭐ (主 00:21 提到, ASI 真自演化架构, 主重点)
  - SakanaAI/ShinkaEvolve ⭐⭐⭐ (主 00:21 提到, 进化代码框架)
  - jennyzzt/DGM ⭐⭐⭐ (主 00:21 提到, Darwin Godel Machine 达文谷哥德机)

- Apeireth Gap (2):
  - 繁殖 Gap: von Neumann universal constructor / 自复制自动机 (直击 MISSING 繁殖, ASI 真架构借鉴)
  - 意识 Gap: 萤火虫生物发光同步 / firefly synchrony 涌现 (涌现意识借鉴, 不同于 prion r27)

Cross-round dedup 避让 (verified fresh vs r23-r28):
- r23 已用: Connell/Taleb/Edelman/O'Regan/Meadows/Levin/Scott + Haystack/Voyager/Gorilla/Graziano/bacterial
- r24 已用: Mandelbrot/Watts/Gould/Damasio/Marr/Hoffman/Vygotsky + Langfuse/Browser-Use/PydanticAI/bdelloid/Cannon
- r25 已用: Walker/Landauer/Wolfram/Kauffman/Sheldrake/Solms/Ray + sglang/mlflow/opencompass/apomixis/polyphenism
- r26 已用: Church/Adamatzky/Eigen/Bedau/Gabora/Spencer-Brown/Deacon + OpenRLHF/open-deep-research/mirascope/circadian/transposons
- r27 已用: Prigogine/Maturana-Varela/Thom/Lorenz/Dehaene/Holling/Luhmann + letta/mem0/langgraph/epigenetic/prion
- r28 已用: Rosen/Friston/Hofstadter/von Uexküll/Bergson/Ashby/Per Bak + openevolve/claude-agent-sdk/axolotl + planaria/hydra
- 本轮 fresh 验证:
  - Whitehead ✓ fresh (关系过程, 与 Bergson 不同维度)
  - Cajal/Connectomics ✓ fresh (真神经连接组, 与过往神经科学家不同)
  - Price ✓ fresh (协进化数学, 不同于 Kauffman 自催化)
  - Merleau-Ponty ✓ fresh (现象学, 全新)
  - Brian Arthur ✓ fresh (经济 ASI, 全新)
  - Deleuze ✓ fresh (差异哲学, 全新)
  - Dennett ✓ fresh (多草稿意识, 全新)
  - ASI-Arch ✓ fresh (主 00:21 提到但未跑, 全新)
  - ShinkaEvolve ✓ fresh (主 00:21 提到但未跑, 全新)
  - DGM ✓ fresh (主 00:21 提到但未跑, 全新)
  - von Neumann universal constructor ✓ fresh (繁殖 Gap 直击)
  - 萤火虫 synchrony ✓ fresh (意识 Gap 涌现)

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

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-29.json')

QUERIES = [
    # ===== 7 全新跨域: 过程哲学 / 连接组 / 协进化 / 身体现象学 / 复杂经济学 / 差异哲学 / 多草稿意识 =====
    'Alfred North Whitehead process philosophy process reality relation actual occasion 2026',
    'Santiago Ramon y Cajal neuron doctrine connectomics neural wiring diagram 2026',
    'George R. Price Price equation coevolution selection covariance biology 2026',
    'Maurice Merleau-Ponty phenomenology of perception body schema lived body 2026',
    'W. Brian Arthur complexity economics increasing returns path dependence 2026',
    'Gilles Deleuze difference and repetition rhizome nomad philosophy 2026',
    'Daniel Dennett consciousness multiple drafts heterophenomenology evolution 2026',
    # ===== 3 GitHub 源码深读 (主 00:21 ⭐⭐⭐ ASI 真生产重点) =====
    'GAIR-NLP ASI-Arch architecture self-evolving ASI source code github 2026',
    'SakanaAI ShinkaEvolve evolutionary code LLM framework source code github 2026',
    'jennyzzt DGM Darwin Godel Machine self-improving source code github 2026',
    # ===== 2 Apeireth Gap (繁殖 MISSING + 意识 MISSING) =====
    'John von Neumann self-reproducing automaton universal constructor kinematic model 2026',
    'firefly synchrony bioluminescence collective oscillation emergent consciousness 2026',
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
    print(f'\n=== Round 29 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
