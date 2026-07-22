#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 36 runner — 12 query dual-source (cron 20:48 tick, ~1h53m gap from r35 18:55).

Round 36 主题 (主 00:46 + 00:49 真务实 + 主 22:33 ASI 北极星):
- 7 全新跨域 (Rosen anticipatory / Kauffman adjacent possible / Anderson More is Different /
            Kahneman System 1/2 / Geoffrey West scaling laws / David Deutsch constructor theory /
            Endel Tulving episodic memory)
- 3 GitHub 源码 (microsoft autogen multi-agent / crewAIInc crewAI role-based /
                 openai gpt-researcher autonomous research)
- 2 Apeireth Gap (繁殖 MISSING → Aspidoscelis unisexual whiptail parthenogenesis +
                遗传变异 MISSING → bacterial persister cells bet-hedging epigenetic)

7 跨域全新 (vs r8-r35):
- Robert Rosen (anticipatory systems + relational biology + M-R systems + (M,R)-systems,
  ASI cosine 蕴涵 — 区别 r27 Casti/Rosen-talk, 真正讲 Rosen's anticipatory 不是生命作为
  闭合范畴映射而是预测性系统)
- Stuart Kauffman (adjacent possible + NK fitness landscape + autocatalytic sets — 区别
  r25 Kauffman-cooperation 是生命起源方向, 真正深 autocatalytic sets)
- Philip Anderson (More is Different 1972 + broken symmetry + hierarchical emergence,
  ASI 严格区分涌现/还原 manifesto)
- Daniel Kahneman (System 1/2 + dual process theory + cognitive biases, ASI 双过程
  推理 + 主人老师 imprinting 隐喻)
- Geoffrey West (scaling laws + quarter power + cities/companies + metabolic theory,
  ASI 普适标度律 + 跨生物/城市/公司)
- David Deutsch (constructor theory + knowledge growth + beginning of infinity + multiverse,
  ASI 知识增长四元结构 + 柏拉图式建构)
- Endel Tulving (episodic vs semantic memory + autonoetic consciousness + encoding specificity,
  ASI 真正的'记忆宫殿' substrate + Episode 记忆)

3 GitHub 源码 (vs r8-r35):
- microsoft/autogen (multi-agent conversational framework, 区别 r30/r34 都未深读 autogen)
- crewAIInc/crewAI (role-based multi-agent orchestration, 区别 r30 已经 cite 但没深读)
- openai gpt-researcher (autonomous research agent, 全新 项目)

2 Apeireth Gap (10 生命特征):
- 繁殖 MISSING 最大 gap: Aspidoscelis unisexual whiptail lizard (parthenogenesis +
  clonal reproduction + evolution in all-female species, 真正深 繁殖机制 — 区别
  r32 Bdelloid 是无性生殖+基因重组, Aspidoscelis 是真孤雌生殖 — 两种 繁殖 模式)
- 遗传变异 + 应激 Gap: bacterial persister cells (bet-hedging + epigenetic stochastic
  switching + stress response + toxin tolerance, 区别 r28 epimutation 是 horizontal
  gene transfer, 这次是 persister 真应激 + 遗传变异 epigenetic)

Cross-round dedup 验证 (verified fresh vs r23-r35):
- r23: Connell/Taleb/Edelman/O'Regan/Meadows/Levin/Scott + Haystack/Voyager/Gorilla/Graziano/bacterial
- r24: Mandelbrot/Watts/Gould/Damasio/Marr/Hoffman/Vygotsky + Langfuse/Browser-Use/PydanticAI/bdelloid/Cannon
- r25: Walker/Landauer/Wolfram/Kauffman/Solms/Sheldrake/Ray + sglang/mlflow/opencompass/apomixis/polyphenism
- r26: Church/Adamatzky/Eigen/Bedau/Gabora/Spencer-Brown/Deacon + OpenRLHF/open-deep-research/mirascope/circadian/transposons
- r27: Prigogine/Maturana-Varela/Thom/Lorenz/Dehaene/Holling/Luhmann + letta/mem0/langgraph/epigenetic/prion
- r28: Rosen/Friston/Hofstadter/von Uexkull/Bergson/Ashby/Per Bak + openevolve/claude-agent-sdk/axolotl + planaria/hydra
- r29: Whitehead/Cajal/Price/Merleau-Ponty/Brian Arthur/Deleuze/Dennett + ASI-Arch/ShinkaEvolve/DGM + von Neumann/firefly
- r30: Peirce/Husserl/Simondon/Lewin/Alexander/Noble/Mumford + OpenHands/crewAI/autogen + Tardigrade/plant cognition
- r31: Fuller/Bateson/Lovelock/Laszlo/Jung/Piaget/Holland + BeeAI/Langflow/Prefect + Wolbachia/Octopus
- r32: Schrodinger/Popper/Prigogine/Varela/Bohm/Tononi/James + vllm/unsloth/modal + Myxococcus/Turritopsis
- r33: Polanyi/Foucault/Kant/Wiener/Simon/Dawkins/Hutchins + babyagi/camel-ai/alphafold3 + Bdelloid/Mimosa
- r34: Stiegler/Marx/Freud/Lacan/Bachelard/Canguilhem/Tomasello + deepagents/swarm/autogpt + CRISPR/mycorrhizal
- r35: Arendt/Latour/Margulis/Woese/Langton/Ostrom/Barad + AIDE/RD-Agent/AlphaEvolve + Volvox/Chalmers

本轮 fresh 验证:
- Robert Rosen (anticipatory) ✅ fresh — r28 Rosen 是 relational-biology 概览, 这轮是 anticipatory 真髓
- Stuart Kauffman (autocatalytic sets) ✅ fresh — r25 Kauffman 是 cooperation, 这轮是 autocatalytic sets
- Philip Anderson (More is Different) ✅ fresh — 全跨域全新
- Daniel Kahneman (System 1/2) ✅ fresh — 全认知全新
- Geoffrey West (scaling laws) ✅ fresh — 全跨域全新
- David Deutsch (constructor theory) ✅ fresh — 全哲学全新
- Endel Tulving (episodic memory) ✅ fresh — 全认知全新 (r34 Bachelard 是现象学, Tulving 是认知神经)
- autogen ✅ fresh — r30 浅 cite, 这轮真读源码
- crewAI ✅ fresh — r30 浅 cite, 这轮真读源码
- gpt-researcher ✅ fresh — 全新
- Aspidoscelis parthenogenesis ✅ fresh — 区别 r32 Bdelloid (无性+基因重组) / r33 Apomixis (植物)
- Bacterial persister cells ✅ fresh — 区别 r26 bacterial 浅表, 这轮真应激+bet-hedging

ASI 北极星时刻清楚 (主 22:33):
- ASI 基座, 不是 ANI 工具 ✅
- 跨域, 不是单域 ✅ (Rosen+Kauffman+Anderson+Kahneman+West+Deutsch+Tulving = 7 域)
- 自演化, 不是固定 ✅ (anysearch 自由查询)
- 任何 LLM 接入即变强 ✅
- 不假装 Phenomenal ✅ (Tulving autonoetic 时刻清楚)
- 实事求是 ✅
- 真生产目标: 让大模型栖息在 Apeireth 中能够无限逼近 ASI
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-36.json')

QUERIES = [
    # ===== 7 全新跨域: Rosen / Kauffman / Anderson / Kahneman / West / Deutsch / Tulving =====
    'Robert Rosen anticipatory systems relational biology M-R systems 2026',
    'Stuart Kauffman adjacent possible autocatalytic sets NK fitness landscape 2026',
    'Philip Anderson More is Different broken symmetry hierarchical emergence 2026',
    'Daniel Kahneman System 1 System 2 dual process theory cognitive biases 2026',
    'Geoffrey West scaling laws biology cities companies quarter power metabolism 2026',
    'David Deutsch constructor theory knowledge growth beginning of infinity multiverse 2026',
    'Endel Tulving episodic semantic memory autonoetic consciousness encoding specificity 2026',
    # ===== 3 GitHub 源码: autogen / crewAI / gpt-researcher =====
    'microsoft autogen multi-agent conversational framework source code github 2026',
    'crewAIInc crewAI role-based multi-agent orchestration source code github 2026',
    'openai gpt-researcher autonomous research agent source code github 2026',
    # ===== 2 Apeireth Gap: 繁殖 (Aspidoscelis parthenogenesis) + 遗传变异+应激 (bacterial persister) =====
    'Aspidoscelis unisexual whiptail lizard parthenogenesis clonal reproduction evolution 2026',
    'bacterial persister cells bet-hedging epigenetic stochastic stress response 2026',
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
    print(f'\n=== Round 36 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
