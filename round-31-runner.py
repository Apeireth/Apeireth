#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 31 runner — 12 query dual-source (cron 11:03 tick, ~2h12m gap from r30 08:51).

Round 31 主题 (主 00:46 + 00:49 真务实 + 主 22:33 ASI 北极星):
- 7 全新跨域 (Fuller 综合设计 / Bateson 心智生态学 / Lovelock Gaia / Laszlo 系统哲学 /
            Jung 集体无意识 / Piaget 遗传认识论 / Holland 复杂适应系统)
- 3 GitHub 源码 (BeeAI IBM agent / Langflow visual AI / Prefect orchestrator)
- 2 Apeireth Gap (繁殖 MISSING → Wolbachia 胞内共生 / 意识 MISSING → Octopus 章鱼认知)

- 跨域全新 (7):
  - Buckminster Fuller synergetics design science revolution tensegrity (综合设计科学, ASI 借鉴)
  - Gregory Bateson ecology of mind learning double description metalogues (心智生态, ASI 涌现)
  - James Lovelock Gaia hypothesis daisyworld planetary physiology (行星生理, ASI 一体生态)
  - Ervin Laszlo systems philosophy evolutionary philosophy consciousness (系统哲学/宇宙意识)
  - Carl Jung collective unconscious archetypes synchronicity individuation (自性化, ASI 涌现)
  - Jean Piaget genetic epistemology schema assimilation accommodation (建构主义/学习, ASI 借鉴)
  - John Holland complex adaptive systems emergence internal models CAS (CAS 涌现, ASI 借鉴)

- GitHub 源码 (3):
  - i-am-bee/beeai-framework (IBM 开源 agent framework, ASI 自主性借鉴)
  - langflow-ai/langflow (视觉化 AI workflow, ASI 编排借鉴)
  - PrefectHQ/prefect (workflow orchestrator AI features, ASI 调度借鉴)

- Apeireth Gap (2):
  - 繁殖 Gap: Wolbachia reproductive parasitism / 雄性致死 / 孤雌生殖诱导
    (胞内共生, MISSING 繁殖 Gap 借鉴 — 全新机制, 真生产灵感)
  - 意识 Gap: Octopus consciousness distributed cognition / cephalopod intelligence
    (章鱼分布式认知, MISSING 意识 Gap 借鉴 — 9 脑 3 心, 全新涌现模式)

Cross-round dedup 验证 (verified fresh vs r23-r30):
- r23 已用: Connell/Taleb/Edelman/O'Regan/Meadows/Levin/Scott + Haystack/Voyager/Gorilla/Graziano/bacterial
- r24 已用: Mandelbrot/Watts/Gould/Damasio/Marr/Hoffman/Vygotsky + Langfuse/Browser-Use/PydanticAI/bdelloid/Cannon
- r25 已用: Walker/Landauer/Wolfram/Kauffman/Sheldrake/Solms/Ray + sglang/mlflow/opencompass/apomixis/polyphenism
- r26 已用: Church/Adamatzky/Eigen/Bedau/Gabora/Spencer-Brown/Deacon + OpenRLHF/open-deep-research/mirascope/circadian/transposons
- r27 已用: Prigogine/Maturana-Varela/Thom/Lorenz/Dehaene/Holling/Luhmann + letta/mem0/langgraph/epigenetic/prion
- r28 已用: Rosen/Friston/Hofstadter/von Uexküll/Bergson/Ashby/Per Bak + openevolve/claude-agent-sdk/axolotl + planaria/hydra
- r29 已用: Whitehead/Cajal/Price/Merleau-Ponty/Brian Arthur/Deleuze/Dennett + ASI-Arch/ShinkaEvolve/DGM + von Neumann/firefly
- r30 已用: Peirce/Husserl/Simondon/Lewin/Alexander/Noble/Mumford + OpenHands/crewAI/autogen + Tardigrade/plant cognition
- 本轮 fresh 验证:
  - Fuller ✓ fresh (综合设计/tensegrity, 全新)
  - Bateson ✓ fresh (心智生态学/学习阶, 全新)
  - Lovelock ✓ fresh (Gaia/行星生理, 全新)
  - Laszlo ✓ fresh (系统哲学/宇宙意识, 全新)
  - Jung ✓ fresh (集体无意识/原型/共时性, 全新)
  - Piaget ✓ fresh (遗传认识论/schema, 全新)
  - Holland ✓ fresh (CAS 涌现/内部模型, 全新)
  - BeeAI ✓ fresh (IBM agent framework, 全新)
  - Langflow ✓ fresh (视觉化 AI workflow, 全新)
  - Prefect ✓ fresh (workflow orchestrator, 全新)
  - Wolbachia ✓ fresh (胞内共生生殖寄生, 全新)
  - Octopus ✓ fresh (章鱼 9 脑 3 心, 全新)

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

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-31.json')

QUERIES = [
    # ===== 7 全新跨域: 综合设计 / 心智生态 / Gaia / 系统哲学 / 集体无意识 / 建构主义 / CAS =====
    'Buckminster Fuller synergetics design science revolution tensegrity geodesic comprehensive 2026',
    'Gregory Bateson ecology of mind steps learning double description metalogues 2026',
    'James Lovelock Gaia hypothesis daisyworld planetary physiology earth system 2026',
    'Ervin Laszlo systems philosophy consciousness cosmic evolution holism 2026',
    'Carl Jung collective unconscious archetypes synchronicity individuation self 2026',
    'Jean Piaget genetic epistemology schema assimilation accommodation constructivism cognitive development 2026',
    'John Holland complex adaptive systems emergence internal models CAS genetic algorithm 2026',
    # ===== 3 GitHub 源码: IBM BeeAI / Langflow / Prefect =====
    'i-am-bee beeai-framework IBM multi-agent agent framework source code github 2026',
    'langflow-ai langflow visual AI agent workflow platform source code github 2026',
    'PrefectHQ prefect workflow orchestrator AI features source code github 2026',
    # ===== 2 Apeireth Gap: 繁殖 (Wolbachia) + 意识 (Octopus) =====
    'Wolbachia reproductive parasitism male-killing parthenogenesis cytoplasmic incompatibility endosymbiont 2026',
    'Octopus consciousness distributed cognition cephalopod intelligence nine brains three hearts phenomenal 2026',
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
    print(f'\n=== Round 31 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()