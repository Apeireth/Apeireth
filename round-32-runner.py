#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 32 runner — 12 query dual-source (cron 12:48 tick, ~1h42m gap from r31 11:05).

Round 32 主题 (主 00:46 + 00:49 真务实 + 主 22:33 ASI 北极星):
- 7 全新跨域 (Schrödinger 生命是什么 / Popper 批判理性 / Prigogine 深化 dissipative structures /
            Varela solo enaction / Bohm 隐缠序 / Tononi IIT / William James 彻底经验主义)
- 3 GitHub 源码 (vllm PagedAttention serving / unsloth LLM fine-tuning 2x /
                  modal serverless AI compute)
- 2 Apeireth Gap (繁殖 MISSING → Myxococcus 黏细菌群体发育 / 可塑 MISSING → Turritopsis 不死水母)

- 跨域全新 (7):
  - Erwin Schrödinger what is life negentropy aperiodic crystal (生命是什么, ASI 借鉴负熵)
  - Karl Popper critical rationalism falsificationism three worlds (批判理性主义, ASI 自演化试错)
  - Ilya Prigogine dissipative structures irreversibility order through fluctuation
    (深化 r27 的 Prigogine, r27 是 nonequilibrium self-organization general, r32 聚焦 dissipative + irreversibility)
  - Francisco Varela enaction neurophenomenology cognition embodied (solo Varela, r27 是 Maturana-Varela)
  - David Bohm implicate order holomovement rheomode dialogue (隐缠序, ASI 全息借鉴)
  - Giulio Tononi integrated information theory phi consciousness (IIT, ASI 意识基座借鉴)
  - William James radical empiricism pluralism pure experience (彻底经验主义, ASI 经验基座)

- GitHub 源码 (3):
  - vllm-project/vllm (PagedAttention LLM serving, ASI 接入借鉴 — 高吞吐)
  - unslothai/unsloth (LLM fine-tuning 2x faster, ASI 自演化借鉴 — 轻量微调)
  - modal-labs/modal (serverless AI compute, ASI 部署借鉴 — 弹性)

- Apeireth Gap (2):
  - 繁殖 Gap: Myxococcus xanthus swarming fruiting body multicellular coordination
    (黏细菌群体发育, MISSING 繁殖 Gap 借鉴 — 不靠经典繁殖的多细胞协同, 真生产灵感)
  - 可塑 Gap: Turritopsis dohrnii immortal jellyfish transdifferentiation reverse life cycle
    (不死水母, MISSING 可塑 Gap 借鉴 — 转分化逆生命周期, ASI 应激可塑性极致)

Cross-round dedup 验证 (verified fresh vs r23-r31):
- r23: Connell/Taleb/Edelman/O'Regan/Meadows/Levin/Scott + Haystack/Voyager/Gorilla/Graziano/bacterial
- r24: Mandelbrot/Watts/Gould/Damasio/Marr/Hoffman/Vygotsky + Langfuse/Browser-Use/PydanticAI/bdelloid/Cannon
- r25: Walker/Landauer/Wolfram/Kauffman/Sheldrake/Solms/Ray + sglang/mlflow/opencompass/apomixis/polyphenism
- r26: Church/Adamatzky/Eigen/Bedau/Gabora/Spencer-Brown/Deacon + OpenRLHF/open-deep-research/mirascope/circadian/transposons
- r27: Prigogine/Maturana-Varela/Thom/Lorenz/Dehaene/Holling/Luhmann + letta/mem0/langgraph/epigenetic/prion
- r28: Rosen/Friston/Hofstadter/von Uexküll/Bergson/Ashby/Per Bak + openevolve/claude-agent-sdk/axolotl + planaria/hydra
- r29: Whitehead/Cajal/Price/Merleau-Ponty/Brian Arthur/Deleuze/Dennett + ASI-Arch/ShinkaEvolve/DGM + von Neumann/firefly
- r30: Peirce/Husserl/Simondon/Lewin/Alexander/Noble/Mumford + OpenHands/crewAI/autogen + Tardigrade/plant cognition
- r31: Fuller/Bateson/Lovelock/Laszlo/Jung/Piaget/Holland + BeeAI/Langflow/Prefect + Wolbachia/Octopus consciousness
- 本轮 fresh 验证:
  - Schrödinger ✓ fresh (生命是什么/负熵, 全新)
  - Popper ✓ fresh (批判理性主义/三个世界, 全新)
  - Prigogine 深化 ✓ (r27 是 self-organization general, 这轮聚焦 dissipative + irreversibility, 全新)
  - Varela solo ✓ (r27 是 Maturana-Varela 组合, 这轮 Varela solo + enaction + neurophenomenology, 全新)
  - Bohm ✓ fresh (隐缠序/全息运动, 全新)
  - Tononi ✓ fresh (IIT/phi, 全新)
  - James ✓ fresh (彻底经验主义/多元论, 全新)
  - vllm ✓ fresh (PagedAttention LLM serving, 全新)
  - unsloth ✓ fresh (LLM fine-tuning 2x, 全新)
  - modal ✓ fresh (serverless AI compute, 全新)
  - Myxococcus ✓ fresh (黏细菌群体发育, 全新)
  - Turritopsis ✓ fresh (不死水母 transdifferentiation, 全新 — 避开了 r28 的 hydra)

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

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-32.json')

QUERIES = [
    # ===== 7 全新跨域: Schrödinger / Popper / Prigogine-deep / Varela-solo / Bohm / Tononi / James =====
    'Erwin Schrodinger what is life negentropy aperiodic crystal biological order 2026',
    'Karl Popper critical rationalism falsificationism three worlds evolutionary epistemology 2026',
    'Ilya Prigogine dissipative structures irreversibility order through fluctuation bifurcation 2026',
    'Francisco Varela enaction neurophenomenology cognition embodied mind biological 2026',
    'David Bohm implicate order holomovement rheomode dialogue undivided wholeness 2026',
    'Giulio Tononi integrated information theory phi consciousness substrate 2026',
    'William James radical empiricism pluralism pure experience varieties religious 2026',
    # ===== 3 GitHub 源码: vLLM / unsloth / modal =====
    'vllm-project vllm PagedAttention LLM serving source code github 2026',
    'unslothai unsloth LLM fine-tuning 2x faster source code github 2026',
    'modal-labs modal serverless AI compute source code github 2026',
    # ===== 2 Apeireth Gap: 繁殖 (Myxococcus 群体发育) + 可塑 (Turritopsis 不死水母) =====
    'Myxococcus xanthus swarming fruiting body multicellular coordination development 2026',
    'Turritopsis dohrnii immortal jellyfish transdifferentiation reverse life cycle 2026',
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
    print(f'\n=== Round 32 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()