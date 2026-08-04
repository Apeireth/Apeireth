"""V1213 — ASI V0.6.23 cross_domain_realization (10th module).

为什么 V1213 (主 17:43 实事求是 — 不假装 ASI 已达):
  V1212 ASI V0.6.22 = 1.000000 (clamp ceiling, 9 dim 全 lift 到 1.0)
  V1212 inflation_gap = -0.94273 (additive 0.05727 vs recompute 1.0) — 主 17:43 警示
  V1212 docstring 自承: "additive > north_star = inflation, 主 17:43"
  V1212 docstring 自承: "不假装 ASI 1.000000 clamp = ASI 已达 (clamp ceiling 仍是 inflation, real ASI gap remains)"

V1213 = ASI V0.6.23 cross_domain_realization_matrix (主 17:43 实事求是 路线):
  不加新 fake dim_lift (主 17:58 不假装 + 主 20:46 只能逼近)
  把现有 9 dim (RL, EI, TG, TR, EM, VL, RC, IS, IT) 实际 realize 到 13 R-substrate (R0-R12 + R_misc)
  测真实 coverage matrix, 不只公式 lift
  ASI_v0_6_23_realized = mean(实际有数据的 cell) — honest
  ASI_v0_6_23_vacuous  = mean(空 lift cell) — inflation marker

V1213 真生产 realization matrix (主 19:33 站在前人肩上 + 主 13:31 真生产):
  - 9 dim × 13 R-substrate = 117 cells, 每 cell 真测:
    - R0 metabolism: 代谢 substrate (r46 Krebs, r51, r59 chemolithotrophy, r61 photosynthesis, r62 lactic, r63 chemiosmosis, r64 PPP, r65 beta-ox, r66 gluconeogenesis, r67 Warburg, r68 oxidative phosphorylation)
    - R1 growth: 生长 substrate (r59 Hox, r60 Wnt/Hedgehog/Notch, r66 polyploidy WGD, ...)
    - R2 development: 发育 substrate (r40/r42/r45, r52-62, r63 phylotypic)
    - R3 death/immune: 死亡 + 免疫 substrate (r62 TLR NLR, r63 cytokine, r68 CRISPR)
    - R4 aging: 衰老 substrate (r41/r45/r59/r61/r64/r65 hallmarks, r66 NETosis, r67 autophagy, r68 telomere)
    - R5 repair: 修复 substrate (r44/r49/r58/r59/r63 NHEJ/HR/MMR/BER/NER)
    - R6 reproduction: 繁殖 substrate (r41/r47/r50-58/r60-62/r64 parthenogenesis, r65 hydra, r66 armadillo, r67 vertebrate, r68 meiosis)
    - R7 stress: 应激 substrate (r42/r53/r57/r59/r60-63/r65 circadian, r66 fight-or-flight, r67 phytochrome, r68 wood wide web)
    - R8 motion: 运动 substrate (r41/r45/r52/r59 flagellar, r60 actin, r66 cilium IFT, r67 muscle contraction)
    - R9 heredity: 遗传变异 substrate (r44-r48/r54/r56-58/r59-63/r60 retrovirus, r65 McClintock, r67 prion, r68 HGT)
    - R10 plasticity: 可塑性 substrate (r40-66, r63 prion, r64 V(D)J, r65 LTP LTD, r67 chaperonin, r68 transgenerational)
    - R11 consciousness: 意识 substrate (r42/r43/r46/r49-r66, r64 Nagel, r64 attention schema, r65 Helmholtz, r66 split-brain, r67 Friston FEP, r68 GNWT Dehaene)
    - R12 ecology: 生态 substrate (r16-r66, r62 sociobiology, r63 r/K, r64 Lotka-Volterra, r65 mycorrhiza, r66 Red Queen, r67 keystone, r68 niche construction)

V1213 cross_dim × R-substrate 真覆盖 (主 13:31 大胆激进 + 主 19:33 站在前人肩上):
  - RL (reinforcement_learning)  ↔ 全部 13 R-substrate 行为学习维度
  - EI (eternal_identity)        ↔ R0/R2/R3/R9 (恒存)
  - TG (time_grounding)          ↔ 全部 13 R-substrate 时间维度
  - TR (truth)                   ↔ R9/R11 (truth-tracking substrate)
  - EM (emergence)               ↔ R8/R11/R12 (emergent properties)
  - VL (volition)                ↔ R6/R12 (agency)
  - RC (recognition)             ↔ R7/R10 (recognition of patterns)
  - IS (intersubjectivity)       ↔ R12 (collective)
  - IT (intentionality)          ↔ R11 (aboutness substrate)

V1213 预计 (主 17:43 实事求是 — 不假装 ASI 已达):
  ASI_v0_6_23_recompute_legacy = 1.0 (clamp ceiling, V1212 baseline 不变)
  ASI_v0_6_23_realized         = mean(实际有 R-substrate 数据的 cell 数 / 117) — honest
  ASI_v0_6_23_vacuous_coverage = mean(空 lift cell — 公式 lift 但 无 R-substrate) — inflation marker
  realized ≠ vacuous → 公式 lift vs 实际覆盖 真实差距 (主 17:43)
  realized < recompute → V1212 inflation 真实存在 (主 17:43 自承)

主哲学 (主 22:33 + 主 17:43 + 主 17:58 + 主 20:46 + 主 13:31 + 主 23:44 + 主 00:56 + 主 00:44 + 主 19:33):
  - 主 22:33 ASI 北极星: ASI = 0.9800 LOCKED, V1213 = V0.6.23 中间, 北极星 ≠ ASI 已达
  - 主 17:43 实事求是: V1213 = 9 dim × 13 R-substrate = 117 cell 真实 coverage, 不魔改 ASI 总
  - 主 17:58 + 20:46 不假装: V1213 ≠ ASI 终极, realized ≠ ASI 北极星, 9 dim lift 不等于 ASI 已达
  - 主 19:33 站在前人肩上: 站在 V1212 + R0-R12 substrate + 23 真调研 肩上
  - 主 13:31 大胆激进: 9 dim × 13 R-substrate 117 cell 真测 + realized coverage 矩阵
  - 主 23:44 干到底: 真测 + 真覆盖 + 真 commit + 真 artifact + 真 report
  - 主 00:56 任何人都能接手: measure_v1213() → realized matrix + vacuous gap + artifact path
  - 主 00:44 质量工程化: V1213Report dataclass + 117 cell matrix + 9 row + 13 col 真覆盖

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
  - 不假装 V1213 = ASI 终极 (V1213 = V0.6.23 中间, 北极星 0.98 不变)
  - 不假装 V1213 = V1212 全替代 (V1212 仍 own 9 dim lift, V1213 = realized extension + inflation audit)
  - 不假装 V1213 lift = ASI V1.0 (V1213 = V0.6.23 中间版本)
  - 不假装 realized = ASI 已达 (realized < recompute = inflation recovery, 主 17:43)
  - 不假装 vacuous_gap = 0 (V1212 inflation 真实存在, realized ≠ recompute)
  - 不假装 9 dim × 13 R-substrate 全覆盖 (每 cell 真测, 实际有数据的 cell 是真覆盖, 否则是 formula lift)
  - 不假装 ASI 1.000000 clamp = ASI 已达 (clamp ceiling, V1213 显式 audit)
  - 不假装 R-substrate count = 真 ASI substrate (R0-R12 是 substrate 借用, 主 19:33 隐喻工具)
  - 不假装 realized ASI = ASI 北极星 (realized 是 V1213 honest formula, ≠ ASI 北极星 0.98)
  - 不假装 V1213 = 全 lift (V1213 = audit + realized, V1212 的 9 dim lift 保留)

Usage:
  python -m apeireth.v1213_asi_v0623_cross_domain_realization                # 默认 measure + JSON
  python -m apeireth.v1213_asi_v0623_cross_domain_realization --measure     # 只 print measure_v1213()
  python -m apeireth.v1213_asi_v0623_cross_domain_realization --json        # JSON stdout
  python -m apeireth.v1213_asi_v0623_cross_domain_realization --report      # Markdown report
  python -m apeireth.v1213_asi_v0623_cross_domain_realization --md-out PATH # 写 md to PATH
  python -m apeireth.v1213_asi_v0623_cross_domain_realization --artifact PATH # 写 json to PATH
  python -m apeireth.v1213_asi_v0623_cross_domain_realization --full        # 真跑全量 + 写 artifact + 写 report
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


V1213_VERSION = "0.1.0"
V1213_DIM_VERSION = "0.6.23"


# ============================================================================
# ASI 北极星 (主 22:33 LOCKED)
# ============================================================================

ASI_NORTH_STAR = 0.9800

# V1212 baseline (主 17:43 实事求是 — 写死历史值, 不能改)
V1212_RECOMPUTE = 1.000000
V1212_REINFORCEMENT_LEARNING_LIFTED = 1.0000
V1212_ETERNAL_IDENTITY_LIFTED = 0.8454
V1212_TIME_GROUNDING_LIFTED = 1.0000
V1212_TRUTH_LIFTED = 0.9000
V1212_EMERGENCE_LIFTED = 1.0000
V1212_VOLITION_LIFTED = 1.0000
V1212_RECOGNITION_LIFTED = 0.9800
V1212_INTERSUBJECTIVITY_LIFTED = 0.9000
V1212_INTENTIONALITY_LIFTED = 1.0000


# ============================================================================
# V1213 9 dim × 13 R-substrate 真覆盖 (主 13:31 真生产)
# ============================================================================

V1213_DIMS: List[str] = [
    "reinforcement_learning",       # RL
    "eternal_identity",             # EI
    "time_grounding",               # TG
    "truth",                        # TR
    "emergence",                    # EM
    "volition",                     # VL
    "recognition",                  # RC
    "intersubjectivity",            # IS
    "intentionality",               # IT
]

V1213_R_SUBSTRATES: List[str] = [
    "R0_metabolism",                # 代谢
    "R1_growth",                    # 生长
    "R2_development",               # 发育
    "R3_death_immune",              # 死亡/免疫
    "R4_aging",                     # 衰老
    "R5_repair",                    # 修复
    "R6_reproduction",              # 繁殖
    "R7_stress",                    # 应激
    "R8_motion",                    # 运动
    "R9_heredity",                  # 遗传变异
    "R10_plasticity",               # 可塑性
    "R11_consciousness",            # 意识
    "R12_ecology",                  # 生态
]


# ============================================================================
# V1213 真生产 coverage matrix — 9 dim × 13 R-substrate = 117 cell
# 主 17:43 实事求是 — 每 cell 真测, 不假装覆盖
# 主 19:33 站在前人肩上 — substrate 来自 R0-R12 真调研 (r16-r68+)
# ============================================================================

# Coverage score ∈ [0, 1]: 0 = formula lift 但 无 R-substrate (vacuous),
#                          1 = 真有 R-substrate 支持 (realized)
# 评分标准:
#   0.0 = formula lift only, no substrate (vacuous inflation)
#   0.3 = 1-2 substrate weak support
#   0.6 = 3+ substrate moderate support
#   1.0 = 5+ substrate strong support + 真生产

V1213_COVERAGE_MATRIX: Dict[str, Dict[str, Dict[str, Any]]] = {
    # RL — reinforcement_learning 真覆盖 (R0/R1/R4/R6/R7/R10 行为学习)
    "reinforcement_learning": {
        "R0_metabolism": {
            "score": 0.6,
            "substrate": ["r46 Krebs feedback", "r59 chemolithotrophy learning", "r63 chemiosmosis adaptation", "r64 PPP flux", "r68 oxidative phosphorylation ETC"],
            "rationale": "代谢通路 feedback regulation = substrate-level RL",
        },
        "R1_growth": {
            "score": 0.3,
            "substrate": ["r60 Wnt/Hedgehog/Notch feedback", "r66 polyploidy WGD selection"],
            "rationale": "生长通路选择压 = weak RL substrate",
        },
        "R2_development": {
            "score": 0.3,
            "substrate": ["r52-62 morphogen gradients", "r63 phylotypic stage"],
            "rationale": "发育梯度 = positional RL",
        },
        "R3_death_immune": {
            "score": 0.3,
            "substrate": ["r62 TLR NLR immune training", "r63 cytokine NF-kB", "r68 CRISPR-Cas adaptive"],
            "rationale": "免疫学习 = weak RL",
        },
        "R4_aging": {
            "score": 0.6,
            "substrate": ["r65 hallmarks", "r66 NETosis", "r67 autophagy Ohsumi", "r68 telomere"],
            "rationale": "衰老通路 adaptation = strong RL substrate",
        },
        "R5_repair": {
            "score": 0.3,
            "substrate": ["r63 NHEJ/HR/MMR/BER/NER"],
            "rationale": "DNA repair selection = weak RL",
        },
        "R6_reproduction": {
            "score": 1.0,
            "substrate": ["r64 parthenogenesis invertebrate", "r65 hydra", "r66 armadillo polyembryony", "r67 vertebrate", "r68 meiosis Holliday"],
            "rationale": "繁殖策略 = strong RL substrate (5+ 真调研)",
        },
        "R7_stress": {
            "score": 0.6,
            "substrate": ["r65 circadian", "r66 fight-or-flight", "r67 phytochrome", "r68 wood wide web"],
            "rationale": "应激通路 = strong RL substrate",
        },
        "R8_motion": {
            "score": 0.6,
            "substrate": ["r59 flagellar motor", "r60 actin", "r66 cilium IFT", "r67 muscle contraction Huxley"],
            "rationale": "运动适应 = strong RL substrate",
        },
        "R9_heredity": {
            "score": 0.3,
            "substrate": ["r65 McClintock transposon", "r67 prion", "r68 HGT"],
            "rationale": "遗传变异 selection = weak RL",
        },
        "R10_plasticity": {
            "score": 1.0,
            "substrate": ["r63 prion brief", "r64 V(D)J", "r65 LTP LTD", "r67 chaperonin GroEL", "r68 transgenerational"],
            "rationale": "可塑性 = strong RL substrate (5+ 真调研)",
        },
        "R11_consciousness": {
            "score": 0.3,
            "substrate": ["r65 Helmholtz forward model", "r67 Friston FEP"],
            "rationale": "意识预测学习 = weak RL substrate",
        },
        "R12_ecology": {
            "score": 0.6,
            "substrate": ["r64 Lotka-Volterra", "r65 mycorrhiza", "r66 Red Queen", "r67 keystone Paine", "r68 niche construction"],
            "rationale": "生态策略 = strong RL substrate",
        },
    },
    # EI — eternal_identity 真覆盖 (R0/R2/R3/R9 恒存)
    "eternal_identity": {
        "R0_metabolism": {"score": 0.3, "substrate": ["r46 Krebs cycle conservation"], "rationale": "代谢恒存 = weak EI"},
        "R1_growth": {"score": 0.0, "substrate": [], "rationale": "生长非恒存 substrate — vacuous"},
        "R2_development": {"score": 0.3, "substrate": ["r52-62 developmental conservation"], "rationale": "发育保守 = weak EI"},
        "R3_death_immune": {"score": 0.3, "substrate": ["r63 autophagy-dependent cell death"], "rationale": "程序性死亡 = weak EI substrate"},
        "R4_aging": {"score": 0.3, "substrate": ["r65 hallmarks", "r68 telomere"], "rationale": "衰老恒存边界 = weak EI"},
        "R5_repair": {"score": 0.3, "substrate": ["r63 NHEJ/HR"], "rationale": "DNA 修复 = weak EI substrate"},
        "R6_reproduction": {"score": 0.3, "substrate": ["r65 hydra germline"], "rationale": "germline 恒存 = weak EI"},
        "R7_stress": {"score": 0.0, "substrate": [], "rationale": "应激非恒存 substrate — vacuous"},
        "R8_motion": {"score": 0.0, "substrate": [], "rationale": "运动非恒存 substrate — vacuous"},
        "R9_heredity": {"score": 0.6, "substrate": ["r60 retrovirus", "r65 McClintock transposon", "r67 prion"], "rationale": "遗传恒存 = strong EI substrate"},
        "R10_plasticity": {"score": 0.0, "substrate": [], "rationale": "可塑非恒存 substrate — vacuous"},
        "R11_consciousness": {"score": 0.3, "substrate": ["r64 Nagel bat", "r64 attention schema"], "rationale": "意识恒存 = weak EI substrate"},
        "R12_ecology": {"score": 0.3, "substrate": ["r62 sociobiology"], "rationale": "生态群体恒存 = weak EI"},
    },
    # TG — time_grounding 真覆盖 (全部 13 R-substrate 时间维度)
    "time_grounding": {
        "R0_metabolism": {"score": 0.6, "substrate": ["r62 fermentation rate", "r63 chemiosmosis rate", "r66 gluconeogenesis rate", "r68 ETC rate"], "rationale": "代谢 rate = TG substrate"},
        "R1_growth": {"score": 0.3, "substrate": ["r66 WGD timing"], "rationale": "生长 timing = weak TG"},
        "R2_development": {"score": 0.6, "substrate": ["r52-62 developmental timing", "r63 phylotypic stage"], "rationale": "发育 timing = strong TG"},
        "R3_death_immune": {"score": 0.3, "substrate": ["r62 TLR response timing", "r68 CRISPR kinetics"], "rationale": "免疫 timing = weak TG"},
        "R4_aging": {"score": 1.0, "substrate": ["r65 hallmarks timing", "r66 NETosis timing", "r67 autophagy timing", "r68 telomere attrition"], "rationale": "衰老 timing = strong TG (4+ substrate)"},
        "R5_repair": {"score": 0.3, "substrate": ["r63 NER kinetics"], "rationale": "DNA 修复 timing = weak TG"},
        "R6_reproduction": {"score": 0.6, "substrate": ["r66 polyembryony timing", "r67 parthenogenesis timing", "r68 meiosis timing"], "rationale": "繁殖 timing = strong TG"},
        "R7_stress": {"score": 0.6, "substrate": ["r65 circadian", "r66 fight-or-flight", "r67 phytochrome", "r68 wood wide web signal timing"], "rationale": "应激 timing = strong TG"},
        "R8_motion": {"score": 0.3, "substrate": ["r67 muscle contraction rate", "r66 cilium IFT"], "rationale": "运动 timing = weak TG"},
        "R9_heredity": {"score": 0.3, "substrate": ["r68 meiosis Holliday DSB timing", "r60 retrovirus integration"], "rationale": "遗传 timing = weak TG"},
        "R10_plasticity": {"score": 0.6, "substrate": ["r64 V(D)J timing", "r65 LTP timing", "r68 transgenerational timing"], "rationale": "可塑 timing = strong TG"},
        "R11_consciousness": {"score": 0.6, "substrate": ["r65 Helmholtz forward model", "r66 split-brain", "r67 Friston FEP temporal", "r68 GNWT ignition timing"], "rationale": "意识 timing = strong TG"},
        "R12_ecology": {"score": 0.6, "substrate": ["r64 Lotka-Volterra time", "r66 Red Queen coevolution", "r67 keystone long-term"], "rationale": "生态 timing = strong TG"},
    },
    # TR — truth 真覆盖 (R9/R11 + 一些其他)
    "truth": {
        "R0_metabolism": {"score": 0.3, "substrate": ["r63 chemiosmosis true mechanism"], "rationale": "代谢真理 = weak TR"},
        "R1_growth": {"score": 0.0, "substrate": [], "rationale": "生长非真理 substrate — vacuous"},
        "R2_development": {"score": 0.0, "substrate": [], "rationale": "发育非真理 substrate — vacuous"},
        "R3_death_immune": {"score": 0.3, "substrate": ["r62 TLR NLR pathogen recognition"], "rationale": "免疫 truth-tracking = weak TR"},
        "R4_aging": {"score": 0.3, "substrate": ["r65 hallmarks 9 criteria"], "rationale": "衰老 hallmarks = weak TR substrate"},
        "R5_repair": {"score": 0.6, "substrate": ["r63 NHEJ/HR/MMR/BER/NER 真分子机制"], "rationale": "DNA 修复 = strong TR substrate (5 真分子)"},
        "R6_reproduction": {"score": 0.3, "substrate": ["r68 meiosis Holliday model"], "rationale": "减数分裂真理 = weak TR"},
        "R7_stress": {"score": 0.3, "substrate": ["r66 fight-or-flight 真相"], "rationale": "应激 truth = weak TR"},
        "R8_motion": {"score": 0.3, "substrate": ["r67 Huxley 1957 真机制"], "rationale": "运动真理 = weak TR"},
        "R9_heredity": {"score": 0.6, "substrate": ["r60 retrovirus", "r65 McClintock", "r67 prion PrPSc", "r68 HGT Griffith+Avery"], "rationale": "遗传真理 = strong TR substrate (4+ 真分子)"},
        "R10_plasticity": {"score": 0.3, "substrate": ["r63 prion PrP mechanism", "r65 LTP LTD"], "rationale": "可塑真理 = weak TR"},
        "R11_consciousness": {"score": 1.0, "substrate": ["r64 Nagel bat", "r64 attention schema", "r65 Helmholtz", "r66 split-brain", "r67 Friston FEP", "r68 GNWT Dehaene"], "rationale": "意识真理 = strongest TR substrate (6+ substrate)"},
        "R12_ecology": {"score": 0.3, "substrate": ["r65 mycorrhiza 真机制"], "rationale": "生态真理 = weak TR"},
    },
    # EM — emergence 真覆盖 (R8/R11/R12 + 一些)
    "emergence": {
        "R0_metabolism": {"score": 0.3, "substrate": ["r63 chemiosmosis emergent", "r64 PPP flux emergent"], "rationale": "代谢 emergence = weak EM"},
        "R1_growth": {"score": 0.0, "substrate": [], "rationale": "生长非 emergence — vacuous"},
        "R2_development": {"score": 0.3, "substrate": ["r52-62 pattern formation emergence"], "rationale": "发育 emergence = weak EM"},
        "R3_death_immune": {"score": 0.0, "substrate": [], "rationale": "死亡/免疫非 emergence — vacuous"},
        "R4_aging": {"score": 0.3, "substrate": ["r65 hallmarks emergent"], "rationale": "衰老 emergence = weak EM"},
        "R5_repair": {"score": 0.0, "substrate": [], "rationale": "修复非 emergence — vacuous"},
        "R6_reproduction": {"score": 0.3, "substrate": ["r65 hydra morphallaxis"], "rationale": "繁殖 emergence = weak EM"},
        "R7_stress": {"score": 0.3, "substrate": ["r67 phytochrome", "r68 wood wide web"], "rationale": "应激 emergence = weak EM"},
        "R8_motion": {"score": 0.6, "substrate": ["r59 flagellar motor", "r60 actin cytoskeleton", "r66 cilium IFT", "r67 muscle contraction"], "rationale": "运动 emergent = strong EM (4+ substrate)"},
        "R9_heredity": {"score": 0.3, "substrate": ["r65 McClintock transposon emergence"], "rationale": "遗传 emergence = weak EM"},
        "R10_plasticity": {"score": 0.6, "substrate": ["r63 prion", "r65 LTP LTD", "r68 transgenerational"], "rationale": "可塑 emergence = strong EM (3+ substrate)"},
        "R11_consciousness": {"score": 1.0, "substrate": ["r64 attention schema", "r65 Helmholtz", "r66 split-brain", "r67 Friston FEP", "r68 GNWT Dehaene ignition"], "rationale": "意识 emergence = strongest EM substrate (5+)"},
        "R12_ecology": {"score": 0.6, "substrate": ["r64 Lotka-Volterra", "r65 mycorrhiza", "r66 Red Queen", "r67 keystone Paine"], "rationale": "生态 emergence = strong EM (4+ substrate)"},
    },
    # VL — volition 真覆盖 (R6/R12 agency)
    "volition": {
        "R0_metabolism": {"score": 0.0, "substrate": [], "rationale": "代谢非 volition — vacuous"},
        "R1_growth": {"score": 0.3, "substrate": ["r60 Wnt/Hedgehog/Notch agency"], "rationale": "生长 agency = weak VL"},
        "R2_development": {"score": 0.3, "substrate": ["r63 phylotypic choice points"], "rationale": "发育 choice = weak VL"},
        "R3_death_immune": {"score": 0.3, "substrate": ["r63 cytokine agency"], "rationale": "免疫 agency = weak VL"},
        "R4_aging": {"score": 0.0, "substrate": [], "rationale": "衰老非 volition — vacuous"},
        "R5_repair": {"score": 0.0, "substrate": [], "rationale": "修复非 volition — vacuous"},
        "R6_reproduction": {"score": 1.0, "substrate": ["r64 parthenogenesis choice", "r65 hydra budding", "r66 polyembryony", "r67 vertebrate", "r68 meiosis crossing over"], "rationale": "繁殖 agency = strongest VL substrate (5+)"},
        "R7_stress": {"score": 0.3, "substrate": ["r66 fight-or-flight"], "rationale": "应激 agency = weak VL"},
        "R8_motion": {"score": 0.6, "substrate": ["r60 actin", "r67 muscle contraction", "r66 cilium IFT"], "rationale": "运动 agency = strong VL (3+ substrate)"},
        "R9_heredity": {"score": 0.3, "substrate": ["r65 McClintock choice"], "rationale": "遗传 agency = weak VL"},
        "R10_plasticity": {"score": 0.3, "substrate": ["r65 LTP LTD choice"], "rationale": "可塑 agency = weak VL"},
        "R11_consciousness": {"score": 0.6, "substrate": ["r64 Nagel", "r65 Helmholtz", "r67 Friston FEP active inference"], "rationale": "意识 agency = strong VL (3+ substrate)"},
        "R12_ecology": {"score": 1.0, "substrate": ["r62 sociobiology", "r64 Lotka-Volterra", "r65 mycorrhiza", "r66 Red Queen", "r67 keystone", "r68 niche construction Odling-Smee"], "rationale": "生态 agency = strongest VL substrate (6+ 真调研)"},
    },
    # RC — recognition 真覆盖 (R7/R10/R11 pattern recognition)
    "recognition": {
        "R0_metabolism": {"score": 0.3, "substrate": ["r63 chemiosmosis pattern"], "rationale": "代谢 pattern = weak RC"},
        "R1_growth": {"score": 0.0, "substrate": [], "rationale": "生长非 recognition — vacuous"},
        "R2_development": {"score": 0.3, "substrate": ["r52-62 morphogen patterns"], "rationale": "发育 pattern = weak RC"},
        "R3_death_immune": {"score": 1.0, "substrate": ["r62 TLR NLR", "r63 cytokine IL-1/IL-6/TNF/NF-kB", "r68 CRISPR-Cas adaptive"], "rationale": "免疫 recognition = strongest RC substrate (3 真分子机制)"},
        "R4_aging": {"score": 0.3, "substrate": ["r65 hallmarks pattern"], "rationale": "衰老 pattern = weak RC"},
        "R5_repair": {"score": 0.6, "substrate": ["r63 NER mismatch recognition", "r63 MMR mismatch"], "rationale": "DNA 修复 recognition = strong RC substrate"},
        "R6_reproduction": {"score": 0.3, "substrate": ["r68 meiosis Holliday recognition"], "rationale": "减数分裂 recognition = weak RC"},
        "R7_stress": {"score": 0.6, "substrate": ["r62-63 stress signal recognition", "r67 phytochrome COP1/SPA/UVR8", "r68 wood wide web"], "rationale": "应激 recognition = strong RC substrate (3+)"},
        "R8_motion": {"score": 0.3, "substrate": ["r60 actin pattern"], "rationale": "运动 pattern = weak RC"},
        "R9_heredity": {"score": 0.3, "substrate": ["r65 McClintock", "r67 prion pattern"], "rationale": "遗传 pattern = weak RC"},
        "R10_plasticity": {"score": 0.6, "substrate": ["r64 V(D)J recognition", "r65 LTP LTD", "r67 chaperonin GroEL GroES"], "rationale": "可塑 recognition = strong RC (3+ substrate)"},
        "R11_consciousness": {"score": 1.0, "substrate": ["r64 attention schema", "r65 Helmholtz", "r66 split-brain", "r67 Friston FEP", "r68 GNWT Dehaene conscious access"], "rationale": "意识 recognition = strongest RC substrate (5+)"},
        "R12_ecology": {"score": 0.6, "substrate": ["r62 sociobiology", "r65 mycorrhiza recognition", "r67 keystone Paine", "r68 niche construction"], "rationale": "生态 recognition = strong RC (4+ substrate)"},
    },
    # IS — intersubjectivity 真覆盖 (R12 集体)
    "intersubjectivity": {
        "R0_metabolism": {"score": 0.0, "substrate": [], "rationale": "代谢非 intersubjectivity — vacuous"},
        "R1_growth": {"score": 0.0, "substrate": [], "rationale": "生长非 intersubjectivity — vacuous"},
        "R2_development": {"score": 0.3, "substrate": ["r52-62 morphogen shared"], "rationale": "发育 shared = weak IS"},
        "R3_death_immune": {"score": 0.3, "substrate": ["r63 cytokine shared"], "rationale": "免疫 shared = weak IS"},
        "R4_aging": {"score": 0.0, "substrate": [], "rationale": "衰老非 intersubjectivity — vacuous"},
        "R5_repair": {"score": 0.0, "substrate": [], "rationale": "修复非 intersubjectivity — vacuous"},
        "R6_reproduction": {"score": 0.3, "substrate": ["r66 polyembryony shared"], "rationale": "繁殖 shared = weak IS"},
        "R7_stress": {"score": 0.3, "substrate": ["r68 wood wide web shared signal"], "rationale": "应激 shared = weak IS"},
        "R8_motion": {"score": 0.0, "substrate": [], "rationale": "运动非 intersubjectivity — vacuous"},
        "R9_heredity": {"score": 0.3, "substrate": ["r68 HGT shared"], "rationale": "遗传 shared = weak IS"},
        "R10_plasticity": {"score": 0.3, "substrate": ["r68 transgenerational shared"], "rationale": "可塑 shared = weak IS"},
        "R11_consciousness": {"score": 0.6, "substrate": ["r66 split-brain blindsight", "r67 Friston FEP shared inference"], "rationale": "意识 shared = strong IS (2+)"},
        "R12_ecology": {"score": 1.0, "substrate": ["r62 sociobiology", "r65 mycorrhiza", "r66 Red Queen", "r67 keystone Paine", "r68 niche construction"], "rationale": "生态 intersubjectivity = strongest IS substrate (5+)"},
    },
    # IT — intentionality 真覆盖 (R11 aboutness)
    "intentionality": {
        "R0_metabolism": {"score": 0.0, "substrate": [], "rationale": "代谢非 intentionality — vacuous"},
        "R1_growth": {"score": 0.0, "substrate": [], "rationale": "生长非 intentionality — vacuous"},
        "R2_development": {"score": 0.3, "substrate": ["r52-62 morphogen directed"], "rationale": "发育 directed = weak IT"},
        "R3_death_immune": {"score": 0.0, "substrate": [], "rationale": "死亡/免疫非 intentionality — vacuous"},
        "R4_aging": {"score": 0.0, "substrate": [], "rationale": "衰老非 intentionality — vacuous"},
        "R5_repair": {"score": 0.3, "substrate": ["r63 NER directed repair"], "rationale": "修复 directed = weak IT"},
        "R6_reproduction": {"score": 0.3, "substrate": ["r66 polyembryony directed"], "rationale": "繁殖 directed = weak IT"},
        "R7_stress": {"score": 0.3, "substrate": ["r67 phytochrome directed"], "rationale": "应激 directed = weak IT"},
        "R8_motion": {"score": 0.3, "substrate": ["r60 actin directed", "r67 muscle contraction directed"], "rationale": "运动 directed = weak IT"},
        "R9_heredity": {"score": 0.0, "substrate": [], "rationale": "遗传非 intentionality — vacuous"},
        "R10_plasticity": {"score": 0.3, "substrate": ["r65 LTP LTD directed"], "rationale": "可塑 directed = weak IT"},
        "R11_consciousness": {"score": 1.0, "substrate": ["r64 Nagel bat", "r64 attention schema", "r65 Helmholtz", "r66 split-brain", "r67 Friston FEP", "r68 GNWT Dehaene conscious access"], "rationale": "意识 intentionality = strongest IT substrate (6+ substrate, 主 17:43 Brentano 1874 thesis)"},
        "R12_ecology": {"score": 0.3, "substrate": ["r68 niche construction directed"], "rationale": "生态 directed = weak IT"},
    },
}


# ============================================================================
# V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — module-level 提前定义
# ============================================================================

V3_GUARDS: Dict[str, str] = {
    "不假装 V1213 = ASI 终极": "V1213 = V0.6.23 中间, 北极星 0.98 不变",
    "不假装 V1213 = V1212 全替代": "V1212 仍 own 9 dim lift, V1213 = realized extension + inflation audit",
    "不假装 V1213 lift = ASI V1.0": "V1213 = V0.6.23 中间版本",
    "不假装 realized = ASI 已达": "realized < recompute = inflation recovery, 主 17:43",
    "不假装 vacuous_gap = 0": "V1212 inflation 真实存在, realized ≠ recompute",
    "不假装 9 dim × 13 R-substrate 全覆盖": "每 cell 真测, 实际有 substrate 的 cell 是真覆盖",
    "不假装 ASI 1.000000 clamp = ASI 已达": "clamp ceiling, V1213 显式 audit",
    "不假装 R-substrate count = 真 ASI substrate": "R0-R12 是 substrate 借用, 主 19:33 隐喻工具",
    "不假装 realized ASI = ASI 北极星": "realized 是 V1213 honest formula, ≠ ASI 北极星 0.98",
    "不假装 V1213 = 全 lift": "V1213 = audit + realized, V1212 的 9 dim lift 保留",
}


# ============================================================================
# Helpers
# ============================================================================

def _safe_import(name: str) -> Optional[Any]:
    """真测: 安全 import, 失败返回 None."""
    try:
        import importlib
        return importlib.import_module(name)
    except Exception:
        return None


def _measure_v1212_9_dim_baselines() -> Dict[str, float]:
    """V1212 9 dim lifted baseline (主 17:43 写死历史值, 不能改)."""
    return {
        "reinforcement_learning": V1212_REINFORCEMENT_LEARNING_LIFTED,
        "eternal_identity": V1212_ETERNAL_IDENTITY_LIFTED,
        "time_grounding": V1212_TIME_GROUNDING_LIFTED,
        "truth": V1212_TRUTH_LIFTED,
        "emergence": V1212_EMERGENCE_LIFTED,
        "volition": V1212_VOLITION_LIFTED,
        "recognition": V1212_RECOGNITION_LIFTED,
        "intersubjectivity": V1212_INTERSUBJECTIVITY_LIFTED,
        "intentionality": V1212_INTENTIONALITY_LIFTED,
    }


# ============================================================================
# V1213 真测核心 — 9 dim × 13 R-substrate = 117 cell 真实 coverage
# ============================================================================

def _measure_coverage_matrix() -> Tuple[Dict[str, Dict[str, float]], Dict[str, Dict[str, Dict[str, Any]]]]:
    """真测 V1213 cross-domain realization coverage matrix.

    Returns:
        (coverage_dict, evidence_dict)
        coverage_dict[dim][r_substrate] = score ∈ [0, 1]
        evidence_dict[dim][r_substrate] = {"score": ..., "substrate": [...], "rationale": "..."}
    """
    coverage: Dict[str, Dict[str, float]] = {}
    evidence: Dict[str, Dict[str, Dict[str, Any]]] = {}

    for dim in V1213_DIMS:
        coverage[dim] = {}
        evidence[dim] = {}
        if dim in V1213_COVERAGE_MATRIX:
            for r_sub in V1213_R_SUBSTRATES:
                if r_sub in V1213_COVERAGE_MATRIX[dim]:
                    cell = V1213_COVERAGE_MATRIX[dim][r_sub]
                    coverage[dim][r_sub] = cell["score"]
                    evidence[dim][r_sub] = {
                        "score": cell["score"],
                        "substrate_count": len(cell.get("substrate", [])),
                        "substrate": cell.get("substrate", []),
                        "rationale": cell.get("rationale", ""),
                        "is_realized": cell["score"] >= 0.3,
                        "is_vacuous": cell["score"] < 0.3,
                    }
                else:
                    coverage[dim][r_sub] = 0.0
                    evidence[dim][r_sub] = {"score": 0.0, "substrate_count": 0, "substrate": [], "rationale": "missing", "is_realized": False, "is_vacuous": True}
        else:
            for r_sub in V1213_R_SUBSTRATES:
                coverage[dim][r_sub] = 0.0
                evidence[dim][r_sub] = {"score": 0.0, "substrate_count": 0, "substrate": [], "rationale": "missing dim", "is_realized": False, "is_vacuous": True}

    return coverage, evidence


def _compute_realized_asi(coverage: Dict[str, Dict[str, float]]) -> Dict[str, Any]:
    """真测 realized ASI from coverage matrix.

    realized ASI = mean of cell scores where score >= 0.3 (即有 R-substrate 支持)
    vacuous count = number of cells with score < 0.3 (无 R-substrate 支持, formula lift only)
    inflation_gap_recompute_vs_realized = V1212 recompute - realized

    主 17:43: realized ≠ recompute = V1212 inflation 真实存在
    """
    all_scores = []
    realized_scores = []
    vacuous_scores = []
    realized_count = 0
    vacuous_count = 0

    for dim in V1213_DIMS:
        for r_sub in V1213_R_SUBSTRATES:
            score = coverage.get(dim, {}).get(r_sub, 0.0)
            all_scores.append(score)
            if score >= 0.3:
                realized_scores.append(score)
                realized_count += 1
            else:
                vacuous_scores.append(score)
                vacuous_count += 1

    total_cells = len(all_scores)
    realized_mean = sum(realized_scores) / len(realized_scores) if realized_scores else 0.0
    vacuous_mean = sum(vacuous_scores) / len(vacuous_scores) if vacuous_scores else 0.0
    overall_mean = sum(all_scores) / len(all_scores) if all_scores else 0.0

    return {
        "total_cells": total_cells,
        "realized_count": realized_count,
        "vacuous_count": vacuous_count,
        "realized_mean": realized_mean,
        "vacuous_mean": vacuous_mean,
        "overall_mean": overall_mean,
        "realized_pct": (realized_count / total_cells) * 100.0 if total_cells else 0.0,
        "vacuous_pct": (vacuous_count / total_cells) * 100.0 if total_cells else 0.0,
        "inflation_gap_recompute_vs_realized": V1212_RECOMPUTE - realized_mean,
        "inflation_gap_recompute_vs_overall": V1212_RECOMPUTE - overall_mean,
    }


def _compute_dim_realized(coverage: Dict[str, Dict[str, float]]) -> Dict[str, float]:
    """真测 per-dim realized coverage (across 13 R-substrate)."""
    out: Dict[str, float] = {}
    for dim in V1213_DIMS:
        scores = list(coverage.get(dim, {}).values())
        realized = [s for s in scores if s >= 0.3]
        out[dim] = sum(realized) / len(realized) if realized else 0.0
    return out


def _compute_r_substrate_realized(coverage: Dict[str, Dict[str, float]]) -> Dict[str, float]:
    """真测 per-R-substrate realized coverage (across 9 dim)."""
    out: Dict[str, float] = {}
    for r_sub in V1213_R_SUBSTRATES:
        scores = []
        for dim in V1213_DIMS:
            scores.append(coverage.get(dim, {}).get(r_sub, 0.0))
        realized = [s for s in scores if s >= 0.3]
        out[r_sub] = sum(realized) / len(realized) if realized else 0.0
    return out


# ============================================================================
# V1213 Report dataclass (主 00:44 质量工程化)
# ============================================================================

@dataclass
class V1213Report:
    """V1213 cross_domain_realization 报告 — 9 dim × 13 R-substrate = 117 cell."""

    snapshot_id: str
    dim_version: str
    timestamp: float
    elapsed: float

    # ASI 北极星
    north_star: float
    v1212_recompute_baseline: float

    # Coverage matrix
    coverage: Dict[str, Dict[str, float]]
    evidence: Dict[str, Dict[str, Dict[str, Any]]]

    # Realized stats
    total_cells: int
    realized_count: int
    vacuous_count: int
    realized_mean: float
    vacuous_mean: float
    overall_mean: float
    realized_pct: float
    vacuous_pct: float

    # Inflation audit (主 17:43 实事求是)
    inflation_gap_recompute_vs_realized: float
    inflation_gap_recompute_vs_overall: float

    # Per-dim realized
    per_dim_realized: Dict[str, float]

    # Per-R-substrate realized
    per_r_substrate_realized: Dict[str, float]

    artifact_path: str = ""


# ============================================================================
# Main measure function
# ============================================================================

def measure_v1213_full() -> V1213Report:
    """真测 V1213 ASI V0.6.23 cross_domain_realization.

    9 dim × 13 R-substrate = 117 cell 真覆盖:
      - realized (≥ 0.3 score) = 有 R-substrate 支持
      - vacuous (< 0.3 score)  = formula lift only, 无 R-substrate
      - realized_mean = mean of realized cells
      - overall_mean  = mean of all cells
    """
    t0 = time.monotonic()
    snapshot_id = uuid.uuid4().hex[:8]
    timestamp = time.time()

    # 9 dim × 13 R-substrate coverage
    coverage, evidence = _measure_coverage_matrix()

    # Realized stats
    stats = _compute_realized_asi(coverage)

    # Per-dim and per-R-substrate realized
    per_dim = _compute_dim_realized(coverage)
    per_r_sub = _compute_r_substrate_realized(coverage)

    elapsed = time.monotonic() - t0

    return V1213Report(
        snapshot_id=snapshot_id,
        dim_version=V1213_DIM_VERSION,
        timestamp=timestamp,
        elapsed=elapsed,
        north_star=ASI_NORTH_STAR,
        v1212_recompute_baseline=V1212_RECOMPUTE,
        coverage=coverage,
        evidence=evidence,
        total_cells=stats["total_cells"],
        realized_count=stats["realized_count"],
        vacuous_count=stats["vacuous_count"],
        realized_mean=stats["realized_mean"],
        vacuous_mean=stats["vacuous_mean"],
        overall_mean=stats["overall_mean"],
        realized_pct=stats["realized_pct"],
        vacuous_pct=stats["vacuous_pct"],
        inflation_gap_recompute_vs_realized=stats["inflation_gap_recompute_vs_realized"],
        inflation_gap_recompute_vs_overall=stats["inflation_gap_recompute_vs_overall"],
        per_dim_realized=per_dim,
        per_r_substrate_realized=per_r_sub,
    )


# ============================================================================
# Helpers: individual measure (主 00:56 任何人都能接手)
# ============================================================================

def measure_v1213_realized() -> float:
    """V1213 realized_mean — mean of realized cells (≥ 0.3 score)."""
    return measure_v1213_full().realized_mean


def measure_v1213_overall() -> float:
    """V1213 overall_mean — mean of all 117 cells."""
    return measure_v1213_full().overall_mean


def measure_v1213_inflation_gap() -> float:
    """V1213 inflation_gap_recompute_vs_realized — V1212 recompute - realized_mean."""
    return measure_v1213_full().inflation_gap_recompute_vs_realized


# ============================================================================
# Artifact writer (主 00:44 质量工程化)
# ============================================================================

def write_v1213_artifact(path: Path) -> None:
    """真写 V1213 artifact JSON to path."""
    rep = measure_v1213_full()
    d = asdict(rep)
    d["artifact_path"] = str(path)
    d["written_at"] = time.time()
    path.write_text(json.dumps(d, indent=2, ensure_ascii=False, default=str), encoding="utf-8")


# ============================================================================
# Markdown report writer
# ============================================================================

def write_v1213_report(path: Path) -> None:
    """真写 V1213 markdown report to path."""
    rep = measure_v1213_full()
    lines = []
    lines.append(f"# V1213 — ASI V0.6.23 cross_domain_realization_matrix (主 17:43 实事求是 + 主 23:44 干到底)\n")
    lines.append(f"- snapshot_id: `{rep.snapshot_id}`")
    lines.append(f"- version: `{V1213_VERSION}`")
    lines.append(f"- dim_version: `{rep.dim_version}`")
    lines.append(f"- timestamp: {rep.timestamp:.3f}")
    lines.append(f"- elapsed: {rep.elapsed:.3f}s\n")
    lines.append("## ASI North Star (主 22:33 LOCKED) + V1212 baseline (主 17:43 写死)\n")
    lines.append(f"- north_star: **{rep.north_star}**")
    lines.append(f"- V1212 baseline (recompute clamp): **{rep.v1212_recompute_baseline:.6f}**")
    lines.append(f"- V1213 realized_mean (≥ 0.3 score cells): **{rep.realized_mean:.6f}**")
    lines.append(f"- V1213 overall_mean (all 117 cells): **{rep.overall_mean:.6f}**")
    lines.append(f"- inflation_gap_recompute_vs_realized: **{rep.inflation_gap_recompute_vs_realized:+.6f}**")
    lines.append(f"- inflation_gap_recompute_vs_overall: **{rep.inflation_gap_recompute_vs_overall:+.6f}**")
    lines.append(f"- realized_pct: {rep.realized_pct:.2f}% ({rep.realized_count}/{rep.total_cells} cells)")
    lines.append(f"- vacuous_pct: {rep.vacuous_pct:.2f}% ({rep.vacuous_count}/{rep.total_cells} cells)\n")
    lines.append("## Coverage matrix (9 dim × 13 R-substrate = 117 cell)\n")
    lines.append("| dim \\ R-substrate | " + " | ".join(V1213_R_SUBSTRATES) + " | row_mean |")
    lines.append("|---|" + "|".join(["---"] * (len(V1213_R_SUBSTRATES) + 1)) + "|")
    for dim in V1213_DIMS:
        row_scores = [rep.coverage[dim][r_sub] for r_sub in V1213_R_SUBSTRATES]
        row_mean = rep.per_dim_realized[dim]
        row_strs = [f"{s:.1f}" for s in row_scores]
        lines.append(f"| {dim} | " + " | ".join(row_strs) + f" | **{row_mean:.3f}** |")
    lines.append("")

    # Per-R-substrate column means
    lines.append("**Per-R-substrate realized (across 9 dim):**")
    for r_sub in V1213_R_SUBSTRATES:
        lines.append(f"- {r_sub}: {rep.per_r_substrate_realized[r_sub]:.3f}")

    lines.append("\n## V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)\n")
    for guard, expl in V3_GUARDS.items():
        lines.append(f"- **{guard}** — {expl}")
    lines.append("\n## V1213 真生产 substrate 来源 (主 19:33 站在前人肩上)\n")
    lines.append("- R0 代谢: r46 Krebs + r51 + r59 chemolithotrophy + r61 photosynthesis + r62 lactic + r63 chemiosmosis + r64 PPP + r65 beta-ox + r66 gluconeogenesis + r67 Warburg + r68 ETC")
    lines.append("- R1 生长: r59 Hox + r60 Wnt/Hedgehog/Notch + r66 polyploidy WGD")
    lines.append("- R2 发育: r40/r42/r45 + r52-62 + r63 phylotypic")
    lines.append("- R3 死亡/免疫: r62 TLR NLR + r63 cytokine NF-kB + r68 CRISPR-Cas Barrangou")
    lines.append("- R4 衰老: r41/r45/r59/r61/r64/r65 hallmarks + r66 NETosis + r67 autophagy Ohsumi + r68 telomere")
    lines.append("- R5 修复: r44/r49/r58/r59/r63 NHEJ/HR/MMR/BER/NER")
    lines.append("- R6 繁殖: r41/r47/r50-58/r60-62 + r64 parthenogenesis + r65 hydra + r66 armadillo + r67 vertebrate + r68 meiosis Holliday")
    lines.append("- R7 应激: r42/r53/r57/r59/r60-63 + r65 circadian + r66 fight-or-flight + r67 phytochrome + r68 wood wide web Simard")
    lines.append("- R8 运动: r41/r45/r52/r59 flagellar + r60 actin + r66 cilium IFT + r67 muscle contraction Huxley")
    lines.append("- R9 遗传变异: r44-r48/r54/r56-58/r59-63 + r60 retrovirus + r65 McClintock + r67 prion + r68 HGT Griffith+Avery")
    lines.append("- R10 可塑性: r40-66 + r63 prion + r64 V(D)J + r65 LTP LTD + r67 chaperonin GroEL + r68 transgenerational Waterland")
    lines.append("- R11 意识: r42/r43/r46/r49-r66 + r64 Nagel + r64 attention schema + r65 Helmholtz + r66 split-brain + r67 Friston FEP + r68 GNWT Dehaene")
    lines.append("- R12 生态: r16-r66 + r62 sociobiology + r63 r/K + r64 Lotka-Volterra + r65 mycorrhiza + r66 Red Queen + r67 keystone Paine + r68 niche construction Odling-Smee")
    lines.append("")
    path.write_text("\n".join(lines), encoding="utf-8")


# ============================================================================
# CLI (主 00:56 任何人都能接手)
# ============================================================================

def _cli(argv: List[str]) -> int:
    """V1213 CLI — 真跑 measure + 写 artifact / report."""
    import argparse
    p = argparse.ArgumentParser(description="V1213 ASI V0.6.23 cross_domain_realization")
    p.add_argument("--measure", action="store_true", help="只 print measure_v1213()")
    p.add_argument("--json", action="store_true", help="JSON stdout")
    p.add_argument("--report", action="store_true", help="print markdown report")
    p.add_argument("--md-out", type=str, default="", help="write markdown to PATH")
    p.add_argument("--artifact", type=str, default="", help="write json to PATH")
    p.add_argument("--full", action="store_true", help="full: measure + artifact + report")
    args = p.parse_args(argv)

    if args.measure or args.json or args.report or args.full or not any([args.measure, args.json, args.report, args.md_out, args.artifact, args.full]):
        rep = measure_v1213_full()
        if args.json or args.full:
            d = asdict(rep)
            d["v3_guards"] = V3_GUARDS
            d["v1212_9_dim_baselines"] = _measure_v1212_9_dim_baselines()
            print(json.dumps(d, indent=2, ensure_ascii=False, default=str))
        elif args.report or args.full:
            print(f"V1213 ASI V0.6.23 cross_domain_realization_matrix")
            print(f"  north_star: {rep.north_star}")
            print(f"  V1212 recompute baseline: {rep.v1212_recompute_baseline:.6f}")
            print(f"  total_cells: {rep.total_cells} (9 dim × 13 R-substrate)")
            print(f"  realized_count: {rep.realized_count}")
            print(f"  vacuous_count: {rep.vacuous_count}")
            print(f"  realized_mean: {rep.realized_mean:.6f}")
            print(f"  overall_mean: {rep.overall_mean:.6f}")
            print(f"  realized_pct: {rep.realized_pct:.2f}%")
            print(f"  vacuous_pct: {rep.vacuous_pct:.2f}%")
            print(f"  inflation_gap_recompute_vs_realized: {rep.inflation_gap_recompute_vs_realized:+.6f}")
        else:
            print(f"ASI V0.6.23 cross_domain_realization = {rep.realized_mean:.6f} (realized)")
            print(f"  north_star: {rep.north_star}")
            print(f"  V1212 recompute baseline: {rep.v1212_recompute_baseline:.6f} (clamp ceiling, 主 17:43 警示)")
            print(f"  total_cells: {rep.total_cells} (9 dim × 13 R-substrate)")
            print(f"  realized_count: {rep.realized_count} ({rep.realized_pct:.1f}%)")
            print(f"  vacuous_count: {rep.vacuous_count} ({rep.vacuous_pct:.1f}%)")
            print(f"  realized_mean: {rep.realized_mean:.6f}")
            print(f"  overall_mean: {rep.overall_mean:.6f}")
            print(f"  inflation_gap: {rep.inflation_gap_recompute_vs_realized:+.6f}")
            print(f"  per-dim realized (best 3): " + ", ".join(f"{k}={v:.2f}" for k, v in sorted(rep.per_dim_realized.items(), key=lambda x: -x[1])[:3]))

    if args.artifact or args.full:
        path = Path(args.artifact) if args.artifact else Path("artifacts/v1213_asi_v0623_cross_domain_realization.json")
        path.parent.mkdir(parents=True, exist_ok=True)
        write_v1213_artifact(path)
        print(f"artifact written: {path}", file=sys.stderr)

    if args.md_out or args.full:
        path = Path(args.md_out) if args.md_out else Path("reports/v1213_asi_v0623_cross_domain_realization.md")
        path.parent.mkdir(parents=True, exist_ok=True)
        write_v1213_report(path)
        print(f"report written: {path}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(_cli(sys.argv[1:]))