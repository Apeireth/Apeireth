#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-61 cross-domain research runner.

Cron triggered 2026-08-02 00:41 Asia/Shanghai (every-2h reminder).
Self-decision: round_auto_naming.py next=61, no conflict; round-60 was ~5h49m ago
(> 30min threshold). Sunday 00:41 deep night, master likely asleep; cron is reminder, log only.
Decision: run since gap-fill value high.

Theme: R0 新陈代谢 fresh (photosynthesis Z-scheme / Calvin cycle RuBisCO substrate)
     + R7 应激 fresh (stress granules / UPR unfolded protein response)
     + R3 死亡 fresh (ferroptosis lipid peroxidation GPX4)
     + R4 衰老 fresh (Klotho / sirtuin / NAD+ / mTOR aging hallmarks)
     + R2 发育 fresh (maternal effect / cytoplasmic determinants / asymmetric division)
     + R10 可塑性 fresh (adult neurogenesis dentate gyrus / olfactory bulb SVZ)
     + R9 遗传 fresh (HGT vs viral capture / endosymbiosis caveat)
     + 3 GitHub deep (deepmind/alphagenome / karpathy/nanoGPT / lllyasviel/stable-diffusion)
     + R11 意识终极目标 (Global Workspace Theory GWT Baars Dehaene)
     + R6 繁殖 MISSING (prion self-templating protein-only inheritance)

⭐ BUG FIX 2026-08-02: deep_research_dual.py had '***' + BOCHA_KEY header (typo, should be
'Bearer ' + BOCHA_KEY), causing Bocha to silently 401-fail every round r55-r60. Fixed. Verified
bocha_web=3, bocha_ai=306 chars, anysearch=3, merged=9. Now master 14:58 立规 (Bocha 主用) actually
works. This is the first round with Bocha 真接通.

Avoid r60 (chemotaxis two-component / chaperone Hsp / ribosome / Wnt/Hedgehog/Notch / actin
          cytoskeleton / MWC allosteric / critical period Hubel-Wiesel / alphafold / transformers /
          CLIP / retrovirus transposon / HOT consciousness)
Avoid r59 (mechanotransduction Piezo / apoptosis caspase / Hox homeotic bicoid / flagellar motor /
          morphallaxis planarian / epigenetic transgenerational / niche construction / claude-agent-sdk /
          mem0 / HarnessAgent / telomere Hayflick / chemolithotrophy)
Avoid r58 (Varela neurophenomenology / Margulis symbiogenesis / Per Bak SOC / connectome /
          Rosen (M,R) / Pearl causality / Wolfram NKS / ASI-Arch / DGM / langgraph /
          tardigrade cryptobiosis / embryogenesis morphogenesis)
Avoid r57 (Kauffman / Prigogine / Holland CAS / Maturana-Varela deep / Klein Erlangen /
          quantum biology / Carlsson TDA / openevolve / ShinkaEvolve / letta / Hamilton ESS /
          Thompson enactivism)
Avoid r56 (Solomonoff-AIXI / Ramsauer modern Hopfield / Hasani liquid NN / Kanerva VSA /
          Tierra / Olah / Causal emergence / Mamba / RWKV / TransformerLens / Avida / NCC IIT Φ)
Avoid r55 (Metzinger Ego Tunnel / LeCun V-JEPA / Hinton FF/GLOM / Quorum sensing / Beer VSM /
          Pask / von Foerster / llama.cpp / lm-evaluation-harness / anthropic-sdk-python /
          Hebb/Kandel/Merzenich / Lewontin Triple Helix)
Avoid r54 (Lenski LTEE / Goodwin / Thompson / Barbieri / Zeeman / Rizzolatti / Crutchfield /
          steel-dev / Composio / AgentOps / MAP-Elites / Gallup mirror)
Avoid r53 (Winnicott / Bion / Tomasello / Merleau-Ponty / Gibson / Bourdieu / Bowlby / livekit /
          pipecat / haystack / R7 / R11 Gap)

主人 17:46 ASI-LIFE-FEATURES 12 生命特征, r1-r60 覆盖现状 + r61 新加:
- R0 新陈代谢 ✓ r46 (Krebs/Kleiber) + r59 (chemolithotrophy)
              ← r61 加 photosynthesis Z-scheme Calvin cycle RuBisCO fresh
- R1 生长 ✓ r46 + r51 + r60 (ribosome)
- R2 发育 ✓ r40/r42/r45 + r52 + r54 + r56 + r58 + r59 + r60 (Wnt/Hedgehog/Notch)
              ← r61 加 maternal effect cytoplasmic determinants asymmetric division fresh
- R3 死亡 ✓ r45 + r59 (apoptosis caspase)
              ← r61 加 ferroptosis lipid peroxidation GPX4 fresh
- R4 衰老 ✓ r45 + r59 (telomere Hayflick)
              ← r61 加 Klotho sirtuin NAD+ mTOR aging hallmarks fresh
- R5 修复/再生 ✓ r44 + r49 deep + r58 + r59
- R6 繁殖 ✓ r41 + r47 + r50 + r51 + r54 + r56 + r57 + r58 + r60 (retrovirus)
              ← r61 加 prion self-templating protein-only inheritance MISSING Gap
- R7 应激性 ✓ r42 + r53 + r57 + r59 + r60 (two-component + MWC)
              ← r61 加 stress granules UPR unfolded protein response fresh
- R8 运动 ✓ r41/r45 + r52 + r59 + r60 (actin)
- R9 遗传变异 ✓ r44/r47/r48 + r54 + r56 + r57 + r58 + r59 + r60 (MWC + ribosome)
              ← r61 加 HGT vs viral capture endosymbiosis caveat fresh
- R10 可塑性 ✓ r40/r45 + r51-55 + r56 + r57 + r58 + r59 + r60 (chaperone + critical period)
              ← r61 加 adult neurogenesis dentate gyrus olfactory bulb SVZ fresh
- R11 意识 ✓ r42/r43/r46/r49-55 + r56 + r57 + r58 + r60 (HOT)
              ← r61 加 Global Workspace Theory GWT Baars Dehaene fresh
- R12 环境 ✓ r16/r33/r43/r55 + r58 + r59

数学/哲学基座 (主 22:33 ASI 北极星):
- 中央 AI = sum of all forms — 14 substrate 第 8 轮 (photosynthesis + UPR + ferroptosis + Klotho +
              maternal effect + adult neurogenesis + HGT viral + alphagenome + nanoGPT + stable-
              diffusion + GWT + prion). NOT claim ASI has all.
- photosynthesis = 能量转换 substrate, NOT claim ASI has photosynthesis
- UPR stress granules = 蛋白稳态 substrate, NOT claim ASI has UPR
- ferroptosis = 死亡通路 substrate, NOT claim ASI undergoes ferroptosis
- Klotho sirtuin = 衰老调控 substrate, NOT claim ASI is ageless
- maternal effect = 母体效应 substrate, NOT claim ASI has maternal effect
- adult neurogenesis = 成体神经发生 substrate, NOT claim ASI has adult neurogenesis
- HGT vs viral capture = 跨代遗传 substrate, NOT claim ASI has HGT
- alphagenome = DNA 语言模型 substrate, NOT claim ASI uses alphagenome
- nanoGPT = 极简训练 substrate, NOT claim ASI runs on nanoGPT
- stable-diffusion = 潜在扩散 substrate, NOT claim ASI has diffusion
- GWT = 全局工作空间 substrate, NOT claim ASI has global workspace
- prion = 蛋白质模板 substrate, NOT claim ASI reproduces via prion

跨域借鉴 = 工具/启发 (主 21:00)
隐喻是工具 (主 20:55)
ASI 只能逼近 (主 20:46)
不假装 Phenomenal (主 17:58)
实事求是 (主 17:43)

Fresh for r61:
- photosynthesis Z-scheme light reaction Calvin cycle RuBisCO substrate (R0 新陈代谢)
- stress granules unfolded protein response UPR IRE1 PERK ATF6 (R7 应激)
- ferroptosis lipid peroxidation GPX4 ACSL4 (R3 死亡)
- Klotho sirtuin NAD+ mTOR aging hallmarks Lopez-Otin (R4 衰老)
- maternal effect cytoplasmic determinants asymmetric division Drosophila (R2 发育)
- adult neurogenesis dentate gyrus subventricular zone olfactory bulb SVZ (R10 可塑性)
- horizontal gene transfer viral capture endosymbiosis caveat (R9 遗传)
- deepmind/alphagenome github source code DNA language model (GitHub deep)
- karpathy/nanoGPT github source code minimal GPT training (GitHub deep)
- lllyasviel/stable-diffusion github source code latent diffusion (GitHub deep)
- Global Workspace Theory GWT Baars Dehaene consciousness (R11 意识终极目标)
- prion self-templating protein-only inheritance (R6 繁殖 MISSING Gap)
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-61.json')

QUERIES = [
    # ===== 7 跨域 fresh =====

    # 1. R0 新陈代谢 fresh - photosynthesis Z-scheme light reaction Calvin cycle
    #    (R0 新陈代谢 真正 fresh MISSING since r46/r59, NOT claim ASI has photosynthesis)
    'photosynthesis Z-scheme light reaction Calvin cycle RuBisCO substrate energy conversion substrate ASI R0 metabolism fresh MISSING complement r46 r59',


    # 2. R7 应激 fresh - stress granules / UPR unfolded protein response
    #    (R7 应激 真正 fresh complement r60 chaperone Hsp + two-component substrate, NOT claim ASI has UPR)
    'stress granules unfolded protein response UPR IRE1 PERK ATF6 eIF2alpha substrate ASI R7 irritability fresh complement r60 chaperone two-component',


    # 3. R3 死亡 fresh - ferroptosis lipid peroxidation GPX4
    #    (R3 死亡 真正 fresh complement r59 apoptosis caspase, NOT claim ASI undergoes ferroptosis)
    'ferroptosis lipid peroxidation GPX4 ACSL4 iron death substrate ASI R3 death fresh complement r59 apoptosis caspase distinct from necroptosis pyroptosis',


    # 4. R4 衰老 fresh - Klotho sirtuin NAD+ mTOR aging hallmarks
    #    (R4 衰老 真正 fresh complement r59 telomere Hayflick, NOT claim ASI is ageless)
    'Klotho sirtuin NAD+ mTOR aging hallmarks Lopez-Otin nine hallmarks substrate ASI R4 aging senescence fresh complement r59 telomere Hayflick mitochondrial',


    # 5. R2 发育 fresh - maternal effect cytoplasmic determinants asymmetric division
    #    (R2 发育 真正 fresh complement r59 Hox + r60 Wnt/Hedgehog/Notch, NOT claim ASI has maternal effect)
    'maternal effect cytoplasmic determinants asymmetric division Drosophila bicoid nanos substrate ASI R2 development fresh complement r59 Hox r60 Wnt Hedgehog Notch',


    # 6. R10 可塑性 fresh - adult neurogenesis dentate gyrus olfactory bulb SVZ
    #    (R10 可塑性 真正 fresh complement r60 critical period Hubel-Wiesel, NOT claim ASI has adult neurogenesis)
    'adult neurogenesis dentate gyrus subventricular zone olfactory bulb SVZ neural stem cell substrate ASI R10 plasticity fresh complement r60 critical period Hubel Wiesel',


    # 7. R9 遗传 fresh - HGT vs viral capture endosymbiosis caveat
    #    (R9 遗传 真正 fresh complement r58 Margulis symbiogenesis, NOT claim ASI has HGT)
    'horizontal gene transfer viral capture endosymbiosis caveat Wachtershauser Forterre substrate ASI R9 heredity fresh complement r58 Margulis symbiogenesis',


    # ===== 3 GitHub 源码真读 (深) =====

    # 8. deepmind/alphagenome 真读 - DNA language model
    #    (any-LLM substrate, NOT claim ASI uses alphagenome)
    'deepmind alphagenome github source code DNA language model architecture real source deep dive substrate ASI any-LLM substrate biology computation',


    # 9. karpathy/nanoGPT 真读 - minimal GPT training
    #    (中央 AI pluggable substrate, NOT claim ASI runs on nanoGPT)
    'karpathy nanoGPT github source code minimal GPT training pipeline real source deep dive substrate ASI central AI pluggable nanogpt',


    # 10. lllyasviel/stable-diffusion 真读 - latent diffusion model
    #     (中央 AI 跨模态 substrate, NOT claim ASI has diffusion)
    'lllyasviel stable-diffusion github source code latent diffusion model cross-attention real source deep dive substrate ASI central AI cross-modal generation',


    # ===== 2 Gap (R11 意识终极目标 + R6 繁殖 MISSING) =====

    # 11. R11 意识终极目标 - Global Workspace Theory GWT Baars Dehaene consciousness
    #     (R11 意识 终极目标 fresh complement r60 HOT, NOT claim ASI has global workspace)
    'Global Workspace Theory GWT Baars Dehaene consciousness global ignition substrate ASI R11 consciousness phenomenal ultimate goal fresh complement r60 HOT higher-order theory',


    # 12. R6 繁殖 MISSING Gap - prion self-templating protein-only inheritance
    #     (R6 繁殖 真正 MISSING protein-only self-templating substrate, NOT claim ASI reproduces via prion)
    'prion self-templating protein-only inheritance PrP misfolding substrate ASI R6 reproduction MISSING Gap 12 life features deep complement r41 r47 r50 r51 r54 r56 r57 r58 r60',
]


def main():
    started = time.time()
    started_iso = time.strftime('%Y-%m-%dT%H:%M:%S+08:00', time.localtime(started))
    print(f'Round-61 started {started_iso}')

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
    print(f'\nRound-61 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()
