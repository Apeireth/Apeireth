#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 22 runner — 12 query dual-source (主 16:48 cron 2h tick, 自决 + 1h55m gap).

Round 22 主题: 7 全新语言/系统/认知/哲学/经济/数字 (避 r15-r21) + 3 GitHub 源码深读 + 2 繁殖/遗传 Gap

- 跨域全新 (7):
  - Sapir-Whorf 语言相对性 (语言认知)
  - Stafford Beer Viable System Model VSM (组织控制论, 极 ASI 4 范式)
  - Rodney Brooks subsumption architecture (具身 AI, intelligence without representation)
  - Thomas Kuhn 结构科学革命 (范式转换, ASI 哲学映射)
  - Brian Arthur increasing returns (复杂经济学, 自我强化机制)
  - Alexander Bogdanov tectology 万能组织学 (20c 系统论先驱, 早 Bertalanffy)
  - Edward Fredkin digital physics (信息本体论, 数字哲学)

- GitHub 源码深读 (3):
  - DSPy (Stanford NLP, 程序化 LLM 调用, BootstrapFewShot 优化器)
  - tinygrad (George Hotz 极简 DL 框架, <10K 行实现 PyTorch 主力 ops)
  - AlphaEvolve (DeepMind 2025, 进化 + LLM 自主 code evolution)

- Apeireth Gap (2):
  - 繁殖+可塑 Gap: Yamanaka iPSC induced pluripotency (4 转录因子 Oct4/Sox2/Klf4/cMyc 重编程)
  - 遗传+应激 Gap: Baldwin 效应 + 现代 Lamarckian 进化 (表观遗传连接器)

Cross-round dedup 避让:
- r15: Prigogine/Kauffman/stigmergy/Bateson/Turing morph/Lovelock/Friston/letta/MetaGPT/Devin/IdentityCard/endosymbiosis
- r16: Schrödinger/Merleau-Ponty/Varela/Ostrom/lambda/morphogenetic/niche/langgraph/openevolve/claude-sdk/epigenetic/spore
- r17: Friston/topos IIT/Maturana+vF/Gaia/GWT/SOC/ASI-Arch/ShinkaEvolve/DGM/HGT/chemotaxis
- r18: Penrose Orch-OR/Dennett/Pribram/Haken/Spinoza/Wolfram NKS/Modern Hopfield/openai-agents/mem0/langchain LCEL/prion/quorum
- r19: Piaget/Hofstadter loops/Tomasello/Luhmann/Whitehead/Dewey/Stiegler/autogen/crewAI/MetaGPT/Tierra/Koch PCI Dehaene
- r20: Canguilhem/Simondon/Bergson/Deleuze/Nancy/Heidegger Zollikon/Sclavi/smol-course/Mojo MAX/OpenHands/Waddington/transgenerational
- r21: Tarde/Latour/Andy Clark 4E/Girard linear/Carlsson TDA/Wachtershauser/Pei Wang NARS/agno/camel/langflow/JCVI minimal cell/Anil Seth
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-22.json')

QUERIES = [
    # ===== 7 全新跨域: 语言/系统/认知/哲学/经济/数字 =====
    'Edward Sapir Benjamin Whorf linguistic relativity hypothesis language shapes thought 2026',
    'Stafford Beer Viable System Model VSM organizational cybernetics recursion viability 2026',
    'Rodney Brooks subsumption architecture embodied AI intelligence without representation robotics 2026',
    'Thomas Kuhn structure scientific revolutions paradigm incommensurability normal science 2026',
    'Brian Arthur increasing returns complexity economics self-reinforcing mechanisms path dependence 2026',
    'Alexander Bogdanov tectology universal organizational science systems theory precursor 2026',
    'Edward Fredkin digital physics everything is information ontology cellular automata universe 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 真读源码不止 README) =====
    'DSPy Stanford NLP programmatic LM pipeline optimizers BootstrapFewShot source code github 2026',
    'tinygrad George Hotz minimal deep learning framework source code github architecture 2026',
    'DeepMind AlphaEvolve evolutionary code agent LLM automated discovery source code github 2026',
    # ===== 2 Apeireth Gap: 繁殖/可塑 + 遗传/应激 =====
    'Yamanaka induced pluripotent stem cells iPSC somatic reprogramming Oct4 Sox2 Klf4 cMyc factors 2026',
    'Baldwin effect modern Lamarckian genetic assimilation epigenetic inheritance evolution 2026',
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
    print(f'\n=== Round 22 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
