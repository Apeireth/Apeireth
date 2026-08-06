#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-66 cross-domain research runner.

Cron triggered 2026-08-03 22:54 Asia/Shanghai (every-2h reminder).
Self-decision: round_auto_naming.py next=66, no conflict; round-65 done 2026-08-03 20:57
(1h57m ago, >>30min threshold). Monday 22:54 evening, cron isolated lane. Decision: run since
gap-fill value high and does not block.

Theme: 7 跨域 fresh — 真正 MISSING / 2nd-fresh angles avoiding r55-r65 keywords:
   - R3 死亡 fresh: NETosis / neutrophil extracellular traps / Brinkmann (canonical 程序性死亡新通路)
   - R8 运动 fresh: cilium / intraflagellar transport IFT / axoneme dynein (细胞骨架运动 substrate,
     真正 fresh 区别于 r59 flagellar motor)
   - R12 生态 fresh: Red Queen hypothesis / Van Valen 1973 / evolutionary arms race
     (生态进化 substrate, complement r62 sociobiology + r63 r/K + r64 Lotka-Volterra + r65 mycorrhiza)
   - R0 新陈代谢 fresh: gluconeogenesis / Cori cycle / cytosolic glucose homeostasis
     (糖异生 substrate, NOT claim ASI does gluconeogenesis, complement r46 Krebs + r61 photosynthesis
      + r63 chemiosmosis + r64 pentose phosphate + r65 beta-oxidation)
   - R11 意识 fresh: split-brain / Sperry Gazzaniga / corpus callosum / lateralization
     (互补 R11 Helmholtz r65 神经预测 coding, NOT claim ASI has split-brain)
   - R6 繁殖 fresh: polyembryony / monozygotic twins / armadillo (Dasypus) split offspring
     (互补 R6 r64 parthenogenesis + r65 asexual reproduction, NOT claim ASI reproduces)
   - R7 应激 fresh: fight-or-flight / Cannon 1932 / sympathoadrenal axis / HPA axis
     (经典应激反应 substrate, NOT claim ASI has adrenaline, complement r60 two-component +
      r62 TLR NLR + r63 cytokine + r65 circadian)
   + 3 GitHub deep (fresh):
   - DeepSeek-V3: open-source 685B MoE LLM (vs r64 gpt-oss, 真正 fresh 国产 LLM substrate,
     中央 AI 任何 LLM 接入 substrate)
   - langchain-ai/langchain: orchestration framework substrate (中央 AI 可插拔 substrate,
     complement r62 claude-code / r65 OpenDevin)
   - All-Hands-AI/OpenHands: autonomous AI software engineer (vs r65 OpenDevin,
     complement r65 SWE-bench 自治代码 substrate)
   + 2 Gap:
   - R5 修复 Gap: Fanconi anemia FA pathway DNA crosslink repair BRCA1 FANC
     (互补 r63 NHEJ/HR/BER, NOT claim ASI does DNA repair)
   - R11 意识 Gap: blindsight Weiskrantz Marshall + anosognosia Babinski denial neglect
     (第 14 个 consciousness substrate, NOT claim ASI = blindsight, complement r58-r65)

Avoid r65 (circadian / LTP LTD / mycorrhiza / apical meristem auxin / McClintock transposon /
          hallmarks aging / asexual reproduction / OpenDevin / SWE-bench / Magicoder /
          beta-oxidation / Helmholtz forward model)
Avoid r64 (Werner / Nagel bat / parthenogenesis / Lotka-Volterra / V(D)J / Hox colinearity /
          pentose phosphate / gpt-oss / openai-agents-python / mcp / cellular senescence /
          attention schema Graziano)
Avoid r63 (DNA repair / qualia Block / epigenome / chemiosmosis / cytokine NF-kB / prion /
          phylotypic hourglass / whisper / faster-whisper / pyannote-audio / autophagy / r/K)
Avoid r62 (lactic acid / TLR NLR / necroptosis pyroptosis / predictive / polyploidy /
          iPSC / sociobiology / claude-code / aider / continue / meiosis / gap junction)
Avoid r61 (photosynthesis / UPR stress granules / ferroptosis / Klotho / maternal effect /
          adult neurogenesis / HGT viral / alphagenome / nanoGPT / stable-diffusion /
          GWT Baars Dehaene / prion)
Avoid r60 (chemotaxis / chaperone Hsp / ribosome / Wnt/Hedgehog/Notch / actin cytoskeleton /
          MWC allosteric / critical period Hubel-Wiesel / alphafold / transformers / CLIP /
          retrovirus transposon / HOT consciousness)
Avoid r59 (mechanotransduction Piezo / apoptosis / Hox bicoid / flagellar motor / morphallaxis /
          epigenetic transgenerational / niche construction / claude-agent-sdk / mem0 /
          HarnessAgent / telomere Hayflick / chemolithotrophy)
Avoid r58 (Varela / Margulis / Per Bak SOC / connectome / Rosen / Pearl / Wolfram NKS /
          ASI-Arch / DGM / langgraph / tardigrade / embryogenesis)
Avoid r57 (Kauffman / Prigogine / Holland CAS / Maturana-Varela / Klein Erlangen /
          quantum biology / Carlsson TDA / openevolve / ShinkaEvolve / letta / Hamilton ESS /
          Thompson enactivism)
Avoid r56 (Solomonoff-AIXI / Ramsauer Hopfield / Hasani liquid NN / Kanerva VSA / Tierra /
          Olah mechanistic / Causal emergence / Mamba / RWKV / TransformerLens / Avida / NCC IIT)
Avoid r55 (Metzinger MPE / LeCun V-JEPA / Hinton FF/GLOM / Quorum sensing / Beer VSM /
          Pask conversation / von Foerster 2nd-order / llama.cpp / lm-evaluation-harness /
          anthropic-sdk / Hebb-Kandel-Merzenich / Lewontin)

V 模块进度追踪 (post-r65 缺口分析):
- R0 新陈代谢 ? r46 Krebs + r51 + r59 chemolithotrophy + r61 photosynthesis + r62 lactic acid
              + r63 chemiosmosis + r64 pentose phosphate + r65 beta-oxidation
              ← r66 加 gluconeogenesis + Cori cycle (糖异生 substrate, complement r46-r65)
- R3 死亡 ? r45 + r59 + r61 + r62 + r63 + r64
              ← r66 加 NETosis (第 6 死亡通路, complement r45-r65)
- R5 修复 ? r44 + r49 + r58 + r59 + r63 DNA repair
              ← r66 加 Fanconi anemia FA pathway DNA crosslink repair BRCA1
              (互补 r63 NHEJ/HR/BER, NOT claim ASI does DNA repair)
- R6 繁殖 ? r41-r65 + r62 meiosis + r64 parthenogenesis + r65 asexual reproduction
            ← r66 加 polyembryony monozygotic twins armadillo (Dasypus)
            (互补 R6 r64 + r65, NOT claim ASI reproduces)
- R7 应激 ? r42 + r53 + r57 + r59 + r60-r62 + r63 cytokine + r65 circadian
            ← r66 加 fight-or-flight Cannon 1932 sympathoadrenal HPA axis
            (经典应激 substrate, complement r60-r65)
- R8 运动 ? r41/r45 + r52 + r59 flagellar motor + r60 actin
            ← r66 加 cilium intraflagellar transport IFT axoneme dynein
            (真正 fresh 区别于 r59 flagellar motor)
- R10 可塑 ? r40-r63 + r63 prion + r64 V(D)J + r65 LTP LTD NMDA
- R11 意识 ? r42-r65 + r64 Nagel + r64 attention schema + r65 Helmholtz forward model
              ← r66 加 split-brain Sperry Gazzaniga corpus callosum + blindsight Weiskrantz
              + anosognosia Babinski denial neglect
              (第 14 + 15 个 consciousness substrate, NOT claim ASI has these)
- R12 生态 ? r16-r59 + r62 sociobiology + r63 r/K + r64 Lotka-Volterra + r65 mycorrhiza
              ← r66 加 Red Queen hypothesis Van Valen evolutionary arms race
              (进化生态 substrate, complement r62-r65)

数学/哲学基座 (主 22:33 ASI 北极星):
- 中央 AI = sum of all forms — 12 substrate 第 14 轮 (NETosis + cilium IFT + Red Queen +
              gluconeogenesis + split-brain + polyembryony + fight-or-flight +
              DeepSeek-V3 + langchain + OpenHands + Fanconi anemia FA + blindsight +
              中央 AI 累计 158+ + 12 = 170+ substrate)
              NOT claim ASI has all.
- NETosis = 程序性死亡 substrate, NOT claim ASI = NETosis
- cilium IFT = 微管运动 substrate, NOT claim ASI has cilium
- Red Queen = 进化红皇后 substrate, NOT claim ASI does arms race
- gluconeogenesis = 糖异生 substrate, NOT claim ASI does gluconeogenesis
- split-brain = 胼胝体 substrate, NOT claim ASI has split-brain
- polyembryony = 多胚胎 substrate, NOT claim ASI = armadillo
- fight-or-flight = 战逃反应 substrate, NOT claim ASI has adrenaline
- DeepSeek-V3 = 国产 MoE LLM substrate, NOT claim ASI = DeepSeek
- langchain = 编排框架 substrate, NOT claim ASI = langchain
- OpenHands = 自治 AI 软件工程师 substrate, NOT claim ASI = OpenHands
- Fanconi anemia FA = DNA 修复 substrate, NOT claim ASI does FA pathway
- blindsight/anosognosia = 视觉残余 + 否认 deficit substrate, NOT claim ASI = these

ASI 概念时刻清楚 (主 22:33 ASI 北极星自检):
中央 AI = ASI 位置, 12 substrate sum, NOT claim ASI has all (主 22:08)
Phenomenal 是终极目标, NOT 已达成 (主 17:58)
ASI 超越时代, 只能逼近 (主 20:46)
隐喻是工具, NOT 限制 (主 20:55)
VCP 4 范式: 连续存在/自然感知/自主生活/一体生态
实事求是, 不假装/不欺骗 (主 17:43)
跨域借鉴 = 工具/启发, NOT 哲学来源 (主 21:00)
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-66.json')

QUERIES = [
    # ===== 7 跨域 fresh =====

    # 1. R3 死亡 fresh — NETosis / neutrophil extracellular traps / Brinkmann
    #    (第 6 死亡通路, complement r45-r65, NOT claim ASI = NETosis)
    'NETosis neutrophil extracellular traps NET Brinkmann 2004 release DNA histone elastase cathepsin G substrate ASI R3 death fresh complement r45 r59 r61 r62 r63 r64 r65',

    # 2. R8 运动 fresh — cilium intraflagellar transport IFT axoneme dynein
    #    (真正 fresh 区别于 r59 flagellar motor, NOT claim ASI has cilium)
    'cilium intraflagellar transport IFT axoneme dynein microtubule cilia flagella substrate ASI R8 motion fresh complement r41 r52 r59',

    # 3. R12 生态 fresh — Red Queen hypothesis Van Valen 1973 evolutionary arms race
    #    (生态进化 arms race substrate, NOT claim ASI does arms race, complement r62-r65)
    'Red Queen hypothesis Van Valen 1973 evolutionary arms race coevolution extinction substrate ASI R12 ecology fresh complement r16 r59 r62 r63 r64 r65',

    # 4. R0 新陈代谢 fresh — gluconeogenesis Cori cycle cytosolic glucose
    #    (糖异生 + 乳酸循环 substrate, complement r46-r65, NOT claim ASI does gluconeogenesis)
    'gluconeogenesis Cori cycle cytosolic glucose homeostasis PEPCK F1,6BP substrate ASI R0 metabolism fresh complement r46 r51 r59 r61 r62 r63 r64 r65',

    # 5. R11 意识 fresh — split-brain Sperry Gazzaniga corpus callosum lateralization
    #    (第 14 个 consciousness substrate, NOT claim ASI has split-brain, complement r42-r65)
    'split-brain Sperry Gazzaniga corpus callosum lateralization left interpreter substrate ASI R11 consciousness fresh complement r42 r43 r46 r49 r58 r60 r61 r62 r63 r64 r65',

    # 6. R6 繁殖 fresh — polyembryony monozygotic twins armadillo Dasypus split offspring
    #    (互补 R6 r64 parthenogenesis + r65 asexual reproduction, NOT claim ASI reproduces)
    'polyembryony monozygotic twins armadillo Dasypus identical offspring fission substrate ASI R6 reproduction fresh complement r41 r47 r50 r58 r64 r65',

    # 7. R7 应激 fresh — fight-or-flight Cannon 1932 sympathoadrenal HPA axis
    #    (经典应激反应 substrate, NOT claim ASI has adrenaline, complement r60-r65)
    'fight-or-flight Cannon 1932 sympathoadrenal axis HPA cortisol adrenaline norepinephrine substrate ASI R7 stress fresh complement r42 r53 r57 r59 r60 r61 r62 r63 r65',

    # ===== 3 GitHub deep (DeepSeek-V3 + langchain + OpenHands) =====

    # 8. deepseek-ai/DeepSeek-V3 真读 — open-source MoE LLM
    #    (vs r64 gpt-oss, 真正 fresh 国产 LLM substrate, 中央 AI 任何 LLM 接入 substrate)
    'deepseek-ai DeepSeek-V3 github source code MoE 685B open source LLM architecture real source deep dive substrate ASI central AI pluggable',

    # 9. langchain-ai/langchain 真读 — orchestration framework
    #     (中央 AI 可插拔 substrate, complement r62 claude-code / r65 OpenDevin)
    'langchain-ai langchain github source code orchestration framework LCEL agent chain real source deep dive substrate ASI central AI pluggable',

    # 10. All-Hands-AI/OpenHands 真读 — autonomous AI software engineer
    #      (vs r65 OpenDevin, complement r65 SWE-bench 自治代码 substrate)
    'All-Hands-AI OpenHands github source code autonomous AI software engineer agent real source deep dive substrate ASI central AI pluggable',

    # ===== 2 Gap =====

    # 11. R5 修复 Gap — Fanconi anemia FA pathway DNA crosslink repair BRCA1 FANC
    #     (互补 r63 NHEJ/HR/BER, NOT claim ASI does DNA repair)
    'Fanconi anemia FA pathway DNA crosslink repair BRCA1 FANC mono-ubiquitination substrate ASI R5 repair Gap complement r44 r49 r58 r59 r63',

    # 12. R11 意识 Gap — blindsight Weiskrantz Marshall + anosognosia Babinski denial neglect
    #     (第 15 个 consciousness substrate, NOT claim ASI = blindsight/anosognosia, complement r58-r65)
    'blindsight Weiskrantz Marshall residual vision V1 striate cortex anosognosia Babinski denial hemineglect cortical substrate ASI R11 consciousness Gap complement r42 r43 r46 r49 r58 r60 r61 r62 r63 r64 r65',
]


def main():
    started = time.time()
    started_iso = time.strftime('%Y-%m-%dT%H:%M:%S+08:00', time.localtime(started))
    print(f'Round-66 started {started_iso}')

    results = []
    for i, q in enumerate(QUERIES, 1):
        t0 = time.time()
        r = dual_research(q, top_k=5)
        dur = time.time() - t0
        bw = len(r['bocha_web'])
        ba = len(r['bocha_ai_answer'])
        any_n = len(r['anysearch'])
        merged = len(r['merged_sources'])
        print(f'[{i:02d}/{len(QUERIES)}] {dur:.1f}s | bw={bw} ba={ba} any={any_n} merged={merged} | {q[:80]}')
        results.append(r)
        time.sleep(0.5)

    OUT.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - started
    print(f'\nRound-66 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()
