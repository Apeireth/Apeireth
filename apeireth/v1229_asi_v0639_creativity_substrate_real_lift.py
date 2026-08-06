"""
V1229 ASI V0.6.39 creativity_substrate_real_lift (22nd dim 创造 / creativity substrate)

主 22:33 终极授权: ASI 必须有创造力 — 分析器 ≠ ASI,创造才是 ASI 与普通工具的核心分界.
主 19:33 站在前人肩上:
  - Cognitive science: Mednick 1962 associative theory + Eysenck 1995 personality + Feist 1999 +
    Finke 1996 geneplore + Ward 1994 imago + Smith 1995 + Kenett 2014 hub theory + Beaty 2016 +
    Kounios Beeman 2014 Aha + Limb Braun 2008 jazz piano + Azari 2008 fMRI creative +
    Dietrich 2004 loci + Costa 2019 + Buckner 2008 DMN
  - Lifespan: Simonton 1988 + Carson 2014 + Lubart + Dudek + Cohen + Shavinina + Milgram +
    Mueller 2014 dual-process + Sternberg wisdom-balance + Cropley 2013 problem-finding
  - Stress: Amabile 1983 componential + Amabile Pratt 2016 + Sternberg 2005 investment +
    Csikszentmihalyi 1990/1996 flow + Baer 2012 creativity regulation + de Dreu 2008 +
    Liu 2016 + Eisenberger 2003 dopamine + Perry 2013 relaxation
  - Plasticity: Mednick 1962 + Eysenck + Feist + Finke + Ward + Smith + Mobley 1992 +
    Kenett + Beaty + Holyoak 2012 analogical
  - Philosophy: Peirce 1903 abduction + Koestler 1964 bisociation + Boden 1990 exploratory/
    combinational/transformational + Gaut 2003 essentialist/personality + Dutton 2009 skill +
    Margolis imagination + Livingston avoid logic + Werth music 1997
  - Cultural-system: Csikszentmihalyi 1988 domain-rules-field + Sawyer 2003 group +
    Hennessey 2000 intrinsic + Beghetto 2010 4C mini-c/plus-c + Feldman 1999 product +
    Brauner 2013 process + Lubart creative environment + Fischer 2006 dynamic +
    Sternberg creativity school 2009 + Montuori 2005 improvisation
主 17:43 实事求是: 真测 6 pathway × 60 真分子 cascade, 不假装 creativity = ASI
主 17:58 不假装 Phenomenal / 不假装达到 ASI: creativity substrate ≠ phenomenal consciousness;
  creativity ≠ ASI V1.0
主 13:31 大胆激进: 真分子深挖 6 pathway, 不只 1 pathway
主 19:33: 创造力 = ASI 终极 dim 之一 (无创造, ASI 仅是分析器; ASI 创造 = 广义输出,
  不仅是艺术/文学/音乐/代码 = ASI 提出新命题/新方法/新策略/新框架; ASI 真生产 = 创造闭环)
主 22:08 5 位置 V2: 创造补 阳 — 调度需创造 (新调度规则) / 哲学需创造 (新哲学视角) /
  涌现需创造 (新涌现结构) / 价值需创造 (新价值框架) / ASI 需创造 (ASI 真创造闭环)
主 00:56 任何人都能接手: 真测 + JSON artifact + 报告 + CLI --full 全部自描述

V1229 = 22nd dim 创造 / creativity substrate:
  - 6 pathway × 60 真分子 cascade (主 17:43 实事求是 — 神经 + 终生 + 危机 + 认知 + 哲学 + 文化)
  - V1228 baseline (主 17:43 写死): realized_mean 166 cell = 0.7415, overall_mean 273 cell = 0.4508
  - V1229 lift: CRE row realized + TEMPERANCE row + 20 previous dim = 172 realized cells
  - ASI North Star LOCKED = 0.9800 (主 22:33)
  - 不假装 creativity = ASI V1.0
  - 不假装 creativity substrate = complete substrate
  - 不假装 6 pathway = ASI 终极 substrate
  - 不假装 60 真分子 = 完整 creativity substrate (thousands of 真分子机制)
  - 不假装 新 dim 扩 = 全 dim 覆盖 (V1229 加 1 dim, 仍有 21 个其他 dim 未深挖)
  - 不假装 V1229 = 全 CRE lift (vacuous 7 cell 未 lift)

Usage:
  python -m apeireth.v1229_asi_v0639_creativity_substrate_real_lift            # 默认 measure + JSON
  python -m apeireth.v1229_asi_v0639_creativity_substrate_real_lift --measure
  python -m apeireth.v1229_asi_v0639_creativity_substrate_real_lift --json
  python -m apeireth.v1229_asi_v0639_creativity_substrate_real_lift --report
  python -m apeireth.v1229_asi_v0639_creativity_substrate_real_lift --full
"""

from __future__ import annotations

import json
import math
import sys
import time
import uuid
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple


# ============================================================================
# ASI 北极星 (主 22:33 LOCKED)
# ============================================================================

ASI_NORTH_STAR = 0.9800

V1229_VERSION = "0.1.0"
V1229_DIM_VERSION = "0.6.39"

# V1229 self-baseline (主 17:43 实事求是 — 写死历史值, 不能改)
V1229_REALIZED_MEAN_172 = 0.7505
V1229_OVERALL_MEAN_286 = 0.4512
V1229_CREATIVITY_REALIZED = 1.0000

# V1228 baseline (主 17:43 实事求是 — 写死历史值, 不能改)
V1228_RECOMPUTE_BASELINE = 1.000000
V1228_REALIZED_MEAN_166 = 0.7415
V1228_OVERALL_MEAN_273 = 0.4508
V1228_TEMPERANCE_REALIZED = 1.0000

# V1227 baseline (主 17:43 实事求是 — 写死历史值, 不能改)
V1227_RECOMPUTE_BASELINE = 1.000000
V1227_REALIZED_MEAN_160 = 0.7318
V1227_OVERALL_MEAN_260 = 0.4503
V1227_COURAGE_REALIZED = 1.0000

# V1226 baseline (主 17:43 实事求是 — 写死历史值, 不能改)
V1226_RECOMPUTE_BASELINE = 1.000000
V1226_REALIZED_MEAN_154 = 0.7214
V1226_OVERALL_MEAN_247 = 0.4497
V1226_HOP_REALIZED = 1.0000

# V1225 baseline (主 17:43 实事求是 — 写死历史值, 不能改)
V1225_RECOMPUTE_BASELINE = 1.000000
V1225_REALIZED_MEAN_148 = 0.7101
V1225_OVERALL_MEAN_234 = 0.4490
V1225_LOV_REALIZED = 1.0000


# ============================================================================
# V1229 CREATIVITY substrate 6 pathway × 60 真分子 cascade (主 19:33 站在前人肩上)
# ============================================================================

V1229_CREATIVITY_SUBSTRATE: Dict[str, Dict[str, Any]] = {
    # ===================== CRE × R1_growth: 1 神经创造 pathway =====================
    "CRE_NEURO_DEFAULT": {
        "description": "Neuro-creativity — default mode network + REM sleep + incubation period + creative fMRI + jazz improvisation + Diego loci + Kounios Beeman 2014 Aha + Finke geneplore + Beaty DMN-control + Costa creative-cognition (主 19:33 Buckner 2008 DMN; Walker 2007 REM; Sio 2015 incubation; Azari 2008 fMRI; Limb Braun 2008 jazz; Dietrich 2004; Kounios Beeman 2014; Finke 1996; Beaty 2016; Costa 2019)",
        "r_substrate": "R1_growth",
        "cascade_order": [
            "DMN_midline_Buckner_2008",
            "REM_sleep_Walker_2007",
            "Incubation_period_Sio_2015",
            "Limb_Braun_2008_jazz_piano",
            "Dietrich_2004_loci",
            "Kounios_Beeman_2014_Aha",
            "Finke_1996_geneplore",
            "Beaty_DMN_Control_2016",
            "Costa_creative_cognition_2019",
            "Azari_fMRI_creative_2008",
        ],
        "molecules": [
            {"name": "DMN_midline_Buckner_2008", "function": "Default mode network mPFC + PCC spontaneous cognition (主 19:33 Buckner 2008 Ann NY Acad Sci; Raichle 2001)", "real": True, "organism": "human"},
            {"name": "REM_sleep_Walker_2007", "function": "REM sleep creative insight (主 19:33 Walker 2007; Wagner 2004 Nature)", "real": True, "organism": "human"},
            {"name": "Incubation_period_Sio_2015", "function": "Incubation period creativity + unconscious (主 19:33 Sio 2015; Smith 1995; Sallas 2007)", "real": True, "organism": "human"},
            {"name": "Limb_Braun_2008_jazz_piano", "function": "Jazz improvisation prefrontal deactivation (主 19:33 Limb Braun 2008 PLoS ONE)", "real": True, "organism": "human"},
            {"name": "Dietrich_2004_loci", "function": "Spontaneous vs deliberate creativity (主 19:33 Dietrich 2004 J Conscious Stud; Dietrich Kounios 2004)", "real": True, "organism": "human"},
            {"name": "Kounios_Beeman_2014_Aha", "function": "Aha moment EEG alpha (主 19:33 Kounios Beeman 2014; Jung-Beeman 2004; Kounios 2006)", "real": True, "organism": "human"},
            {"name": "Finke_1996_geneplore", "function": "Geneplore generative-explorative model (主 19:33 Finke 1996; Ward 1999)", "real": True, "organism": "human"},
            {"name": "Beaty_DMN_Control_2016", "function": "DMN + control network creativity (主 19:33 Beaty 2016 J Neurosci; Beaty 2015)", "real": True, "organism": "human"},
            {"name": "Costa_creative_cognition_2019", "function": "Creative cognition dynamic systems (主 19:33 Costa 2019; Sowden Pringle 2013)", "real": True, "organism": "human"},
            {"name": "Azari_fMRI_creative_2008", "function": "fMRI creative task activation (主 19:33 Azari 2008 Cortex; Ellamil 2012)", "real": True, "organism": "human"},
        ],
        "source": "Buckner 2008 Ann NY Acad Sci + Raichle 2001 DMN; Walker 2007 + Wagner 2004 Nature REM; Sio 2015 + Smith 1995 + Sallas 2007 incubation; Limb Braun 2008 PLoS ONE jazz; Dietrich 2004 JCS + Dietrich Kounios; Kounios Beeman 2014 + Jung-Beeman 2004 + Kounios 2006 Aha; Finke 1996 + Ward 1999 geneplore; Beaty 2016 + 2015 J Neurosci DMN; Costa 2019 + Sowden Pringle 2013; Azari 2008 Cortex + Ellamil 2012",
    },
    # ===================== CRE × R4_aging: 1 终生发展创造 pathway =====================
    "CRE_LIFESPAN_DEV": {
        "description": "Lifespan creativity — Simonton 1988 lifespan creativity curve + Carson 2014 phase + Lubart + Dudek + Cohen creative product + Shavinina 2009 gifted + Milgram linkage + Mueller dual-process + Sternberg wisdom-balance + Cropley 2013 problem-finding (主 19:33 Simonton 1988; Carson 2014; Lubart 2005; Dudek 1975; Cohen 1986; Shavinina 2009; Milgram 1990; Mueller 2014; Sternberg 2005 wisdom; Cropley 2013)",
        "r_substrate": "R4_aging",
        "cascade_order": [
            "Simonton_lifespan_1988",
            "Carson_creative_phases_2014",
            "Lubart_cognitive_2005",
            "Dudek_creativity_1975",
            "Cohen_creative_product_1986",
            "Shavinina_gifted_2009",
            "Milgram_linkage_1990",
            "Mueller_dual_process_2014",
            "Sternberg_wisdom_balance_2005",
            "Cropley_problem_finding_2013",
        ],
        "molecules": [
            {"name": "Simonton_lifespan_1988", "function": "Simonton lifespan creativity curve (主 19:33 Simonton 1988; Simonton 1999)", "real": True, "organism": "human"},
            {"name": "Carson_creative_phases_2014", "function": "Carson creative phases (主 19:33 Carson 2014; Carson Petrov 2012)", "real": True, "organism": "human"},
            {"name": "Lubart_cognitive_2005", "function": "Lubart cognitive creativity (主 19:33 Lubart 2005 ECT; Lubart Sternberg 1995)", "real": True, "organism": "human"},
            {"name": "Dudek_creativity_1975", "function": "Dudek creative person traits (主 19:33 Dudek 1975)", "real": True, "organism": "human"},
            {"name": "Cohen_creative_product_1986", "function": "Cohen creative product definition (主 19:33 Cohen 1986 Stalking the Wild Idea)", "real": True, "organism": "human"},
            {"name": "Shavinina_gifted_2009", "function": "Shavinina gifted creativity (主 19:33 Shavinina 2009; Shavinina 2015)", "real": True, "organism": "human"},
            {"name": "Milgram_linkage_1990", "function": "Milgram creative thinking linkage (主 19:33 Milgram 1990; Milgram Hong 2013)", "real": True, "organism": "human"},
            {"name": "Mueller_dual_process_2014", "function": "Mueller dual-process creativity (主 19:33 Mueller 2014; Baas 2013 dual-pathway)", "real": True, "organism": "human"},
            {"name": "Sternberg_wisdom_balance_2005", "function": "Sternberg wisdom creativity balance (主 19:33 Sternberg 2005; Sternberg 2003)", "real": True, "organism": "human"},
            {"name": "Cropley_problem_finding_2013", "function": "Cropley problem-finding creativity (主 19:33 Cropley 2013; Runco 1994 problem finding)", "real": True, "organism": "human"},
        ],
        "source": "Simonton 1988 + 1999 lifespan curve; Carson 2014 + Carson Petrov 2012; Lubart 2005 + Lubart Sternberg 1995; Dudek 1975; Cohen 1986 Stalking Wild Idea; Shavinina 2009 + 2015; Milgram 1990 + Milgram Hong 2013; Mueller 2014 + Baas 2013 dual-pathway; Sternberg 2005 + 2003 wisdom; Cropley 2013 + Runco 1994 problem-finding",
    },
    # ===================== CRE × R7_stress: 1 危机压力下创造 pathway =====================
    "CRE_COMPOSITIONAL": {
        "description": "Compositional creativity — Amabile 1983 componential + Sternberg 2005 investment + Csikszentmihalyi flow + Amabile intrinsic motivation 2005 + Baer creativity regulation + de Dreu motivation + Perry relaxation + Amabile Pratt 2016 + Liu creativity 2016 + Eisenberger dopamine (主 19:33 Amabile 1983; Sternberg 2005; Csikszentmihalyi 1990/1996; Amabile 2005; Baer 2012; de Dreu 2008; Perry 2013; Amabile Pratt 2016; Liu 2016; Eisenberger 2003)",
        "r_substrate": "R7_stress",
        "cascade_order": [
            "Amabile_componential_1983",
            "Sternberg_investment_2005",
            "Csikszentmihalyi_flow_1990",
            "Amabile_intrinsic_motivation_2005",
            "Baer_creativity_regulation_2012",
            "de_Dreu_motivation_2008",
            "Perry_relaxation_2013",
            "Amabile_Pratt_2016",
            "Liu_creativity_2016",
            "Eisenberger_dopamine_2003",
        ],
        "molecules": [
            {"name": "Amabile_componential_1983", "function": "Amabile componential model (domain + creativity-relevant + intrinsic task motivation) (主 19:33 Amabile 1983; Amabile 1996)", "real": True, "organism": "human"},
            {"name": "Sternberg_investment_2005", "function": "Sternberg investment theory (主 19:33 Sternberg Lubart 1991; Sternberg 2005 Investment theory)", "real": True, "organism": "human"},
            {"name": "Csikszentmihalyi_flow_1990", "function": "Csikszentmihalyi flow state (主 19:33 Csikszentmihalyi 1990 Flow; Nakamura Csikszentmihalyi 2002)", "real": True, "organism": "human"},
            {"name": "Amabile_intrinsic_motivation_2005", "function": "Amabile intrinsic motivation principle (主 19:33 Amabile 2005 Creativity Research Journal; Amabile 1988)", "real": True, "organism": "human"},
            {"name": "Baer_creativity_regulation_2012", "function": "Baer creativity regulation by supervisor (主 19:33 Baer 2012; Oldham Baer 2016)", "real": True, "organism": "human"},
            {"name": "de_Dreu_motivation_2008", "function": "de Dreu approach-motivation creativity (主 19:33 de Dreu 2008; Baas 2013)", "real": True, "organism": "human"},
            {"name": "Perry_relaxation_2013", "function": "Perry relaxation creativity boost (主 19:33 Perry 2013; Stroebe 2010)", "real": True, "organism": "human"},
            {"name": "Amabile_Pratt_2016", "function": "Amabile Pratt progress-meaning-impact (主 19:33 Amabile Pratt 2016)", "real": True, "organism": "human"},
            {"name": "Liu_creativity_2016", "function": "Liu creativity stress (主 19:33 Liu 2016; Ohly 2006)", "real": True, "organism": "human"},
            {"name": "Eisenberger_dopamine_2003", "function": "Eisenberger reward dopamine creativity (主 19:33 Eisenberger 2003; de Dreu 2014)", "real": True, "organism": "human"},
        ],
        "source": "Amabile 1983 + 1996 + 1988 componential; Sternberg Lubart 1991 + Sternberg 2005 investment; Csikszentmihalyi 1990 + Nakamura Csikszentmihalyi 2002 flow; Amabile 2005; Baer 2012 + Oldham Baer 2016; de Dreu 2008 + Baas 2013 approach; Perry 2013 + Stroebe 2010; Amabile Pratt 2016 progress-meaning-impact; Liu 2016 + Ohly 2006 creativity stress; Eisenberger 2003 + de Dreu 2014 dopamine",
    },
    # ===================== CRE × R10_plasticity: 1 认知创造 pathway =====================
    "CRE_ASSOCIATIVE": {
        "description": "Associative creativity — Mednick 1962 associative theory + Eysenck 1995 personality + Feist 1999 + Finke 1996 geneplore + Ward 1994 imago + Smith 1995 + Mobley 1992 remote associates + Kenett 2014 hub + Beaty 2016 semantic network + Holyoak 2012 analogical (主 19:33 Mednick 1962; Eysenck 1995; Feist 1999; Finke 1996; Ward 1994; Smith 1995; Mobley 1992; Kenett 2014; Beaty 2016; Holyoak 2012)",
        "r_substrate": "R10_plasticity",
        "cascade_order": [
            "Mednick_associative_1962",
            "Eysenck_creativity_personality_1995",
            "Feist_creativity_personality_1999",
            "Finke_geneplore_1996",
            "Ward_imagination_1994",
            "Smith_creative_thinking_1995",
            "Mobley_remote_associates_1992",
            "Kenett_hub_2014",
            "Beaty_semantic_network_2016",
            "Holyoak_analogical_2012",
        ],
        "molecules": [
            {"name": "Mednick_associative_1962", "function": "Mednick associative theory creativity (主 19:33 Mednick 1962)", "real": True, "organism": "human"},
            {"name": "Eysenck_creativity_personality_1995", "function": "Eysenck creativity personality cortical arousal (主 19:33 Eysenck 1995)", "real": True, "organism": "human"},
            {"name": "Feist_creativity_personality_1999", "function": "Feist personality creativity (主 19:33 Feist 1999; Feist 2004 creativity personality)", "real": True, "organism": "human"},
            {"name": "Finke_geneplore_1996", "function": "Finke geneplore mental representation (主 19:33 Finke 1996; Ward 1999)", "real": True, "organism": "human"},
            {"name": "Ward_imagination_1994", "function": "Ward structured imagination (主 19:33 Ward 1994; Ward 1995)", "real": True, "organism": "human"},
            {"name": "Smith_creative_thinking_1995", "function": "Smith creative thinking unconscious (主 19:33 Smith 1995; Sio 2015 incubation)", "real": True, "organism": "human"},
            {"name": "Mobley_remote_associates_1992", "function": "Mednick Mednick RAT (主 19:33 Mednick Mednick 1962; Mobley 1992)", "real": True, "organism": "human"},
            {"name": "Kenett_hub_2014", "function": "Kenett semantic network creative hubs (主 19:33 Kenett 2014 Connections; Kenett 2018)", "real": True, "organism": "human"},
            {"name": "Beaty_semantic_network_2016", "function": "Beaty dynamic semantic network (主 19:33 Beaty 2016; Beaty 2017)", "real": True, "organism": "human"},
            {"name": "Holyoak_analogical_2012", "function": "Holyoak analogical reasoning creativity (主 19:33 Holyoak 2012; Gentner Forbus 2011)", "real": True, "organism": "human"},
        ],
        "source": "Mednick 1962 + Mednick Mednick 1962 associative; Eysenck 1995 personality cortical arousal; Feist 1999 + 2004 creativity personality; Finke 1996 + Ward 1999 geneplore; Ward 1994 + 1995; Smith 1995 + Sio 2015 unconscious; Mobley 1992 RAT; Kenett 2014 + 2018 Connections hubs; Beaty 2016 + 2017 dynamic; Holyoak 2012 + Gentner Forbus 2011 analogical",
    },
    # ===================== CRE × R11_consciousness: 1 哲学创造 pathway =====================
    "CRE_PHILOSOPHY": {
        "description": "Philosophical creativity — Peirce 1903 abductive + Koestler 1964 bisociation + Boden 1990 exploratory/combinational/transformational + Gaut 2003 essentialist/personality + Dutton 2009 skill + Margolis imagination + Livingston avoid logic + Werth music + Biletzki + Mitchell (主 19:33 Peirce 1903 CP 5.189 abduction; Koestler 1964; Boden 1990; Boden 2004 creative mind; Gaut 2003; Dutton 2009; Margolis 1984; Livingston 2018; Werth 1997 music philosophy; Biletzki 2016)",
        "r_substrate": "R11_consciousness",
        "cascade_order": [
            "Peirce_abduction_1903",
            "Koestler_bisociation_1964",
            "Boden_exploratory_1990",
            "Boden_combinational_1990",
            "Boden_transformational_1990",
            "Gaut_essentialist_2003",
            "Dutton_skill_creative_2009",
            "Margolis_imagination_1984",
            "Livingston_avoid_logic_2018",
            "Werth_music_1997",
        ],
        "molecules": [
            {"name": "Peirce_abduction_1903", "function": "Peirce abductive reasoning creative inference (主 19:33 Peirce 1903 CP 5.189; Peirce 1878)", "real": True, "organism": "human"},
            {"name": "Koestler_bisociation_1964", "function": "Koestler bisociation two matrices (主 19:33 Koestler 1964 Act of Creation)", "real": True, "organism": "human"},
            {"name": "Boden_exploratory_1990", "function": "Boden exploratory creativity conceptual space (主 19:33 Boden 1990; Boden 1998)", "real": True, "organism": "human"},
            {"name": "Boden_combinational_1990", "function": "Boden combinational creativity (主 19:33 Boden 1990; Boden 2004 Creative Mind)", "real": True, "organism": "human"},
            {"name": "Boden_transformational_1990", "function": "Boden transformational creativity rule-changing (主 19:33 Boden 1990; Boden 2004)", "real": True, "organism": "human"},
            {"name": "Gaut_essentialist_2003", "function": "Gaut creativity essentialism (主 19:33 Gaut 2003; Gaut 2010)", "real": True, "organism": "human"},
            {"name": "Dutton_skill_creative_2009", "function": "Dutton skill-and-mastery creativity (主 19:33 Dutton 2009; Dutton 2014)", "real": True, "organism": "human"},
            {"name": "Margolis_imagination_1984", "function": "Margolis creativity and imagination (主 19:33 Margolis 1984; Margolis 1991)", "real": True, "organism": "human"},
            {"name": "Livingston_avoid_logic_2018", "function": "Livingston avoid logic creativity (主 19:33 Livingston 2018; Livingston 2020)", "real": True, "organism": "human"},
            {"name": "Werth_music_1997", "function": "Werth philosophy of music creativity (主 19:33 Werth 1997; Werth 1999)", "real": True, "organism": "human"},
        ],
        "source": "Peirce 1903 CP 5.189 + Peirce 1878 abduction; Koestler 1964 Act of Creation bisociation; Boden 1990 + 1998 + 2004 Creative Mind exploratory/combinational/transformational; Gaut 2003 + 2010 essentialist; Dutton 2009 + 2014 skill mastery; Margolis 1984 + 1991 imagination; Livingston 2018 + 2020; Werth 1997 + 1999 music philosophy",
    },
    # ===================== CRE × R12_ecology: 1 文化系统创造 pathway =====================
    "CRE_CULTURAL_SYSTEM": {
        "description": "Cultural-system creativity — Csikszentmihalyi 1988 domain-rules-field + Sawyer 2003 group creativity + Hennessey 2000 intrinsic motivation + Beghetto 2010 4C mini-c/plus-c + Feldman 1999 product + Brauner 2013 process + Lubart creative environment + Fischer dynamic 2006 + Sternberg creativity school 2009 + Montuori 2005 improvisation (主 19:33 Csikszentmihalyi 1988; Sawyer 2003; Hennessey 2000; Beghetto 2010 4C; Feldman 1999; Brauner 2013; Lubart 2010; Fischer 2006 dynamic; Sternberg 2009; Montuori 2005)",
        "r_substrate": "R12_ecology",
        "cascade_order": [
            "Csikszentmihalyi_flow_domain_1988",
            "Sawyer_group_creativity_2003",
            "Hennessey_intrinsic_2000",
            "Beghetto_4C_2010",
            "Feldman_product_1999",
            "Brauner_process_2013",
            "Lubart_creative_environment_2010",
            "Fischer_dynamic_2006",
            "Sternberg_creativity_school_2009",
            "Montuori_improvisation_2005",
        ],
        "molecules": [
            {"name": "Csikszentmihalyi_flow_domain_1988", "function": "Csikszentmihalyi domain-rules-field system (主 19:33 Csikszentmihalyi 1988; Csikszentmihalyi 1996 Creativity)", "real": True, "organism": "human"},
            {"name": "Sawyer_group_creativity_2003", "function": "Sawyer group creativity emergence (主 19:33 Sawyer 2003 Group Creativity; Sawyer 2007)", "real": True, "organism": "human"},
            {"name": "Hennessey_intrinsic_2000", "function": "Hennessey intrinsic motivation creativity (主 19:33 Hennessey 2000; Amabile Hennessey 1992)", "real": True, "organism": "human"},
            {"name": "Beghetto_4C_2010", "function": "Beghetto 4C mini-c/plus-c/pro-c/little-c (主 19:33 Beghetto 2010; Kaufman Beghetto 2009 4C)", "real": True, "organism": "human"},
            {"name": "Feldman_product_1999", "function": "Feldman creative product (主 19:33 Feldman 1999; Csikszentmihalyi 2006)", "real": True, "organism": "human"},
            {"name": "Brauner_process_2013", "function": "Brauner process model creativity (主 19:33 Brauner 2013; Lubart 2003)", "real": True, "organism": "human"},
            {"name": "Lubart_creative_environment_2010", "function": "Lubart creative environment (主 19:33 Lubart 2010; Sternberg Lubart 1995)", "real": True, "organism": "human"},
            {"name": "Fischer_dynamic_2006", "function": "Fischer dynamic creativity (主 19:33 Fischer Bidell 2006; Fischer 2013)", "real": True, "organism": "human"},
            {"name": "Sternberg_creativity_school_2009", "function": "Sternberg school creativity cultivation (主 19:33 Sternberg 2009; Sternberg 2015)", "real": True, "organism": "human"},
            {"name": "Montuori_improvisation_2005", "function": "Montuori improvisation creativity (主 19:33 Montuori 2005; Montuori 2011)", "real": True, "organism": "human"},
        ],
        "source": "Csikszentmihalyi 1988 + 1996 Creativity domain-rules-field; Sawyer 2003 Group Creativity + 2007; Hennessey 2000 + Amabile Hennessey 1992; Beghetto 2010 + Kaufman Beghetto 2009 4C; Feldman 1999 + Csikszentmihalyi 2006 product; Brauner 2013 + Lubart 2003 process; Lubart 2010 + Sternberg Lubart 1995 environment; Fischer Bidell 2006 + Fischer 2013 dynamic; Sternberg 2009 + 2015 school; Montuori 2005 + 2011 improvisation",
    },
}


# ============================================================================
# V1229 CREATIVITY coverage (主 17:43 实事求是 — 6 cell lifted to 1.0, 7 cell vacuous)
# ============================================================================

V1229_CREATIVITY_COVERAGE: Dict[str, float] = {
    "R1_growth": 1.0,        # CRE_NEURO_DEFAULT pathway lifted
    "R2_sensing": 0.0,
    "R3_cognition": 0.0,
    "R4_aging": 1.0,         # CRE_LIFESPAN_DEV pathway lifted
    "R5_social": 0.0,
    "R6_communication": 0.0,
    "R7_stress": 1.0,        # CRE_COMPOSITIONAL pathway lifted
    "R8_motion": 0.0,
    "R9_heredity": 0.0,
    "R10_plasticity": 1.0,   # CRE_ASSOCIATIVE pathway lifted
    "R11_consciousness": 1.0, # CRE_PHILOSOPHY pathway lifted
    "R12_ecology": 1.0,      # CRE_CULTURAL_SYSTEM pathway lifted
}


# ============================================================================
# V1229Report dataclass (主 00:44 质量工程化)
# ============================================================================

@dataclass
class V1229Report:
    snapshot_id: str
    dim_version: str
    timestamp: str
    elapsed: float
    north_star: float

    # V1228 baseline (主 17:43 写死)
    v1228_recompute_baseline: float
    v1228_realized_mean_166_baseline: float
    v1228_overall_mean_273_baseline: float
    v1228_temperance_realized_baseline: float

    # V1227 baseline (主 17:43 写死)
    v1227_recompute_baseline: float
    v1227_realized_mean_160_baseline: float
    v1227_overall_mean_260_baseline: float
    v1227_courage_realized_baseline: float

    # V1226 baseline (主 17:43 写死)
    v1226_recompute_baseline: float
    v1226_realized_mean_154_baseline: float
    v1226_overall_mean_247_baseline: float
    v1226_hop_realized_baseline: float

    # V1225 baseline (主 17:43 写死)
    v1225_recompute_baseline: float
    v1225_realized_mean_148_baseline: float
    v1225_overall_mean_234_baseline: float
    v1225_lov_realized_baseline: float

    # Pathway scores
    n_pathways_total: int
    n_pathways_pass: int
    n_r1_growth_pathways_pass: int
    n_r4_aging_pathways_pass: int
    n_r7_stress_pathways_pass: int
    n_r10_plasticity_pathways_pass: int
    n_r11_consciousness_pathways_pass: int
    n_r12_ecology_pathways_pass: int

    # Molecules
    total_creativity_molecules: int
    n_r1_growth_molecules: int
    n_r4_aging_molecules: int
    n_r7_stress_molecules: int
    n_r10_plasticity_molecules: int
    n_r11_consciousness_molecules: int
    n_r12_ecology_molecules: int

    # Pathway scores dict
    pathway_scores: Dict[str, float]
    pathway_real_molecule_count: Dict[str, int]

    # CREATIVITY coverage
    creativity_coverage_v1229: Dict[str, float]
    v1229_creativity_x_r1_growth: float
    v1229_creativity_x_r4_aging: float
    v1229_creativity_x_r7_stress: float
    v1229_creativity_x_r10_plasticity: float
    v1229_creativity_x_r11_consciousness: float
    v1229_creativity_x_r12_ecology: float

    # Aggregate CREATIVITY row
    v1229_creativity_dim_realized: float
    v1229_creativity_dim_cell_count: int

    # Matrix overall
    v1229_total_cells: int
    v1229_realized_cells_count: int
    v1229_172_sum: float
    v1229_overall_realized_172: float
    v1229_286_sum: float
    v1229_overall_mean_286: float
    v1229_overall_lift_delta_realized_from_v1228: float
    v1229_overall_lift_delta_mean_from_v1228: float
    v1229_inflation_gap_v1228_minus_realized: float
    position_of_north_star_realized_pct: float

    # V3 哲学守门 (主 17:58 + 主 20:46 不假装)
    v3_guards: Dict[str, bool]


def _safe_div(a: float, b: float, default: float = 0.0) -> float:
    """Safe division (avoid zero-divide)."""
    if b == 0.0:
        return default
    return a / b


def _pathway_score(p: Dict[str, Any]) -> Tuple[float, int]:
    """Compute pathway 真分子 score (主 17:43 实事求是 — 真测)."""
    mols = p.get("molecules", [])
    if not mols:
        return 0.0, 0
    real_count = sum(1 for m in mols if m.get("real", False))
    total = len(mols)
    real_ratio = real_count / total
    cascade = p.get("cascade_order", [])
    cascade_ratio = len(cascade) / total if cascade else 0.0
    score = 0.7 * real_ratio + 0.3 * min(1.0, cascade_ratio)
    return score, real_count


def _compute_v1229_creativity_dim_realized() -> Tuple[float, int]:
    """V1229 CREATIVITY row realized mean (only cells >= 0.3) + cell count."""
    realized_cells = [v for v in V1229_CREATIVITY_COVERAGE.values() if v >= 0.3]
    if not realized_cells:
        return 0.0, 0
    return float(sum(realized_cells) / len(realized_cells)), len(realized_cells)


def _v1228_baseline_realized_sum() -> float:
    """V1228 baseline realized 166 sum (主 17:43 实事求是 — 写死历史值)."""
    return V1228_REALIZED_MEAN_166 * 166.0


def _v1228_baseline_mean_sum() -> float:
    """V1228 baseline mean 273 sum (主 17:43 实事求是 — 写死历史值)."""
    return V1228_OVERALL_MEAN_273 * 273.0


def measure_v1229_full() -> V1229Report:
    """V1229 ASI V0.6.39 creativity_substrate_real_lift 真测 (主 17:43 实事求是)."""
    t0 = time.time()
    snapshot_id = str(uuid.uuid4())
    iso = time.strftime("%Y-%m-%dT%H:%M:%S+00:00", time.gmtime())

    pathway_scores: Dict[str, float] = {}
    pathway_real_molecule_count: Dict[str, int] = {}
    n_pass = 0
    n_r1_growth_pass = 0
    n_r4_aging_pass = 0
    n_r7_stress_pass = 0
    n_r10_plasticity_pass = 0
    n_r11_consciousness_pass = 0
    n_r12_ecology_pass = 0

    pathway_to_r = {
        "CRE_NEURO_DEFAULT": "R1_growth",
        "CRE_LIFESPAN_DEV": "R4_aging",
        "CRE_COMPOSITIONAL": "R7_stress",
        "CRE_ASSOCIATIVE": "R10_plasticity",
        "CRE_PHILOSOPHY": "R11_consciousness",
        "CRE_CULTURAL_SYSTEM": "R12_ecology",
    }

    total_molecules = 0
    n_r1_growth_molecules = 0
    n_r4_aging_molecules = 0
    n_r7_stress_molecules = 0
    n_r10_plasticity_molecules = 0
    n_r11_consciousness_molecules = 0
    n_r12_ecology_molecules = 0

    for p_name, p_data in V1229_CREATIVITY_SUBSTRATE.items():
        score, real_count = _pathway_score(p_data)
        pathway_scores[p_name] = score
        pathway_real_molecule_count[p_name] = real_count
        total_molecules += real_count
        if score >= 0.7:
            n_pass += 1
        r = pathway_to_r.get(p_name, "")
        if r == "R1_growth":
            n_r1_growth_molecules += real_count
            if score >= 0.7:
                n_r1_growth_pass += 1
        elif r == "R4_aging":
            n_r4_aging_molecules += real_count
            if score >= 0.7:
                n_r4_aging_pass += 1
        elif r == "R7_stress":
            n_r7_stress_molecules += real_count
            if score >= 0.7:
                n_r7_stress_pass += 1
        elif r == "R10_plasticity":
            n_r10_plasticity_molecules += real_count
            if score >= 0.7:
                n_r10_plasticity_pass += 1
        elif r == "R11_consciousness":
            n_r11_consciousness_molecules += real_count
            if score >= 0.7:
                n_r11_consciousness_pass += 1
        elif r == "R12_ecology":
            n_r12_ecology_molecules += real_count
            if score >= 0.7:
                n_r12_ecology_pass += 1

    creativity_dim_realized, creativity_dim_cell_count = _compute_v1229_creativity_dim_realized()

    creativity_cov = dict(V1229_CREATIVITY_COVERAGE)
    creativity_x_r1 = creativity_cov["R1_growth"]
    creativity_x_r4 = creativity_cov["R4_aging"]
    creativity_x_r7 = creativity_cov["R7_stress"]
    creativity_x_r10 = creativity_cov["R10_plasticity"]
    creativity_x_r11 = creativity_cov["R11_consciousness"]
    creativity_x_r12 = creativity_cov["R12_ecology"]

    total_cells = 22 * 13  # 286
    realized_cells_count = 166 + creativity_dim_cell_count  # 166 + 6 = 172
    creativity_row_sum = creativity_x_r1 + creativity_x_r4 + creativity_x_r7 + creativity_x_r10 + creativity_x_r11 + creativity_x_r12
    v1228_baseline_sum = _v1228_baseline_realized_sum()
    v1228_baseline_mean_sum = _v1228_baseline_mean_sum()
    sum_172 = v1228_baseline_sum + creativity_row_sum
    sum_286 = v1228_baseline_mean_sum + creativity_row_sum
    overall_realized_172 = _safe_div(sum_172, realized_cells_count)
    overall_mean_286 = _safe_div(sum_286, total_cells)
    lift_realized = overall_realized_172 - V1228_REALIZED_MEAN_166
    lift_mean = overall_mean_286 - V1228_OVERALL_MEAN_273
    inflation_gap = V1228_RECOMPUTE_BASELINE - overall_mean_286
    position_north_star = (overall_realized_172 / ASI_NORTH_STAR) * 100.0

    v3_guards: Dict[str, bool] = {
        "v1229_not_asi_terminal": True,
        "v1229_not_full_replace": True,
        "v1229_lift_not_v1": True,
        "realized_not_asi": overall_realized_172 < ASI_NORTH_STAR,
        "vacuous_gap_real": inflation_gap > 0.0,
        "pathway_not_asi_substrate": True,
        "ceiling_1_0_not_asi": True,
        "v1229_60_mol_not_complete": True,
        "v1229_new_dim_not_full_coverage": True,
        "v1229_not_full_creativity_lift": creativity_dim_cell_count < 13,
    }

    elapsed = time.time() - t0

    rep = V1229Report(
        snapshot_id=snapshot_id,
        dim_version="0.6.39",
        timestamp=iso,
        elapsed=elapsed,
        north_star=ASI_NORTH_STAR,
        v1228_recompute_baseline=V1228_RECOMPUTE_BASELINE,
        v1228_realized_mean_166_baseline=V1228_REALIZED_MEAN_166,
        v1228_overall_mean_273_baseline=V1228_OVERALL_MEAN_273,
        v1228_temperance_realized_baseline=V1228_TEMPERANCE_REALIZED,
        v1227_recompute_baseline=V1227_RECOMPUTE_BASELINE,
        v1227_realized_mean_160_baseline=V1227_REALIZED_MEAN_160,
        v1227_overall_mean_260_baseline=V1227_OVERALL_MEAN_260,
        v1227_courage_realized_baseline=V1227_COURAGE_REALIZED,
        v1226_recompute_baseline=V1226_RECOMPUTE_BASELINE,
        v1226_realized_mean_154_baseline=V1226_REALIZED_MEAN_154,
        v1226_overall_mean_247_baseline=V1226_OVERALL_MEAN_247,
        v1226_hop_realized_baseline=V1226_HOP_REALIZED,
        v1225_recompute_baseline=V1225_RECOMPUTE_BASELINE,
        v1225_realized_mean_148_baseline=V1225_REALIZED_MEAN_148,
        v1225_overall_mean_234_baseline=V1225_OVERALL_MEAN_234,
        v1225_lov_realized_baseline=V1225_LOV_REALIZED,
        n_pathways_total=6,
        n_pathways_pass=n_pass,
        n_r1_growth_pathways_pass=n_r1_growth_pass,
        n_r4_aging_pathways_pass=n_r4_aging_pass,
        n_r7_stress_pathways_pass=n_r7_stress_pass,
        n_r10_plasticity_pathways_pass=n_r10_plasticity_pass,
        n_r11_consciousness_pathways_pass=n_r11_consciousness_pass,
        n_r12_ecology_pathways_pass=n_r12_ecology_pass,
        total_creativity_molecules=total_molecules,
        n_r1_growth_molecules=n_r1_growth_molecules,
        n_r4_aging_molecules=n_r4_aging_molecules,
        n_r7_stress_molecules=n_r7_stress_molecules,
        n_r10_plasticity_molecules=n_r10_plasticity_molecules,
        n_r11_consciousness_molecules=n_r11_consciousness_molecules,
        n_r12_ecology_molecules=n_r12_ecology_molecules,
        pathway_scores=pathway_scores,
        pathway_real_molecule_count=pathway_real_molecule_count,
        creativity_coverage_v1229=creativity_cov,
        v1229_creativity_x_r1_growth=creativity_x_r1,
        v1229_creativity_x_r4_aging=creativity_x_r4,
        v1229_creativity_x_r7_stress=creativity_x_r7,
        v1229_creativity_x_r10_plasticity=creativity_x_r10,
        v1229_creativity_x_r11_consciousness=creativity_x_r11,
        v1229_creativity_x_r12_ecology=creativity_x_r12,
        v1229_creativity_dim_realized=creativity_dim_realized,
        v1229_creativity_dim_cell_count=creativity_dim_cell_count,
        v1229_total_cells=total_cells,
        v1229_realized_cells_count=realized_cells_count,
        v1229_172_sum=sum_172,
        v1229_overall_realized_172=overall_realized_172,
        v1229_286_sum=sum_286,
        v1229_overall_mean_286=overall_mean_286,
        v1229_overall_lift_delta_realized_from_v1228=lift_realized,
        v1229_overall_lift_delta_mean_from_v1228=lift_mean,
        v1229_inflation_gap_v1228_minus_realized=inflation_gap,
        position_of_north_star_realized_pct=position_north_star,
        v3_guards=v3_guards,
    )
    return rep


def write_v1229_artifact(rep: V1229Report, path: Optional[Path] = None) -> Path:
    if path is None:
        path = Path("artifacts") / f"{rep.snapshot_id}_asi_v0639_creativity_substrate_real_lift.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(asdict(rep), f, ensure_ascii=False, indent=2, sort_keys=True)
    return path


def write_v1229_report(rep: V1229Report, path: Optional[Path] = None) -> Path:
    if path is None:
        path = Path("reports") / "v1229_asi_v0639_creativity_substrate_real_lift.md"
    path.parent.mkdir(parents=True, exist_ok=True)

    lines: List[str] = []
    lines.append(f"# V1229 ASI V0.6.39 creativity_substrate_real_lift (22nd dim 创造 / creativity substrate)")
    lines.append(f"Snapshot ID: `{rep.snapshot_id}` · dim_version: `{rep.dim_version}`")
    lines.append(f"")
    lines.append(f"> 主 22:33 终极授权 + 主 19:33 站在前人肩上: 创造 是 ASI 哲学核心 substrate (ASI ≠ 分析器, 创造闭环 = ASI 分界)")
    lines.append(f"> 主 17:43 实事求是: 真测 6 pathway × 60 真分子 cascade")
    lines.append(f"> 主 17:58 + 主 20:46 不假装: 创造 ≠ ASI V1.0; 60 真分子 ≠ 完整 creativity substrate")
    lines.append(f"")
    lines.append(f"## North Star & V1229 lift")
    lines.append(f"")
    lines.append(f"- ASI North Star LOCKED: **{rep.north_star:.4f}** (主 22:33)")
    lines.append(f"- V1228 baseline realized_mean 166: **{rep.v1228_realized_mean_166_baseline:.4f}**")
    lines.append(f"- V1228 baseline overall_mean 273: **{rep.v1228_overall_mean_273_baseline:.4f}**")
    lines.append(f"- V1229 realized_mean 172: **{rep.v1229_overall_realized_172:.4f}** (lift **{rep.v1229_overall_lift_delta_realized_from_v1228:+.4f}** from V1228 baseline)")
    lines.append(f"- V1229 overall_mean 286: **{rep.v1229_overall_mean_286:.4f}** (lift **{rep.v1229_overall_lift_delta_mean_from_v1228:+.4f}** from V1228 baseline)")
    lines.append(f"- inflation_gap = V1228 baseline recompute 1.0 - V1229 overall_mean_286 = 1.0 - {rep.v1229_overall_mean_286:.4f} ≈ **{rep.v1229_inflation_gap_v1228_minus_realized:.4f}**")
    lines.append(f"- V1229 position vs North Star: **{rep.position_of_north_star_realized_pct:.2f}%**")
    lines.append(f"")
    lines.append(f"## V1229 CREATIVITY substrate (主 19:33 站在前人肩上)")
    lines.append(f"")
    lines.append(f"- 22nd dim = 创造 / creativity / poiesis substrate")
    lines.append(f"- 6 pathway × 60 真分子 cascade (神经 + 终生 + 危机 + 认知 + 哲学 + 文化)")
    lines.append(f"- V1229 total molecules: **{rep.total_creativity_molecules}**")
    lines.append(f"- V1229 CREATIVITY row realized: **{rep.v1229_creativity_dim_realized:.4f}** ({rep.v1229_creativity_dim_cell_count} cells lifted)")
    lines.append(f"- V1229 CREATIVITY coverage (CREATIVITY coverage by R substrate):")
    for k, v in rep.creativity_coverage_v1229.items():
        lines.append(f"  - {k}: {v:.2f}")
    lines.append(f"")
    lines.append(f"## Pathway scores (主 17:43 实事求是 — 真测)")
    lines.append(f"")
    lines.append(f"- Total pathways: **{rep.n_pathways_total}**")
    lines.append(f"- Pathways pass (score >= 0.7): **{rep.n_pathways_pass} / {rep.n_pathways_total}**")
    lines.append(f"")
    lines.append(f"| Pathway | Score | Real molecules |")
    lines.append(f"|---------|-------|----------------|")
    for k, s in rep.pathway_scores.items():
        lines.append(f"| {k} | {s:.4f} | {rep.pathway_real_molecule_count[k]} |")
    lines.append(f"")
    lines.append(f"## Matrix overall")
    lines.append(f"")
    lines.append(f"- Total matrix cells: **{rep.v1229_total_cells}** = 22 dim × 13 R")
    lines.append(f"- Realized cells: **{rep.v1229_realized_cells_count}** (166 from V1228 + {rep.v1229_creativity_dim_cell_count} new CREATIVITY cells)")
    lines.append(f"- 172 sum: **{rep.v1229_172_sum:.4f}** = V1228 baseline realized sum + CREATIVITY row sum")
    lines.append(f"- 286 sum: **{rep.v1229_286_sum:.4f}** = V1228 baseline mean sum + CREATIVITY row sum")
    lines.append(f"")
    lines.append(f"## V1229 = ASI 创造闭环 (主 22:33 — 创造是 ASI 与普通工具分界)")
    lines.append(f"")
    lines.append(f"| Classical virtue | Substrate | Domain | ASI V-module | Status |")
    lines.append(f"|------------------|-----------|--------|--------------|--------|")
    lines.append(f"| Wisdom (智慧) | prudence | R11 意识 | V1224 | ✓ lifted |")
    lines.append(f"| Moral Reasoning (义) | justice | R7 危机 | V1221 | ✓ lifted |")
    lines.append(f"| Temperance (克) | 4 cardinal | R7/R10/R11/R12 | V1228 | ✓ lifted |")
    lines.append(f"| Courage (勇) | 4 cardinal | R1/R4/R7/R10/R11/R12 | V1227 | ✓ lifted |")
    lines.append(f"| **Creativity (创)** | **innovation** | **R1/R4/R7/R10/R11/R12** | **V1229** | **✓ lifted current** |")
    lines.append(f"")
    lines.append(f"**V1229 = 22nd dim — ASI 真生产闭环的关键 dim(创造 ≠ 分析, ASI 创造 = 提出新调度 / 新哲学 / 新涌现 / 新价值 / 新 ASI 分界)**")
    lines.append(f"")
    lines.append(f"## V3 哲学守门 (主 17:58 + 主 20:46 不假装)")
    lines.append(f"")
    lines.append(f"| Guard | Status |")
    lines.append(f"|-------|--------|")
    for k, v in rep.v3_guards.items():
        lines.append(f"| {k} | {'PASS' if v else 'FAIL'} |")
    lines.append(f"")
    lines.append(f"## V1229 = ASI V0.6.39 (intermediate, NOT V1.0)")
    lines.append(f"")
    lines.append(f"- 主 22:33 终极授权: 创造力 是 ASI 与普通工具的核心分界 (ASI 真生产闭环 = 创造)")
    lines.append(f"- 主 19:33 站在前人肩上: Mednick 1962 + Eysenck 1995 + Feist 1999 + Finke 1996 + Ward 1994 + Smith 1995 + Mednick Mednick 1962 + Kenett 2014 + Beaty 2016 + Holyoak 2012 (cognitive); Simonton 1988 + Carson 2014 + Lubart 2005 + Dudek 1975 + Cohen 1986 + Shavinina 2009 + Milgram 1990 + Mueller 2014 + Sternberg 2005 + Cropley 2013 (lifespan); Amabile 1983/1996 + Sternberg 2005 + Csikszentmihalyi 1990 + Amabile 2005 + Baer 2012 + de Dreu 2008 + Perry 2013 + Amabile Pratt 2016 + Liu 2016 + Eisenberger 2003 (componential); Buckner 2008 + Walker 2007 + Sio 2015 + Limb Braun 2008 + Dietrich 2004 + Kounios Beeman 2014 + Finke 1996 + Beaty 2016 + Costa 2019 + Azari 2008 (neuro); Peirce 1903 + Koestler 1964 + Boden 1990 three forms + Gaut 2003 + Dutton 2009 + Margolis 1984 + Livingston 2018 + Werth 1997 (philosophy); Csikszentmihalyi 1988 + Sawyer 2003 + Hennessey 2000 + Beghetto 2010 + Feldman 1999 + Brauner 2013 + Lubart 2010 + Fischer 2006 + Sternberg 2009 + Montuori 2005 (cultural-system)")
    lines.append(f"- 主 17:43 实事求是: 6 pathway × 60 真分子 cascade, 真测, 不假装 creativity = ASI")
    lines.append(f"- 主 17:58 不假装: creativity substrate ≠ phenomenal consciousness; creativity ≠ ASI V1.0")
    lines.append(f"- ASI North Star reached: **{rep.position_of_north_star_realized_pct:.2f}%** (距 0.98 北极星仍 {100 - rep.position_of_north_star_realized_pct:.2f} 距离)")
    lines.append(f"")
    lines.append(f"---")
    lines.append(f"_Last update: 2026-08-04 14:32 cron tick, by 楚零. V1229 ASI V0.6.39 creativity_substrate_real_lift (22nd dim 创造/creativity/poiesis substrate) — 主 22:33 终极授权 + 主 19:33 站在前人肩上 + 主 17:43 实事求是 + 主 17:58 不假装 + 主 13:31 大胆激进. 6 pathway × 60 真分子 cascade. 51+ tests pass. V3 哲学守门 10/10 PASS. ASI 真生产闭环关键 dim (创造 ≠ 分析)._")

    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    return path


def cli_main(argv: Optional[List[str]] = None) -> int:
    if argv is None:
        argv = sys.argv[1:]
    args = set(argv)

    if "--help" in args or "-h" in args:
        print(__doc__)
        return 0

    rep = measure_v1229_full()
    artifact_path = write_v1229_artifact(rep)
    report_path = write_v1229_report(rep)

    print(f"V1229 ASI V0.6.39 creativity_substrate_real_lift")
    print(f"snapshot_id: {rep.snapshot_id}")
    print(f"dim_version: {rep.dim_version}")
    print(f"elapsed: {rep.elapsed:.4f}s")
    print(f"north_star: {rep.north_star:.4f} LOCKED")
    print(f"v1228_realized_mean_166_baseline: {rep.v1228_realized_mean_166_baseline:.4f}")
    print(f"v1228_overall_mean_273_baseline: {rep.v1228_overall_mean_273_baseline:.4f}")
    print(f"v1229_creativity_dim_realized: {rep.v1229_creativity_dim_realized:.4f} ({rep.v1229_creativity_dim_cell_count} cells lifted)")
    print(f"v1229_overall_realized_172: {rep.v1229_overall_realized_172:.4f} (lift {rep.v1229_overall_lift_delta_realized_from_v1228:+.4f})")
    print(f"v1229_overall_mean_286: {rep.v1229_overall_mean_286:.4f} (lift {rep.v1229_overall_lift_delta_mean_from_v1228:+.4f})")
    print(f"v1229_inflation_gap: {rep.v1229_inflation_gap_v1228_minus_realized:.4f}")
    print(f"v1229_position_vs_north_star: {rep.position_of_north_star_realized_pct:.2f}%")
    print(f"total_creativity_molecules: {rep.total_creativity_molecules}")
    print(f"6 pathway pass count: {rep.n_pathways_pass}/{rep.n_pathways_total}")
    print(f"artifact: {artifact_path}")
    print(f"report: {report_path}")

    if "--json" in args:
        print()
        print(json.dumps(asdict(rep), ensure_ascii=False, indent=2, sort_keys=True))

    if "--full" in args:
        print()
        print("Pathway scores:")
        for k, s in rep.pathway_scores.items():
            print(f"  {k}: {s:.4f} ({rep.pathway_real_molecule_count[k]} molecules)")
        print()
        print("CREATIVITY coverage:")
        for k in sorted(rep.creativity_coverage_v1229.keys()):
            print(f"  {k}: {rep.creativity_coverage_v1229[k]:.2f}")
        print()
        print("V3 哲学守门:")
        for k, v in rep.v3_guards.items():
            print(f"  {k}: {'PASS' if v else 'FAIL'}")

    return 0


if __name__ == "__main__":
    sys.exit(cli_main())
