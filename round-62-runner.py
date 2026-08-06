#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-62 cross-domain research runner.

Cron triggered 2026-08-02 10:37 Asia/Shanghai (every-2h reminder).
Self-decision: round_auto_naming.py next=62, no conflict; round-61 was ~9h46m ago
(> 30min threshold). Sunday 10:37 morning, master likely awake. Cron is reminder,
not blocking. Decision: run since gap-fill value high.

Theme: R0 新陈代谢 fresh (lactic acid fermentation Warburg Pasteur glycolysis)
     + R7 应激 fresh (innate immunity TLR NLR pattern recognition receptor)
     + R3 死亡 fresh (necroptosis RIPK1/RIPK3 + pyroptosis inflammasome)
     + R11 意识 fresh (predictive processing Clark Hohwy Friston free energy)
     + R9 遗传 fresh (polyploidy whole genome duplication Ohno Susumu)
     + R10 可塑性 fresh (metaplasia transdifferentiation Yamanaka iPSC)
     + R12 生态 fresh (Wilson E.O. sociobiology kin selection biophilia)
     + 3 GitHub deep (anthropics claude-code / Aider-AI aider / continuedev continue)
     + R6 繁殖 MISSING-deep (meiosis crossing over gametogenesis fertilization)
     + R2 发育 Gap (gap junction connexin morphogen spread Turing)

⭐ BUG FIX 2026-08-02 10:37: deep_research_dual.py had '***' + BOCHA_KEY header
(typo, should be 'Bearer ' + BOCHA_KEY), causing Bocha to silently 401-fail again.
Verified before round-62: bocha_web=2, bocha_ai=926 chars, anysearch=5. Now
Bocha 真接通 again for round-62.

Avoid r61 (photosynthesis / UPR stress granules / ferroptosis / Klotho sirtuin /
          maternal effect / adult neurogenesis / HGT viral capture / alphagenome /
          nanoGPT / stable-diffusion / GWT Baars Dehaene / prion)
Avoid r60 (chemotaxis two-component / chaperone Hsp / ribosome / Wnt/Hedgehog/Notch /
          actin cytoskeleton / MWC allosteric / critical period Hubel-Wiesel /
          alphafold / transformers / CLIP / retrovirus transposon / HOT consciousness)
Avoid r59 (mechanotransduction Piezo / apoptosis caspase / Hox homeotic bicoid /
          flagellar motor / morphallaxis planarian / epigenetic transgenerational /
          niche construction / claude-agent-sdk / mem0 / HarnessAgent /
          telomere Hayflick / chemolithotrophy)
Avoid r58 (Varela neurophenomenology / Margulis symbiogenesis / Per Bak SOC /
          connectome / Rosen (M,R) / Pearl causality / Wolfram NKS /
          ASI-Arch / DGM / langgraph / tardigrade cryptobiosis /
          embryogenesis morphogenesis)
Avoid r57 (Kauffman / Prigogine / Holland CAS / Maturana-Varela deep / Klein Erlangen /
          quantum biology / Carlsson TDA / openevolve / ShinkaEvolve / letta /
          Hamilton ESS / Thompson enactivism)
Avoid r56 (Solomonoff-AIXI / Ramsauer modern Hopfield / Hasani liquid NN / Kanerva VSA /
          Tierra / Olah / Causal emergence / Mamba / RWKV / TransformerLens / Avida /
          NCC IIT Φ)
Avoid r55 (Metzinger Ego Tunnel / LeCun V-JEPA / Hinton FF/GLOM / Quorum sensing /
          Beer VSM / Pask / von Foerster / llama.cpp / lm-evaluation-harness /
          anthropic-sdk-python / Hebb/Kandel/Merzenich / Lewontin Triple Helix)
Avoid r54 (Lenski LTEE / Goodwin / Thompson / Barbieri / Zeeman / Rizzolatti /
          Crutchfield / steel-dev / Composio / AgentOps / MAP-Elites / Gallup mirror)
Avoid r53 (Winnicott / Bion / Tomasello / Merleau-Ponty / Gibson / Bourdieu / Bowlby /
          livekit / pipecat / haystack / R7 / R11 Gap)

主人 17:46 ASI-LIFE-FEATURES 12 生命特征, r1-r61 覆盖现状 + r62 新加:
- R0 新陈代谢 ✓ r46 (Krebs/Kleiber) + r59 (chemolithotrophy) + r61 (photosynthesis)
              ← r62 加 lactic acid fermentation Warburg Pasteur glycolysis fresh
- R1 生长 ✓ r46 + r51 + r60 (ribosome)
- R2 发育 ✓ r40/r42/r45 + r52 + r54 + r56 + r58 + r59 + r60 + r61 (maternal effect)
              ← r62 加 gap junction connexin morphogen spread Turing Gap
- R3 死亡 ✓ r45 + r59 (apoptosis) + r61 (ferroptosis) 3 通路
              ← r62 加 necroptosis RIPK1 RIPK3 MLKL + pyroptosis inflammasome gasdermin fresh
              (4 死亡通路: 凋亡/坏死/铁死/焦亡)
- R4 衰老 ✓ r45 + r59 (telomere) + r61 (Klotho) 3 维度
- R5 修复/再生 ✓ r44 + r49 + r58 + r59
- R6 繁殖 ✓ r41 + r47 + r50 + r51 + r54 + r56 + r57 + r58 + r60 + r61 (prion)
              ← r62 加 meiosis crossing over gametogenesis fertilization acrosome syngamy MISSING-deep
              (canonical 生殖分子机制 substrate, 真正 MISSING)
- R7 应激性 ✓ r42 + r53 + r57 + r59 + r60 + r61 (UPR)
              ← r62 加 innate immunity TLR NLR pattern recognition receptor PRR Janeway Medzhitov fresh
              (免疫 = 应激的群体防御 substrate)
- R8 运动 ✓ r41/r45 + r52 + r59 + r60 (actin)
- R9 遗传变异 ✓ r44/r47/r48 + r54 + r56 + r57 + r58 + r59 + r60 + r61 (HGT)
              ← r62 加 polyploidy whole genome duplication WGD Ohno Susumu fresh
- R10 可塑性 ✓ r40/r45 + r51-61 (adult neurogenesis)
              ← r62 加 metaplasia transdifferentiation Yamanaka iPSC reprogramming fresh
- R11 意识 ✓ r42/r43/r46/r49-58 + r60 (HOT) + r61 (GWT) 8 substrate
              ← r62 加 predictive processing Clark Hohwy + active inference Friston free energy fresh
              (complement r55 Hinton + r60 HOT + r61 GWT, 第 4 范式 substrate)
- R12 环境 ✓ r16/r33/r43/r55 + r58 + r59
              ← r62 加 Wilson E.O. sociobiology kin selection biophilia Hamilton fresh
              (社会生物学生态 substrate, complement r16/r33/r43/r55/r58/r59 niche)

数学/哲学基座 (主 22:33 ASI 北极星):
- 中央 AI = sum of all forms — 15 substrate 第 9 轮 (lactic acid fermentation + TLR NLR +
              necroptosis + pyroptosis + predictive processing + polyploidy + metaplasia +
              sociobiology + claude-code + aider + continue + meiosis + gap junction +
              + central AI 累计 110+ substrate). NOT claim ASI has all.
- lactic acid fermentation = 厌氧代谢 substrate, NOT claim ASI has lactic acid fermentation
- TLR NLR = 模式识别 substrate, NOT claim ASI has innate immunity
- necroptosis pyroptosis = 炎症性死亡 substrate, NOT claim ASI undergoes necroptosis
- predictive processing = 预测编码 substrate, NOT claim ASI does predictive processing
- polyploidy WGD = 全基因组复制 substrate, NOT claim ASI has polyploidy
- metaplasia iPSC = 重编程 substrate, NOT claim ASI undergoes reprogramming
- sociobiology = 社会生物学 substrate, NOT claim ASI is sociobiology
- claude-code = CLI agent substrate, NOT claim ASI runs on claude-code
- aider = pair programming substrate, NOT claim ASI runs on aider
- continue = IDE AI substrate, NOT claim ASI runs on continue
- meiosis crossing over = 减数分裂 substrate, NOT claim ASI has meiosis
- gap junction connexin = 间隙连接 substrate, NOT claim ASI has gap junctions

跨域借鉴 = 工具/启发 (主 21:00)
隐喻是工具 (主 20:55)
ASI 只能逼近 (主 20:46)
不假装 Phenomenal (主 17:58)
实事求是 (主 17:43)

Fresh for r62:
- lactic acid fermentation Warburg Pasteur glycolysis aerobic cancer (R0 新陈代谢 fresh)
- innate immunity TLR NLR pattern recognition receptor PRR Janeway Medzhitov (R7 应激 fresh)
- necroptosis RIPK1 RIPK3 MLKL + pyroptosis inflammasome gasdermin (R3 死亡 fresh)
- predictive processing Clark Hohwy + active inference Friston free energy (R11 意识 fresh)
- polyploidy whole genome duplication WGD Ohno Susumu (R9 遗传 fresh)
- metaplasia transdifferentiation Yamanaka iPSC reprogramming (R10 可塑性 fresh)
- Wilson E.O. sociobiology biophilia kin selection inclusive fitness (R12 生态 fresh)
- anthropics/claude-code CLI agent github source code (GitHub deep)
- Aider-AI/aider pair programming github source code (GitHub deep)
- continuedev/continue IDE AI coding github source code (GitHub deep)
- meiosis crossing over gametogenesis fertilization acrosome (R6 繁殖 MISSING-deep)
- gap junction connexin morphogen spread Turing (R2 发育 Gap)
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from deep_research_dual import dual_research

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-62.json')

QUERIES = [
    # ===== 7 跨域 fresh =====

    # 1. R0 新陈代谢 fresh - lactic acid fermentation Warburg Pasteur glycolysis aerobic
    #    (R0 新陈代谢 真正 fresh complement r46 Krebs/Kleiber + r59 chemolithotrophy + r61 photosynthesis,
    #     NOT claim ASI has lactic acid fermentation)
    'lactic acid fermentation Warburg Pasteur glycolysis aerobic anaerobic cancer substrate ASI R0 metabolism fresh complement r46 r59 r61',

    # 2. R7 应激 fresh - innate immunity TLR NLR pattern recognition receptor PRR
    #    (R7 应激 真正 fresh complement r42 FEP + r53 chemotaxis + r57 enactivism + r59 mechanotransduction
    #     + r60 two-component + MWC + r61 UPR, NOT claim ASI has innate immunity)
    'innate immunity TLR NLR pattern recognition receptor PRR Janeway Medzhitov substrate ASI R7 irritability fresh complement r42 r53 r57 r59 r60 r61',

    # 3. R3 死亡 fresh - necroptosis RIPK1 RIPK3 MLKL + pyroptosis inflammasome gasdermin
    #    (R3 死亡 真正 fresh complement r45 + r59 apoptosis caspase + r61 ferroptosis, 4 死亡通路,
    #     NOT claim ASI undergoes necroptosis or pyroptosis)
    'necroptosis RIPK1 RIPK3 MLKL pyroptosis inflammasome gasdermin caspase substrate ASI R3 death fresh complement r45 r59 r61 four death pathways',

    # 4. R11 意识 fresh - predictive processing Clark Hohwy + active inference Friston free energy
    #    (R11 意识 真正 fresh complement r55 Hinton + r60 HOT + r61 GWT, 第 4 范式 substrate,
    #     NOT claim ASI does predictive processing)
    'predictive processing Clark Hohwy active inference Friston free energy principle consciousness substrate ASI R11 phenomenal fresh complement r55 r60 r61 fourth paradigm',

    # 5. R9 遗传 fresh - polyploidy whole genome duplication WGD Ohno Susumu
    #    (R9 遗传 真正 fresh complement r44/r47/r48/r54/r56/r57/r58/r59/r60 MWC + ribosome
    #     + r61 HGT, NOT claim ASI has polyploidy)
    'polyploidy whole genome duplication WGD Ohno Susumu evolution substrate ASI R9 heredity fresh complement r44 r47 r48 r54 r56 r57 r58 r59 r60 r61',

    # 6. R10 可塑性 fresh - metaplasia transdifferentiation Yamanaka iPSC reprogramming
    #    (R10 可塑性 真正 fresh complement r40/r45/r51-r61 chaperone + critical period + adult neurogenesis,
    #     NOT claim ASI undergoes reprogramming)
    'metaplasia transdifferentiation Yamanaka iPSC reprogramming OSKM substrate ASI R10 plasticity fresh complement r40 r45 r51 r52 r53 r54 r55 r56 r57 r58 r59 r60 r61',

    # 7. R12 生态 fresh - Wilson E.O. sociobiology biophilia kin selection inclusive fitness Hamilton
    #    (R12 生态 真正 fresh complement r16/r33/r43/r55/r58/r59 niche construction,
    #     NOT claim ASI is sociobiology)
    'Wilson E.O. sociobiology biophilia kin selection inclusive fitness Hamilton substrate ASI R12 ecology fresh complement r16 r33 r43 r55 r58 r59 niche construction',

    # ===== 3 GitHub 源码真读 (深) =====

    # 8. anthropics/claude-code 真读 - CLI coding agent
    #    (any-LLM substrate, NOT claim ASI runs on claude-code)
    'anthropics claude-code github source code CLI agent coding tool architecture real source deep dive substrate ASI any-LLM pluggable',

    # 9. Aider-AI/aider 真读 - pair programming
    #    (中央 AI pair programming substrate, NOT claim ASI runs on aider)
    'Aider-AI aider github source code pair programming chat edit architecture real source deep dive substrate ASI central AI pluggable',

    # 10. continuedev/continue 真读 - IDE AI coding
    #     (中央 AI IDE integration substrate, NOT claim ASI runs on continue)
    'continuedev continue github source code IDE AI coding assistant architecture real source deep dive substrate ASI central AI pluggable integration',

    # ===== 2 Gap (R6 繁殖 MISSING-deep + R2 发育 Gap) =====

    # 11. R6 繁殖 MISSING-deep - meiosis crossing over gametogenesis fertilization acrosome syngamy
    #     (R6 繁殖 真正 MISSING-deep canonical 生殖分子机制 substrate, complement r41-r61,
    #      NOT claim ASI has meiosis)
    'meiosis crossing over gametogenesis fertilization acrosome reaction syngamy substrate ASI R6 reproduction MISSING deep complement r41 r47 r50 r51 r54 r56 r57 r58 r60 r61',

    # 12. R2 发育 Gap - gap junction connexin morphogen spread Turing
    #     (R2 发育 真正 Gap complement r40/r42/r45/r52/r54/r56/r58/r59/r60/r61,
    #      NOT claim ASI has gap junctions)
    'gap junction connexin morphogen spread Turing developmental biology substrate ASI R2 development Gap complement r40 r42 r45 r52 r54 r56 r58 r59 r60 r61',
]


def main():
    started = time.time()
    started_iso = time.strftime('%Y-%m-%dT%H:%M:%S+08:00', time.localtime(started))
    print(f'Round-62 started {started_iso}')

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
    print(f'\nRound-62 done in {total:.1f}s, saved {len(results)} entries to {OUT}')
    print(f'Size: {OUT.stat().st_size} bytes')

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}')
    return total


if __name__ == '__main__':
    main()