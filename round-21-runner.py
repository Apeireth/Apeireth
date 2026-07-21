#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 21 runner — 12 query dual-source (主 14:48 cron 2h tick, 自决 + 1h56m gap).

Round 21 主题: 7 全新哲学/认知/复杂/生命/数学 (避开 r15/16/17/18/19/20) + 3 GitHub 源码深读 + 2 繁殖/意识 Gap

- 跨域全新 (7):
  - Tarde 单子论/模仿律 (社会微观物理)
  - Latour 行动者网络理论 (非人行动者/物的政治生态)
  - Andy Clark 延展心智/4E cognition (区别于预测编码 Friston)
  - Girard 线性逻辑/证明网 (资源敏感计算, consumable resources)
  - Carlsson 持续同调/TDA (拓扑数据分析/数据形状)
  - Wachtershauser 铁硫世界/代谢优先起源 (表面化学)
  - Pei Wang NARS 非公理推理系统 (真正 AGI 架构)

- GitHub 源码深读 (3):
  - agno-agi/agno 多 agent 框架
  - camel-ai/camel 交流 agent 角色扮演 + inception prompting
  - langflow-ai/langflow 可视化 agent 流程

- Apeireth Gap (2): 繁殖 (合成生物学最小细胞 + LLM 自我复制) + 意识 (Anil Seth 内感推理 + 生物自然主义)

Cross-round dedup 避开:
- r15: Prigogine/Kauffman/stigmergy/Bateson/Turing morphogenesis/Lovelock/Friston/letta/MetaGPT/Devin/IdentityCard/endosymbiosis
- r16: Schrödinger/Merleau-Ponty/Varela-Thompson/Ostrom/lambda/morphogenetic/niche construction/langgraph/openevolve/claude-agent-sdk/epigenetic/spore
- r17: Friston/category topos/IIT Tononi/Maturana+vF/Gaia+Daisyworld/GWT/SOC Bak/ASI-Arch/ShinkaEvolve/DGM/HGT virolution/chemotaxis
- r18: Penrose Orch-OR/Dennett intentional/Pribram holonomic/Haken synergetics/Spinoza conatus/Wolfram NKS/Modern Hopfield/openai-agents-python/mem0/langchain LCEL/prion/quorum sensing
- r19: Piaget/Hofstadter strange loops/Tomasello/Luhmann/Whitehead/Dewey/Stiegler/autogen/crewAI/MetaGPT/Tierra Avida/Koch PCI Dehaene Rosenthal HOT
- r20: Canguilhem/Simondon/Bergson/Deleuze/Nancy/Heidegger Zollikon/Sclavi/smol-course/Mojo MAX/OpenHands/phenotypic plasticity/transgenerational epigenetic
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-21.json')

QUERIES = [
    # ===== 7 全新跨域哲学/认知/复杂/生命/数学 =====
    'Gabriel Tarde monadology imitation laws social microphysics micro-sociology 2026',
    'Bruno Latour actor-network theory reassembling the social non-human agency materiality 2026',
    'Andy Clark extended mind thesis 4E cognition cognitive scaffolding naturalism 2026',
    'Girard linear logic proof nets ludics resource-sensitive computation philosophy 2026',
    'Carlsson persistent homology topological data analysis shape data manifold topology 2026',
    'Gunther Wachtershauser iron-sulfur world metabolism-first origin of life surface chemistry 2026',
    'Pei Wang NARS non-axiomatic reasoning system AGI architecture inference learning github 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 真读源码不止 README) =====
    'agno-agi agno multi-agent framework source code architecture memory knowledge github 2026',
    'camel-ai camel communicative agents role-playing inception prompting source code github 2026',
    'langflow-ai langflow visual agent flow builder source code architecture github 2026',
    # ===== 2 Apeireth Gap: 繁殖 + 意识 (主 17:46 MISSING 12 生命特征) =====
    'JCVI minimal cell synthetic biology self-replicating protocell engineering genome 2026',
    'Anil Seth interoceptive inference biological naturalism consciousness theory 2026',
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
    print(f'\n=== Round 21 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()