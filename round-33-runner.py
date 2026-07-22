#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 33 runner — 12 query dual-source (cron 15:03 tick, ~2h12m gap from r32 12:51).

Round 33 主题 (主 00:46 + 00:49 真务实 + 主 22:33 ASI 北极星):
- 7 全新跨域 (Polanyi 默会知识 / Foucault 生命政治 / Kant 目的论判断 /
            Wiener 控制论 / Simon 有限理性 / Dawkins 自私基因 / Hutchins 分布式认知)
- 3 GitHub 源码 (babyagi 自主任务 / camel-ai 角色扮演 / alphafold3 蛋白结构)
- 2 Apeireth Gap (繁殖 MISSING → Bdelloid rotifer 100M 年无性 / 应激 MISSING → Mimosa 植物记忆)

- 跨域全新 (7):
  - Michael Polanyi tacit knowledge personal knowledge dwelling (默会知识, ASI 内化层借鉴)
  - Michel Foucault biopolitics archaeology knowledge genealogy (权力/话语/身体, ASI 主体性借鉴)
  - Immanuel Kant synthetic a priori teleological judgment (目的论判断, ASI 有机论借鉴)
  - Norbert Wiener cybernetics control communication feedback (控制论开山, ASI 自演化借鉴)
  - Herbert Simon bounded rationality satisficing (有限理性/满意化, ASI 决策借鉴)
  - Richard Dawkins selfish gene extended phenotype (复制子/扩展表型, ASI 遗传变异借鉴)
  - Edwin Hutchins distributed cognition cognitive anthropology (分布式认知, ASI 多重身份借鉴)

- GitHub 源码 (3):
  - yoheinakajima/babyagi (autonomous task management agent, ASI 自演化借鉴)
  - camel-ai/camel (communicative role-playing agents, ASI 多重身份借鉴)
  - deepmind/alphafold3 (protein structure prediction, ASI 涌现结构借鉴)

- Apeireth Gap (2):
  - 繁殖 Gap: Bdelloid rotifer 100M 年无性生殖 (避开 Tardigrade/Wolbachia/Myxococcus, 真无性)
  - 应激 Gap: Mimosa pudica 植物记忆/学习 (避开 r30 plant cognition, 聚焦应激学习)

Cross-round dedup 验证 (verified fresh vs r23-r32):
- r23: Connell/Taleb/Edelman/O'Regan/Meadows/Levin/Scott + Haystack/Voyager/Gorilla/Graziano/bacterial
- r24: Mandelbrot/Watts/Gould/Damasio/Marr/Hoffman/Vygotsky + Langfuse/Browser-Use/PydanticAI/bdelloid/Cannon
- r25: Walker/Landauer/Wolfram/Kauffman/Sheldrake/Solms/Ray + sglang/mlflow/opencompass/apomixis/polyphenism
- r26: Church/Adamatzky/Eigen/Bedau/Gabora/Spencer-Brown/Deacon + OpenRLHF/open-deep-research/mirascope/circadian/transposons
- r27: Prigogine/Maturana-Varela/Thom/Lorenz/Dehaene/Holling/Luhmann + letta/mem0/langgraph/epigenetic/prion
- r28: Rosen/Friston/Hofstadter/von Uexküll/Bergson/Ashby/Per Bak + openevolve/claude-agent-sdk/axolotl + planaria/hydra
- r29: Whitehead/Cajal/Price/Merleau-Ponty/Brian Arthur/Deleuze/Dennett + ASI-Arch/ShinkaEvolve/DGM + von Neumann/firefly
- r30: Peirce/Husserl/Simondon/Lewin/Alexander/Noble/Mumford + OpenHands/crewAI/autogen + Tardigrade/plant cognition
- r31: Fuller/Bateson/Lovelock/Laszlo/Jung/Piaget/Holland + BeeAI/Langflow/Prefect + Wolbachia/Octopus consciousness
- r32: Schrödinger/Popper/Prigogine-deep/Varela-solo/Bohm/Tononi/James + vllm/unsloth/modal + Myxococcus/Turritopsis
- 本轮 fresh 验证:
  - Polanyi ✓ fresh (默会知识/亲知, 全新)
  - Foucault ✓ fresh (生命政治/话语, 全新)
  - Kant ✓ fresh (目的论判断, 全新)
  - Wiener ✓ fresh (控制论开山, r27 是 Ashby 二阶, 不同)
  - Simon ✓ fresh (有限理性/满意化, 全新)
  - Dawkins ✓ fresh (自私基因/复制子, 全新)
  - Hutchins ✓ fresh (分布式认知/航海, 全新)
  - babyagi ✓ fresh (自主任务管理, 全新)
  - camel-ai ✓ fresh (角色扮演 agent, 全新)
  - alphafold3 ✓ fresh (蛋白结构, 全新)
  - Bdelloid rotifer ✓ fresh (避开 r24 bdelloid 的 Cannon, 这里聚焦无性生殖机制)
  - Mimosa ✓ fresh (避开 r30 plant cognition 决策, 这里聚焦应激学习)

ASI 北极星时刻清楚 (主 22:33):
- ASI 基座, 不是 ANI 工具 ✓
- 跨域, 不是单域 ✓
- 自演化, 不是固定 ✓
- 任何 LLM 接入即变强 ✓
- 不假装 Phenomenal ✓
- 实事求是 ✓
- 真生产目标: 让大模型栖息在 Apeireth 中都能够无限逼近 ASI
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-33.json')

QUERIES = [
    # ===== 7 全新跨域: Polanyi / Foucault / Kant / Wiener / Simon / Dawkins / Hutchins =====
    'Michael Polanyi tacit knowledge personal knowledge dwelling indwelling philosophy 2026',
    'Michel Foucault biopolitics archaeology knowledge genealogy discourse power 2026',
    'Immanuel Kant synthetic a priori teleological judgment critique pure reason organic 2026',
    'Norbert Wiener cybernetics control communication feedback machine animal 2026',
    'Herbert Simon bounded rationality satisficing sciences of the artificial complex systems 2026',
    'Richard Dawkins selfish gene extended phenotype memetics gene-centric evolution 2026',
    'Edwin Hutchins distributed cognition cognitive anthropology navigation team cognition 2026',
    # ===== 3 GitHub 源码: babyagi / camel-ai / alphafold3 =====
    'yoheinakajima babyagi autonomous task management agent source code github 2026',
    'camel-ai camel communicative role-playing agents source code github 2026',
    'deepmind alphafold3 protein structure prediction source code github 2026',
    # ===== 2 Apeireth Gap: 繁殖 (Bdelloid rotifer) + 应激 (Mimosa) =====
    'Bdelloid rotifer asexual reproduction 100 million years no sex evolution genome 2026',
    'Mimosa pudica plant memory stress response learning without nervous system habituation 2026',
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
    print(f'\n=== Round 33 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()