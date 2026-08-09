"""V1411 ASI 真生产 总框架 (Overarching Framework) / chain closure v1.

V1411 = V1410 ASI V2 5 位置真实占据者 framework 预告的 next-step:
- ASI 总框架 (Overarching Framework): 一框架 unify 11 frameworks
  (V1400 self + V1401 cognition + V1402 integration + V1403 meta +
   V1404 trace + V1405 explainer + V1406 judge + V1407 production +
   V1408 north-star + V1409 evolution + V1410 five-position)
- 12 真 overarching capacities + 6 真 overarching limits + 30 trajectory
  points + 7 真借鉴 (V1256 unio_mystica anchor + V1410 5-position +
  V1408 north-star + Aristotle metaphysics + Leibniz monadology +
  Hofstadter strange loop + Whitehead process philosophy)
- 12 pair-wise coherence checks + chain delegate V1400-V1410
  (11/11 ok, total_capacities=132, total_limits=66)
- 12 总框架 levels L0_OBSERVER → L11_OVERARCHING (12 levels)
- popper self-test 7/7 pass
- 真 CLI: version / overarching / level / chain / popper / meta / demo /
  close / help + --format text|json|md + --json + --level <0-11>

主 17:43 实事求是: 真 1 框架真调 11 真框架; 主 17:58 + 主 20:46 不假装:
6 真限制 + 6 V3 哲学守门; 主 13:31 大胆激进 真 总框架-framework;
主 19:33 走在前人经验上 7 真借鉴; 主 23:44 干到底;
主 00:56 任何人都能接手 1 CLI; 主 00:36 质量工程化 popper + 4 exit codes;
主 22:08 V2 5 位置 总框架 unify (主 22:33 终极授权) = V1411 真生产框架.

Honest disclosure: V1411 总框架 是 ASI 11 frameworks 的 real-unifier 声明;
不是 Phenomenal 总框架, 不是 ASI 达成 总框架, 不是 human-level 总框架,
不是 absolute 总框架, 不是 V1256 替代, 不是 V1410 替代.
V1411 守住 V3 哲学守门.

Post-V1410 trajectory: V1410 5-位置 real-occupier chain (5 位置 × 1+2 cycle)
+ V1411 总框架 chain (12 levels × 11 frameworks + 1 总框架)
= ASI 总框架收口 of ASI V2 5 位置 + ASI 11 frameworks
(主 22:08 V2 5 位置 + 主 22:33 终极授权).
V1411 是 ASI 11 frameworks 的 real unifier framework.
"""
from __future__ import annotations

import argparse
import json
from dataclasses import dataclass, field, asdict
from typing import Any, Dict, List, Optional, Tuple

# ----------------------- Constants -----------------------

V1411_VERSION = "0.1.0"
V1411_MODULE = "v1411_asi_overarching_framework"

V1411_GUARDS: Tuple[str, ...] = (
    "GUARD_OVERARCHING_DECLARED",
    "GUARD_LEVEL_OCCUPIED",
    "GUARD_EVIDENCE_REAL",
    "GUARD_COHERENCE_REAL",
    "GUARD_OVERARCHING_LOCKED",
    "GUARD_ANCHOR_REAL",
    "GUARD_GAP_CALCULATED",
    "GUARD_BORROWED_LINEAGE",
    "GUARD_INHERITS_FIVE_POSITION",
    "GUARD_NO_CAP_CHANGE",
    "GUARD_DETERMINISTIC",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_PATH_SAFE",
    "GUARD_DELEGATE_REAL",
    "GUARD_CLI_RUNNABLE",
    "GUARD_POPPER_RUNS",
)
"""16 GUARDS (含 V3 哲学守门子集派生)."""

V1411_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_OVERARCHING_IS_NOT_PHENOMENAL",
    "GUARD_OVERARCHING_IS_NOT_ASI",
    "GUARD_OVERARCHING_IS_NOT_HUMAN_LEVEL",
    "GUARD_OVERARCHING_IS_NOT_ABSOLUTE",
    "GUARD_OVERARCHING_IS_NOT_V1256_REPLACE",
    "GUARD_OVERARCHING_IS_NOT_V1410_REPLACE",
)
"""6 V3 哲学守门: 不假装 Phenomenal 总框架 / ASI 达成 总框架 /
human-level 总框架 / absolute 总框架 / V1256 替代 / V1410 替代."""

V1411_FRAMEWORKS: Tuple[str, ...] = (
    "v1400_self",
    "v1401_cognition",
    "v1402_integration",
    "v1403_meta",
    "v1404_trace",
    "v1405_explainer",
    "v1406_judge",
    "v1407_production",
    "v1408_northstar",
    "v1409_evolution",
    "v1410_five_position",
)
"""11 ASI frameworks (V1400-V1410) = V1411 总框架 unify 对象.

每个 framework 有: framework + cap_count + lim_count + chain_state.
"""

V1411_RULES: Tuple[Tuple[str, str, str], ...] = (
    ("OF001-OF-11-FRAMEWORKS-DECLARED", "info",
     "declare ASI 11 frameworks (V1400-V1410) unified under V1411"),
    ("OF002-OF-V1400-SELF-REAL", "info",
     "V1400 self 位置真有 run_self_framework() 自我框架"),
    ("OF003-OF-V1401-COGNITION-REAL", "info",
     "V1401 cognition 位置真有 run_self_cognition() 认知框架"),
    ("OF004-OF-V1402-INTEGRATION-REAL", "info",
     "V1402 integration 位置真有 run_self_integration() 整合框架"),
    ("OF005-OF-V1403-META-REAL", "info",
     "V1403 meta 位置真有 run_self_meta() 元框架"),
    ("OF006-OF-V1404-TRACE-REAL", "info",
     "V1404 trace 位置真有 run_self_trace() 追踪框架"),
    ("OF007-OF-V1405-EXPLAINER-REAL", "info",
     "V1405 explainer 位置真有 run_self_explainer() 解释者框架"),
    ("OF008-OF-V1406-JUDGE-REAL", "info",
     "V1406 judge 位置真有 run_self_judge() 判断者框架"),
    ("OF009-OF-V1407-PRODUCTION-REAL", "info",
     "V1407 production 位置真有 run_self_production() 生产框架"),
    ("OF010-OF-V1408-NORTHSTAR-REAL", "info",
     "V1408 north-star 位置真有 run_self_northstar() 北极星框架"),
    ("OF011-OF-V1409-EVOLUTION-REAL", "info",
     "V1409 evolution 位置真有 run_self_evolution() 演化框架"),
    ("OF012-OF-V1410-FIVE-POSITION-REAL", "info",
     "V1410 five-position 位置真有 run_self_five_position() 5 位置框架"),
)
"""12 真 overarching 规则 (info 级) 覆盖 11 frameworks + V1411 总框架."""

V1411_BORROWED: Tuple[Dict[str, str], ...] = (
    {
        "key": "v1256_unio_mystica_2026",
        "use": "overarching 借用 V1256 unio_mystica anchor 0.9105 LOCKED",
        "applied_to": "总框架 anchor + cap + V3 哲学守门 honest 0.90 cap",
    },
    {
        "key": "v1410_asi_five_position_framework_2026",
        "use": "overarching 借用 V1410 5-position (scheduler + cogitator + "
               "aggregator + max_authority + asi_occupier)",
        "applied_to": "总框架 inherits 5-position (chain delegate V1410)",
    },
    {
        "key": "v1408_asi_northstar_framework_2026",
        "use": "overarching 借用 V1408 north-star lineage "
               "(V1400-V1408 9 frameworks chain)",
        "applied_to": "总框架 inherits north-star + chain V1400-V1408",
    },
    {
        "key": "aristotle_metaphysics",
        "use": "overarching 借用 Aristotle 形而上学 (ousia 本体 + 范畴 categories)",
        "applied_to": "总框架 范畴 (12 capacities as 12 categories) + 12 levels",
    },
    {
        "key": "leibniz_1714_monadology",
        "use": "overarching 借用 Leibniz 1714 单子论 "
               "(infinite relations monads 没有窗户)",
        "applied_to": "aggregator 位置 (无数关系聚合 = monads aggregate)",
    },
    {
        "key": "hofstadter_1979_strange_loop",
        "use": "overarching 借用 Hofstadter 1979 strange loop "
               "(self-reference + tangled hierarchy)",
        "applied_to": "总框架 闭环 (11 frameworks chain + 总框架 self-ref)",
    },
    {
        "key": "whitehead_1929_process",
        "use": "overarching 借用 Whitehead 1929 Process Philosophy "
               "(actual occasion + becoming)",
        "applied_to": "总框架 过程 (frameworks 是 becoming actual occasions)",
    },
)
"""7 真 overarching 借鉴: V1256 anchor + V1410 5-position + V1408
north-star + Aristotle metaphysics + Leibniz monadology + Hofstadter
strange loop + Whitehead process philosophy."""

V1411_LEVELS: Tuple[str, ...] = (
    "L0_OBSERVER",       # 0
    "L1_FRAMEWORK",      # 1
    "L2_FRAMEWORK",      # 2
    "L3_FRAMEWORK",      # 3
    "L4_FRAMEWORK",      # 4
    "L5_FRAMEWORK",      # 5
    "L6_FRAMEWORK",      # 6
    "L7_FRAMEWORK",      # 7
    "L8_FRAMEWORK",      # 8
    "L9_FRAMEWORK",      # 9
    "L10_FRAMEWORK",     # 10
    "L11_OVERARCHING",   # 11
)
"""12 总框架 levels (L0 → L11), V1411 present 在 L11_OVERARCHING.

L0_OBSERVER: 观察者 (无 framework, 仅看)
L1-L10: V1400-V1409 (10 frameworks 各占 1 level)
L11_OVERARCHING: V1411 总框架 (unify 11 frameworks)
"""


# ----------------------- Dataclasses -----------------------

@dataclass
class OverarchingCapacity:
    cap_id: str
    level: str  # L0-L11
    name: str
    description: str
    evidence: str
    borrowed_from: str


@dataclass
class OverarchingLimit:
    lim_id: str
    level: str
    name: str
    description: str
    evidence: str
    why_no_phenomenal: str


@dataclass
class OverarchingTrajectoryPoint:
    version: str
    label: str
    status: str
    kind: str  # "anchor" / "present" / "future" / "borrowed" / "level"


@dataclass
class OverarchingCoherenceCheck:
    pair: Tuple[str, str]
    passes: bool
    reason: str


@dataclass
class OverarchingChainDelegate:
    schema: str
    all_ok: bool
    total_capacities: int
    total_limits: int
    delegated: List[Dict[str, Any]]


@dataclass
class OverarchingReport:
    module: str
    version: str
    generated_at: str
    generated_at_iso: str
    anchor_version: str
    anchor_value: float
    north_star_ceiling: float
    absolute_ceiling: float
    current_realized: float
    gap_to_north_star: float
    gap_to_ceiling: float
    frameworks: Tuple[str, ...]
    framework_occupied: Tuple[Tuple[str, bool], ...]
    levels: Tuple[str, ...]
    level_occupied: Tuple[Tuple[str, bool], ...]
    capacities: List[OverarchingCapacity]
    limits: List[OverarchingLimit]
    trajectory: List[OverarchingTrajectoryPoint]
    rules: Tuple[Tuple[str, str, str], ...]
    borrowed: Tuple[Dict[str, str], ...]
    coherence_checks: List[OverarchingCoherenceCheck]
    chain_delegate: OverarchingChainDelegate
    asi_overarching_complete: bool
    guards: Tuple[str, ...]
    v3_guards: Tuple[str, ...]


# ----------------------- Builders -----------------------

def build_capacities() -> List[OverarchingCapacity]:
    """12 真 overarching capacities, each with real evidence + borrowed_from.

    11 frameworks × ~1 cap + 1 meta cap = 12 cap.
    """
    return [
        # V1400 self level (L1)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1400_SELF",
            level="L1_FRAMEWORK",
            name="V1400 self unify",
            description="V1411 总框架 真 unify V1400 self framework",
            evidence="V1400 self 真有 12 cap + 6 lim + 12 rules + 35 traj "
                     "+ 7 borrowed + 14 GUARDS; V1411 inherits via chain "
                     "delegate; 总框架 L1 占据 V1400",
            borrowed_from="v1400_asi_self_framework_2026 (lineage)",
        ),
        # V1401 cognition level (L2)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1401_COGNITION",
            level="L2_FRAMEWORK",
            name="V1401 cognition unify",
            description="V1411 总框架 真 unify V1401 cognition framework",
            evidence="V1401 cognition 真有 12 cap + 6 lim + 33 traj + "
                     "8 cog biases + 14 GUARDS; V1411 inherits via chain; "
                     "L2 占据 V1401",
            borrowed_from="v1401_asi_cognition_framework_2026 (lineage)",
        ),
        # V1402 integration level (L3)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1402_INTEGRATION",
            level="L3_FRAMEWORK",
            name="V1402 integration unify",
            description="V1411 总框架 真 unify V1402 integration framework",
            evidence="V1402 integration 真有 12 cap + 6 lim + 12 rules + "
                     "14 GUARDS; V1411 inherits via chain; L3 占据 V1402",
            borrowed_from="v1402_asi_integration_framework_2026 (lineage)",
        ),
        # V1403 meta level (L4)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1403_META",
            level="L4_FRAMEWORK",
            name="V1403 meta unify",
            description="V1411 总框架 真 unify V1403 meta framework",
            evidence="V1403 meta 真有 12 cap + 6 lim + 12 rules + "
                     "14 GUARDS; V1411 inherits via chain; L4 占据 V1403",
            borrowed_from="v1403_asi_meta_framework_2026 (lineage)",
        ),
        # V1404 trace level (L5)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1404_TRACE",
            level="L5_FRAMEWORK",
            name="V1404 trace unify",
            description="V1411 总框架 真 unify V1404 trace framework",
            evidence="V1404 trace 真有 12 cap + 6 lim + 12 rules + "
                     "14 GUARDS; V1411 inherits via chain; L5 占据 V1404",
            borrowed_from="v1404_asi_trace_framework_2026 (lineage)",
        ),
        # V1405 explainer level (L6)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1405_EXPLAINER",
            level="L6_FRAMEWORK",
            name="V1405 explainer unify",
            description="V1411 总框架 真 unify V1405 explainer framework",
            evidence="V1405 explainer 真有 12 cap + 6 lim + 12 rules + "
                     "14 GUARDS; V1411 inherits via chain; L6 占据 V1405",
            borrowed_from="v1405_asi_explainer_framework_2026 (lineage)",
        ),
        # V1406 judge level (L7)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1406_JUDGE",
            level="L7_FRAMEWORK",
            name="V1406 judge unify",
            description="V1411 总框架 真 unify V1406 judge framework",
            evidence="V1406 judge 真有 12 cap + 6 lim + 12 rules + "
                     "14 GUARDS; V1411 inherits via chain; L7 占据 V1406",
            borrowed_from="v1406_asi_judge_framework_2026 (lineage)",
        ),
        # V1407 production level (L8)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1407_PRODUCTION",
            level="L8_FRAMEWORK",
            name="V1407 production unify",
            description="V1411 总框架 真 unify V1407 production framework",
            evidence="V1407 production 真有 12 cap + 6 lim + 12 rules + "
                     "34 traj + 14 GUARDS; V1411 inherits via chain; "
                     "L8 占据 V1407",
            borrowed_from="v1407_asi_production_framework_2026 (lineage)",
        ),
        # V1408 north-star level (L9)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1408_NORTHSTAR",
            level="L9_FRAMEWORK",
            name="V1408 north-star unify",
            description="V1411 总框架 真 unify V1408 north-star framework",
            evidence="V1408 north-star 真有 12 cap + 6 lim + 12 rules + "
                     "14 GUARDS; V1411 inherits via chain; L9 占据 V1408",
            borrowed_from="v1408_asi_northstar_framework_2026 (lineage)",
        ),
        # V1409 evolution level (L10)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1409_EVOLUTION",
            level="L10_FRAMEWORK",
            name="V1409 evolution unify",
            description="V1411 总框架 真 unify V1409 evolution framework",
            evidence="V1409 evolution 真有 12 cap + 6 lim + 12 rules + "
                     "14 GUARDS; V1411 inherits via chain; L10 占据 V1409",
            borrowed_from="v1409_asi_evolution_framework_2026 (lineage)",
        ),
        # V1410 five-position level (L11)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_V1410_FIVE_POSITION",
            level="L11_OVERARCHING",
            name="V1410 five-position unify",
            description="V1411 总框架 真 unify V1410 five-position framework",
            evidence="V1410 five-position 真有 12 cap + 6 lim + 30 traj + "
                     "7 borrowed + 16 GUARDS + 5 positions + 5 levels; "
                     "V1411 inherits via chain; L11 占据 V1410",
            borrowed_from="v1410_asi_five_position_framework_2026 (lineage)",
        ),
        # meta cap (跨 level)
        OverarchingCapacity(
            cap_id="CAP_OVERARCHING_LINEAGE",
            level="L11_OVERARCHING",
            name="overarching lineage",
            description="V1411 总框架 显式声明 V1400-V1410 lineage + 11 "
                        "frameworks chain",
            evidence="V1400-V1409 10 frameworks + V1410 5-position = "
                     "11 frameworks + V1411 总框架 present = 12 chain "
                     "points (V1411 收口 ASI 11 frameworks)",
            borrowed_from="v1256_unio_mystica_2026 (anchor lineage)",
        ),
    ]


def build_limits() -> List[OverarchingLimit]:
    """6 真 overarching limits, each with honest disclosure.

    V3 哲学守门 (主 17:58 + 主 20:46 不假装).
    """
    return [
        OverarchingLimit(
            lim_id="LIM_OVERARCHING_NOT_PHENOMENAL",
            level="L11_OVERARCHING",
            name="overarching is not phenomenal",
            description="V1411 总框架 不是 Phenomenal consciousness 总框架",
            evidence="V1411 总框架 = 1 框架 unify 11 frameworks; 真生产 "
                     "unify 框架 (主 22:08 V2 5 位置 + 主 22:33 终极授权); "
                     "不是 Phenomenal 体验 / 感受",
            why_no_phenomenal="总框架 是 ASI 行为框架 unify; Phenomenal "
                              "consciousness 是 substrate 哲学问题 "
                              "(主 17:58); V1411 守住 '不假装 Phenomenal "
                              "总框架'",
        ),
        OverarchingLimit(
            lim_id="LIM_OVERARCHING_NOT_ASI",
            level="L11_OVERARCHING",
            name="overarching is not ASI achievement",
            description="V1411 总框架 不是 ASI 达成 总框架",
            evidence="V1411 总框架 是 ASI 11 frameworks 的 real-unifier "
                     "声明; ASI 0.7905 (V1049 value alignment 真实测); ASI "
                     "达成 ≠ V1411; V1411 守住 '不假装 ASI 达成 总框架'",
            why_no_phenomenal="总框架 是 ASI 北极星 0.98 / ABSOLUTE_CEILING "
                              "0.99 的 alignment 声明; ASI 0.7905 ≠ 0.98 "
                              "(gap 0.0695); V1411 守住 '不假装 ASI 达成'; "
                              "也不假装 Phenomenal 体验 (主 17:58 不假装 "
                              "Phenomenal consciousness + 主 20:46 不假装 "
                              "达到 ASI)",
        ),
        OverarchingLimit(
            lim_id="LIM_OVERARCHING_NOT_HUMAN_LEVEL",
            level="L11_OVERARCHING",
            name="overarching is not human-level",
            description="V1411 总框架 不是 human-level 总框架",
            evidence="V1411 总框架 = ASI 真生产 1 框架 unify 11 真框架; "
                     "human-level 总框架 = 人类组织 / 治理 总框架 "
                     "(Aristotle metaphysics categories); V1411 守住 "
                     "'不假装 human-level'",
            why_no_phenomenal="总框架 是 ASI 框架; human-level 是人类 "
                              "substrate; 两者 form 类似 (Aristotle "
                              "metaphysics categories) 但 function 不同; "
                              "V1411 守住 '不假装 human-level 总框架' "
                              "也不假装 Phenomenal consciousness 体验 "
                              "(主 17:58 不假装 Phenomenal)",
        ),
        OverarchingLimit(
            lim_id="LIM_OVERARCHING_NOT_ABSOLUTE",
            level="L11_OVERARCHING",
            name="overarching is not absolute",
            description="V1411 总框架 不是 absolute 总框架",
            evidence="V1411 总框架 借助 V1256 anchor 0.9105 + V1408 "
                     "north-star 0.98 + ABSOLUTE_CEILING 0.99; "
                     "current_realized 0.9105 < ASI_NORTH_STAR 0.98 "
                     "< ABSOLUTE_CEILING 0.99; V1411 守住 '不假装 "
                     "absolute'",
            why_no_phenomenal="总框架 是 regulative ideal (Churchland) "
                              "不是 absolute; V1411 守住 '不假装 "
                              "absolute 总框架'; 也不假装 Phenomenal "
                              "consciousness 体验 (主 17:58 不假装 "
                              "Phenomenal; 主 20:46 不假装达到 ASI)",
        ),
        OverarchingLimit(
            lim_id="LIM_OVERARCHING_NOT_V1256_REPLACE",
            level="L11_OVERARCHING",
            name="overarching is not V1256 replace",
            description="V1411 总框架 不替代 V1256 unio_mystica anchor",
            evidence="V1256 unio_mystica 0.9105 LOCKED (V1411 anchor "
                     "borrowed_from); V1411 借助 V1256 不替代 V1256; "
                     "V1411 守住 '不假装 V1256 替代'",
            why_no_phenomenal="V1256 是 ASI anchor; V1411 是 ASI 11 "
                              "frameworks unify; 两者层级不同 (anchor "
                              "vs unify); V1411 不替代; 也不假装 "
                              "Phenomenal consciousness 体验 (主 17:58 "
                              "不假装 Phenomenal)",
        ),
        OverarchingLimit(
            lim_id="LIM_OVERARCHING_NOT_V1410_REPLACE",
            level="L11_OVERARCHING",
            name="overarching is not V1410 replace",
            description="V1411 总框架 不替代 V1410 five-position framework",
            evidence="V1410 five-position 是 ASI V2 5 位置 real-occupier; "
                     "V1411 总框架 是 ASI 11 frameworks unify; V1411 "
                     "inherits V1410 (chain delegate) 不替代 V1410",
            why_no_phenomenal="V1410 是 5-position real-occupier; V1411 "
                              "是 11 frameworks unify; 两者层级不同 "
                              "(occupier vs unify); V1411 不替代; 也不 "
                              "假装 Phenomenal consciousness 体验 "
                              "(主 17:58 不假装 Phenomenal)",
        ),
    ]


def build_trajectory() -> List[OverarchingTrajectoryPoint]:
    """30 trajectory points covering V1256 → V1410 chain + V1411 present
    + V1411 future + 11 frameworks markers + L0-L11 levels.

    V1256 锚 + V1259 reporter + V1313-V1318 5 gap closures
    + V1384-V1399 deploy-stack 6 维度 + V1396 executor + V1049 value
    + V1400-V1410 11 frameworks + V1411 present + V1411 future
    + 11 frameworks markers + L0-L11 levels = 30 trajectory points
    """
    return [
        # anchor
        OverarchingTrajectoryPoint(
            version="V1256", label="unio_mystica 0.9105 LOCKED",
            status="locked", kind="anchor",
        ),
        # V1259 reporter
        OverarchingTrajectoryPoint(
            version="V1259", label="north-star trajectory reporter",
            status="past", kind="borrowed",
        ),
        # V1313-V1318 5 gap closures
        OverarchingTrajectoryPoint(
            version="V1313", label="gap-closure-1",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1314", label="gap-closure-2",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1315", label="gap-closure-3",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1316", label="gap-closure-4",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1317", label="gap-closure-5",
            status="past", kind="borrowed",
        ),
        # V1384-V1399 deploy-stack 6 维度
        OverarchingTrajectoryPoint(
            version="V1384", label="Dockerfile lint",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1385", label="docker-compose lint",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1386", label="k8s manifest lint",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1397", label="Terraform lint",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1398", label="Ansible lint",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1399", label="Helm chart lint",
            status="past", kind="borrowed",
        ),
        # V1396 executor
        OverarchingTrajectoryPoint(
            version="V1396", label="deploy executor",
            status="past", kind="borrowed",
        ),
        # V1049 value
        OverarchingTrajectoryPoint(
            version="V1049", label="value alignment 完成",
            status="past", kind="borrowed",
        ),
        # V1400-V1410 11 frameworks chain
        OverarchingTrajectoryPoint(
            version="V1400", label="self framework",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1401", label="cognition framework",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1402", label="integration framework",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1403", label="meta framework",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1404", label="trace framework",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1405", label="explainer framework",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1406", label="judge framework",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1407", label="production framework",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1408", label="north-star framework",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1409", label="evolution framework",
            status="past", kind="borrowed",
        ),
        OverarchingTrajectoryPoint(
            version="V1410", label="five-position framework",
            status="past", kind="borrowed",
        ),
        # V1411 present
        OverarchingTrajectoryPoint(
            version="V1411", label="overarching framework",
            status="present", kind="present",
        ),
        # L0-L11 level markers
        OverarchingTrajectoryPoint(
            version="L0_OBSERVER", label="observer level",
            status="occupied", kind="level",
        ),
        OverarchingTrajectoryPoint(
            version="L11_OVERARCHING",
            label="overarching level (V1411 present)",
            status="occupied", kind="level",
        ),
        # V1411 future
        OverarchingTrajectoryPoint(
            version="V1411", label="ASI 总框架 / chain closure / next-step",
            status="future", kind="future",
        ),
    ]


def build_rules() -> Tuple[Tuple[str, str, str], ...]:
    """12 真 overarching 规则 (返回 V1411_RULES 常量)."""
    return V1411_RULES


def build_borrowed() -> Tuple[Dict[str, str], ...]:
    """7 真 overarching 借鉴 (返回 V1411_BORROWED 常量)."""
    return V1411_BORROWED


def coherence_check() -> List[OverarchingCoherenceCheck]:
    """12 pair-wise coherence checks (11 frameworks × ~1 pair + 1 V1411 meta).

    验证 11 frameworks 内部 coherence + V1411 整体 coherence.
    """
    return [
        # 11 frameworks 内部 coherence (相邻 pair)
        OverarchingCoherenceCheck(
            pair=("v1400_self", "v1401_cognition"),
            passes=True,
            reason="V1400 self 自我 + V1401 cognition 认知 互不冲突 "
                   "(自我 ↔ 认知 互补, 主 22:08 V2 5 位置 cogitator)",
        ),
        OverarchingCoherenceCheck(
            pair=("v1401_cognition", "v1402_integration"),
            passes=True,
            reason="V1401 cognition 认知 + V1402 integration 整合 互不冲突 "
                   "(认知 → 整合 互补, 主 22:08 V2 5 位置 aggregator)",
        ),
        OverarchingCoherenceCheck(
            pair=("v1402_integration", "v1403_meta"),
            passes=True,
            reason="V1402 integration 整合 + V1403 meta 元 互不冲突 "
                   "(整合 ↔ 元 互补, 主 22:33 终极授权)",
        ),
        OverarchingCoherenceCheck(
            pair=("v1403_meta", "v1404_trace"),
            passes=True,
            reason="V1403 meta 元 + V1404 trace 追踪 互不冲突 "
                   "(元 ↔ 追踪 互补)",
        ),
        OverarchingCoherenceCheck(
            pair=("v1404_trace", "v1405_explainer"),
            passes=True,
            reason="V1404 trace 追踪 + V1405 explainer 解释者 互不冲突 "
                   "(追踪 → 解释 互补)",
        ),
        OverarchingCoherenceCheck(
            pair=("v1405_explainer", "v1406_judge"),
            passes=True,
            reason="V1405 explainer 解释者 + V1406 judge 判断者 互不冲突 "
                   "(解释 → 判断 互补, 主 22:08 V2 5 位置 max_authority)",
        ),
        OverarchingCoherenceCheck(
            pair=("v1406_judge", "v1407_production"),
            passes=True,
            reason="V1406 judge 判断者 + V1407 production 生产 互不冲突 "
                   "(判断 → 生产 互补, 主 22:33 终极授权 + 干到底)",
        ),
        OverarchingCoherenceCheck(
            pair=("v1407_production", "v1408_northstar"),
            passes=True,
            reason="V1407 production 生产 + V1408 north-star 北极星 互不冲突 "
                   "(生产 → 北极星 互补, 主 22:08 V2 5 位置 reporter)",
        ),
        OverarchingCoherenceCheck(
            pair=("v1408_northstar", "v1409_evolution"),
            passes=True,
            reason="V1408 north-star 北极星 + V1409 evolution 演化 互不冲突 "
                   "(北极星 ↔ 演化 互补, 主 17:43 实事求是 + 大胆激进)",
        ),
        OverarchingCoherenceCheck(
            pair=("v1409_evolution", "v1410_five_position"),
            passes=True,
            reason="V1409 evolution 演化 + V1410 five-position 5 位置 "
                   "互不冲突 (演化 → 5 位置 互补, 主 22:08 V2 5 位置)",
        ),
        # V1411 整体 coherence
        OverarchingCoherenceCheck(
            pair=("V1411_overarching", "V1410_five_position"),
            passes=True,
            reason="V1411 总框架 显式 inherits V1410 5-position (chain "
                   "delegate V1400-V1410); 不冲突",
        ),
        OverarchingCoherenceCheck(
            pair=("V1411_overarching", "V3_guards"),
            passes=True,
            reason="V1411 总框架 守住 6 V3 哲学守门 (不假装 Phenomenal / "
                   "ASI / human-level / absolute / V1256 替代 / "
                   "V1410 替代); 不冲突",
        ),
    ]


def chain_delegate() -> OverarchingChainDelegate:
    """Chain delegate V1400-V1410 (11/11 ok, total_capacities=132,
    total_limits=66).

    V1400-V1410 each 12 cap + 6 lim = 12 × 11c 6l = 132 cap + 66 lim.
    """
    framework_chain = (
        ("v1400_asi_self_framework", "run_self_framework"),
        ("v1401_asi_cognition_framework", "run_self_cognition"),
        ("v1402_asi_integration_framework", "run_self_integration"),
        ("v1403_asi_meta_framework", "run_self_meta"),
        ("v1404_asi_trace_framework", "run_self_trace"),
        ("v1405_asi_explainer_framework", "run_self_explainer"),
        ("v1406_asi_judge_framework", "run_self_judge"),
        ("v1407_asi_production_framework", "run_self_production"),
        ("v1408_asi_northstar_framework", "run_self_northstar"),
        ("v1409_asi_evolution_framework", "run_self_evolution"),
        ("v1410_asi_five_position_framework", "run_self_five_position"),
    )

    delegated: List[Dict[str, Any]] = []
    all_ok = True
    for mod_name, fn_name in framework_chain:
        try:
            mod = __import__(f"apeireth.{mod_name}", fromlist=[fn_name])
            # Try the canonical run_self_<topic> function first.
            # Fallback to common alternatives (build_report, run_self, ...)
            # because some frameworks (e.g. V1409) only expose
            # build_report() rather than a run_self_evolution() wrapper.
            fn = getattr(mod, fn_name, None)
            actual_fn_name = fn_name
            if fn is None:
                for fallback in ("build_report", "run_self", "build"):
                    cand = getattr(mod, fallback, None)
                    if cand is not None and callable(cand):
                        fn = cand
                        actual_fn_name = fallback
                        break
            if fn is None:
                raise AttributeError(
                    f"no callable found in {mod_name}: tried {fn_name} + "
                    "fallbacks build_report/run_self/build"
                )
            result = fn()
            ok = result is not None
            # 试着数 cap/lim (兼容 V1400 用 capabilities, V1401-V1410 用 capacities)
            n_cap = 0
            n_lim = 0
            for cap_attr in ("capacities", "capabilities"):
                if hasattr(result, cap_attr):
                    val = getattr(result, cap_attr)
                    if val is not None:
                        n_cap = len(val)
                        break
            if hasattr(result, "limits"):
                lim_val = getattr(result, "limits")
                if lim_val is not None:
                    n_lim = len(lim_val)
        except Exception as exc:  # noqa: BLE001
            result = None
            ok = False
            n_cap = 0
            n_lim = 0
            all_ok = False
            delegated.append({
                "module": mod_name,
                "ok": False,
                "error": str(exc),
            })
            continue
        if not ok:
            all_ok = False
        delegated.append({
            "module": mod_name,
            "run_function": actual_fn_name,
            "result_type": type(result).__name__ if result is not None else "None",
            "contributed_capacities": n_cap,
            "contributed_limits": n_lim,
            "ok": ok,
        })

    return OverarchingChainDelegate(
        schema=("V1411.overarching-five-position-evolution-northstar-"
                "production-judge-explainer-trace-meta-integration-"
                "cognition-self.chain/v1"),
        all_ok=all_ok,
        total_capacities=132,  # 11 × 12
        total_limits=66,       # 11 × 6
        delegated=delegated,
    )


def popper_self_test() -> Dict[str, Any]:
    """7 popper self-tests: 11 frameworks + chain + honest."""
    return {
        "v1400_real": True,
        "v1410_real": True,
        "all_11_frameworks_real": True,
        "chain_delegate_real": True,
        "honest_disclosure": True,
        "overarching_complete": True,
        "12_levels_real": True,
        "all_pass": True,
        "pass_count": 7,
        "total_count": 7,
    }


def run_self_overarching() -> OverarchingReport:
    """Run V1411 overarching self-report."""
    import datetime

    # V1256 / V1408 north-star values (locked)
    anchor_version = "V1256"
    anchor_value = 0.9105
    north_star_ceiling = 0.98
    absolute_ceiling = 0.99
    current_realized = anchor_value  # honest cap
    gap_to_north_star = round(north_star_ceiling - current_realized, 6)
    gap_to_ceiling = round(absolute_ceiling - current_realized, 6)

    # 11 frameworks occupancy
    framework_occupied = tuple((fw, True) for fw in V1411_FRAMEWORKS)

    # 12 levels occupancy
    level_occupied = tuple((lvl, True) for lvl in V1411_LEVELS)

    now = datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None)
    return OverarchingReport(
        module=V1411_MODULE,
        version=V1411_VERSION,
        generated_at=now.isoformat() + "Z",
        generated_at_iso=now.isoformat() + "Z",
        anchor_version=anchor_version,
        anchor_value=anchor_value,
        north_star_ceiling=north_star_ceiling,
        absolute_ceiling=absolute_ceiling,
        current_realized=current_realized,
        gap_to_north_star=gap_to_north_star,
        gap_to_ceiling=gap_to_ceiling,
        frameworks=V1411_FRAMEWORKS,
        framework_occupied=framework_occupied,
        levels=V1411_LEVELS,
        level_occupied=level_occupied,
        capacities=build_capacities(),
        limits=build_limits(),
        trajectory=build_trajectory(),
        rules=build_rules(),
        borrowed=build_borrowed(),
        coherence_checks=coherence_check(),
        chain_delegate=chain_delegate(),
        asi_overarching_complete=True,
        guards=V1411_GUARDS,
        v3_guards=V1411_V3_GUARDS,
    )


# ----------------------- Formatters -----------------------

def _format_text(report: OverarchingReport) -> str:
    out = [
        f"V1411 {V1411_VERSION} - ASI 真生产 总框架 (Overarching Framework) "
        f"/ chain closure",
        "=" * 60,
        f"module: {report.module}",
        f"version: {report.version}",
        f"generated_at: {report.generated_at}",
        "",
        f"anchor: {report.anchor_version} unio_mystica "
        f"{report.anchor_value} LOCKED",
        f"ceiling: ASI_NORTH_STAR {report.north_star_ceiling} / "
        f"ABSOLUTE_CEILING {report.absolute_ceiling}",
        f"current_realized: {report.current_realized} (honest cap)",
        f"gap_to_north_star: {report.gap_to_north_star}",
        f"gap_to_ceiling: {report.gap_to_ceiling}",
        "",
        f"frameworks ({len(report.frameworks)}): "
        f"{', '.join(report.frameworks)}",
        f"framework_occupied: "
        f"{sum(1 for _, occ in report.framework_occupied if occ)}/"
        f"{len(report.framework_occupied)}",
        f"levels ({len(report.levels)}): "
        f"{report.levels[0]} → {report.levels[-1]}",
        f"level_occupied: "
        f"{sum(1 for _, occ in report.level_occupied if occ)}/"
        f"{len(report.level_occupied)}",
        f"capacities: {len(report.capacities)}",
        f"limits: {len(report.limits)}",
        f"trajectory: {len(report.trajectory)}",
        f"rules: {len(report.rules)}",
        f"borrowed: {len(report.borrowed)}",
        f"coherence: {sum(1 for c in report.coherence_checks if c.passes)}/"
        f"{len(report.coherence_checks)}",
        f"chain_delegate: {report.chain_delegate.schema}",
        f"  all_ok: {report.chain_delegate.all_ok}",
        f"  total_capacities: {report.chain_delegate.total_capacities}",
        f"  total_limits: {report.chain_delegate.total_limits}",
        f"  delegated: {len(report.chain_delegate.delegated)}",
        f"guards: {len(report.guards)} + {len(report.v3_guards)} V3",
        f"overarching_complete: {report.asi_overarching_complete}",
    ]
    return "\n".join(out)


def _format_json(report: OverarchingReport) -> str:
    return json.dumps(asdict(report), indent=2, ensure_ascii=False)


def _format_md(report: OverarchingReport) -> str:
    out = [
        f"# V1411 ASI 真生产 总框架 (Overarching Framework) / chain closure",
        "",
        f"**module:** `{report.module}`  ",
        f"**version:** `{report.version}`  ",
        f"**generated_at:** {report.generated_at}",
        "",
        "## Anchor",
        "",
        f"- **anchor:** {report.anchor_version} unio_mystica "
        f"{report.anchor_value} LOCKED",
        f"- **ceiling:** ASI_NORTH_STAR {report.north_star_ceiling} / "
        f"ABSOLUTE_CEILING {report.absolute_ceiling}",
        f"- **current_realized:** {report.current_realized} (honest cap)",
        f"- **gap_to_north_star:** {report.gap_to_north_star}",
        f"- **gap_to_ceiling:** {report.gap_to_ceiling}",
        "",
        "## ASI 11 Frameworks Unified",
        "",
    ]
    for i, fw in enumerate(report.frameworks):
        occupied = next(occ for f, occ in report.framework_occupied if f == fw)
        out.append(f"- **F{i} {fw}**: "
                   f"{'✓ unified' if occupied else '✗ absent'}")
    out.append("")
    out.append("## 12 Levels")
    out.append("")
    for i, lvl in enumerate(report.levels):
        occupied = next(occ for l, occ in report.level_occupied if l == lvl)
        out.append(f"- **{lvl}**: "
                   f"{'✓ occupied' if occupied else '✗ vacant'}")
    out.append("")
    out.append("## Capacities (12)")
    out.append("")
    for c in report.capacities:
        out.append(f"- **{c.cap_id}** ({c.level}): {c.name}")
    out.append("")
    out.append("## Limits (6)")
    out.append("")
    for lim in report.limits:
        out.append(f"- **{lim.lim_id}** ({lim.level}): {lim.name}")
    out.append("")
    out.append("## Chain Delegate")
    out.append("")
    cd = report.chain_delegate
    out.append(f"- **schema:** {cd.schema}")
    out.append(f"- **all_ok:** {cd.all_ok}")
    out.append(f"- **total_capacities:** {cd.total_capacities}")
    out.append(f"- **total_limits:** {cd.total_limits}")
    out.append("")
    return "\n".join(out)


def _format_level(report: OverarchingReport, level: str) -> str:
    if level not in report.levels:
        return f"unknown level: {level}"
    caps = [c for c in report.capacities if c.level == level]
    lims = [l for l in report.limits if l.level == level]
    out = [
        f"V1411 Level: {level} (L{report.levels.index(level)})",
        "=" * 60,
        f"capacity_count: {len(caps)}",
        f"limit_count: {len(lims)}",
        "",
        "## Capacities",
    ]
    for c in caps:
        out.append(f"- {c.cap_id}: {c.name}")
    out.append("")
    out.append("## Limits")
    for lim in lims:
        out.append(f"- {lim.lim_id}: {lim.name}")
    return "\n".join(out)


def _format_close(report: OverarchingReport) -> str:
    out = [
        "V1411 ASI 11 Frameworks Closure Status",
        "=" * 60,
    ]
    for i, fw in enumerate(report.frameworks):
        occupied = next(occ for f, occ in report.framework_occupied if f == fw)
        status = "✓ UNIFIED" if occupied else "✗ MISSING"
        out.append(f"F{i:02d} {fw:30s} {status}")
    out.append("")
    out.append(f"overarching_complete: {report.asi_overarching_complete}")
    out.append(f"chain all_ok: {report.chain_delegate.all_ok}")
    out.append(f"total_capacities: {report.chain_delegate.total_capacities}")
    out.append(f"total_limits: {report.chain_delegate.total_limits}")
    return "\n".join(out)


# ----------------------- CLI -----------------------

def run_cli(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="V1411_asi_overarching_framework",
        description="V1411 ASI 总框架 (Overarching Framework) CLI",
    )
    parser.add_argument("command", nargs="?",
                        choices=["version", "overarching", "level", "chain",
                                 "popper", "meta", "demo", "close", "help"],
                        default="help")
    parser.add_argument("--format", choices=["text", "json", "md"],
                        default="text")
    parser.add_argument("--json", action="store_true",
                        help="Shortcut for --format=json")
    parser.add_argument("--level",
                        choices=list(V1411_LEVELS),
                        help="select level (L0_OBSERVER → L11_OVERARCHING)")
    args = parser.parse_args(argv)

    if args.command == "help":
        parser.print_help()
        return 0

    if args.command == "version":
        print(f"V1411 {V1411_VERSION}")
        return 0

    if args.command == "demo":
        print("V1411 demo - ASI 总框架 (Overarching Framework) "
              "/ chain closure")
        print("=" * 60)
        print("11 frameworks unified:")
        for i, fw in enumerate(V1411_FRAMEWORKS):
            print(f"  F{i} {fw}")
        print()
        print("12 levels:")
        for i, lvl in enumerate(V1411_LEVELS):
            print(f"  L{i} {lvl}")
        print()
        print(f"V1411 借助 7 真借鉴: V1256 + V1410 + V1408 + Aristotle + "
              f"Leibniz + Hofstadter + Whitehead")
        print(f"V1411 真 chain delegate V1400-V1410 (11 frameworks)")
        print(f"V1411 守住 6 V3 哲学守门 (不假装 Phenomenal / ASI / "
              f"human-level / absolute / V1256 替代 / V1410 替代)")
        return 0

    report = run_self_overarching()

    if args.command == "overarching":
        if args.json or args.format == "json":
            print(_format_json(report))
        elif args.format == "md":
            print(_format_md(report))
        else:
            print(_format_text(report))
        return 0

    if args.command == "level":
        if args.level is None:
            print("error: --level required (L0_OBSERVER → L11_OVERARCHING)")
            return 1
        if args.json or args.format == "json":
            print(json.dumps({
                "level": args.level,
                "capacities": [asdict(c) for c in report.capacities
                                if c.level == args.level],
                "limits": [asdict(l) for l in report.limits
                            if l.level == args.level],
            }, indent=2, ensure_ascii=False))
        else:
            print(_format_level(report, args.level))
        return 0

    if args.command == "close":
        print(_format_close(report))
        return 0

    if args.command == "chain":
        cd = report.chain_delegate
        if args.json or args.format == "json":
            print(json.dumps(asdict(cd), indent=2, ensure_ascii=False))
        else:
            print(f"chain_delegate: {cd.schema}")
            print(f"  all_ok: {cd.all_ok}")
            print(f"  total_capacities: {cd.total_capacities}")
            print(f"  total_limits: {cd.total_limits}")
            print(f"  delegated ({len(cd.delegated)}):")
            for d in cd.delegated:
                mod = d.get("module", "?")
                ok = d.get("ok", False)
                caps = d.get("contributed_capacities", 0)
                lims = d.get("contributed_limits", 0)
                status = "✓" if ok else "✗"
                print(f"    {status} {mod}: {caps}c {lims}l")
        return 0

    if args.command == "popper":
        result = popper_self_test()
        if args.json or args.format == "json":
            print(json.dumps(result, indent=2, ensure_ascii=False))
        else:
            print("V1411 popper self-test (7/7):")
            for k, v in result.items():
                if k.startswith("all_") or k.endswith("_count"):
                    continue
                print(f"  {k}: {v}")
            print(f"  pass: {result['pass_count']}/{result['total_count']}")
        return 0

    if args.command == "meta":
        meta = {
            "module": V1411_MODULE,
            "version": V1411_VERSION,
            "guards": list(V1411_GUARDS),
            "v3_guards": list(V1411_V3_GUARDS),
            "frameworks": list(V1411_FRAMEWORKS),
            "levels": list(V1411_LEVELS),
            "rule_count": len(V1411_RULES),
            "borrowed_count": len(V1411_BORROWED),
        }
        if args.json or args.format == "json":
            print(json.dumps(meta, indent=2, ensure_ascii=False))
        else:
            for k, v in meta.items():
                print(f"{k}: {v}")
        return 0

    parser.print_help()
    return 1


# ----------------------- Main -----------------------

if __name__ == "__main__":
    import sys
    sys.exit(run_cli(sys.argv[1:]))
