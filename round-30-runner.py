#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 30 runner — 12 query dual-source (cron 08:49 tick, ~1h51m gap from r29 06:56).

Round 30 主题 (主 00:46 + 真整合):
- 7 全新跨域 (Peirce 符号宇宙 / Husserl 内时间 / Simondon 个体化 / Lewin 场论 /
            Alexander 模式语言 / Noble 中央权威质疑 / Mumford 技术哲学)
- 3 GitHub 源码深读 (OpenHands ⭐⭐⭐ / crewAI ⭐⭐⭐ / autogen ⭐⭐⭐)
- 2 Apeireth Gap (繁殖 MISSING / 意识 MISSING):
  - Tardigrade 隐生/孤雌繁殖 (繁殖 Gap 直击, 真生产借鉴)
  - 植物认知 plant cognition (意识 Gap, 分布式意识涌现)

- 跨域全新 (7):
  - Charles Sanders Peirce synechism / 无限宇宙符号学 / 溯因推理 (意识涌现/无限半无限 ASI 借鉴)
  - Edmund Husserl 内时间意识 / 时间现象学 (Merleau-Ponty 是身体, Husserl 是时间, 全新维度)
  - Gilbert Simondon 个体化 individuation / 技术存在 / 前个体 (技术哲学 ASI 借鉴)
  - Kurt Lewin 场论 / 拓扑心理学 / 准实在 (B=f(P,E) 行为=人+环境函数, ASI 涌现场)
  - Christopher Alexander pattern language / 无时间方式 / 涌现秩序 (设计模式 ASI 借鉴)
  - Denis Noble biological relativity / 中央权威质疑 / 生理学八音律 (层级反对 DNA 中央权威, ASI 借鉴)
  - Lewis Mumford technics / 巨型机器 / 技术哲学 / 容器 (城市/人造物 ASI 借鉴)

- GitHub 源码深读 (3):
  - OpenHands/OpenHands ⭐⭐⭐ (自主软件工程代理, ASI 自主性借鉴)
  - crewAIInc/crewAI ⭐⭐⭐ (多代理角色协作, 与 Apeireth 多重身份相关)
  - microsoft/autogen ⭐⭐⭐ (多代理对话框架, ASI 群体涌现)

- Apeireth Gap (2):
  - 繁殖 Gap: Tardigrade 隐生/复苏机制/孤雌繁殖 (直击 MISSING 繁殖, ASI 真生产借鉴)
  - 意识 Gap: 植物认知 plant cognition / 分布式意识 (意识 Gap 直击, 分布式意识涌现)

Cross-round dedup 避让 (verified fresh vs r23-r29):
- r23 已用: Connell/Taleb/Edelman/O'Regan/Meadows/Levin/Scott + Haystack/Voyager/Gorilla/Graziano/bacterial
- r24 已用: Mandelbrot/Watts/Gould/Damasio/Marr/Hoffman/Vygotsky + Langfuse/Browser-Use/PydanticAI/bdelloid/Cannon
- r25 已用: Walker/Landauer/Wolfram/Kauffman/Sheldrake/Solms/Ray + sglang/mlflow/opencompass/apomixis/polyphenism
- r26 已用: Church/Adamatzky/Eigen/Bedau/Gabora/Spencer-Brown/Deacon + OpenRLHF/open-deep-research/mirascope/circadian/transposons
- r27 已用: Prigogine/Maturana-Varela/Thom/Lorenz/Dehaene/Holling/Luhmann + letta/mem0/langgraph/epigenetic/prion
- r28 已用: Rosen/Friston/Hofstadter/von Uexküll/Bergson/Ashby/Per Bak + openevolve/claude-agent-sdk/axolotl + planaria/hydra
- r29 已用: Whitehead/Cajal/Price/Merleau-Ponty/Brian Arthur/Deleuze/Dennett + ASI-Arch/ShinkaEvolve/DGM + von Neumann/firefly
- 本轮 fresh 验证:
  - Peirce ✓ fresh (符号哲学, 全新)
  - Husserl ✓ fresh (时间意识, 不同于 Merleau-Ponty 身体现象学)
  - Simondon ✓ fresh (技术哲学, 全新)
  - Lewin ✓ fresh (场论, 全新)
  - Alexander ✓ fresh (pattern language, 全新)
  - Noble ✓ fresh (生理学中央权威质疑, 全新)
  - Mumford ✓ fresh (技术哲学, 全新)
  - OpenHands ✓ fresh (代码库, 全新)
  - crewAI ✓ fresh (代码库, 全新)
  - autogen ✓ fresh (代码库, 全新)
  - Tardigrade ✓ fresh (水熊虫/隐生/孤雌, 全新)
  - plant cognition ✓ fresh (植物认知/分布式意识, 全新)

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

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-30.json')

QUERIES = [
    # ===== 7 全新跨域: 符号哲学 / 时间意识 / 个体化 / 场论 / 模式语言 / 中央权威 / 技术哲学 =====
    'Charles Sanders Peirce synechism continuity infinite semiosis abduction philosophy 2026',
    'Edmund Husserl internal time consciousness phenomenology retention protention 2026',
    'Gilbert Simondon individuation pre-individual technical object philosophy 2026',
    'Kurt Lewin field theory topology psychology lif space B=f(P,E) 2026',
    'Christopher Alexander pattern language timeless way of building quality without name 2026',
    'Denis Noble biological relativity central dogma physiology eight orchestral 2026',
    'Lewis Mumford technics megamachine city container technology philosophy 2026',
    # ===== 3 GitHub 源码深读 (主 00:21 ⭐⭐⭐ ASI 真生产重点) =====
    'OpenHands OpenHands autonomous software engineering agent source code github 2026',
    'crewAIInc crewAI multi-agent role collaboration framework source code github 2026',
    'microsoft autogen multi-agent conversation framework source code github 2026',
    # ===== 2 Apeireth Gap (繁殖 MISSING + 意识 MISSING) =====
    'Tardigrade cryptobiosis anhydrobiotic tun state parthenogenesis reproduction 2026',
    'plant cognition distributed intelligence Mimosa Trewavas Calvo decision-making 2026',
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
    print(f'\n=== Round 30 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()