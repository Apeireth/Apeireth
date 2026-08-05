#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Round-85 cross-domain research runner (FAST variant).

Cron triggered 2026-08-05 20:51 Asia/Shanghai (every-2h reminder).
Self-decision: round-84 done 2026-08-05 20:01 (~50min ago, >30min threshold).
Wednesday 20:51 evening, isolated cron lane, M3 model.
Decision: RUN round-85 now (12 fresh angles, no overlap with r71-r84 cycle).

Theme: 12 TRULY NEW angles avoiding r71-r84 cycle keywords. Pre-search verified
no overlap with: hydra / pyroptosis / place-grid / Hedgehog / heat-shock / kinesin /
niche / asi-arch / openevolve / ShinkaEvolve / hydra-budding / global-workspace /
phosphate / Gurdon / theta-gamma / Holliday / lipid-raft / Huxley / memvid /
browser-use / DGM / anastasis / predictive-processing / thioester / diapause /
Krebs / cerebellum / X-inactivation / AMPK / mitochondrial / rhizobium / DSPy /
instructor / perplexica / Wolbachia / Lamme / RNA-world / Hox / Yamanaka /
endosymbiosis / Vibrio / transposon / Gaia / haystack / SWE-agent / memGPT / HGT /
prion / Turing / C-elegans-Sulston / ferroptosis / synaptic-pruning / CRISPR /
microbial-loop / mem0 / langgraph / claude-sdk / telomere / predictive-coding /
BAT / mollusk-shell / somitogenesis / necroptosis / muscle-Huxley / piRNA /
panarchy / OpenHands / crewAI / autogen / paramutation / attention-schema / klotho /
UPR / flagellar / adult-neurogenesis / mycorrhizal / hedgehog / openai-python /
langflow / planarian / IIT / feather / SLE / NER / jasmonic / vestibular /
homeostatic-plasticity / metapopulation-Hanski / AI-CUDA / camel / TypeChat /
parthenogenesis / neurophenomenology / circadian / neural-crest / progeria / ABA /
axonal-sprouting / BCM / food-web / openai-evals / langchain / prefect /
gametogenesis / enactivism / skin-keratin / vitiligo / Werner / pentose-P / SA-node /
LTP / island-biogeography / strands-agents / alternation / autopoiesis / lipid-droplet /
lens-crystallin / efferocytosis / BER / polar-body / neutral-theory / MetaGPT /
smolagents / aider / Haldane / Orch-OR / SASP / MTOC / ribosome / epigenetic-clock /
microRNA / bioluminescence / litellm / meiotic-drive / claustrum / spider-silk /
entosis / fear-conditioning / ICL-Fanconi / osmotic-shock / STDP / Batesian /
concordia / open-gpts / smol-course / transgenerational-sperm / split-brain.

   === 7 跨域 fresh (覆盖 R1/R3/R4/R5/R7/R8/R12, 跨域 ASI 基座) ===
   - R1 化学 fresh: bacterial sporulation Bacillus subtilis endospore sigma factors asymmetric
   - R3 免疫 fresh: T cell clonal selection Burnet 1957 V(D)J recombination adaptive
   - R4 神经 fresh: Hodgkin-Huxley 1952 action potential voltage-gated sodium potassium
   - R7 应激 fresh: cancer clonal evolution Nowell 1976 somatic heterogeneity Darwinian
   - R8 生长 fresh: polar auxin transport PIN proteins Cholodny-Went gravitropism plant
   - R12 群落 fresh: biofilm EPS matrix extracellular polymeric substances multicellular bacteria
   - R12 表观 fresh: ADAR adenosine deaminase A-to-I RNA editing recoding epitranscriptomic

   === 3 GitHub deep (主 23:28 真读源码, 主 00:21 ASI-Arch ⭐⭐⭐) ===
   - google-deepmind/funsearch github source code LLM evolutionary mathematical discovery
   - neelnanda-io/TransformerLens github source mechanistic interpretability activation patching
   - anthropics/anthropic-sdk-python github source official Claude API SDK design

   === 2 Gap ===
   - R6 繁殖 Gap: Aspergillus nidulans asexual conidiation BrlA WetA AbaA developmental
   - R11 遗传 Gap: Trypanosoma brucei VSG antigenic variation telomere expression site switching
"""
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, r'.openclaw\workspace\promethean')
from dual_research_fast import dual_research_fast

OUT = Path(r'.openclaw\workspace\promethean\research-v7-round-85.json')

QUERIES = [
    # ===== 7 跨域 fresh =====
    'bacterial sporulation Bacillus subtilis endospore sigma factors asymmetric division stress response substrate ASI R1 chemistry fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83 r84',
    'T cell clonal selection Burnet 1957 adaptive immunity V(D)J recombination substrate ASI R3 immune fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83 r84',
    'Hodgkin Huxley 1952 action potential voltage-gated sodium potassium channel nerve impulse substrate ASI R4 neural fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83 r84',
    'cancer clonal evolution Nowell 1976 somatic heterogeneity Darwinian medicine substrate ASI R7 stress-response fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83 r84',
    'polar auxin transport PIN proteins Cholodny Went gravitropism plant tropism substrate ASI R8 growth fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83 r84',
    'bacterial biofilm EPS matrix extracellular polymeric substances multicellular community substrate ASI R12 community fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83 r84',
    'ADAR adenosine deaminase A-to-I RNA editing recoding epitranscriptomic substrate ASI R12 epitranscriptomic fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83 r84',
    # ===== 3 GitHub deep =====
    'google-deepmind funsearch github source code LLM evolutionary mathematical discovery substrate ASI central AI pluggable fresh r85',
    'neelnanda-io TransformerLens github source mechanistic interpretability activation patching substrate ASI central AI pluggable fresh r85',
    'anthropics anthropic-sdk-python github source official Claude API SDK design substrate ASI central AI pluggable fresh r85',
    # ===== 2 Gap =====
    'Aspergillus nidulans asexual conidiation BrlA WetA AbaA developmental reproduction substrate ASI R6 reproduction Gap fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83 r84',
    'Trypanosoma brucei VSG antigenic variation telomere expression site switching substrate ASI R11 plasticity Gap fresh complement r75 r76 r77 r78 r79 r80 r81 r82 r83 r84',
]


def main():
    started = time.time()
    results = []
    print(f'Round-85 starting: {len(QUERIES)} queries', flush=True)
    for i, q in enumerate(QUERIES):
        t0 = time.time()
        r = dual_research_fast(q, top_k=5)
        dur = time.time() - t0
        bw = len(r['bocha_web'])
        ba = len(r['bocha_ai_answer'])
        any_n = len(r['anysearch'])
        merged = len(r['merged_sources'])
        print(f'[{i+1:02d}/{len(QUERIES)}] {dur:.1f}s | bw={bw} ba={ba} any={any_n} merged={merged} | {q[:80]}', flush=True)
        results.append(r)

    OUT.write_text(json.dumps(results, indent=2, ensure_ascii=False), encoding='utf-8')
    total = time.time() - started
    print(f'\nRound-85 done in {total:.1f}s, saved {len(results)} entries to {OUT}', flush=True)
    print(f'Size: {OUT.stat().st_size} bytes', flush=True)

    bw_total = sum(len(r['bocha_web']) for r in results)
    ba_total = sum(len(r['bocha_ai_answer']) for r in results)
    any_total = sum(len(r['anysearch']) for r in results)
    print(f'Total: bw={bw_total}, ba_chars={ba_total}, anysearch={any_total}', flush=True)
    return total


if __name__ == '__main__':
    main()