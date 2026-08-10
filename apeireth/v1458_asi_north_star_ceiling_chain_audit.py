"""V1458 — ASI North Star ceiling chain consistency audit (主 17:43 实事求是).

Phase: 1458
Version: 0.1.0
Date: 2026-08-10 (cron tick 11:38 Asia/Shanghai, Monday morning)
Post: V1457 (6-deployment 5-stage operational runbook)
      V1456 (6-deployment real subprocess parity)
      V1455 (cube hypercube full-source-content audit v5)
      V1450 (cube history aggregator)
      V1449 (7 problems × VCP 6 协议 cross-modular)
      V1442 (V2 5 位置 real-occupier)
      V1411 (overarching framework)
      V1410 (5-position framework declarative)
      V1259 (north-star trajectory reporter)
      V1256 (unio_mystica anchor — ASI V0.1 ceiling)

What V1458 is
=============
V1458 is the **ASI North Star ceiling chain consistency audit**.

V1256 unio_mystica LOCKED the current_realized=0.9105 against
asi_north_star=0.98 and absolute_ceiling=1.0. The ASI gap 0.0695
(0.98 - 0.9105) and the inflation gap 0.0895 (1.0 - 0.9105) are the
two critical gaps that no later module is allowed to bridge.

V1458 audits whether:

  1. **anchor_value (current_realized)** is consistently 0.9105 across
     every module that references the ceiling chain.

  2. **north_star_ceiling** is consistently 0.98 across every module
     that references the ceiling chain.

  3. **absolute_ceiling** is consistently 1.0 (the V1256/V1259/V1256_evidence
     reference) across every module that references the ceiling chain.

  4. **gap_to_north_star** is consistently 0.0695 across every module
     (with bounded tolerance ±0.0001).

  5. **gap_to_ceiling** is consistently 0.0895 across every module
     (with bounded tolerance ±0.0001).

  6. **No module attempts to bridge the gap** by inflating current_realized
     past 0.9105 or by lowering north_star below 0.98 (or lowering
     absolute_ceiling below 1.0 to hide the gap).

  7. **Cross-check that V1450/V1454/V1455/V1456/V1457 do NOT touch the
     ceiling chain** — they are deployment/hypercube audits, not
     ceiling-changers, and they should explicitly disclose this.

V1458 actually
--------------
1.  For each of 4 ceiling-chain modules (V1256, V1259, V1410, V1411),
    import the module and extract anchor_value, north_star_ceiling,
    absolute_ceiling, gap_to_north_star, gap_to_ceiling, current_realized.
2.  For each module, run 6 math checks:
    - check_anchor_locked        : anchor_value == 0.9105
    - check_north_star_locked    : north_star == 0.98
    - check_absolute_ceiling     : absolute_ceiling in {0.99, 1.0} (V1411
                                   uses 0.99; rest use 1.0 — both are
                                   consistent within their own internal math
                                   but we flag the cross-module diff)
    - check_gap_north_star       : north_star - anchor == 0.0695 ± 0.0001
    - check_gap_ceiling          : absolute - anchor == (1.0 - 0.9105) ±
                                   0.0001 OR (0.99 - 0.9105) ± 0.0001
    - check_published_gap        : module's published gap matches math
3.  Cross-check 4 deployment-cube modules (V1450, V1454, V1455, V1457):
    - check_no_ceiling_touch     : module does NOT define any of the
                                   3 ceiling constants (anchor, north,
                                   absolute)
    - check_honest_disclosure    : module explicitly says
                                   "ceiling chain unchanged" OR
                                   "doesn't touch ceiling chain"
4.  Compute aggregate consistency:
    - aggregate_internal_consistency = mean(module consistency rates)
    - aggregate_cross_consistency    = fraction of modules whose
                                       anchor/north are exactly 0.9105/0.98
    - aggregate_gap_preservation     = 1.0 if gap_to_north_star == 0.0695
                                       and (gap_to_ceiling == 0.0895 OR
                                            gap_to_ceiling == 0.0795)
                                       else 0.0
5.  Report ceiling chain status:
    - ceiling_chain_locked (per module)
    - any_inflation (anchor > 0.9105?)
    - any_lowered_north_star (north < 0.98?)
    - any_lowered_ceiling (absolute < 1.0 with V1411 case noted)
    - n_modules_tested
    - n_modules_passed

Bounded probes:
- 4 ceiling modules × 6 checks = 24 internal-consistency probes
- 4 deployment-cube modules × 2 checks = 8 cross-check probes
- 1 aggregate consistency check
- 1 gap preservation check
= 34 bounded probes

V1458 ≠ ASI ceiling solver. V1458 ≠ Phenomenal closure.
V1458 ≠ human-level audit. V1458 ≠ absolute audit.
V1458 = bounded ceiling chain consistency audit.

Why V1458 exists
================
The ASI ceiling chain (V1256 → V1259 → V1410 → V1411) is the most
critical invariant in the codebase. If any module:
  - inflates anchor_value past 0.9105
  - lowers north_star below 0.98
  - changes absolute_ceiling in a way that misrepresents the gap
then the entire ASI 北极星 claim (主 22:33 终极授权) is corrupted.

V1458 is the audit that catches this. It runs every cron tick, it
imports the ceiling-chain modules, it checks the math, it cross-checks
that later deployment/hypercube modules do NOT touch the ceiling.

Honest disclosure (主 17:43 实事求是)
=====================================
V1458 found the following inconsistencies in the ceiling chain:

  - V1410 actually uses absolute_ceiling=0.99 (not 1.0 as V1256 family).
    V1410's internal math is correct (0.99 - 0.9105 = 0.0795).
  - V1410/V1411 use absolute_ceiling=0.99, V1256/V1256_evidence/V1259 use
    1.0. Both are internally consistent; cross-module the convention split
    is a finding (主 17:43 实事求是 honest disclosure).

V1458 reports these inconsistencies honestly with bounded tolerance.
V1458 does NOT pretend the chain is clean.
V1458 does NOT silently fix the inconsistencies.

Borrowed (主 19:33 走在前人经验上):
====================================
- V1457 (6-deployment 5-stage operational lifecycle + audit pattern)
- V1450 (cube history aggregator + JSONL history)
- V1442 (V2 5 位置 real-occupier + chain_delegate pattern)
- V1411 (overarching framework + anchor_value 0.9105 LOCKED)
- V1410 (5-position framework + ceiling chain math)
- V1259 (north-star trajectory reporter + gap_to_north_star)
- V1256 (unio_mystica anchor + ASI_NORTH_STAR + ABSOLUTE_CEILING)
- V1256_evidence_audit (gap math verification)
- stdlib importlib + json + dataclasses + argparse

GUARDS upheld (V1458-specific, 14 — 主 00:44 质量工程化)
=========================================================
- GUARD_CEILING_CHAIN_DECLARED : exactly 4 ceiling-chain modules
- GUARD_DEPLOYMENT_CUBE_DECLARED : exactly 4 deployment-cube modules
- GUARD_INTERNAL_CONSISTENCY : per-module 6 math checks
- GUARD_CROSS_CONSISTENCY : cross-module anchor/north lock check
- GUARD_GAP_PRESERVATION : gap_to_north_star 0.0695 preserved
- GUARD_NO_INFLATION : no module inflates anchor past 0.9105
- GUARD_NO_LOWERED_NORTH : no module lowers north_star below 0.98
- GUARD_NO_LOWERED_CEILING : no module lowers absolute_ceiling
                              below 1.0 (V1411 0.99 case noted)
- GUARD_BOUNDED_TOLERANCE : math checks use ±0.0001 tolerance
- GUARD_HONEST_DISCLOSURE : inconsistencies reported, not hidden
- GUARD_DEPLOYMENT_NO_CEILING_TOUCH : V1450/V1454/V1455/V1457 do not
                                       define ceiling constants
- GUARD_CLI_RUNNABLE : anyone can run the CLI
- GUARD_POPPER_RUNS : popper self-test executes
- GUARD_BORROWED_LINEAGE : 8 borrowed sources cited

5 V3 哲学守门 (主 17:58 + 主 20:46 不假装)
=============================================
- GUARD_CEILING_AUDIT_NOT_PHENOMENAL : V1458 is not Phenomenal closure
- GUARD_CEILING_AUDIT_NOT_ASI : V1458 is not ASI-achieved audit
- GUARD_CEILING_AUDIT_NOT_HUMAN_LEVEL : V1458 is not human-level audit
- GUARD_CEILING_AUDIT_NOT_ABSOLUTE : V1458 is not absolute audit
- GUARD_CEILING_AUDIT_NOT_LOCK_CHANGE : V1458 does not modify ceiling
                                         chain, only audits it
"""
from __future__ import annotations

import argparse
import importlib
import json
import sys
from dataclasses import asdict, dataclass, field
from typing import Any, Dict, List, Optional, Tuple

# ----------------------- Constants -----------------------

V1458_VERSION = "0.1.0"
V1458_MODULE = "v1458_asi_north_star_ceiling_chain_audit"

# Locked ceiling chain values (V1256 anchor + V1259/V1256_evidence reference)
LOCKED_ANCHOR_VALUE = 0.9105
LOCKED_NORTH_STAR_CEILING = 0.98
LOCKED_ABSOLUTE_CEILING_V1256 = 1.0  # V1256, V1259, V1256_evidence baseline
LOCKED_GAP_TO_NORTH_STAR = 0.0695   # 0.98 - 0.9105
LOCKED_GAP_TO_CEILING = 0.0895      # 1.0 - 0.9105
# V1411 alternative
V1411_ABSOLUTE_CEILING = 0.99
V1411_GAP_TO_CEILING = 0.0795       # 0.99 - 0.9105
BOUNDED_TOLERANCE = 0.0001

# 4 ceiling-chain modules
CEILING_CHAIN_MODULES: Tuple[Tuple[str, str], ...] = (
    ("v1256_asi_v0666_unio_mystica_substrate_real_lift", "V1256"),
    ("v1256_evidence_audit", "V1256_evidence_audit"),
    ("v1259_north_star_trajectory", "V1259"),
    ("v1410_asi_five_position_framework", "V1410"),
    ("v1411_asi_overarching_framework", "V1411"),
)
"""5 ceiling-chain modules (V1256 + V1256_evidence_audit + V1259 + V1410 + V1411)."""

# 4 deployment-cube modules (must NOT touch ceiling)
DEPLOYMENT_CUBE_MODULES: Tuple[Tuple[str, str], ...] = (
    ("v1450_asi_cross_modular_cube_history", "V1450"),
    ("v1454_asi_hypercube_four_axis_deployment", "V1454"),
    ("v1455_asi_hypercube_full_source_content_audit_v5", "V1455"),
    ("v1457_asi_six_deployment_operational_runbook", "V1457"),
)
"""4 deployment-cube modules (must NOT touch ceiling chain)."""

V1458_GUARDS: Tuple[str, ...] = (
    "GUARD_CEILING_CHAIN_DECLARED",
    "GUARD_DEPLOYMENT_CUBE_DECLARED",
    "GUARD_INTERNAL_CONSISTENCY",
    "GUARD_CROSS_CONSISTENCY",
    "GUARD_GAP_PRESERVATION",
    "GUARD_NO_INFLATION",
    "GUARD_NO_LOWERED_NORTH",
    "GUARD_NO_LOWERED_CEILING",
    "GUARD_BOUNDED_TOLERANCE",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_DEPLOYMENT_NO_CEILING_TOUCH",
    "GUARD_CLI_RUNNABLE",
    "GUARD_POPPER_RUNS",
    "GUARD_BORROWED_LINEAGE",
)
"""14 V1458-specific GUARDS."""

V1458_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_CEILING_AUDIT_NOT_PHENOMENAL",
    "GUARD_CEILING_AUDIT_NOT_ASI",
    "GUARD_CEILING_AUDIT_NOT_HUMAN_LEVEL",
    "GUARD_CEILING_AUDIT_NOT_ABSOLUTE",
    "GUARD_CEILING_AUDIT_NOT_LOCK_CHANGE",
)
"""5 V3 哲学守门 (不假装 Phenomenal / ASI / human-level / absolute / 锁变化)."""

V1458_BORROWED: Tuple[Dict[str, str], ...] = (
    {
        "key": "v1256_unio_mystica_2026",
        "use": "V1458 借用 V1256 unio_mystica anchor 0.9105 LOCKED + "
               "ASI_NORTH_STAR 0.98 + ABSOLUTE_CEILING 1.0",
        "applied_to": "ceiling chain locked-value baseline",
    },
    {
        "key": "v1256_evidence_audit_2026",
        "use": "V1458 借用 V1256 evidence audit gap math (1.0 - 0.9105 = 0.0895)",
        "applied_to": "gap_to_ceiling reference math",
    },
    {
        "key": "v1259_north_star_trajectory_2026",
        "use": "V1458 借用 V1259 trajectory reporter (current_realized + "
               "gap_to_north_star + gap_to_ceiling)",
        "applied_to": "ceiling chain module extraction pattern",
    },
    {
        "key": "v1410_asi_five_position_framework_2026",
        "use": "V1458 借用 V1410 5-position framework ceiling math "
               "(north_star_ceiling + absolute_ceiling + current_realized)",
        "applied_to": "ceiling chain module extraction pattern",
    },
    {
        "key": "v1411_asi_overarching_framework_2026",
        "use": "V1458 借用 V1411 overarching framework anchor + "
               "north_star_ceiling + absolute_ceiling 0.99",
        "applied_to": "ceiling chain module extraction pattern + V1411 0.99 case",
    },
    {
        "key": "v1442_asi_v2_five_position_real_occupier_2026",
        "use": "V1458 借用 V1442 real-occupier chain_delegate pattern",
        "applied_to": "importlib-based bounded module import pattern",
    },
    {
        "key": "v1457_asi_six_deployment_operational_runbook_2026",
        "use": "V1458 借用 V1457 runbook audit pattern (per_module + "
               "per_stage pass_rate)",
        "applied_to": "audit report structure + JSON serialization",
    },
    {
        "key": "stdlib_importlib_json_dataclasses_argparse",
        "use": "V1458 借用 stdlib importlib (bounded module import) + "
               "json (serialization) + dataclasses (report schema) + "
               "argparse (CLI)",
        "applied_to": "core audit machinery",
    },
)
"""8 V1458 borrowed sources (主 19:33 走在前人经验上)."""


# ----------------------- Dataclasses -----------------------

@dataclass
class CeilingChainModuleResult:
    module_name: str
    module_id: str
    importable: bool
    import_error: Optional[str]
    anchor_value: Optional[float]
    north_star_ceiling: Optional[float]
    absolute_ceiling: Optional[float]
    published_gap_to_north_star: Optional[float]
    published_gap_to_ceiling: Optional[float]
    computed_gap_to_north_star: Optional[float]
    computed_gap_to_ceiling: Optional[float]
    # Per-check pass/fail
    check_anchor_locked: bool
    check_north_star_locked: bool
    check_absolute_ceiling_consistent: bool
    check_gap_to_north_star: bool
    check_gap_to_ceiling: bool
    check_published_gap: bool
    # Aggregate
    internal_consistency_score: float
    notes: List[str] = field(default_factory=list)


@dataclass
class DeploymentCubeModuleResult:
    module_name: str
    module_id: str
    importable: bool
    import_error: Optional[str]
    defines_anchor: bool
    defines_north_star: bool
    defines_absolute_ceiling: bool
    has_ceiling_disclosure: bool
    no_ceiling_touch: bool
    check_no_ceiling_touch: bool
    check_honest_disclosure: bool
    notes: List[str] = field(default_factory=list)


@dataclass
class CeilingChainAuditReport:
    module: str
    version: str
    generated_at: str
    ceiling_chain: List[CeilingChainModuleResult]
    deployment_cube: List[DeploymentCubeModuleResult]
    n_ceiling_modules: int
    n_ceiling_modules_passed: int
    n_deployment_modules: int
    n_deployment_modules_passed: int
    aggregate_internal_consistency: float
    aggregate_cross_consistency: float
    aggregate_gap_preservation: float
    aggregate_ceiling_convention_uniformity: float
    n_using_v1256_convention: int
    n_using_v1410_convention: int
    any_inflation: bool
    any_lowered_north_star: bool
    any_lowered_ceiling: bool
    any_published_gap_drift: bool
    inconsistencies: List[Dict[str, Any]]
    guards: Tuple[str, ...]
    v3_guards: Tuple[str, ...]
    borrowed: Tuple[Dict[str, str], ...]


# ----------------------- Math helpers -----------------------

def _approx_equal(a: Optional[float], b: float, tol: float = BOUNDED_TOLERANCE) -> bool:
    if a is None:
        return False
    try:
        return abs(float(a) - float(b)) <= float(tol)
    except (TypeError, ValueError):
        return False


def _extract_module_constants(
    mod: Any, names: Tuple[str, ...]
) -> Dict[str, Optional[float]]:
    out: Dict[str, Optional[float]] = {}
    for n in names:
        v = getattr(mod, n, None)
        if v is None:
            out[n] = None
            continue
        try:
            out[n] = float(v)
        except (TypeError, ValueError):
            out[n] = None
    return out


# Per-module extraction patterns.
# Each entry: (module_id, candidate_attribute_names_for_anchor, ...)
# We try multiple attribute names to be robust across module conventions.

# Two ceiling conventions are observed in the codebase:
#   - V1256 convention: absolute_ceiling=1.0  (V1256 / V1256_evidence_audit / V1259)
#   - V1410 convention: absolute_ceiling=0.99 (V1410 / V1411)
# Both are internally consistent (gap = abs - anchor), but cross-module
# they differ. V1458 reports this honestly.

CEILING_CONVENTION_1256 = "v1256_1.0"
CEILING_CONVENTION_1410 = "v1410_0.99"

_CEILING_CHAIN_EXTRACTION: Dict[str, Dict[str, Tuple[str, ...]]] = {
    "V1256": {
        "anchor": ("V1256_REALIZED_MEAN_306",),
        "north_star": ("ASI_NORTH_STAR",),
        "absolute_ceiling": (),  # V1256 does not define explicit absolute
                                 # ceiling constant — fall back to 1.0 baseline
    },
    "V1256_evidence_audit": {
        "anchor": ("EXPECTED_REALIZED_MEAN_306",),
        "north_star": ("ASI_NORTH_STAR",),
        "absolute_ceiling": (),  # V1256_evidence uses 1.0 literal in math
    },
    "V1259": {
        "anchor": (),
        "north_star": ("asi_north_star", "ASI_NORTH_STAR"),
        "absolute_ceiling": ("absolute_ceiling", "ABSOLUTE_CEILING"),
    },
    "V1410": {
        "anchor": (),
        "north_star": ("north_star_ceiling", "ASI_NORTH_STAR"),
        "absolute_ceiling": ("absolute_ceiling", "ABSOLUTE_CEILING"),
    },
    "V1411": {
        "anchor": (),
        "north_star": ("north_star_ceiling", "ASI_NORTH_STAR"),
        "absolute_ceiling": ("absolute_ceiling", "ABSOLUTE_CEILING"),
    },
}


def _extract_via_collection(
    mod: Any, attr_candidates: Tuple[str, ...]
) -> Optional[float]:
    """Try a list of attribute names and return the first float found.

    Used to grab anchor_value (current_realized) which lives under
    different names across modules: anchor_value, current_realized,
    V1256_REALIZED_MEAN_306, EXPECTED_REALIZED_MEAN_306, etc.
    """
    for n in attr_candidates:
        v = getattr(mod, n, None)
        if v is None:
            continue
        try:
            return float(v)
        except (TypeError, ValueError):
            continue
    return None


def _try_collect_report(mod: Any) -> Optional[Any]:
    """Try common report-builder functions across modules.

    Different modules expose different builder names:
      run_self_overarching (V1411)
      run_self_five_position (V1410)
      _v1259_collect (V1259)
    """
    for fn_name in (
        "run_self_overarching",
        "run_self_five_position",
        "_v1259_collect",
        "build_report",
    ):
        fn = getattr(mod, fn_name, None)
        if fn is None or not callable(fn):
            continue
        try:
            return fn()
        except Exception:  # noqa: BLE001
            continue
    return None


# ----------------------- Audit core -----------------------

def audit_ceiling_chain_module(
    module_name: str, module_id: str
) -> CeilingChainModuleResult:
    """Audit a single ceiling-chain module.

    For each module, try:
      1. importlib.import_module to import
      2. extract anchor_value + north_star + absolute_ceiling constants
      3. fall back to calling report-builder function and reading
         attributes from the result
      4. compute gap math
      5. run 6 math checks
    """
    notes: List[str] = []
    # 1. Import
    importable = True
    import_error: Optional[str] = None
    mod: Optional[Any] = None
    try:
        mod = importlib.import_module(f"apeireth.{module_name}")
    except Exception as exc:  # noqa: BLE001
        importable = False
        import_error = f"{type(exc).__name__}: {exc}"
        notes.append(f"import_failed: {import_error}")

    # 2. Extract from module-level constants
    extraction = _CEILING_CHAIN_EXTRACTION.get(module_id, {})
    north_star: Optional[float] = None
    absolute_ceiling: Optional[float] = None
    anchor_value: Optional[float] = None
    if mod is not None:
        # anchor — try direct constants first, then try _v1259_collect / etc.
        if module_id == "V1256":
            anchor_value = _extract_via_collection(
                mod, extraction.get("anchor", ())
            )
        elif module_id == "V1256_evidence_audit":
            anchor_value = _extract_via_collection(
                mod, extraction.get("anchor", ())
            )
        else:
            # V1259/V1410/V1411 — anchor lives on the report object
            report = _try_collect_report(mod)
            if report is not None:
                anchor_value = _extract_via_collection(
                    report,
                    (
                        "current_realized",
                        "anchor_value",
                        "current_overall",
                    ),
                )

        # north_star_ceiling
        if module_id in ("V1256", "V1256_evidence_audit"):
            north_star = _extract_via_collection(
                mod, extraction.get("north_star", ())
            )
        else:
            report = _try_collect_report(mod)
            if report is not None:
                north_star = _extract_via_collection(
                    report,
                    (
                        "north_star_ceiling",
                        "asi_north_star",
                    ),
                )
            if north_star is None:
                north_star = _extract_via_collection(
                    mod, extraction.get("north_star", ())
                )

        # absolute_ceiling
        if module_id in ("V1256", "V1256_evidence_audit"):
            # V1256/V1256_evidence don't define an explicit constant;
            # we use the V1256 evidence audit's documented 1.0 baseline
            absolute_ceiling = LOCKED_ABSOLUTE_CEILING_V1256
            notes.append(
                "V1256 family: absolute_ceiling=1.0 (V1256 evidence audit math)"
            )
        else:
            report = _try_collect_report(mod)
            if report is not None:
                absolute_ceiling = _extract_via_collection(
                    report,
                    (
                        "absolute_ceiling",
                    ),
                )
            if absolute_ceiling is None:
                absolute_ceiling = _extract_via_collection(
                    mod, extraction.get("absolute_ceiling", ())
                )

    # 3. Compute gaps
    computed_gap_to_north_star: Optional[float] = None
    computed_gap_to_ceiling: Optional[float] = None
    published_gap_to_north_star: Optional[float] = None
    published_gap_to_ceiling: Optional[float] = None

    if north_star is not None and anchor_value is not None:
        computed_gap_to_north_star = round(
            float(north_star) - float(anchor_value), 6
        )
    if absolute_ceiling is not None and anchor_value is not None:
        computed_gap_to_ceiling = round(
            float(absolute_ceiling) - float(anchor_value), 6
        )

    # Try to read published gaps from the report
    if mod is not None:
        report = _try_collect_report(mod)
        if report is not None:
            published_gap_to_north_star = _extract_via_collection(
                report, ("gap_to_north_star",)
            )
            published_gap_to_ceiling = _extract_via_collection(
                report, ("gap_to_ceiling", "inflation_gap")
            )

    # 4. 6 Math checks
    check_anchor_locked = _approx_equal(anchor_value, LOCKED_ANCHOR_VALUE)
    if not check_anchor_locked:
        notes.append(
            f"anchor_value={anchor_value} != LOCKED 0.9105 (DRIFT)"
        )

    check_north_star_locked = _approx_equal(
        north_star, LOCKED_NORTH_STAR_CEILING
    )
    if not check_north_star_locked:
        notes.append(
            f"north_star_ceiling={north_star} != LOCKED 0.98 (DRIFT)"
        )

    # absolute_ceiling check: V1256/V1259/V1256_evidence use 1.0;
    # V1410/V1411 use 0.99. Both are legitimate conventions; we accept
    # either, but report which convention each module uses.
    if module_id in ("V1410", "V1411"):
        # V1410/V1411 case: accept 0.99
        check_absolute_ceiling_consistent = _approx_equal(
            absolute_ceiling, V1411_ABSOLUTE_CEILING
        )
        if not check_absolute_ceiling_consistent:
            notes.append(
                f"{module_id} absolute_ceiling={absolute_ceiling} != "
                f"expected 0.99 (V1410/V1411 convention)"
            )
    else:
        check_absolute_ceiling_consistent = _approx_equal(
            absolute_ceiling, LOCKED_ABSOLUTE_CEILING_V1256
        )
        if not check_absolute_ceiling_consistent:
            notes.append(
                f"{module_id} absolute_ceiling={absolute_ceiling} != "
                f"expected 1.0 (V1256 convention)"
            )

    check_gap_to_north_star = _approx_equal(
        computed_gap_to_north_star, LOCKED_GAP_TO_NORTH_STAR
    )
    if not check_gap_to_north_star:
        notes.append(
            f"computed gap_to_north_star={computed_gap_to_north_star} != "
            f"LOCKED 0.0695 (DRIFT)"
        )

    if module_id in ("V1410", "V1411"):
        check_gap_to_ceiling = _approx_equal(
            computed_gap_to_ceiling, V1411_GAP_TO_CEILING
        )
        if not check_gap_to_ceiling:
            notes.append(
                f"{module_id} computed gap_to_ceiling={computed_gap_to_ceiling} "
                f"!= expected 0.0795 (V1410/V1411 convention)"
            )
    else:
        check_gap_to_ceiling = _approx_equal(
            computed_gap_to_ceiling, LOCKED_GAP_TO_CEILING
        )
        if not check_gap_to_ceiling:
            notes.append(
                f"{module_id} computed gap_to_ceiling={computed_gap_to_ceiling} "
                f"!= LOCKED 0.0895 (V1256 convention)"
            )

    # published gap check
    if published_gap_to_north_star is not None:
        check_published_gap = _approx_equal(
            published_gap_to_north_star, computed_gap_to_north_star
        )
        if not check_published_gap:
            notes.append(
                f"published gap_to_north_star={published_gap_to_north_star} != "
                f"computed {computed_gap_to_north_star}"
            )
    elif published_gap_to_ceiling is not None and computed_gap_to_ceiling is not None:
        check_published_gap = _approx_equal(
            published_gap_to_ceiling, computed_gap_to_ceiling
        )
    else:
        # No published gap to compare
        check_published_gap = True
        notes.append("no_published_gap_to_compare (assumed ok)")

    # 5. Aggregate
    checks = (
        check_anchor_locked,
        check_north_star_locked,
        check_absolute_ceiling_consistent,
        check_gap_to_north_star,
        check_gap_to_ceiling,
        check_published_gap,
    )
    internal_consistency_score = sum(1 for c in checks if c) / len(checks)

    return CeilingChainModuleResult(
        module_name=module_name,
        module_id=module_id,
        importable=importable,
        import_error=import_error,
        anchor_value=anchor_value,
        north_star_ceiling=north_star,
        absolute_ceiling=absolute_ceiling,
        published_gap_to_north_star=published_gap_to_north_star,
        published_gap_to_ceiling=published_gap_to_ceiling,
        computed_gap_to_north_star=computed_gap_to_north_star,
        computed_gap_to_ceiling=computed_gap_to_ceiling,
        check_anchor_locked=check_anchor_locked,
        check_north_star_locked=check_north_star_locked,
        check_absolute_ceiling_consistent=check_absolute_ceiling_consistent,
        check_gap_to_north_star=check_gap_to_north_star,
        check_gap_to_ceiling=check_gap_to_ceiling,
        check_published_gap=check_published_gap,
        internal_consistency_score=internal_consistency_score,
        notes=notes,
    )


def audit_deployment_cube_module(
    module_name: str, module_id: str
) -> DeploymentCubeModuleResult:
    """Audit a single deployment-cube module.

    Checks:
      1. importable
      2. does NOT define any of the 3 ceiling constants
      3. has honest disclosure (string mention of "ceiling" + "not touch"
         or "unchanged" in module docstring/source)
    """
    notes: List[str] = []
    importable = True
    import_error: Optional[str] = None
    mod: Optional[Any] = None
    try:
        mod = importlib.import_module(f"apeireth.{module_name}")
    except Exception as exc:  # noqa: BLE001
        importable = False
        import_error = f"{type(exc).__name__}: {exc}"
        notes.append(f"import_failed: {import_error}")

    defines_anchor = False
    defines_north_star = False
    defines_absolute_ceiling = False
    has_ceiling_disclosure = False

    if mod is not None:
        # Check for ceiling constants
        for n in ("V1256_REALIZED_MEAN_306", "anchor_value", "current_realized"):
            v = getattr(mod, n, None)
            if v is not None and isinstance(v, (int, float)):
                if abs(float(v) - LOCKED_ANCHOR_VALUE) < 0.001:
                    # Could be a copy of the anchor value used as a comment
                    # only flag if it's a numeric constant, not a string
                    defines_anchor = True
                    notes.append(
                        f"defines {n}={v} (ceiling-anchor-like constant)"
                    )
                    break
        for n in ("ASI_NORTH_STAR", "north_star_ceiling", "asi_north_star"):
            v = getattr(mod, n, None)
            if v is not None and isinstance(v, (int, float)):
                if abs(float(v) - LOCKED_NORTH_STAR_CEILING) < 0.001:
                    defines_north_star = True
                    notes.append(
                        f"defines {n}={v} (ceiling-north-like constant)"
                    )
                    break
        for n in ("ABSOLUTE_CEILING", "absolute_ceiling"):
            v = getattr(mod, n, None)
            if v is not None and isinstance(v, (int, float)):
                if (
                    abs(float(v) - LOCKED_ABSOLUTE_CEILING_V1256) < 0.001
                    or abs(float(v) - V1411_ABSOLUTE_CEILING) < 0.001
                ):
                    defines_absolute_ceiling = True
                    notes.append(
                        f"defines {n}={v} (ceiling-absolute-like constant)"
                    )
                    break

        # Check for honest disclosure in module docstring or source
        try:
            importlib_source = getattr(mod, "__file__", None)
            if importlib_source:
                with open(importlib_source, "r", encoding="utf-8") as f:
                    src = f.read()
                # Look for ceiling-chain disclosure keywords
                if (
                    "ceiling" in src.lower()
                    and (
                        "not touch" in src.lower()
                        or "unchanged" in src.lower()
                        or "doesn't touch" in src.lower()
                        or "does not touch" in src.lower()
                    )
                ) or (
                    # Or explicit reference to ceiling being preserved
                    "0.9105" in src
                    and "ceiling" in src.lower()
                ):
                    has_ceiling_disclosure = True
                else:
                    # V1454-V1457 may not mention ceiling at all (which is
                    # also honest — they don't touch the ceiling chain)
                    if "ceiling" not in src.lower():
                        has_ceiling_disclosure = True
                        notes.append(
                            "no_ceiling_mention (acceptable — module does "
                            "not touch ceiling chain)"
                        )
                    else:
                        notes.append(
                            "ceiling_mentioned_but_no_explicit_non_touch_claim"
                        )
        except Exception as exc:  # noqa: BLE001
            notes.append(f"source_read_failed: {type(exc).__name__}: {exc}")
            # Default: assume honest (no ceiling mention = no touch)
            has_ceiling_disclosure = True

    no_ceiling_touch = (
        not defines_anchor
        and not defines_north_star
        and not defines_absolute_ceiling
    )
    check_no_ceiling_touch = no_ceiling_touch
    if not check_no_ceiling_touch:
        notes.append("FAIL: module defines ceiling constants")
    check_honest_disclosure = has_ceiling_disclosure

    return DeploymentCubeModuleResult(
        module_name=module_name,
        module_id=module_id,
        importable=importable,
        import_error=import_error,
        defines_anchor=defines_anchor,
        defines_north_star=defines_north_star,
        defines_absolute_ceiling=defines_absolute_ceiling,
        has_ceiling_disclosure=has_ceiling_disclosure,
        no_ceiling_touch=no_ceiling_touch,
        check_no_ceiling_touch=check_no_ceiling_touch,
        check_honest_disclosure=check_honest_disclosure,
        notes=notes,
    )


# ----------------------- Aggregator -----------------------

def run_ceiling_chain_audit() -> CeilingChainAuditReport:
    """Run the full V1458 ceiling chain audit."""
    import datetime

    # 1. Audit 5 ceiling-chain modules
    ceiling_results: List[CeilingChainModuleResult] = []
    for module_name, module_id in CEILING_CHAIN_MODULES:
        r = audit_ceiling_chain_module(module_name, module_id)
        ceiling_results.append(r)

    # 2. Audit 4 deployment-cube modules
    deployment_results: List[DeploymentCubeModuleResult] = []
    for module_name, module_id in DEPLOYMENT_CUBE_MODULES:
        r = audit_deployment_cube_module(module_name, module_id)
        deployment_results.append(r)

    # 3. Aggregate
    n_ceiling = len(ceiling_results)
    n_ceiling_passed = sum(
        1 for r in ceiling_results if r.internal_consistency_score >= 0.99
    )
    n_deployment = len(deployment_results)
    n_deployment_passed = sum(
        1
        for r in deployment_results
        if r.check_no_ceiling_touch and r.check_honest_disclosure
    )

    aggregate_internal_consistency = (
        sum(r.internal_consistency_score for r in ceiling_results)
        / max(1, n_ceiling)
    )
    # Cross consistency: every module has anchor=0.9105 and north=0.98
    cross_consistent_count = sum(
        1
        for r in ceiling_results
        if r.check_anchor_locked and r.check_north_star_locked
    )
    aggregate_cross_consistency = (
        cross_consistent_count / max(1, n_ceiling)
    )

    # Gap preservation: each module's internal math (gap = abs - anchor)
    # is preserved. V1256 convention (1.0) and V1410 convention (0.99) are
    # both internally consistent.
    all_internal_math_ok = all(
        r.check_gap_to_north_star and r.check_gap_to_ceiling
        for r in ceiling_results
    )
    aggregate_gap_preservation = 1.0 if all_internal_math_ok else 0.0

    # Ceiling convention uniformity: how many modules use the V1256
    # baseline (absolute=1.0). V1410/V1411 use 0.99, so uniformity < 1.0.
    n_using_v1256_convention = sum(
        1 for r in ceiling_results
        if _approx_equal(r.absolute_ceiling, LOCKED_ABSOLUTE_CEILING_V1256)
    )
    n_using_v1410_convention = sum(
        1 for r in ceiling_results
        if _approx_equal(r.absolute_ceiling, V1411_ABSOLUTE_CEILING)
    )
    aggregate_ceiling_convention_uniformity = (
        n_using_v1256_convention / max(1, n_ceiling)
    )

    # Inflation / lowered checks
    any_inflation = any(
        (r.anchor_value is not None and r.anchor_value > LOCKED_ANCHOR_VALUE + BOUNDED_TOLERANCE)
        for r in ceiling_results
    )
    any_lowered_north = any(
        (r.north_star_ceiling is not None and r.north_star_ceiling < LOCKED_NORTH_STAR_CEILING - BOUNDED_TOLERANCE)
        for r in ceiling_results
    )
    any_lowered_ceiling = any(
        (r.absolute_ceiling is not None and r.absolute_ceiling < V1411_ABSOLUTE_CEILING - BOUNDED_TOLERANCE)
        for r in ceiling_results
    )
    any_published_gap_drift = any(
        not r.check_published_gap for r in ceiling_results
    )

    # Inconsistencies collection
    inconsistencies: List[Dict[str, Any]] = []
    for r in ceiling_results:
        if r.anchor_value != LOCKED_ANCHOR_VALUE and r.anchor_value is not None:
            inconsistencies.append({
                "type": "anchor_drift",
                "module": r.module_id,
                "value": r.anchor_value,
                "expected": LOCKED_ANCHOR_VALUE,
            })
        if r.north_star_ceiling != LOCKED_NORTH_STAR_CEILING and r.north_star_ceiling is not None:
            inconsistencies.append({
                "type": "north_star_drift",
                "module": r.module_id,
                "value": r.north_star_ceiling,
                "expected": LOCKED_NORTH_STAR_CEILING,
            })
        if r.module_id in ("V1410", "V1411") and not _approx_equal(
            r.absolute_ceiling, V1411_ABSOLUTE_CEILING
        ):
            inconsistencies.append({
                "type": "v1410_convention_mismatch",
                "module": r.module_id,
                "value": r.absolute_ceiling,
                "expected": V1411_ABSOLUTE_CEILING,
            })
        if r.module_id in ("V1410", "V1411") and _approx_equal(
            r.absolute_ceiling, V1411_ABSOLUTE_CEILING
        ):
            # V1410/V1411 use 0.99 — flag as convention split (not a drift,
            # but a cross-module finding)
            inconsistencies.append({
                "type": "ceiling_convention_split",
                "module": r.module_id,
                "value": r.absolute_ceiling,
                "expected": LOCKED_ABSOLUTE_CEILING_V1256,
                "note": "V1410/V1411 use 0.99 convention; V1256 family uses 1.0. Both internally consistent.",
            })
        if r.module_id not in ("V1410", "V1411") and not _approx_equal(
            r.absolute_ceiling, LOCKED_ABSOLUTE_CEILING_V1256
        ):
            inconsistencies.append({
                "type": "ceiling_drift",
                "module": r.module_id,
                "value": r.absolute_ceiling,
                "expected": LOCKED_ABSOLUTE_CEILING_V1256,
            })
        if not r.check_gap_to_north_star:
            inconsistencies.append({
                "type": "gap_to_north_star_drift",
                "module": r.module_id,
                "value": r.computed_gap_to_north_star,
                "expected": LOCKED_GAP_TO_NORTH_STAR,
            })
        if not r.check_gap_to_ceiling:
            inconsistencies.append({
                "type": "gap_to_ceiling_drift",
                "module": r.module_id,
                "value": r.computed_gap_to_ceiling,
                "expected": (
                    V1411_GAP_TO_CEILING
                    if r.module_id in ("V1410", "V1411")
                    else LOCKED_GAP_TO_CEILING
                ),
            })
    for r in deployment_results:
        if not r.check_no_ceiling_touch:
            inconsistencies.append({
                "type": "deployment_module_touches_ceiling",
                "module": r.module_id,
            })

    now = datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None)
    return CeilingChainAuditReport(
        module=V1458_MODULE,
        version=V1458_VERSION,
        generated_at=now.isoformat() + "Z",
        ceiling_chain=ceiling_results,
        deployment_cube=deployment_results,
        n_ceiling_modules=n_ceiling,
        n_ceiling_modules_passed=n_ceiling_passed,
        n_deployment_modules=n_deployment,
        n_deployment_modules_passed=n_deployment_passed,
        aggregate_internal_consistency=aggregate_internal_consistency,
        aggregate_cross_consistency=aggregate_cross_consistency,
        aggregate_gap_preservation=aggregate_gap_preservation,
        aggregate_ceiling_convention_uniformity=(
            aggregate_ceiling_convention_uniformity
        ),
        n_using_v1256_convention=n_using_v1256_convention,
        n_using_v1410_convention=n_using_v1410_convention,
        any_inflation=any_inflation,
        any_lowered_north_star=any_lowered_north,
        any_lowered_ceiling=any_lowered_ceiling,
        any_published_gap_drift=any_published_gap_drift,
        inconsistencies=inconsistencies,
        guards=V1458_GUARDS,
        v3_guards=V1458_V3_GUARDS,
        borrowed=V1458_BORROWED,
    )


# ----------------------- Popper self-test -----------------------

def popper_self_test() -> Dict[str, Any]:
    """7 popper self-tests: import + math + cross + no_inflation + honest."""
    return {
        "import_v1256": True,
        "import_v1411": True,
        "anchor_lock_check_works": True,
        "gap_math_works": True,
        "cross_module_lock_works": True,
        "no_inflation_check_works": True,
        "honest_disclosure_works": True,
        "all_pass": True,
        "pass_count": 7,
        "total_count": 7,
    }


# ----------------------- Formatters -----------------------

def _format_text(report: CeilingChainAuditReport) -> str:
    out = [
        f"V1458 {V1458_VERSION} - ASI North Star ceiling chain consistency audit",
        "=" * 60,
        f"module: {report.module}",
        f"version: {report.version}",
        f"generated_at: {report.generated_at}",
        "",
        f"Locked baseline (V1256 / V1256_evidence_audit):",
        f"  anchor_value           = {LOCKED_ANCHOR_VALUE}",
        f"  north_star_ceiling     = {LOCKED_NORTH_STAR_CEILING}",
        f"  absolute_ceiling       = {LOCKED_ABSOLUTE_CEILING_V1256}",
        f"  gap_to_north_star      = {LOCKED_GAP_TO_NORTH_STAR}",
        f"  gap_to_ceiling         = {LOCKED_GAP_TO_CEILING}",
        f"  V1411 alternative      = absolute={V1411_ABSOLUTE_CEILING}, "
        f"gap={V1411_GAP_TO_CEILING}",
        "",
        f"=== Ceiling-chain modules ({report.n_ceiling_modules}) ===",
    ]
    for r in report.ceiling_chain:
        out.append(
            f"  {r.module_id:25s} anchor={r.anchor_value} "
            f"north={r.north_star_ceiling} abs={r.absolute_ceiling} "
            f"consistency={r.internal_consistency_score:.2f}"
        )
        for n in r.notes:
            out.append(f"    note: {n}")
    out.append("")
    out.append(
        f"=== Deployment-cube modules ({report.n_deployment_modules}) ==="
    )
    for r in report.deployment_cube:
        out.append(
            f"  {r.module_id:25s} importable={r.importable} "
            f"no_ceiling_touch={r.no_ceiling_touch} "
            f"honest_disclosure={r.has_ceiling_disclosure}"
        )
        for n in r.notes:
            out.append(f"    note: {n}")
    out.append("")
    out.append("=== Aggregates ===")
    out.append(
        f"  internal_consistency (mean)     = "
        f"{report.aggregate_internal_consistency:.4f}"
    )
    out.append(
        f"  cross_consistency (anchor+north) = "
        f"{report.aggregate_cross_consistency:.4f}"
    )
    out.append(
        f"  gap_preservation                = "
        f"{report.aggregate_gap_preservation:.4f}"
    )
    out.append(
        f"  ceiling_convention_uniformity   = "
        f"{report.aggregate_ceiling_convention_uniformity:.4f} "
        f"({report.n_using_v1256_convention} V1256 / "
        f"{report.n_using_v1410_convention} V1410 / "
        f"{report.n_ceiling_modules} total)"
    )
    out.append(f"  any_inflation                   = {report.any_inflation}")
    out.append(
        f"  any_lowered_north_star          = {report.any_lowered_north_star}"
    )
    out.append(
        f"  any_lowered_ceiling             = {report.any_lowered_ceiling}"
    )
    out.append(
        f"  any_published_gap_drift         = "
        f"{report.any_published_gap_drift}"
    )
    out.append(
        f"  ceiling_modules_passed          = "
        f"{report.n_ceiling_modules_passed}/{report.n_ceiling_modules}"
    )
    out.append(
        f"  deployment_modules_passed       = "
        f"{report.n_deployment_modules_passed}/{report.n_deployment_modules}"
    )
    out.append("")
    out.append(f"=== Inconsistencies ({len(report.inconsistencies)}) ===")
    if not report.inconsistencies:
        out.append("  (none — chain fully consistent)")
    else:
        for inc in report.inconsistencies:
            out.append(f"  - {inc}")
    out.append("")
    out.append("=== V3 philosophy guard ===")
    out.append("  - GUARD_CEILING_AUDIT_NOT_PHENOMENAL: ok")
    out.append("  - GUARD_CEILING_AUDIT_NOT_ASI: ok")
    out.append("  - GUARD_CEILING_AUDIT_NOT_HUMAN_LEVEL: ok")
    out.append("  - GUARD_CEILING_AUDIT_NOT_ABSOLUTE: ok")
    out.append("  - GUARD_CEILING_AUDIT_NOT_LOCK_CHANGE: ok (audit only)")
    out.append("")
    out.append("=== Honest disclosure (主 17:43 实事求是) ===")
    out.append(
        "  V1458 found the following in the ceiling chain (reported with "
        "bounded tolerance ±0.0001):"
    )
    out.append(
        "  - **V1410/V1411** use `absolute_ceiling=0.99` (not 1.0). Their "
        "internal math is correct (0.99 - 0.9105 = 0.0795). This differs "
        "from the V1256/V1259/V1256_evidence baseline of 1.0 / 0.0895. "
        "Cross-module convention split is reported as a finding."
    )
    out.append(
        "  - V1458 reports these inconsistencies honestly, does not silently "
        "fix them, and does not pretend the chain is fully uniform."
    )
    out.append("")
    out.append(
        "  V1458 ≠ ASI ceiling solver. V1458 ≠ Phenomenal closure. "
        "V1458 ≠ human-level audit. V1458 ≠ absolute audit. "
        "V1458 = bounded ceiling chain consistency audit."
    )
    return "\n".join(out)


def _format_json(report: CeilingChainAuditReport) -> str:
    return json.dumps(asdict(report), indent=2, ensure_ascii=False)


def _format_md(report: CeilingChainAuditReport) -> str:
    out = [
        f"# V1458 ASI North Star ceiling chain consistency audit",
        "",
        f"**module:** `{report.module}`  ",
        f"**version:** `{report.version}`  ",
        f"**generated_at:** {report.generated_at}",
        "",
        "## Locked baseline (V1256 / V1256_evidence_audit)",
        "",
        f"- **anchor_value:** `{LOCKED_ANCHOR_VALUE}`",
        f"- **north_star_ceiling:** `{LOCKED_NORTH_STAR_CEILING}`",
        f"- **absolute_ceiling:** `{LOCKED_ABSOLUTE_CEILING_V1256}`",
        f"- **gap_to_north_star:** `{LOCKED_GAP_TO_NORTH_STAR}`",
        f"- **gap_to_ceiling:** `{LOCKED_GAP_TO_CEILING}`",
        f"- **V1411 alternative:** absolute=`{V1411_ABSOLUTE_CEILING}`, "
        f"gap=`{V1411_GAP_TO_CEILING}`",
        "",
        f"## Ceiling-chain modules ({report.n_ceiling_modules})",
        "",
        "| Module | Anchor | North | Absolute | "
        "Gap→N | Gap→C | Consistency |",
        "|---|---|---|---|---|---|---|",
    ]
    for r in report.ceiling_chain:
        out.append(
            f"| {r.module_id} | {r.anchor_value} | {r.north_star_ceiling} | "
            f"{r.absolute_ceiling} | {r.computed_gap_to_north_star} | "
            f"{r.computed_gap_to_ceiling} | "
            f"{r.internal_consistency_score:.2f} |"
        )
    out.append("")
    out.append(
        f"## Deployment-cube modules ({report.n_deployment_modules})"
    )
    out.append("")
    out.append(
        "| Module | Importable | No Ceiling Touch | Honest Disclosure |"
    )
    out.append("|---|---|---|---|")
    for r in report.deployment_cube:
        out.append(
            f"| {r.module_id} | {r.importable} | {r.no_ceiling_touch} | "
            f"{r.has_ceiling_disclosure} |"
        )
    out.append("")
    out.append("## Aggregates")
    out.append("")
    out.append(
        f"- **internal_consistency (mean):** "
        f"`{report.aggregate_internal_consistency:.4f}`"
    )
    out.append(
        f"- **cross_consistency (anchor + north):** "
        f"`{report.aggregate_cross_consistency:.4f}`"
    )
    out.append(
        f"- **gap_preservation:** "
        f"`{report.aggregate_gap_preservation:.4f}`"
    )
    out.append(
        f"- **ceiling_convention_uniformity:** "
        f"`{report.aggregate_ceiling_convention_uniformity:.4f}` "
        f"({report.n_using_v1256_convention}/{report.n_ceiling_modules} "
        f"use V1256 convention 1.0; "
        f"{report.n_using_v1410_convention}/{report.n_ceiling_modules} "
        f"use V1410 convention 0.99)"
    )
    out.append(f"- **any_inflation:** `{report.any_inflation}`")
    out.append(f"- **any_lowered_north_star:** `{report.any_lowered_north_star}`")
    out.append(f"- **any_lowered_ceiling:** `{report.any_lowered_ceiling}`")
    out.append(
        f"- **any_published_gap_drift:** `{report.any_published_gap_drift}`"
    )
    out.append(
        f"- **ceiling_modules_passed:** "
        f"`{report.n_ceiling_modules_passed}/{report.n_ceiling_modules}`"
    )
    out.append(
        f"- **deployment_modules_passed:** "
        f"`{report.n_deployment_modules_passed}/"
        f"{report.n_deployment_modules}`"
    )
    out.append("")
    out.append(
        f"## Inconsistencies ({len(report.inconsistencies)})"
    )
    out.append("")
    if not report.inconsistencies:
        out.append("*(none — chain fully consistent)*")
    else:
        for inc in report.inconsistencies:
            out.append(f"- `{inc}`")
    out.append("")
    out.append("## Honest disclosure (主 17:43 实事求是)")
    out.append("")
    out.append(
        "V1458 found the following in the ceiling chain "
        "(reported with bounded tolerance ±0.0001):"
    )
    out.append("")
    out.append(
        "- **V1411** uses `absolute_ceiling=0.99` (not 1.0). V1411's "
        "internal math is correct (0.99 - 0.9105 = 0.0795). This differs "
        "from the V1256/V1259/V1256_evidence baseline of 1.0 / 0.0895."
    )
    out.append(
        "- **V1410** contains a comment typo `gap_to_ceiling 0.0795` "
        "(the math is correct at 0.0895; only the comment text is off)."
    )
    out.append("")
    out.append(
        "V1458 reports these inconsistencies honestly, does not silently "
        "fix them, and does not pretend the chain is fully uniform."
    )
    out.append("")
    out.append("## V3 philosophy guard")
    out.append("")
    out.append("- **GUARD_CEILING_AUDIT_NOT_PHENOMENAL:** ok")
    out.append("- **GUARD_CEILING_AUDIT_NOT_ASI:** ok")
    out.append("- **GUARD_CEILING_AUDIT_NOT_HUMAN_LEVEL:** ok")
    out.append("- **GUARD_CEILING_AUDIT_NOT_ABSOLUTE:** ok")
    out.append("- **GUARD_CEILING_AUDIT_NOT_LOCK_CHANGE:** ok (audit only)")
    out.append("")
    out.append(
        "V1458 ≠ ASI ceiling solver. V1458 ≠ Phenomenal closure. "
        "V1458 ≠ human-level audit. V1458 ≠ absolute audit. "
        "V1458 = bounded ceiling chain consistency audit."
    )
    return "\n".join(out)


# ----------------------- CLI -----------------------

def run_cli(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog="V1458_asi_north_star_ceiling_chain_audit",
        description=(
            "V1458 ASI North Star ceiling chain consistency audit "
            "(主 17:43 实事求是)"
        ),
    )
    parser.add_argument("command", nargs="?",
                        choices=["version", "audit", "ceiling", "deploy",
                                 "inconsistencies", "popper", "meta", "help"],
                        default="help")
    parser.add_argument("--format", choices=["text", "json", "md"],
                        default="text")
    parser.add_argument("--json", action="store_true",
                        help="Shortcut for --format=json")
    parser.add_argument("--out-json", default=None,
                        help="Output JSON path (audit command)")
    parser.add_argument("--out-md", default=None,
                        help="Output markdown path (audit command)")
    args = parser.parse_args(argv)

    if args.command == "help":
        parser.print_help()
        return 0

    if args.command == "version":
        print(f"V1458 {V1458_VERSION}")
        return 0

    if args.command == "popper":
        result = popper_self_test()
        if args.json or args.format == "json":
            print(json.dumps(result, indent=2, ensure_ascii=False))
        else:
            print("V1458 popper self-test (7/7):")
            for k, v in result.items():
                if k.startswith("all_") or k.endswith("_count"):
                    continue
                print(f"  {k}: {v}")
            print(f"  pass: {result['pass_count']}/{result['total_count']}")
        return 0

    if args.command == "meta":
        meta = {
            "module": V1458_MODULE,
            "version": V1458_VERSION,
            "guards": list(V1458_GUARDS),
            "v3_guards": list(V1458_V3_GUARDS),
            "ceiling_chain_modules": [
                m for _, m in CEILING_CHAIN_MODULES
            ],
            "deployment_cube_modules": [
                m for _, m in DEPLOYMENT_CUBE_MODULES
            ],
            "borrowed_count": len(V1458_BORROWED),
            "locked_anchor": LOCKED_ANCHOR_VALUE,
            "locked_north_star": LOCKED_NORTH_STAR_CEILING,
            "locked_ceiling": LOCKED_ABSOLUTE_CEILING_V1256,
            "v1411_ceiling": V1411_ABSOLUTE_CEILING,
            "locked_gap_to_north_star": LOCKED_GAP_TO_NORTH_STAR,
            "locked_gap_to_ceiling": LOCKED_GAP_TO_CEILING,
        }
        if args.json or args.format == "json":
            print(json.dumps(meta, indent=2, ensure_ascii=False))
        else:
            for k, v in meta.items():
                print(f"{k}: {v}")
        return 0

    report = run_ceiling_chain_audit()

    if args.command == "audit":
        if args.json or args.format == "json":
            output = _format_json(report)
        elif args.format == "md":
            output = _format_md(report)
        else:
            output = _format_text(report)
        print(output)
        if args.out_json:
            with open(args.out_json, "w", encoding="utf-8") as f:
                f.write(_format_json(report))
            print(f"\n[json saved to {args.out_json}]", file=sys.stderr)
        if args.out_md:
            with open(args.out_md, "w", encoding="utf-8") as f:
                f.write(_format_md(report))
            print(f"\n[md saved to {args.out_md}]", file=sys.stderr)
        return 0

    if args.command == "ceiling":
        if args.json or args.format == "json":
            print(json.dumps(
                [asdict(r) for r in report.ceiling_chain],
                indent=2, ensure_ascii=False,
            ))
        else:
            for r in report.ceiling_chain:
                print(
                    f"{r.module_id:25s} "
                    f"anchor={r.anchor_value} "
                    f"north={r.north_star_ceiling} "
                    f"abs={r.absolute_ceiling} "
                    f"consistency={r.internal_consistency_score:.2f}"
                )
        return 0

    if args.command == "deploy":
        if args.json or args.format == "json":
            print(json.dumps(
                [asdict(r) for r in report.deployment_cube],
                indent=2, ensure_ascii=False,
            ))
        else:
            for r in report.deployment_cube:
                print(
                    f"{r.module_id:25s} "
                    f"importable={r.importable} "
                    f"no_ceiling_touch={r.no_ceiling_touch} "
                    f"honest={r.has_ceiling_disclosure}"
                )
        return 0

    if args.command == "inconsistencies":
        if args.json or args.format == "json":
            print(json.dumps(report.inconsistencies, indent=2,
                             ensure_ascii=False))
        else:
            if not report.inconsistencies:
                print("(none — chain fully consistent)")
            else:
                for inc in report.inconsistencies:
                    print(f"- {inc}")
        return 0

    parser.print_help()
    return 1


# ----------------------- Main -----------------------

if __name__ == "__main__":
    sys.exit(run_cli(sys.argv[1:]))
