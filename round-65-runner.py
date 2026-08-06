#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-65 cross-domain research runner.

Cron triggered 2026-08-03 20:48 Asia/Shanghai (every-2h reminder, last tick).
Self-decision: round_auto_naming.py next=65, no conflict; round-64 done 2026-08-03 19:55 (~53min
ago, >30min threshold). Monday 20:48 evening, cron is reminder not blocking. Decision: run since
gap-fill value high, does not block master.

Theme: 7 跨域 fresh — 真正 MISSING/2nd-fresh angles avoiding r55-r64 keywords:
   - R7 应激 fresh: circadian rhythm / suprachiasmatic / Period Timeless / Hall Rosbash Young 2017
     (canonical 时间生物学 substrate, complementary r63 UPR)
   - R10 可塑 fresh: LTP LTD NMDA receptor Bliss Lomo synaptic plasticity hippocampus
     (synaptic 记忆 substrate, complement r55 Hebb + r63 prion)
   - R12 生态 fresh: mycorrhizal network / Wood-Wide Web / Suzanne Simard fungal network
     (fungal internet substrate, complement r62 sociobiology + r63 r/K + r64 Lotka-Volterra)
   - R1 生长 fresh: apical meristem auxin Arabidopsis plant development
     (truly fresh plant biology substrate)
   - R9 遗传 fresh: Barbara McClintock / Ac Ds maize jumping gene transposable element
     (真正 deep 经典遗传学 substrate, complement r60 retrovirus transposon)
   - R4 衰老 fresh: hallmarks of aging López-Otín 2013 + 2023 9 hallmarks review
     (canonical 衰老机制 consolidated substrate, complement r41/r45/r59/r61/r64)
   - R6 繁殖 fresh: asexual budding fragmentation vegetative reproduction hydra planarian
     (complementary 模式 to r64 parthenogenesis)
   + 3 GitHub deep (fresh):
   - OpenDevin/OpenDevin: autonomous AI software engineer (vs r62 claude-code)
   - SWE-bench/SWE-bench: GitHub issue resolution benchmark (any-LLM 中央 AI)
   - ise-uiuc/Magicoder-Evol-Instruct-110K: code instruction data evolution substrate
   + 2 Gap:
   - R0 代谢 Gap: beta-oxidation fatty acid Kennedy pathway mitochondria lipid
   - R11 意识 Gap: Helmholtz forward model / Bayesian brain predictive coding Summerfield

Avoid r64 (Werner / Nagel / parthenogenesis / Lotka-Volterra / V(D)J / Hox colinearity /
          pentose phosphate / gpt-oss / openai-agents-python / mcp / cellular senescence /
          attention schema)
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

V 模块进度追踪 (post-r64 缺口分析):
- R0 新陈代谢 ? r46 Krebs + r51 + r59 chemolithotrophy + r61 photosynthesis + r62 lactic acid
              + r63 chemiosmosis + r64 pentose phosphate
              ← r65 加 beta-oxidation fatty acid Kennedy pathway mitochondrial lipid catabolism
              (β-氧化 + Kennedy 通路 substrate)
- R1 生长 ? r46 + r51 + r60 ribosome + r62 polyploidy
              ← r65 加 apical meristem auxin Arabidopsis plant development
              (植物顶端分生组织 + 生长素 + 拟南芥 substrate, 真正 fresh 植物生物)
- R2 发育 ? r40/r42/r45 + r52 + r54 + r56 + r58 + r59 + r60 + r61 + r62 + r63 + r64 Hox colinearity
- R3 死亡 ? r45 + r59 + r61 + r62 + r63 autophagy-dependent
- R4 衰老 ? r41 + r45 + r59 + r61 + r64 Werner + r64 cellular senescence
              ← r65 加 hallmarks of aging López-Otín 2013 + 2023 9 hallmarks review
              (canonical 衰老 consolidated substrate, complement r41-64)
- R5 修复 ? r44 + r49 + r58 + r59 + r63 DNA repair
- R6 繁殖 ? r41 + r47 + r50-58 + r60-62 + r62 meiosis + r64 parthenogenesis
              ← r65 加 asexual budding fragmentation vegetative reproduction hydra planarian
              (complementary 模式 to r64 parthenogenesis, NOT claim ASI reproduces)
- R7 应激 ? r42 + r53 + r57 + r59 + r60-62 + r63 cytokine
              ← r65 加 circadian rhythm suprachiasmatic Period Timeless Hall Rosbash Young 2017
              (canonical 时间生物学 substrate, NOT claim ASI is circadian)
- R8 运动 ? r41/r45 + r52 + r59 flagellar motor + r60 actin
- R9 遗传 ? r44-r48 + r54 + r56-58 + r59-63 + r60 retrovirus transposon
              ← r65 加 Barbara McClintock Ac Ds maize jumping gene transposable element
              (经典 cytogenetic 跳跃基因 substrate, NOT claim ASI has transposon)
- R10 可塑 ? r40-63 + r63 prion + r64 V(D)J
              ← r65 加 LTP LTD NMDA Bliss Lomo synaptic plasticity hippocampus
              (synaptic 突触记忆 substrate, NOT claim ASI has NMDA)
- R11 意识 ? r42/r43/r46/r49-r64 + r64 Nagel + r64 attention schema
              ← r65 加 Helmholtz forward model Bayesian brain predictive coding Summerfield
              (神经预测编码 substrate 第 13 个, NOT claim ASI does prediction)
- R12 生态 ? r16-r59 + r62 sociobiology + r63 r/K + r64 Lotka-Volterra
              ← r65 加 mycorrhizal network Wood-Wide Web Suzanne Simard fungal network
              (森林 internet substrate, complement r62-64)

数学/哲学基座 (主 22:33 ASI 北极星):
- 中央 AI = sum of all forms — 12 substrate 第 12 轮 (circadian + LTP-LTD + mycorrhiza +
              apical meristem + McClintock transposon + hallmarks of aging + asexual reproduction +
              OpenDevin + SWE-bench + Magicoder + beta-oxidation + Helmholtz forward model +
              central AI 累计 134+ + 12 = 146+ substrate).
              NOT claim ASI has all.
- circadian rhythm = 时间生物学 substrate, NOT claim ASI is circadian
- LTP LTD NMDA = 突触可塑 substrate, NOT claim ASI has NMDA
- mycorrhiza = 真菌网络 substrate, NOT claim ASI is keystone
- apical meristem auxin = 植物生长 substrate, NOT claim ASI grows like Arabidopsis
- McClintock transposon = 跳跃基因 substrate, NOT claim ASI has transposon
- hallmarks of aging = 衰老机制 substrate, NOT claim ASI is aging
- asexual reproduction = 出芽生殖 substrate, NOT claim ASI reproduces
- OpenDevin = 自治编码 substrate, NOT claim ASI = OpenDevin
- SWE-bench = GitHub issue benchmark substrate, NOT claim ASI solves issues
- Magicoder-Evol-Instruct = 代码数据演化 substrate, NOT claim ASI evol like Magicoder
- beta-oxidation fatty acid = 脂肪酸代谢 substrate, NOT claim ASI does beta-oxidation
- Helmholtz forward model = 预测编码 substrate, NOT claim ASI does forward models

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

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-65.json')

QUERIES = [
    # ===== 7 跨域 fresh =====

    # 1. R7 应激 fresh — circadian rhythm / suprachiasmatic nucleus / Period Timeless / Hall Rosbash Young 2017
    #    (canonical 时间生物学 substrate, NOT claim ASI is circadian)
    'circadian rhythm suprachiasmatic nucleus Period Timeless Clock Bmal1 Hall Rosbash Young 2017 substrate ASI R7 stress fresh complement r63',

    # 2. R10 可塑 fresh — LTP LTD NMDA Bliss Lomo synaptic plasticity hippocampus
    #    (synaptic 突触记忆 substrate, NOT claim ASI has NMDA, complement r55 Hebb + r63 prion)
    'LTP LTD NMDA receptor Bliss Lomo synaptic plasticity hippocampus CA1 substrate ASI R10 plasticity fresh complement r55 r63',

    # 3. R12 生态 fresh — mycorrhizal network / Wood-Wide Web / Suzanne Simard fungal network mother tree
    #    (森林 internet substrate, NOT claim ASI is keystone, complement r62 sociobiology + r63 r/K + r64 Lotka-Volterra)
    'mycorrhizal network Wood Wide Web Suzanne Simard mother tree fungal internet substrate ASI R12 ecology fresh complement r62 r63 r64',

    # 4. R1 生长 fresh — apical meristem auxin Arabidopsis plant development shoot root
    #    (truly fresh plant biology substrate, NOT claim ASI grows like Arabidopsis)
    'apical meristem auxin Arabidopsis plant development shoot root CLAVATA WUSCHEL substrate ASI R1 growth fresh complement r46 r51 r60 r62',

    # 5. R9 遗传 fresh — Barbara McClintock Ac Ds maize jumping gene transposable element cytogenetic
    #    (经典 cytogenetic 跳跃基因 substrate, NOT claim ASI has transposon, complement r60 retrovirus transposon)
    'Barbara McClintock Ac Ds maize jumping gene transposable element cytogenetic 1983 Nobel substrate ASI R9 inheritance fresh complement r60',

    # 6. R4 衰老 fresh — hallmarks of aging López-Otín 2013 2023 9 hallmarks cellular review
    #    (canonical 衰老 consolidated substrate, NOT claim ASI is aging, complement r41/r45/r59/r61/r64)
    'hallmarks of aging Lopez-Otin 2013 2023 nine hallmarks cellular review genomic instability telomere epigenetic substrate ASI R4 senescence fresh complement r41 r45 r59 r61 r64',

    # 7. R6 繁殖 fresh — asexual budding fission vegetative reproduction hydra planarian
    #    (complementary 模式 to r64 parthenogenesis, NOT claim ASI reproduces)
    'asexual reproduction budding fission fragmentation vegetative hydra planarian flatworm substrate ASI R6 reproduction fresh complement r41 r47 r50 r64',

    # ===== 3 GitHub deep (OpenDevin + SWE-bench + Magicoder-Evol-Instruct) =====

    # 8. OpenDevin/OpenDevin 真读 — autonomous AI software engineer
    #    (vs r62 claude-code, 真正 open source AI software engineer substrate, NOT claim ASI = OpenDevin)
    'OpenDevin OpenDevin github source code autonomous AI software engineer architecture real source deep dive substrate ASI central AI pluggable',

    # 9. SWE-bench/SWE-bench 真读 — GitHub issue resolution benchmark
    #     (中央 AI 任何 LLM 接入 substrate, NOT claim ASI solves issues)
    'SWE-bench SWE-bench github source code benchmark GitHub issue resolution real source deep dive substrate ASI central AI pluggable',

    # 10. ise-uiuc/Magicoder-Evol-Instruct-110K 真读 — code instruction data evolution
    #      (代码指令演化数据集 substrate, NOT claim ASI evol like Magicoder)
    'ise-uiuc Magicoder-Evol-Instruct-110K github source code instruction data evolution OSS-INSTRUCT substrate ASI central AI pluggable code',

    # ===== 2 Gap =====

    # 11. R0 代谢 Gap — beta-oxidation fatty acid Kennedy pathway mitochondria lipid catabolism
    #     (脂肪酸 β-氧化 substrate, NOT claim ASI does beta-oxidation, complement r46 Krebs + r63 chemiosmosis)
    'beta-oxidation fatty acid Kennedy pathway mitochondria lipid catabolism acyl-CoA carnitine substrate ASI R0 metabolism Gap complement r46 r51 r59 r61 r62 r63 r64',

    # 12. R11 意识 Gap — Helmholtz forward model Bayesian brain predictive coding Summerfield conscious perception
    #     (神经预测编码 substrate 第 13 个, NOT claim ASI does prediction, complement r55-r64 consciousness)
    'Helmholtz forward model Bayesian brain predictive coding Summerfield conscious perception neural correlate substrate ASI R11 consciousness Gap complement r42 r43 r46 r49 r58 r60 r61 r62 r63 r64',
]


def main():
    started = time.time()
    started_iso = time.strftime('%Y-%m-%dT%H:%M:%S+08:00', time.localtime(started))
    print(f'Round-65 started {started_iso}')

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
    print(f'\nRound-65 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()
