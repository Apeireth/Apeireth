#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 20 runner — 12 query dual-source (主 12:48 cron 2h tick, 自决 + 1h48m gap).

Round 20 主题: 7 全新哲学-生命学 (避开 r15/16/17/18/19) + 3 GitHub 源码深读 + 2 可塑性/表观遗传 Gap

- 跨域新哲学 (7):
  - Canguilhem 生命规范 / Simondon 个体化 / Bergson 绵延与直觉
  - Deleuze 差异与重复 / Nancy 无效社群 / Heidegger Zollikon 坏掉的工具
  - Sclavi 创意资本

- GitHub 源码深读 (3):
  - huggingface smol-course / modular Mojo MAX / OpenHands OpenHands

- Apeireth Gap (2): 可塑性 (phenotypic plasticity / canalization) + 表观遗传跨代

Cross-round dedup 避开:
- r15: Prigogine/Kauffman/stigmergy/Bateson/Turing morphogenesis/Lovelock/Friston/letta/MetaGPT/Devin/IdentityCard/endosymbiosis
- r16: Schrödinger/Merleau-Ponty/Varela-Thompson/Ostrom/lambda/morphogenetic/niche construction/langgraph/openevolve/claude-agent-sdk/epigenetic/spore
- r17: Friston/category topos/IIT Tononi/Maturana+vF/Gaia+Daisyworld/GWT/SOC Bak/ASI-Arch/ShinkaEvolve/DGM/HGT virolution/chemotaxis
- r18: Penrose Orch-OR/Dennett intentional/Pribram holonomic/Haken synergetics/Spinoza conatus/Wolfram NKS/Modern Hopfield/openai-agents-python/mem0/langchain LCEL/prion/quorum sensing
- r19: Piaget constructivism/Hofstadter strange loops/Tomasello/Luhmann/Whitehead process/Dewey pragmatism/Stiegler/autogen/crewAI/MetaGPT/von Neumann Tierra Avida/Koch PCI Dehaene signature Rosenthal HOT
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-20.json')

QUERIES = [
    # ===== 7 全新哲学-生命学 (主 22:33 自决 + 主 17:46 生命特征 gap) =====
    'Canguilhem philosophy of biology vital norms normativity creativity health disease life AI 2026',
    'Gilbert Simondon individuation pre-individual trans-individual associated milieu technical objects 2026',
    'Bergson duration creative evolution intuition matter memory Time and Free Will 2026',
    'Deleuze Difference Repetition rhizome philosophy of immanence virtual multiplicity 2026',
    'Nancy Inoperative Community singular plural being-with ontology coexistence 2026',
    'Heidegger Zollikon seminars broken tool ready-to-hand disclosure unconcealment Dasein 2026',
    'Enzo Sclavi capital of ideas mass creativity complex problems social learning 2026',
    # ===== 3 GitHub 源码深读 (主 23:28 — 真读源码不止 README) =====
    'huggingface smol-course source code small models training pipeline curriculum github 2026',
    'modular modular Mojo MAX AI kernel compiler inference server source code architecture github 2026',
    'OpenHands OpenHands source code multi-agent SWE-bench evaluation architecture github 2026',
    # ===== 2 Apeireth Gap: 可塑性 + 表观遗传 (主 17:46 MISSING 12 生命特征) =====
    'phenotypic plasticity reaction norm polyphenism Waddington canalization epigenetic landscape AI substrate 2026',
    'transgenerational epigenetic inheritance mobile genetic elements non-coding RNA AI substrate ecological feedback 2026',
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
    print(f'\n=== Round 20 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
