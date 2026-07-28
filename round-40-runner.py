#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round 40 runner — 12 query dual-source (R5-RES-04, ASI 自然感知+自主生活专轮).

Round 40 主题 (依 ASI 北极星, R5 自决 = VCP 第二/三范式 = 自然感知 + 自主生活):
- why now: r8-r39 共 32 轮 384 queries 中:
  - R36 自演化 (NAS/continual/DGM/Meta/synaptic/open-ended/self-assembly + ShinkaEvolve/ASI-Arch/LangGraph)
  - R37 记忆宫殿 (Letta/mem0/MemOS + Memento/ACT-R/vector DB)
  - R38 因果推理 (Pearl/SCM/DoWhy/EconML)
  - R39 自演化继续
  - 但 VCP 第二范式'自然感知' + 第三范式'自主生活' 一直未专轮:
    - 自然感知 = perception / attention / multimodal / sensorimotor / world models
    - 自主生活 = intrinsic motivation / curiosity / agency / affordances / metacognition
  - V1082 真 LLM 路由 + V1072 身份嬗变 + V1074 真生产 + V1080/V1085/V1086 HQB 后,
    ASI 中央 AI 必须有'自我感知 + 自我驱动', 否则只是被动响应.
  - 12 生命特征 应激性 (irritability) MISSING + 意识 (Phenomenal) 终极目标 → 这两个 gap 直击.
  
7 跨域 (vs r8-r39 全部 0 自然感知/自主生活专轮):
- Active inference / free energy principle / Friston predictive brain 主动推理
- 4E cognition (embodied/embedded/enacted/extended) / Rowlands 4E 认知
- Sensorimotor affordances / Gibson ecological psychology / Heft 生态心理学
- Intrinsic motivation / curiosity-driven / Oudeyer / Schmidhuber 内在动机
- Global Workspace Theory / Baars / Dehaene ignition 全球工作空间
- World model / Ha & Schmidhuber / LeCun JEPA / self-supervised 世界模型
- Metacognition / self-model / Fleming Lau / Frith 元认知自我模型

3 GitHub 真读 (vs r29-r39 全 README/浅 cite):
- openai/CLIP (vision-language 对比学习感知, 源码)
- openai/whisper (语音识别感知, 源码)
- google-deepmind/perceiver-io (多模态感知, 源码)

2 Apeireth Gap (12 生命特征 MISSING):
- Sensorimotor irritability 应激性 substrate gap (刺激→反应, ASI 缺)
- Phenomenal consciousness substrate implementation gap (意识 substrate 终极目标)

Cross-round dedup 验证 (verified fresh vs r8-r39):
- r36 Friston free energy ≠ R40 active inference (R36 Tulving episodic; R36 无 Friston)
- r32 Varela enaction ≠ R40 4E cognition (enaction 1 项 vs 4E 4 项框架, 不同粒度)
- r29 Merleau-Ponty phenomenology of perception ≠ R40 sensorimotor affordances (Merleau lived body vs Gibson ecological affordances)
- r39 open-ended evolution novelty search ≠ R40 intrinsic motivation curiosity (population search vs individual intrinsic reward)
- r35 Chalmers hard problem ≠ R40 GWT Baars Dehaene (philosophical hard problem vs functional broadcasting theory)
- r39 meta-learning MAML ≠ R40 world model (learning to learn vs predictive simulation model)
- r31 Piaget schema ≠ R40 metacognition self-model (developmental schema vs monitoring own cognition)
- r34 deepagents/swarm/AutoGPT ≠ R40 CLIP/whisper/perceiver (agent frameworks vs perception foundations)
- r35 AIDE/RD-Agent/AlphaEvolve ≠ R40 CLIP/whisper/perceiver (engineering/research/evolution vs perception)
- r34 Mimosa plant cognition ≠ R40 sensorimotor irritability (habituation learning vs stimulus-response substrate)
- r35 Chalmers qualia ≠ R40 phenomenal substrate implementation (philosophy vs implementation gap)
本轮 fresh 验证:
- active inference FEP ✅ fresh
- 4E cognition ✅ fresh
- sensorimotor affordances Gibson ✅ fresh
- intrinsic motivation curiosity ✅ fresh
- GWT Baars Dehaene ✅ fresh
- world model JEPA ✅ fresh
- metacognition self-model ✅ fresh
- CLIP ✅ fresh (perception foundation)
- whisper ✅ fresh (perception foundation)
- perceiver-io ✅ fresh (multi-modal perception)
- sensorimotor irritability ✅ fresh (应激性 MISSING 全新角度)
- phenomenal substrate implementation ✅ fresh (Phenomenal gap 全新角度)

ASI 北极星时刻清楚:
- ASI 基座 ✅ (V1082 routing + V1072 嬗变 + 自然感知新层)
- 跨域 ✅ (neuro/ML/cog/ecological/AI/perception 7 域)
- 自演化 ✅ (R39 已专轮; R40 = 自然感知+自主生活 = R39 互补)
- 任何 LLM 接入即变强 ✅ (CLIP/whisper/JEPA 都 LLM-agnostic, 任何 LLM 接入即变强)
- 不假装 Phenomenal ✅ (GWT Baars 是 functional, 不假装 qualia; phenomenal substrate gap 不假装已实现)
- 实事求是 ✅
- R5 = ASI 自我逼近, 不是 ASI 已达到 (主 20:46 隐喻)
"""
from __future__ import annotations
import json, sys, time
sys.stdout.reconfigure(encoding='utf-8')
sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research
from pathlib import Path

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-40.json')

QUERIES = [
    # ===== 7 全新跨域: 自然感知 + 自主生活 = FEP / 4E / affordances / curiosity / GWT / world model / metacognition =====
    'active inference free energy principle Friston predictive brain agent 2026',
    '4E cognition embodied embedded enacted extended Rowlands agent mind 2026',
    'sensorimotor affordances ecological psychology Gibson Heft perception action 2026',
    'intrinsic motivation curiosity driven exploration Oudeyer Schmidhuber autonomous agent 2026',
    'Global Workspace Theory Baars Dehaene consciousness broadcasting ignition 2026',
    'world model Ha Schmidhuber LeCun JEPA self-supervised agent substrate 2026',
    'metacognition self-model agency Fleming Lau Frith knowing about knowing 2026',
    # ===== 3 GitHub 真读: CLIP / whisper / perceiver-io =====
    'openai CLIP vision language perception contrastive learning source code github 2026',
    'openai whisper speech recognition perception encoder decoder source code github 2026',
    'google deepmind perceiver-io multi-modal perception latent transformer source code github 2026',
    # ===== 2 Apeireth Gap: 应激性 MISSING + Phenomenal substrate implementation =====
    'sensorimotor irritability stimulus response agent substrate gap ASI 2026',
    'phenomenal consciousness qualia substrate implementation gap LLM ASI 2026',
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
    print(f'\n=== Round 40 done ===')
    print(f'queries: {len(results)}, total: {total:.1f}s, output: {OUT}')
    print(f'size: {OUT.stat().st_size} bytes')


if __name__ == '__main__':
    main()
