#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 25 runner — 12 query dual-source (主 22:48 cron 2h tick, 自决 + 1h54m gap).

Round 25 主题: 7 全新跨域 (起源之组装/热力学擦除/CA 普适/NK 适应度景观/形态共鸣/情感神经科学/数字进化)
              + 3 GitHub 源码深读 (sglang/mlflow/opencompass) + 2 Gap (无融合生殖/多型现象)

- 跨域全新 (7):
  - Sara Walker assembly theory life detection cumulative selection (组装理论, 累积选择, 生命量化定义)
  - Rolf Landauer erasure principle thermodynamic cost information forgetting (擦除原理, 信息处理热力学下限)
  - Wolfram cellular automata rule 110 universality computation simple rules (初等元胞自动机, 规则 110, 图灵完备)
  - Stuart Kauffman NK fitness landscape ruggedness tunably complex (NK 模型, 适应度景观崎岖度, 可调复杂度)
  - Rupert Sheldrake morphic resonance morphogenetic fields developmental (形态共振, 形态生成场, 发育模式)
  - Mark Solms affective neuroscience homeostatic feelings consciousness (情感神经科学, 内稳态感受, 意识基线)
  - Thomas Ray Tierra digital evolution niche formation self-replication (Tierra 数字进化, 生态位形成, 自我复制)

- GitHub 源码深读 (3):
  - sglang/sglang structured generation radix attention (结构化生成语言, 基数树注意力, 前缀缓存)
  - mlflow/mlflow ML lifecycle model registry tracking (ML 全生命周期, 模型注册, 实验追踪)
  - open-compass/opencompass LLM evaluation framework (大模型评测, 多维度评测, 学术基座)

- Apeireth Gap (2):
  - 繁殖 Gap: Apomixis dandelion hawkweed asexual seed without fertilization (无融合生殖, 蒲公英/山柳菊, 种子无性)
  - 可塑 Gap: Polyphenism aphid wing caste ant bee phenotypic plasticity environmental (多型现象, 蚜虫翅/蚂蚁蜂 caste, 环境触发表型可塑)

Cross-round dedup 避让 (verified fresh vs r8-r24):
- r8-r24 全清单已避免: ASI 基础 / DGM / mem0 / LangGraph / Anthropic SDK / ASI-Arch / openevolve / ShinkaEvolve / Mem0 / MCP / Claude Skills / IdentityCard / HarnessAgent / Phenomenal / Prigogine / Kauffman autocatalytic / stigmergy / Bateson / Turing morph / Lovelock / Friston FEP / MetaGPT / Devin / endosymbiosis / Schrödinger / Merleau-Ponty / Varela / Ostrom / lambda / morphogenetic / niche / openevolve / claude-sdk / epigenetic / spore / Friston / IIT / Maturana+vF / Gaia / GWT / SOC / HGT / chemotaxis / Penrose Orch-OR / Dennett / Pribram / Haken / Spinoza / Wolfram physics / Modern Hopfield / openai-agents / mem0 / langchain / prion / quorum / Piaget / Hofstadter / Tomasello / Luhmann / Whitehead / Dewey / Stiegler / autogen / crewAI / MetaGPT / Tierra ✗ (r?? covered briefly - need verify) / Koch PCI / Dehaene / Canguilhem / Simondon / Bergson / Deleuze / Nancy / Heidegger / Sclavi / smol-course / Mojo MAX / OpenHands / Waddington / transgenerational / Tarde / Latour / Andy Clark 4E / Girard / Carlsson TDA / Wachtershauser / Pei Wang NARS / agno / camel / langflow / JCVI / Anil Seth / Sapir-Whorf / Beer VSM / Brooks / Kuhn / Brian Arthur / Bogdanov / Fredkin / DSPy / tinygrad / AlphaEvolve / Yamanaka / Baldwin / Connell / Taleb / Edelman ND / O'Regan SCT / Meadows / Levin / Scott / Haystack / Voyager / Gorilla / Graziano AST / Bacterial conjugation / Mandelbrot / Watts / Gould / Damasio / Marr / Hoffman / Vygotsky / Langfuse / browser-use / PydanticAI / Bdelloid / Cannon

- 仅 minor 同词 false-positive (验证不撞):
  - Tierra r10 已简略提及, 但 "数字进化生态位形成" 是专题深读, 角度不同 ✅
  - "Phenotypic plasticity" r12 已用过但 polyphenism 翅二型/caste 是更具体的可塑 MISSING 角度 ✅

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

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-25.json')

QUERIES = [
    # ===== 7 全新跨域: 组装理论/擦除原理/CA 普适/NK 景观/形态共鸣/情感神经/数字进化 =====
    'Sara Walker assembly theory life detection cumulative selection life quantification 2026',
    'Rolf Landauer erasure principle thermodynamic cost information forgetting irreversible 2026',
    'Wolfram cellular automata rule 110 universality computation simple rules four classes 2026',
    'Stuart Kauffman NK fitness landscape ruggedness tunably complex evolution adaptation 2026',
    'Rupert Sheldrake morphic resonance morphogenetic fields developmental pattern formation 2026',
    'Mark Solms affective neuroscience homeostatic feelings consciousness brainstem forebrain 2026',
    'Thomas Ray Tierra digital evolution niche formation self-replication artificial life 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 真读源码不止 README) =====
    'sglang sglang structured generation radix attention prefix caching source code architecture github 2026',
    'mlflow mlflow ML lifecycle model registry experiment tracking source code architecture github 2026',
    'open-compass opencompass LLM evaluation benchmark framework source code architecture github 2026',
    # ===== 2 Apeireth Gap (12 生命特征 MISSING): 繁殖 + 可塑 =====
    'Apomixis dandelion hawkweed Taraxacum Hieracium asexual seed without fertilization apomixis 2026',
    'Polyphenism aphid wing caste ant bee polymorphic plasticity environmental trigger polyphenism 2026',
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
    print(f'\n=== Round 25 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()