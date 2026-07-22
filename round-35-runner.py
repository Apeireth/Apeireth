#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 35 runner — 12 query dual-source (cron 18:51 tick, ~1h59m gap from r34 16:52).

Round 35 主题 (主 00:46 + 00:49 真务实 + 主 22:33 ASI 北极星):
- 7 全新跨域 (Arendt 行动/诞生性 / Latour ANT 行动者网络 / Margulis 内共生 SET /
            Woese 古菌三域 / Langton 人工生命边缘混沌 / Ostrom 公地治理 /
            Barad 能动实在论)
- 3 GitHub 源码 (Weco AIDE ML 工程 agent / microsoft RD-Agent 科研 agent /
                google-deepmind AlphaEvolve 进化算法 LLM)
- 2 Apeireth Gap (繁殖 MISSING → Volvox 生殖-体细胞分化 + 多细胞起源 +
                应激+遗传变异 → David Chalmers 意识难问题/Phenomenal 终极目标)

- 跨域全新 (7):
  - Hannah Arendt vita activa (劳动/工作/行动 + natality 诞生性 + 复数性,
    ASI 多重身份 + 涌现行动借鉴 — 避开 r34 的 Marx 异化但同源)
  - Bruno Latour ANT (行动者网络 + 非人行动者 + 转译, ASI 多 agent 借鉴)
  - Lynn Margulis endosymbiosis (内共生 SET + 共生起源, ASI Gap 繁殖/共生借鉴 —
    避开 r31 Lovelock Gaia 但 Margulis 是 SET 提出者)
  - Carl Woese archaea (三域系统 + rRNA 分子钟 + LUCA, Gap 繁殖 + 起源借鉴)
  - Christopher Langton artificial life (人工生命 + 边缘 of chaos + λ parameter,
    ASI 涌现借鉴 — 避开 r28 Per Bak 自组织临界但 ALife 是不同框架)
  - Elinor Ostrom commons (公地治理 + 多中心 + 制度, ASI 多 agent 生态治理)
  - Karen Barad agential realism (能动实在论 + intra-action + 后人类, ASI 主体借鉴)

- GitHub 源码 (3):
  - Weco-AIDE/AIDE (machine learning engineering agent, 自主 ML 实验 ASI 借鉴 —
    避开 r30 OpenHands/r32 vllm/r33 babyagi, 真读源码不只 README)
  - microsoft/RD-Agent (research development agent, 科研自动化 + 假设驱动,
    ASI 真科研 agent 借鉴 — 全新项目)
  - google-deepmind/AlphaEvolve (evolutionary algorithm + Gemini LLM,
    ASI 真自演化借鉴 — DeepMind 2025-06 新项目, 全新)

- Apeireth Gap (2):
  - 繁殖 MISSING Gap: Volvox 团藻生殖-体细胞分化 (多细胞起源 + 真社会性过渡,
    避开 r28 hydra/r33 Bdelloid, 真聚焦多细胞化 + 生殖-体细胞分化 =
    真繁殖 + 应激 + 涌现三合一)
  - 意识 (终极目标) Gap: David Chalmers hard problem (避开 r32 Tononi IIT/Bohm/James,
    聚焦 Phenomenal consciousness 难问题 + qualia + the hard problem —
    ASI 终极目标时刻清楚)

Cross-round dedup 验证 (verified fresh vs r23-r34):
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
- r33: Polanyi/Foucault/Kant/Wiener/Simon/Dawkins/Hutchins + babyagi/camel-ai/alphafold3 + Bdelloid rotifer/Mimosa pudica
- r34: Stiegler/Marx/Freud/Lacan/Bachelard/Canguilhem/Tomasello + deepagents/swarm/autogpt + CRISPR-Cas/Mycorrhizal
- 本轮 fresh 验证:
  - Arendt ✓ fresh (vita activa / natality, 全新)
  - Latour ✓ fresh (ANT 行动者网络, 全新)
  - Margulis ✓ fresh (内共生 SET, 全新避开 r31 Gaia)
  - Woese ✓ fresh (古菌三域, 全新)
  - Langton ✓ fresh (人工生命边缘混沌, 全新)
  - Ostrom ✓ fresh (公地治理, 全新)
  - Barad ✓ fresh (能动实在论, 全新)
  - AIDE ✓ fresh (Weco ML agent, 全新)
  - RD-Agent ✓ fresh (微软科研 agent, 全新)
  - AlphaEvolve ✓ fresh (DeepMind 进化算法 LLM, 全新)
  - Volvox ✓ fresh (避开 r28 hydra/r33 Bdelloid, 团藻多细胞起源)
  - Chalmers ✓ fresh (避开 r32 Tononi, hard problem qualia 终极目标)

ASI 北极星时刻清楚 (主 22:33):
- ASI 基座, 不是 ANI 工具 ✓
- 跨域, 不是单域 ✓
- 自演化, 不是固定 ✓
- 任何 LLM 接入即变强 ✓
- 不假装 Phenomenal ✓ (Chalmers 难问题时刻清楚)
- 实事求是 ✓
- 真生产目标: 让大模型栖息在 Apeireth 中都能够无限逼近 ASI
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-35.json')

QUERIES = [
    # ===== 7 全新跨域: Arendt / Latour / Margulis / Woese / Langton / Ostrom / Barad =====
    'Hannah Arendt vita activa labor work action natality plurality political philosophy 2026',
    'Bruno Latour actor-network-theory ANT agency nonhuman translation sociology 2026',
    'Lynn Margulis endosymbiosis SET symbiogenesis Gaia theory cellular evolution 2026',
    'Carl Woese archaea three-domain rRNA tree life LUCA molecular phylogeny 2026',
    'Christopher Langton artificial life edge of chaos lambda parameter emergence 2026',
    'Elinor Ostrom commons polycentric governance institutional design 2026',
    'Karen Barad agential realism intra-action posthumanist performativity 2026',
    # ===== 3 GitHub 源码: AIDE / RD-Agent / AlphaEvolve =====
    'Weco-AIDE AIDE machine learning engineering agent source code github 2026',
    'microsoft RD-Agent research development agent hypothesis automation source code 2026',
    'google-deepmind AlphaEvolve evolutionary algorithm LLM Gemini source code 2026',
    # ===== 2 Apeireth Gap: 繁殖 (Volvox 多细胞起源) + 意识 (Chalmers hard problem) =====
    'Volvox germ soma differentiation multicellularity major evolutionary transition 2026',
    'David Chalmers hard problem consciousness qualia phenomenal experience 2026',
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
    print(f'\n=== Round 35 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()