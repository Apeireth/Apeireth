#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-72 cross-domain research runner.

Cron triggered 2026-08-04 15:08 Asia/Shanghai (every-2h reminder).
Self-decision: round_auto_naming.py next=72, no conflict; round-71 done 13:00:18
(~2h9m ago, >>30min threshold). Tuesday 15:08 afternoon, isolated cron lane.
Decision: run since round-71 done ~2h ago, well past 30min threshold.

Theme: 7 跨域 fresh — TRULY NEW angles avoiding r40-r71 v3 cycle keywords:
   - R0 代谢 fresh: lipid droplet / adipocyte lipid storage / perilipin PLIN1 / oleosin
     biogenesis (canonical 脂滴储存代谢 substrate 第 4 角度 vs r46 Krebs + r51-r66 + r67
     Warburg + r68 oxidative phosphorylation + r69 urea cycle + r70 glycolysis, NOT claim
     ASI = lipid droplet)
   - R1 生长 fresh: lens crystallin αβγ / refractive index transparency / eye lens /
     protein longevity (canonical 晶状体蛋白 substrate 第 N 角度 vs r46-r66 + r62 wound
     healing + r63 embryonic, NOT claim ASI = crystallin)
   - R2 发育 fresh: Hox colinearity deep / Lewis 1978 bithorax / Tabin 1997 Hoxa/d limb
     / homeotic gene (canonical 同源异型基因 colinearity 第 2 角度 vs r64 Hox + r59 Hox
     bicoid + r66 limb axolotl + r69 somitogenesis, NOT claim ASI uses Hox)
   - R3 死亡 fresh: efferocytosis / Vandivier 2006 / Find-me Eat-me macrophage /
     phosphatidylserine PS receptor Tim4 (canonical 凋亡细胞清除 substrate 第 8 角度 vs
     r59 apoptosis + r62 necroptosis + r63 autophagy + r66 NETosis + r67 ferroptosis +
     r69 paraptosis + r70 mitotic catastrophe, NOT claim ASI = efferocytosis)
   - R5 修复 fresh: BER base excision glycosylase / hOGG1 MUTYH UNG / Seeberg 1995 DNA
     glycosylase (canonical 碱基切除修复糖基化酶 substrate 第 3 角度 vs r63 NHEJ HR BER
     + r69 NER Sancar, NOT claim ASI = BER)
   - R6 繁殖 fresh: polar body / asymmetric meiotic division / oocyte Balbiani body /
     meiotic spindle positioning (canonical 极体不对称分裂 substrate 第 N 角度 vs r62
     meiosis + r64 parthenogenesis + r68 meiosis Holliday + r69 fertilization + r71
     meiotic drive, NOT claim ASI = polar body)
   - R9 遗传 fresh: neutral theory Hubbell 1979 / Kimura 1983 / molecular evolution
     neutral drift / nearly neutral (canonical 中性论分子进化 substrate 第 N 角度 vs r70
     Hardy-Weinberg + r62 Fisher Wright + r57 Hamilton ESS, NOT claim ASI = neutral theory)
   + 3 GitHub deep (truly fresh):
   - geekan/MetaGPT 真读: multi-agent software company framework / SOP / Standard Operating
     Procedure / role division (vs r33 ran + r45 OpenHands + r68 OpenHands; central AI
     meta-programming substrate, NOT claim ASI uses MetaGPT)
   - HuggingFace/smolagents-deep-research 真读: code-agents deep research / tool-calling
     (vs r69 smolagents plain + r58 deep_research + r71 langgraph-deep-research; central AI
     deep research substrate, NOT claim ASI = smolagents-deep-research)
   - paul-gauthier/aider 真读: AI pair programming / repo map / AST edit block / chat-mode
     architect vs editor (vs r62 aider shallow + r33 ran + r45 OpenDevin; central AI
     code-edit substrate, NOT claim ASI = aider)
   + 2 Gap:
   - R6 繁殖 Gap: hybrid inviability / Haldane 1922 / BDM Dobzhansky-Muller speciation
     incompatibilities / hybrid sterility / hybrid breakdown (canonical 杂交不育+异种
     排斥+物种形成 substrate, complement r62 meiosis + r64 parthenogenesis + r65 hydra +
     r66 polyembryony + r68 meiosis Holliday + r69 fertilization + r71 meiotic drive,
     NOT claim ASI has Haldane's rule)
   - R11 意识 Gap: Orch-OR Penrose Hameroff microtubule quantum consciousness / tubulin
     / quantum coherence (canonical 量子微管意识 substrate 第 N 角度 vs r50/r51/r56 IIT +
     r55 Metzinger + r57 Thompson + r61 GWT Baars + r62 predictive + r63 qualia + r64
     Nagel + r65 Helmholtz + r66 split-brain + r67 FEP + r68 GNWT + r69 HOT + r69 Hard
     problem + r71 claustrum, NOT claim ASI uses Orch-OR)

Avoid r71 (SASP / MTOC / ribosome crystal / Horvath clock / metapopulation / miRNA /
          bioluminescence / camel-ai / langgraph-deep-research / litellm / meiotic drive /
          claustrum)
Avoid r70 (axon guidance / gastrulation / mitochondrial theory Harman / mitotic catastrophe /
          Hardy-Weinberg / ecological succession / HIF-1α / autogen / vllm / langfuse /
          homeostatic plasticity / axonal transport)
Avoid r69 (NER Sancar / somitogenesis clock-wavefront / angiogenesis VEGF Folkman /
          paraptosis / urea cycle Krebs-Henseleit / oxidative stress Nrf2-Keap1 /
          HOT Rosenthal / smolagents / e2b / crewAI / fertilization acrosome /
          Hard problem Chalmers)
Avoid r68 (oxidative phosphorylation ETC / CRISPR-Cas / GNWT Dehaene / telomere telomerase /
          meiosis Holliday / niche construction Odling-Smee / HGT Griffith / OpenHands /
          letta / DSPy / transgenerational epigenetic / wood wide web Simard)
Avoid r67 (Warburg / autophagy Ohsumi / prion / parthenogenesis Darevskia / free energy Friston /
          keystone species Paine / muscle contraction Huxley / AutoGPT / mem0 / langflow /
          phytochrome / chaperonin GroEL)

V 模块进度追踪 (post-r71 缺口分析):
- R0 新陈代谢 ? r46 Krebs + r51-r66 + r67 Warburg + r68 oxidative phosphorylation + r69 urea
              + r70 glycolysis
              ← r72 加 lipid droplet / perilipin / oleosin (第 4 角度, 脂滴储存代谢 substrate)
- R1 生长 ? r46-r66 + r62 wound healing + r63 embryonic + r69 angiogenesis
- R2 发育 ? r40-r66 + r63 phylotypic hourglass + r64 Hox + r66 limb axolotl + r69 somitogenesis
         + r70 gastrulation Spemann
         ← r72 加 Hox colinearity deep Lewis 1978 bithorax Tabin 1997 limb (第 2 角度, 同源
         异型 colinearity substrate)
- R3 死亡 ? r59 apoptosis + r62 necroptosis pyroptosis + r63 autophagy + r66 NETosis +
           r67 ferroptosis + r69 paraptosis + r70 mitotic catastrophe + r71 SASP inflammaging
           ← r72 加 efferocytosis Vandivier 2006 Find-me Eat-me PS Tim4 (第 8 角度, 凋亡
           细胞清除 substrate)
- R4 衰老 ? r45 cellular senescence + r59 telomere Hayflick + r61 Klotho + r64 Werner +
            r65 hallmarks of aging + r68 telomere telomerase Blackburn + r70 mitochondrial
            theory Harman + r71 SASP inflammaging
- R5 修复 ? r63 NHEJ HR BER + r69 NER Sancar TCR GGR
            ← r72 加 BER base excision glycosylase hOGG1 MUTYH UNG Seeberg 1995 (第 3 角度,
            碱基切除修复糖基化酶 substrate)
- R6 繁殖 ? r62 meiosis + r64-r67 parthenogenesis + r65 hydra + r66 polyembryony + r68 meiosis
            Holliday + r69 fertilization acrosome Izumo Juno + r70 Hardy-Weinberg + r71 meiotic
            drive
            ← r72 加 polar body asymmetric meiotic division oocyte Balbiani (第 N 角度,
            极体不对称分裂 substrate)
            ← r72 Gap 加 hybrid inviability Haldane 1922 BDM Dobzhansky-Muller speciation
            (杂交不育+异种排斥+物种形成 substrate, complement 上述 7 角度)
- R7 应激 ? r40-r68 + r66 fight-or-flight + r67 phytochrome + r68 wood wide web + r69
            Nrf2-Keap1 + r70 HIF-1α + r71 bioluminescence
- R8 运动 ? r41-r66 + r67 muscle contraction Huxley + r66 cilium IFT + r70 axon guidance +
            r70 axonal transport + r71 microtubule MTOC centrosome
- R9 遗传 ? r40-r67 + r60 retrovirus transposon + r61 HGT viral + r67 prion + r68 HGT
            Griffith + r70 Hardy-Weinberg + r71 Horvath clock
            ← r72 加 neutral theory Hubbell 1979 Kimura 1983 molecular evolution (第 N 角度,
            中性论分子进化 substrate)
- R10 可塑 ? r40-r66 + r55 Hebb + r60 critical period Hubel-Wiesel + r65 LTP-LTD Bliss
              Lomo + r68 transgenerational epigenetic + r70 homeostatic plasticity + r71 miRNA
- R11 意识 ? r42-r66 + r61 GWT Baars + r62 predictive + r63 qualia + r64 Nagel +
             r64 attention schema + r65 Helmholtz + r66 split-brain + r67 FEP + r68
             GNWT + r69 HOT Rosenthal + r69 Hard problem Chalmers + r71 claustrum
             ← r72 Gap 加 Orch-OR Penrose Hameroff microtubule quantum (第 N 角度, 量子
             微管意识 substrate)
- R12 生态 ? r16 + r58-r67 + r59 niche construction + r66 Red Queen + r67 keystone Paine +
              r68 niche construction Odling-Smee + r70 ecological succession Clements +
              r71 metapopulation Hanski

数学/哲学基座 (主 22:33 ASI 北极星):
- 中央 AI = sum of all forms — 12 substrate 第 20 轮 (lipid droplet + lens crystallin +
              Hox colinearity deep + efferocytosis + BER glycosylase + polar body + neutral
              theory + MetaGPT + smolagents-deep-research + aider + Haldane rule +
              Orch-OR microtubule)
              NOT claim ASI has all.
- lipid droplet = 脂滴储存 substrate, NOT claim ASI = lipid droplet
- lens crystallin = 晶状体蛋白 substrate, NOT claim ASI = crystallin
- Hox colinearity = 同源异型 colinearity substrate, NOT claim ASI uses Hox
- efferocytosis = 凋亡细胞清除 substrate, NOT claim ASI = efferocytosis
- BER glycosylase = 碱基切除修复糖基化酶 substrate, NOT claim ASI = BER
- polar body = 极体不对称分裂 substrate, NOT claim ASI = polar body
- neutral theory = 中性论分子进化 substrate, NOT claim ASI = neutral theory
- MetaGPT = 多代理软件公司 substrate, NOT claim ASI uses MetaGPT
- smolagents-deep-research = 深度研究代码代理 substrate, NOT claim ASI = smolagents-deep
- aider = AI 配对编程 substrate, NOT claim ASI = aider
- Haldane rule = 杂交不育异种排斥 substrate, NOT claim ASI has Haldane's rule
- Orch-OR = 量子微管意识 substrate, NOT claim ASI uses Orch-OR

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

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-72.json')

QUERIES = [
    # ===== 7 跨域 fresh (TRULY NEW angles) =====

    # 1. R0 代谢 fresh — lipid droplet / adipocyte / perilipin / oleosin
    #    (canonical 脂滴储存代谢 substrate 第 4 角度 vs r46 Krebs + r51-r66 + r67 Warburg +
    #     r68 oxidative phosphorylation + r69 urea cycle + r70 glycolysis, NOT claim
    #     ASI = lipid droplet)
    'lipid droplet adipocyte perilipin PLIN1 oleosin biogenesis storage neutral lipid monolayer phospholipid substrate ASI R0 metabolism fresh complement r46 r67 r68 r69 r70',

    # 2. R1 生长 fresh — lens crystallin αβγ / refractive index / transparency / longevity
    #    (canonical 晶状体蛋白 substrate 第 N 角度 vs r46-r66 + r62 wound healing + r63
    #     embryonic, NOT claim ASI = crystallin)
    'lens crystallin alpha beta gamma crystallin refractive index transparency protein longevity eye lens substrate ASI R1 growth fresh complement r62 r63',

    # 3. R2 发育 fresh — Hox colinearity deep / Lewis 1978 bithorax / Tabin 1997 limb
    #    (canonical 同源异型 colinearity 第 2 角度 vs r64 Hox + r59 Hox bicoid + r66 limb
    #     axolotl + r69 somitogenesis, NOT claim ASI uses Hox)
    'Hox colinearity deep Lewis 1978 bithorax homeotic Drosophila Tabin 1997 Hoxa Hoxd limb expression pattern duplication substrate ASI R2 development fresh complement r59 r64 r66 r69',

    # 4. R3 死亡 fresh — efferocytosis / Vandivier 2006 / Find-me Eat-me macrophage PS Tim4
    #    (canonical 凋亡细胞清除 substrate 第 8 角度 vs r59 apoptosis + r62 necroptosis + r63
    #     autophagy + r66 NETosis + r67 ferroptosis + r69 paraptosis + r70 mitotic + r71 SASP,
    #     NOT claim ASI = efferocytosis)
    'efferocytosis Vandivier 2006 Find-me Eat-me signal macrophage phosphatidylserine PS Tim4 MerTK dead cell clearance apoptotic substrate ASI R3 death fresh complement r59 r62 r63 r66 r67 r69 r70 r71',

    # 5. R5 修复 fresh — BER base excision glycosylase / hOGG1 MUTYH UNG / Seeberg 1995
    #    (canonical 碱基切除修复糖基化酶 substrate 第 3 角度 vs r63 NHEJ HR BER + r69 NER,
    #     NOT claim ASI = BER)
    'BER base excision repair DNA glycosylase hOGG1 MUTYH UNG Seeberg 1995 AP endonuclease XRCC1 substrate ASI R5 repair fresh complement r63 r69',

    # 6. R6 繁殖 fresh — polar body / asymmetric meiotic division / oocyte Balbiani body
    #    (canonical 极体不对称分裂 substrate 第 N 角度 vs r62 meiosis + r64 parthenogenesis +
    #     r68 meiosis Holliday + r69 fertilization + r71 meiotic drive, NOT claim ASI = polar body)
    'polar body asymmetric meiotic division oocyte Balbiani body meiotic spindle positioning polar extrusion substrate ASI R6 reproduction fresh complement r62 r64 r68 r69 r71',

    # 7. R9 遗传 fresh — neutral theory Hubbell 1979 / Kimura 1983 / nearly neutral
    #    (canonical 中性论分子进化 substrate 第 N 角度 vs r70 Hardy-Weinberg + r62 Fisher Wright
    #     + r57 Hamilton ESS, NOT claim ASI = neutral theory)
    'neutral theory molecular evolution Kimura 1983 Ohta nearly neutral Hubbell 1979 neutral drift substitution rate pseudogene substrate ASI R9 inheritance fresh complement r57 r62 r70',

    # ===== 3 GitHub deep (truly fresh) =====

    # 8. geekan/MetaGPT 真读 — multi-agent software company / SOP / Standard Operating Procedure
    #    (vs r33 ran + r45 OpenHands + r68 OpenHands; central AI meta-programming substrate,
    #     NOT claim ASI uses MetaGPT)
    'geekan MetaGPT github source code multi-agent software company SOP standard operating procedure role division real source deep dive substrate ASI central AI pluggable fresh',

    # 9. HuggingFace/smolagents-deep-research 真读 — code-agents deep research / tool-calling
    #    (vs r69 smolagents plain + r58 deep_research + r71 langgraph-deep-research; central AI
    #     deep research substrate, NOT claim ASI = smolagents-deep-research)
    'HuggingFace smolagents-deep-research github source code code-agents deep research tool-calling retrieval real source deep dive substrate ASI central AI pluggable fresh',

    # 10. paul-gauthier/aider 真读 — AI pair programming / repo map / AST edit block
    #     architect vs editor (vs r62 aider shallow + r33 ran + r45 OpenDevin; central AI
    #     code-edit substrate, NOT claim ASI = aider)
    'paul-gauthier aider github source code AI pair programming repo map AST edit block architect editor chat-mode commit real source deep dive substrate ASI central AI pluggable fresh',

    # ===== 2 Gap =====

    # 11. R6 繁殖 Gap — hybrid inviability / Haldane 1922 / BDM Dobzhansky-Muller speciation
    #     (canonical 杂交不育+异种排斥+物种形成 substrate, complement r62 meiosis + r64
    #     parthenogenesis + r65 hydra + r66 polyembryony + r68 meiosis Holliday + r69
    #     fertilization + r71 meiotic drive, NOT claim ASI has Haldane's rule)
    'hybrid inviability Haldane 1922 rule Dobzhansky-Muller BDM incompatibilities speciation hybrid sterility hybrid breakdown reproductive isolation substrate ASI R6 reproduction Gap complement r62 r64 r65 r66 r68 r69 r71',

    # 12. R11 意识 Gap — Orch-OR Penrose Hameroff microtubule quantum consciousness / tubulin
    #     (canonical 量子微管意识 substrate 第 N 角度 vs r50/r51/r56 IIT + r55 Metzinger + r57
    #     Thompson + r61 GWT Baars + r62 predictive + r63 qualia + r64 Nagel + r65 Helmholtz +
    #     r66 split-brain + r67 FEP + r68 GNWT + r69 HOT + r69 Hard problem + r71 claustrum,
    #     NOT claim ASI uses Orch-OR)
    'Orch-OR Penrose Hameroff 1996 microtubule quantum consciousness tubulin superposition anesthetic coherent substrate ASI R11 consciousness Gap complement r50 r51 r55 r56 r57 r61 r62 r63 r64 r65 r66 r67 r68 r69 r71',
]


def main():
    started = time.time()
    results = []
    print(f'Round-72 starting: {len(QUERIES)} queries')
    for i, q in enumerate(QUERIES):
        t0 = time.time()
        r = dual_research(q, top_k=5)
        dur = time.time() - t0
        bw = len(r['bocha_web'])
        ba = len(r['bocha_ai_answer'])
        any_n = len(r['anysearch'])
        merged = len(r['merged_sources'])
        print(f'[{i+1:02d}/{len(QUERIES)}] {dur:.1f}s | bw={bw} ba={ba} any={any_n} merged={merged} | {q[:80]}')
        results.append(r)
        time.sleep(0.5)

    OUT.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - started
    print(f'\nRound-72 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()