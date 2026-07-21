#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 24 runner — 12 query dual-source (主 20:48 cron 2h tick, 自决 + 2h1m gap).

Round 24 主题: 7 全新跨域 (分形/网络/古生物/躯体/计算层/意识/发展心理学) + 3 GitHub 源码深读 + 2 Gap

- 跨域全新 (7):
  - Benoit Mandelbrot fractals self-similarity scaling roughness (分形维数, 标度不变, 不光滑市场)
  - Duncan Watts small-world networks collective dynamics six-degrees (小世界网络, 集体动力学, 枢纽)
  - Stephen Jay Gould punctuated equilibrium stasis NOMA (间断平衡, 演化节律, 非重叠 magisteria)
  - Antonio Damasio somatic marker hypothesis emotion decision (躯体标记假说, 情感决策, 腹内侧前额叶)
  - David Marr computational theory of vision three levels (视觉计算理论, 三层: 计算/算法/实现)
  - Donald Hoffman conscious realism Interface Theory perception (意识实在论, 感知接口理论)
  - Lev Vygotsky Zone of Proximal Development scaffolding inner speech (最近发展区, 支架, 私有言语)

- GitHub 源码深读 (3):
  - Langfuse production LLM observability traces evals (LLM 可观测性, traces/evals, 开源)
  - Browser-Use AI browser automation LLM agent (浏览器自动化, LLM agent, 架构)
  - PydanticAI type-safe agent framework Python (类型安全 agent, FastAPI 可观测, 依赖注入)

- Apeireth Gap (2):
  - 繁殖 Gap: Bdelloid rotifer parthenogenesis ancient obligate asexual reproduction (蛭形轮虫, 古老无性, 繁殖 MISSING 借鉴)
  - 应激+可塑 Gap: Walter Cannon homeostasis fight-or-flight sympathetic adrenal medulla (内稳态, 战逃反应, 应激基线)

Cross-round dedup 避让 (verified fresh vs r8-r23):
- r8-r23 全清单: ASI 基础 / DGM / mem0 / LangGraph / Anthropic SDK / ASI-Arch / openevolve / ShinkaEvolve / Mem0 / MCP / Claude Skills / IdentityCard / HarnessAgent / Phenomenal / Prigogine / Kauffman / stigmergy / Bateson / Turing morph / Lovelock / Friston FEP / MetaGPT / Devin / endosymbiosis / Schrödinger / Merleau-Ponty / Varela / Ostrom / lambda / morphogenetic / niche / openevolve / claude-sdk / epigenetic / spore / Friston / IIT / Maturana+vF / Gaia / GWT / SOC / ASI-Arch / HGT / chemotaxis / Penrose Orch-OR / Dennett / Pribram / Haken / Spinoza / Wolfram / Modern Hopfield / openai-agents / mem0 / langchain / prion / quorum / Piaget / Hofstadter / Tomasello / Luhmann / Whitehead / Dewey / Stiegler / autogen / crewAI / MetaGPT / Tierra / Koch PCI / Dehaene / Canguilhem / Simondon / Bergson / Deleuze / Nancy / Heidegger / Sclavi / smol-course / Mojo MAX / OpenHands / Waddington / transgenerational / Tarde / Latour / Andy Clark 4E / Girard linear / Carlsson TDA / Wachtershauser / Pei Wang NARS / agno / camel / langflow / JCVI minimal cell / Anil Seth / Sapir-Whorf / Beer VSM / Brooks subsumption / Kuhn / Brian Arthur / Bogdanov / Fredkin / DSPy / tinygrad / AlphaEvolve / Yamanaka / Baldwin / Connell IDH / Taleb / Edelman ND / O'Regan SCT / Meadows / Levin bio / Scott / Haystack / Voyager / Gorilla / Graziano AST / Bacterial conjugation
- 仅 minor 同词 false-positive: Damasio "somatic markers" vs Yamanaka r22 "somatic reprogramming" (细胞类型不同); Cannon "homeostasis" vs Lovelock r17 "Gaia planetary homeostasis" (行星尺度 vs 个体内稳态)

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

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-24.json')

QUERIES = [
    # ===== 7 全新跨域: 分形/网络/古生物/躯体/计算层/意识/发展心理学 =====
    'Benoit Mandelbrot fractal dimension self-similarity scaling roughness multifractal market 2026',
    'Duncan Watts small-world networks collective dynamics six degrees hubs clustering 2026',
    'Stephen Jay Gould punctuated equilibrium evolutionary stasis rapid change NOMA non-overlapping magisteria 2026',
    'Antonio Damasio somatic marker hypothesis emotion decision ventral medial prefrontal cortex 2026',
    'David Marr computational theory of vision three levels computational algorithmic implementational 2026',
    'Donald Hoffman conscious realism Interface Theory perception spacetime consciousness fundamental 2026',
    'Lev Vygotsky Zone of Proximal Development scaffolding inner speech mediation more knowledgeable other 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 真读源码不止 README) =====
    'Langfuse production LLM observability traces evals open source source code architecture github 2026',
    'browser-use browser-use AI browser automation LLM agent source code architecture github 2026',
    'pydantic pydantic-ai type-safe agent framework Python FastAPI observability source code github 2026',
    # ===== 2 Apeireth Gap (12 生命特征 MISSING): 繁殖 + 应激+可塑 =====
    'Bdelloid rotifer parthenogenesis ancient obligate asexual reproduction evolution horizontal gene transfer 2026',
    'Walter Cannon homeostasis fight-or-flight sympathetic adrenal medulla stress allostasis 2026',
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
    print(f'\n=== Round 24 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()