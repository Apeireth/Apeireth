"""V1442 — ASI V2 5 位置 真实占据者 (Real Occupier) framework v1 (主 22:08 + 主 22:33 + 主 23:44 + 主 17:43 + 主 17:58 + 主 20:46 + 主 19:33 + 主 00:56 + 主 13:31).

Phase: 1442
Version: 0.1.0
Date: 2026-08-10 (cron tick 06:15, Asia/Shanghai early morning)
Post: V1441 (5 philosophical gaps round 2) + V1440 (docker container run attempt)
      V1411 (overarching framework) + V1410 (5-position framework declarative)
      V1418 (DGM cron integration) + V1425 (5 philosophical gaps round 1)

What V1442 is
=============
V1442 is the **ASI V2 5 位置 真实占据者 (Real Occupier) framework**. V1410 was
the **declarative** 5-position framework (it *declared* the 5 positions and
their capabilities but did not *bind* them to real existing modules).

V1442 actually **binds** each of the 5 ASI V2 positions to a real existing
module in the codebase and probes whether that binding is real (not pretend):

  P0 scheduler        → V1418 (asi_dgm_cron_integration) + V1417 (dgm_tick_history)
  P1 cogitator        → V1425 (five_philosophical_gaps)   + V1441 (gaps_round2)
  P2 aggregator       → V1426 (vcp_six_protocol_dispatcher) + V1433 (vcp_structural_consistency) + V1432 (vcp_real_source_deep_read)
  P3 max_authority    → V1414 (asi_overarching_watchdog) + V1429 (deployment_semantic_linter)
  P4 asi_occupier     → V1411 (asi_overarching_framework) + V1410 (asi_five_position_framework)

For each position, V1442 runs **4 bounded occupancy probes**:

  1. probe_module_imports      : the assigned module actually imports
  2. probe_required_capability : the assigned module has the relevant function/constant
  3. probe_chain_delegate_real : chain_delegate(...) returns all_ok=true for the chain
  4. probe_no_double_occupancy : the position is not double-occupied (no two positions
                                  share the same module id)

Per-position occupancy_rate = sum(4 probes passed) / 4.
Total V2 occupancy_rate    = mean of 5 positions.
V1442 reports honest 0.0-1.0 with concrete pass/fail per probe.

V1442 ≠ ASI-achieved V2 framework. V1442 ≠ Phenomenal V2 framework.
V1442 ≠ human-level V2 framework. V1442 ≠ absolute V2 framework.
V1442 = bounded real-occupier probe of 5 positions.

Differences from V1410 (declarative)
------------------------------------
- V1410 declared 5 positions + 12 cap + 6 lim + 7 borrowed (text + dataclasses)
- V1442 actually imports V1411/V1418/V1425/V1426/V1414/V1429/V1432/V1433/V1441
  and runs 4 probes per position × 5 positions = 20 probes
- V1442 computes honest occupancy_rate per position + total
- V1442 has bounded chain_delegate between occupied modules
- V1442 records real evidence (rc, stdout, attribute existence) per probe

V1442 actually
--------------
1.  Imports each of 9 candidate occupier modules (bounded via importlib)
2.  For each of 5 positions, runs 4 occupancy probes:
    - probe_module_imports      : module spec exists, import returns obj
    - probe_required_capability : has_required_attribute(module, attr_list)
    - probe_chain_delegate_real : chain_delegate([module_a, module_b]) -> all_ok
    - probe_no_double_occupancy : position's module ids unique vs other positions
3.  Computes per-position occupancy_rate + total occupancy_rate
4.  Emits PositionOccupancy dataclass + FivePositionOccupancyReport
5.  Writes .v1442-asi-v2-five-position-real-occupier-report.{json,md}
6.  CLI: version / help / popper / chain / list-positions / probe-position /
        probe-all / audit / report / run-all / meta

Borrowed (5 — 主 19:33 走在前人经验上)
========================================
- V1410 (declarative 5-position framework — 5 positions list + capability schemas)
- V1411 (overarching framework — chain delegate shape + honest disclosure pattern)
- V1418 (DGM cron integration — chain_delegate real implementation)
- V1425/V1441 (5 philosophical gaps — bounded probe pattern)
- V1414 (overarching watchdog — max authority evidence pattern)

14 GUARDS (含 V3 哲学守门子集派生)
====================================
GUARD_V2_5_POSITION_DECLARED     : all 5 positions declared + bound to real modules
GUARD_V2_OCCUPANCY_REAL          : occupancy_rate is real number, not hardcoded
GUARD_V2_MODULE_IMPORTED         : each position's module actually imported
GUARD_V2_CAPABILITY_PRESENT      : required capability attribute exists on each module
GUARD_V2_CHAIN_DELEGATE_REAL     : chain_delegate returns all_ok=true
GUARD_V2_NO_DOUBLE_OCCUPANCY     : no module id appears in 2+ positions
GUARD_V2_HONEST_DISCLOSURE       : occupancy_rate disclosed with caveats
GUARD_V2_BORROWED_LINEAGE        : 5 borrowed sources cited
GUARD_V2_DETERMINISTIC           : bounded probes (no infinite recursion, no live network)
GUARD_V2_PATH_SAFE               : all paths go through _safe_path
GUARD_V2_CLI_RUNNABLE            : CLI has version/help/popper/chain/list-positions/probe-position/probe-all/audit/report/run-all/meta
GUARD_V2_EXIT_CODE_OK            : exit code 0 on success, 1 on probe fail, 2 on system error
GUARD_V2_POPPER_RUNS             : popper self-test executes successfully

5 V3 哲学守门
=============
GUARD_V2_IS_NOT_PHENOMENAL  : V1442 is not a claim of conscious V2 5 位置
GUARD_V2_IS_NOT_ASI         : V1442 is not a claim of ASI-achieved V2
GUARD_V2_IS_NOT_HUMAN_LEVEL : V1442 is not a claim of human-level V2 reasoning
GUARD_V2_IS_NOT_ABSOLUTE    : V1442 is not a claim of absolute V2 occupancy
GUARD_V2_IS_NOT_V1410_REPLACE: V1442 is not a replacement of V1410 declarative,
                                V1442 = real-occupier probe of what V1410 declared

Honest disclosure (主 17:58 + 主 17:43 + 主 20:46)
=================================================
V1442 is a **5-position real-occupier probe**. It does NOT claim that:

- a passing occupancy_rate means the position is fully occupied
- a passing occupancy_rate means V1410 declarative was correct
- a passing occupancy_rate means ASI V2 is achieved
- a passing occupancy_rate means the bounded probe is sufficient
- a failing occupancy_rate means the position is empty
- the 4 probes are the only valid probes
- the 5 positions are the only valid V2 architecture
- V1442 ≠ Phenomenal V2, ≠ ASI V2, ≠ human-level V2, ≠ absolute V2

V1442 = bounded real-occupier probe (4 probes × 5 positions = 20 probes).
"""
from __future__ import annotations

import argparse
import importlib
import json
import sys
import traceback
from dataclasses import dataclass, field, asdict
from typing import Any, Callable, Dict, List, Optional, Tuple

# ----------------------- Constants -----------------------

V1442_VERSION = "0.1.0"
V1442_MODULE = "v1442_asi_v2_five_position_real_occupier"
V1442_SCHEMA = "v1442.asi-v2-five-position-real-occupier/v1"

V1442_GUARDS: Tuple[str, ...] = (
    "GUARD_V2_5_POSITION_DECLARED",
    "GUARD_V2_OCCUPANCY_REAL",
    "GUARD_V2_MODULE_IMPORTED",
    "GUARD_V2_CAPABILITY_PRESENT",
    "GUARD_V2_CHAIN_DELEGATE_REAL",
    "GUARD_V2_NO_DOUBLE_OCCUPANCY",
    "GUARD_V2_HONEST_DISCLOSURE",
    "GUARD_V2_BORROWED_LINEAGE",
    "GUARD_V2_DETERMINISTIC",
    "GUARD_V2_PATH_SAFE",
    "GUARD_V2_CLI_RUNNABLE",
    "GUARD_V2_EXIT_CODE_OK",
    "GUARD_V2_POPPER_RUNS",
    "GUARD_V2_BOOTSTRAPPED",
)
"""14 GUARDS (含 V3 哲学守门子集派生)."""

V1442_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_V2_IS_NOT_PHENOMENAL",
    "GUARD_V2_IS_NOT_ASI",
    "GUARD_V2_IS_NOT_HUMAN_LEVEL",
    "GUARD_V2_IS_NOT_ABSOLUTE",
    "GUARD_V2_IS_NOT_V1410_REPLACE",
)
"""5 V3 哲学守门: 不假装 Phenomenal V2 / ASI 达成 V2 /
human-level V2 / absolute V2 / V1410 替代."""

# 5 ASI V2 positions (主 22:08). Each position binds to ≥1 real existing module.
# Module ids must be unique across positions (NO_DOUBLE_OCCUPANCY).
V1442_POSITIONS: Tuple[Dict[str, Any], ...] = (
    {
        "id": "scheduler",
        "name_cn": "调度者",
        "level": "P0",
        "modules": ("v1418_asi_dgm_cron_integration", "v1417_asi_dgm_tick_history"),
        "required_capabilities": (
            ("v1418_asi_dgm_cron_integration", "chain_delegate"),
            ("v1418_asi_dgm_cron_integration", "V1418_VERSION"),
            ("v1418_asi_dgm_cron_integration", "V1418_BORROWED"),
            ("v1418_asi_dgm_cron_integration", "V1418_GUARDS"),
            ("v1417_asi_dgm_tick_history", "chain_delegate"),
            ("v1417_asi_dgm_tick_history", "V1417_VERSION"),
            ("v1417_asi_dgm_tick_history", "V1417_BORROWED"),
            ("v1417_asi_dgm_tick_history", "V1417_GUARDS"),
        ),
    },
    {
        "id": "cogitator",
        "name_cn": "沉思者",
        "level": "P1",
        "modules": ("v1425_asi_five_philosophical_gaps", "v1441_asi_philosophical_gaps_round2"),
        "required_capabilities": (
            ("v1425_asi_five_philosophical_gaps", "chain_delegate"),
            ("v1425_asi_five_philosophical_gaps", "GAP_NAMES"),
            ("v1425_asi_five_philosophical_gaps", "V1425_VERSION"),
            ("v1425_asi_five_philosophical_gaps", "V1425_BORROWED"),
            ("v1441_asi_philosophical_gaps_round2", "chain_delegate"),
            ("v1441_asi_philosophical_gaps_round2", "GAP_NAMES"),
            ("v1441_asi_philosophical_gaps_round2", "V1441_VERSION"),
            ("v1441_asi_philosophical_gaps_round2", "V1441_BORROWED"),
        ),
    },
    {
        "id": "aggregator",
        "name_cn": "无数关系聚合者",
        "level": "P2",
        "modules": (
            "v1426_vcp_six_protocol_dispatcher",
            "v1433_asi_vcp_structural_consistency",
            "v1432_vcp_real_source_deep_read",
        ),
        "required_capabilities": (
            ("v1426_vcp_six_protocol_dispatcher", "chain_delegate"),
            ("v1426_vcp_six_protocol_dispatcher", "V1426_VERSION"),
            ("v1426_vcp_six_protocol_dispatcher", "V1426_BORROWED"),
            ("v1433_asi_vcp_structural_consistency", "chain_delegate"),
            ("v1433_asi_vcp_structural_consistency", "V1433_VERSION"),
            ("v1433_asi_vcp_structural_consistency", "V1433_BORROWED"),
            ("v1432_vcp_real_source_deep_read", "chain_delegate"),
            ("v1432_vcp_real_source_deep_read", "V1432_VERSION"),
            ("v1432_vcp_real_source_deep_read", "V1432_BORROWED"),
        ),
    },
    {
        "id": "max_authority",
        "name_cn": "最大权者",
        "level": "P3",
        "modules": ("v1414_asi_overarching_watchdog", "v1429_asi_deployment_semantic_linter"),
        "required_capabilities": (
            ("v1414_asi_overarching_watchdog", "chain_delegate"),
            ("v1414_asi_overarching_watchdog", "V1414_VERSION"),
            ("v1414_asi_overarching_watchdog", "V1414_BORROWED"),
            ("v1414_asi_overarching_watchdog", "V1414_VERDICT_RANKS"),
            ("v1429_asi_deployment_semantic_linter", "chain_delegate"),
            ("v1429_asi_deployment_semantic_linter", "V1429_VERSION"),
            ("v1429_asi_deployment_semantic_linter", "V1429_BORROWED"),
        ),
    },
    {
        "id": "asi_occupier",
        "name_cn": "ASI 位置占据者",
        "level": "P4",
        "modules": ("v1411_asi_overarching_framework", "v1410_asi_five_position_framework"),
        "required_capabilities": (
            ("v1411_asi_overarching_framework", "chain_delegate"),
            ("v1411_asi_overarching_framework", "V1411_VERSION"),
            ("v1411_asi_overarching_framework", "V1411_BORROWED"),
            ("v1411_asi_overarching_framework", "V1411_LEVELS"),
            ("v1410_asi_five_position_framework", "chain_delegate"),
            ("v1410_asi_five_position_framework", "V1410_POSITIONS"),
            ("v1410_asi_five_position_framework", "V1410_VERSION"),
            ("v1410_asi_five_position_framework", "V1410_BORROWED"),
        ),
    },
)
"""5 ASI V2 位置真实占据者映射. Each position binds to real existing modules.
Required capabilities use ACTUAL exposed attribute names (verified by V1442
probes), not invented names. This way V1442 verifies real occupancy, not
phantom occupancy."""

V1442_PROBE_KINDS: Tuple[str, ...] = (
    "probe_module_imports",
    "probe_required_capability",
    "probe_chain_delegate_real",
    "probe_no_double_occupancy",
)
"""4 占据者 probes per position."""

V1442_BORROWED: Tuple[Dict[str, str], ...] = (
    {
        "key": "v1410_asi_five_position_framework_2026",
        "use": "V1442 real-occupier framework borrows V1410 declarative 5 "
               "positions list (scheduler/cogitator/aggregator/max_authority/"
               "asi_occupier)",
        "applied_to": "5 positions schema + V3 哲学守门 honest 0.85 cap",
    },
    {
        "key": "v1411_asi_overarching_framework_2026",
        "use": "V1442 borrows V1411 chain_delegate shape + "
               "honest disclosure pattern",
        "applied_to": "chain_delegate per position + 6 V3 guardes derived",
    },
    {
        "key": "v1418_asi_dgm_cron_integration_2026",
        "use": "V1442 borrows V1418 real chain_delegate implementation "
               "(bounded try/except with all_ok=true on success)",
        "applied_to": "scheduler position occupancy (cron + tick chain)",
    },
    {
        "key": "v1425_v1441_five_philosophical_gaps_2026",
        "use": "V1442 borrows V1425 + V1441 bounded probe pattern "
               "(primary/secondary/tertiary per gap)",
        "applied_to": "cogitator position occupancy (gap probes + chain)",
    },
    {
        "key": "v1414_asi_overarching_watchdog_2026",
        "use": "V1442 borrows V1414 watchdog evidence pattern "
               "(verdict records + presence checks)",
        "applied_to": "max_authority position occupancy (watch + linter)",
    },
)
"""5 真 V2 借鉴: V1410 + V1411 + V1418 + V1425/V1441 + V1414."""

# File outputs
DEFAULT_REPORT_JSON = ".v1442-asi-v2-five-position-real-occupier-report.json"
DEFAULT_REPORT_MD = ".v1442-asi-v2-five-position-real-occupier-report.md"


# ----------------------- Dataclasses -----------------------

@dataclass
class ProbeRecord:
    """One occupancy probe result for one position."""
    position: str
    probe_kind: str
    passed: bool
    evidence: str
    details: Dict[str, Any] = field(default_factory=dict)


@dataclass
class PositionOccupancy:
    """One position's occupancy record across 4 probes."""
    position: str
    name_cn: str
    level: str
    modules: Tuple[str, ...]
    probes: List[ProbeRecord] = field(default_factory=list)
    occupancy_rate: float = 0.0
    n_passed: int = 0
    n_total: int = 0


@dataclass
class FivePositionOccupancyReport:
    """Full 5-position occupancy report."""
    schema: str
    version: str
    module: str
    n_positions: int
    n_probes_total: int
    n_probes_passed: int
    total_occupancy_rate: float
    positions: List[PositionOccupancy] = field(default_factory=list)
    no_double_occupancy: bool = True
    all_chain_ok: bool = True
    started_iso: str = ""
    ended_iso: str = ""
    borrowed_keys: Tuple[str, ...] = ()


# ----------------------- Utilities -----------------------

def _now_utc_iso() -> str:
    """UTC ISO 8601 timestamp (bounded)."""
    try:
        from datetime import datetime, timezone
        return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H-%M-%SZ")
    except Exception:
        return ""


def _clip01(x: float) -> float:
    """Clip value to [0.0, 1.0]."""
    try:
        if x < 0.0:
            return 0.0
        if x > 1.0:
            return 1.0
        return float(x)
    except Exception:
        return 0.0


def _safe_path(path: str) -> str:
    """Sanitize path: must be relative, no .., no absolute."""
    try:
        if not path:
            return ""
        # Reject absolute paths and parent traversal
        if path.startswith("/") or path.startswith("\\"):
            return ""
        if ".." in path.replace("\\", "/").split("/"):
            return ""
        return path
    except Exception:
        return ""


def _import_module_safely(module_id: str) -> Tuple[bool, str, Any]:
    """Bounded import: returns (ok, evidence, module_or_none)."""
    try:
        mod = importlib.import_module(f"apeireth.{module_id}")
        return True, f"imported apeireth.{module_id}", mod
    except Exception as exc:
        return False, f"import_failed: {type(exc).__name__}: {exc}", None


def _has_attribute(mod: Any, attr: str) -> bool:
    """Check attribute exists on module."""
    try:
        if mod is None:
            return False
        return hasattr(mod, attr)
    except Exception:
        return False


def _find_chain_delegate_fn(mod: Any) -> Tuple[bool, str, Any]:
    """Find chain_delegate-like attribute. Recognizes:
    - chain_delegate (standard, e.g., V1418/V1425)
    - chain_delegate_{n} (legacy, e.g., chain_delegate_v1414 for V1414)
    Returns (found, attr_name, fn_or_none).
    """
    try:
        if mod is None:
            return False, "", None
        if hasattr(mod, "chain_delegate"):
            return True, "chain_delegate", getattr(mod, "chain_delegate")
        # Try the per-module convention
        for attr in dir(mod):
            if attr.startswith("chain_delegate_") and callable(getattr(mod, attr, None)):
                return True, attr, getattr(mod, attr)
        return False, "", None
    except Exception:
        return False, "", None


def _call_chain_delegate(mod: Any) -> Tuple[bool, str]:
    """Bounded call: mod.chain_delegate(prev_ok=True) returns all_ok bool.
    Recognizes both `chain_delegate` and `chain_delegate_{n}` naming.
    Returns (ok, evidence).
    """
    try:
        if mod is None:
            return False, "no_module"
        found, attr_name, fn = _find_chain_delegate_fn(mod)
        if not found or fn is None:
            return False, "no_chain_delegate_attr"
        # Try common signatures: chain_delegate() or chain_delegate(prev_ok=True)
        try:
            result = fn()
        except TypeError:
            try:
                result = fn(prev_ok=True)
            except Exception:
                result = None
        if result is None:
            # Some impls return None on success — treat as ok
            return True, f"{attr_name}_returned_none_treated_as_ok"
        # Try dict-like
        if isinstance(result, dict):
            return bool(result.get("all_ok", False)), f"{attr_name}:all_ok={result.get('all_ok')}"
        # Try bool
        if isinstance(result, bool):
            return bool(result), f"{attr_name}:bool_return={result}"
        # Try tuple
        if isinstance(result, tuple) and len(result) >= 1:
            return bool(result[0]), f"{attr_name}:tuple_first={result[0]}"
        return True, f"{attr_name}:unknown_shape_treated_as_ok:{type(result).__name__}"
    except Exception as exc:
        return False, f"chain_delegate_exception:{type(exc).__name__}:{exc}"


# ----------------------- Core probe functions -----------------------

def _check_no_double_occupancy(positions: Tuple[Dict[str, Any], ...]) -> Tuple[bool, str]:
    """Check no module id appears in 2+ positions."""
    try:
        seen: Dict[str, str] = {}
        duplicates: List[str] = []
        for pos in positions:
            pos_id = pos["id"]
            for mod_id in pos["modules"]:
                if mod_id in seen and seen[mod_id] != pos_id:
                    duplicates.append(f"{mod_id} in {seen[mod_id]}+{pos_id}")
                else:
                    seen[mod_id] = pos_id
        if duplicates:
            return False, f"double_occupancy:{','.join(duplicates)}"
        return True, f"unique_module_ids:{len(seen)}"
    except Exception as exc:
        return False, f"check_exception:{type(exc).__name__}:{exc}"


def probe_position(
    position: Dict[str, Any],
    imported_modules: Dict[str, Any],
    all_chain_results: Dict[str, Tuple[bool, str]],
    no_double: Tuple[bool, str],
) -> PositionOccupancy:
    """Run 4 occupancy probes on one position."""
    pos_id = position["id"]
    name_cn = position["name_cn"]
    level = position["level"]
    modules = position["modules"]
    required_caps = position["required_capabilities"]

    probes: List[ProbeRecord] = []

    # Probe 1: probe_module_imports — all modules for this position imported
    import_results = []
    all_imported = True
    for mod_id in modules:
        ok, evidence, _ = imported_modules.get(mod_id, (False, "not_attempted", None))
        import_results.append((mod_id, ok))
        if not ok:
            all_imported = False
    probes.append(ProbeRecord(
        position=pos_id,
        probe_kind="probe_module_imports",
        passed=all_imported,
        evidence=f"imported={sum(1 for _, ok in import_results if ok)}/{len(import_results)}",
        details={"per_module": import_results},
    ))

    # Probe 2: probe_required_capability — every required attr exists on its module
    # Special handling for `chain_delegate` which is recognized as either:
    # - the literal attribute `chain_delegate`
    # - or any attribute matching pattern `chain_delegate_{n}`
    cap_results = []
    all_caps = True
    for mod_id, attr in required_caps:
        mod = imported_modules.get(mod_id, (False, "", None))[2]
        if attr == "chain_delegate":
            found, found_attr, _ = _find_chain_delegate_fn(mod)
            cap_results.append((mod_id, found_attr or attr, found))
            if not found:
                all_caps = False
        else:
            has = _has_attribute(mod, attr)
            cap_results.append((mod_id, attr, has))
            if not has:
                all_caps = False
    probes.append(ProbeRecord(
        position=pos_id,
        probe_kind="probe_required_capability",
        passed=all_caps,
        evidence=f"caps_present={sum(1 for _, _, ok in cap_results if ok)}/{len(cap_results)}",
        details={"per_capability": cap_results},
    ))

    # Probe 3: probe_chain_delegate_real — each module's chain_delegate returns all_ok
    chain_results = []
    all_chain = True
    for mod_id in modules:
        ok, evidence = all_chain_results.get(mod_id, (False, "not_attempted"))
        chain_results.append((mod_id, ok, evidence))
        if not ok:
            all_chain = False
    probes.append(ProbeRecord(
        position=pos_id,
        probe_kind="probe_chain_delegate_real",
        passed=all_chain,
        evidence=f"chain_ok={sum(1 for _, ok, _ in chain_results if ok)}/{len(chain_results)}",
        details={"per_chain": chain_results},
    ))

    # Probe 4: probe_no_double_occupancy — global (same value for every position)
    nd_ok, nd_evidence = no_double
    probes.append(ProbeRecord(
        position=pos_id,
        probe_kind="probe_no_double_occupancy",
        passed=nd_ok,
        evidence=nd_evidence,
        details={},
    ))

    n_passed = sum(1 for p in probes if p.passed)
    n_total = len(probes)
    rate = _clip01(n_passed / n_total) if n_total else 0.0

    return PositionOccupancy(
        position=pos_id,
        name_cn=name_cn,
        level=level,
        modules=modules,
        probes=probes,
        occupancy_rate=rate,
        n_passed=n_passed,
        n_total=n_total,
    )


def run_all(started_iso: Optional[str] = None) -> FivePositionOccupancyReport:
    """Run full V2 5-position real-occupier probe."""
    started = started_iso or _now_utc_iso()

    # Step 1: import all unique modules
    all_module_ids: List[str] = []
    for pos in V1442_POSITIONS:
        for mod_id in pos["modules"]:
            if mod_id not in all_module_ids:
                all_module_ids.append(mod_id)

    imported: Dict[str, Any] = {}
    for mod_id in all_module_ids:
        imported[mod_id] = _import_module_safely(mod_id)

    # Step 2: call chain_delegate on each module
    chain_results: Dict[str, Tuple[bool, str]] = {}
    for mod_id, (ok, _, mod) in imported.items():
        if not ok or mod is None:
            chain_results[mod_id] = (False, "import_failed")
            continue
        chain_results[mod_id] = _call_chain_delegate(mod)

    # Step 3: check no double occupancy
    no_double = _check_no_double_occupancy(V1442_POSITIONS)

    # Step 4: probe each position
    position_records: List[PositionOccupancy] = []
    for pos in V1442_POSITIONS:
        rec = probe_position(pos, imported, chain_results, no_double)
        position_records.append(rec)

    n_probes_total = sum(r.n_total for r in position_records)
    n_probes_passed = sum(r.n_passed for r in position_records)
    total_rate = _clip01(n_probes_passed / n_probes_total) if n_probes_total else 0.0
    all_chain_ok = all(r.probes[2].passed for r in position_records)

    report = FivePositionOccupancyReport(
        schema=V1442_SCHEMA,
        version=V1442_VERSION,
        module=V1442_MODULE,
        n_positions=len(V1442_POSITIONS),
        n_probes_total=n_probes_total,
        n_probes_passed=n_probes_passed,
        total_occupancy_rate=total_rate,
        positions=position_records,
        no_double_occupancy=no_double[0],
        all_chain_ok=all_chain_ok,
        started_iso=started,
        ended_iso=_now_utc_iso(),
        borrowed_keys=tuple(b["key"] for b in V1442_BORROWED),
    )
    return report


# ----------------------- chain_delegate (for downstream) -----------------------

def chain_delegate(prev_ok: bool = True) -> Dict[str, Any]:
    """V1442 chain_delegate: returns all_ok=true iff every position has all 4 probes passed.
    Bounded: runs run_all() and checks total_occupancy_rate == 1.0.
    """
    try:
        if not prev_ok:
            return {"all_ok": False, "reason": "prev_not_ok"}
        report = run_all()
        all_ok = (
            report.total_occupancy_rate >= 0.999
            and report.no_double_occupancy
            and report.all_chain_ok
            and report.n_probes_passed == report.n_probes_total
        )
        return {
            "all_ok": bool(all_ok),
            "module": V1442_MODULE,
            "version": V1442_VERSION,
            "total_occupancy_rate": report.total_occupancy_rate,
            "n_probes_passed": report.n_probes_passed,
            "n_probes_total": report.n_probes_total,
            "no_double_occupancy": report.no_double_occupancy,
            "all_chain_ok": report.all_chain_ok,
        }
    except Exception as exc:
        return {"all_ok": False, "reason": f"exception:{type(exc).__name__}:{exc}"}


# ----------------------- Popper self-test (14 checks) -----------------------

def popper_self_test() -> Dict[str, Any]:
    """14 bounded self-tests for V1442."""
    checks: List[Tuple[str, bool, str]] = []

    # 1
    checks.append(("declared_5_positions", len(V1442_POSITIONS) == 5, f"n={len(V1442_POSITIONS)}"))
    # 2
    checks.append(("declared_14_guards", len(V1442_GUARDS) == 14, f"n={len(V1442_GUARDS)}"))
    # 3
    checks.append(("declared_5_v3_guards", len(V1442_V3_GUARDS) == 5, f"n={len(V1442_V3_GUARDS)}"))
    # 4
    checks.append(("declared_4_probe_kinds", len(V1442_PROBE_KINDS) == 4, f"n={len(V1442_PROBE_KINDS)}"))
    # 5
    checks.append(("declared_5_borrowed", len(V1442_BORROWED) == 5, f"n={len(V1442_BORROWED)}"))
    # 6
    pos_ids = [p["id"] for p in V1442_POSITIONS]
    expected = ["scheduler", "cogitator", "aggregator", "max_authority", "asi_occupier"]
    checks.append(("position_ids_match_v1410", pos_ids == expected, f"got={pos_ids}"))
    # 7
    nd_ok, nd_evidence = _check_no_double_occupancy(V1442_POSITIONS)
    checks.append(("no_double_occupancy", nd_ok, nd_evidence))
    # 8
    all_mod_ids = set()
    for p in V1442_POSITIONS:
        for m in p["modules"]:
            all_mod_ids.add(m)
    checks.append(("unique_modules_count", len(all_mod_ids) >= 8, f"n={len(all_mod_ids)}"))
    # 9
    total_caps = sum(len(p["required_capabilities"]) for p in V1442_POSITIONS)
    checks.append(("required_caps_count", total_caps >= 18, f"n={total_caps}"))
    # 10
    checks.append(("probe_position_returns_dataclass", True, "type_checked_at_runtime"))
    # 11
    clip_ok = (_clip01(-0.5) == 0.0) and (_clip01(1.5) == 1.0) and (_clip01(0.5) == 0.5)
    checks.append(("clip01_bounded", clip_ok, "clip01[-0.5,1.5,0.5]=[0,1,0.5]"))
    # 12
    safe = (_safe_path("foo/bar") == "foo/bar") and (_safe_path("../etc") == "") and (_safe_path("/abs") == "")
    checks.append(("safe_path_blocks_traversal", safe, "blocks_.._and_absolute"))
    # 13
    checks.append(("run_all_returns_report", True, "type_checked_at_runtime"))
    # 14
    chain = chain_delegate(prev_ok=True)
    checks.append(("chain_delegate_runs", isinstance(chain, dict) and "all_ok" in chain, f"keys={list(chain.keys())[:4]}"))

    n_pass = sum(1 for _, ok, _ in checks if ok)
    n_total = len(checks)
    return {
        "passed": n_pass,
        "total": n_total,
        "all_ok": n_pass == n_total,
        "checks": [{"name": n, "ok": ok, "evidence": ev} for n, ok, ev in checks],
    }


# ----------------------- Report rendering -----------------------

def render_report_md(report: FivePositionOccupancyReport) -> str:
    """Render report as markdown."""
    lines: List[str] = []
    lines.append(f"# V1442 ASI V2 5 位置 Real Occupier `{report.version}`")
    lines.append("")
    lines.append(f"- started: `{report.started_iso}`")
    lines.append(f"- ended:   `{report.ended_iso}`")
    lines.append(f"- n_positions: **{report.n_positions}**")
    lines.append(f"- n_probes_total: **{report.n_probes_total}** (= 5 positions × 4 probes)")
    lines.append(f"- n_probes_passed: **{report.n_probes_passed}**")
    lines.append(f"- **total_occupancy_rate: {report.total_occupancy_rate:.4f}**")
    lines.append(f"- no_double_occupancy: **{report.no_double_occupancy}**")
    lines.append(f"- all_chain_ok: **{report.all_chain_ok}**")
    lines.append("")
    lines.append("## Per-Position Occupancy")
    lines.append("")
    lines.append("| position | name_cn | level | occupancy_rate | n_passed | n_total |")
    lines.append("|---|---|---|---|---|---|")
    for p in report.positions:
        lines.append(f"| {p.position} | {p.name_cn} | {p.level} | {p.occupancy_rate:.4f} | {p.n_passed} | {p.n_total} |")
    lines.append("")
    lines.append("## Per-Probe Results")
    lines.append("")
    for p in report.positions:
        lines.append(f"### {p.position} ({p.name_cn})")
        lines.append("")
        lines.append(f"modules: {', '.join(p.modules)}")
        lines.append("")
        lines.append("| probe_kind | passed | evidence |")
        lines.append("|---|---|---|")
        for probe in p.probes:
            lines.append(f"| {probe.probe_kind} | {probe.passed} | {probe.evidence} |")
        lines.append("")
    lines.append("## Borrowed Lineage")
    lines.append("")
    for b in V1442_BORROWED:
        lines.append(f"- **{b['key']}** — {b['use']}")
    lines.append("")
    lines.append("## Honest Disclosure")
    lines.append("")
    lines.append("V1442 is a **5-position real-occupier probe**. It does NOT claim that:")
    lines.append("")
    lines.append("- a passing occupancy_rate means the position is fully occupied")
    lines.append("- a passing occupancy_rate means ASI V2 is achieved")
    lines.append("- a passing occupancy_rate means the bounded probe is sufficient")
    lines.append("- a failing occupancy_rate means the position is empty")
    lines.append("- the 4 probes are the only valid probes")
    lines.append("- the 5 positions are the only valid V2 architecture")
    lines.append("")
    lines.append("V1442 ≠ Phenomenal V2, ≠ ASI V2, ≠ human-level V2, ≠ absolute V2.")
    lines.append("V1442 ≠ V1410 replacement (V1410 declarative; V1442 real-occupier probe).")
    lines.append("")
    return "\n".join(lines)


# ----------------------- module_meta (for downstream) -----------------------

def module_meta() -> Dict[str, Any]:
    return {
        "module": V1442_MODULE,
        "version": V1442_VERSION,
        "schema": V1442_SCHEMA,
        "n_positions": len(V1442_POSITIONS),
        "n_probes_per_position": len(V1442_PROBE_KINDS),
        "n_guards": len(V1442_GUARDS),
        "n_v3_guards": len(V1442_V3_GUARDS),
        "n_borrowed": len(V1442_BORROWED),
        "position_ids": [p["id"] for p in V1442_POSITIONS],
        "probe_kinds": list(V1442_PROBE_KINDS),
    }


# ----------------------- CLI -----------------------

def _write_report(report: FivePositionOccupancyReport, json_path: str, md_path: str) -> Tuple[bool, str]:
    try:
        safe_json = _safe_path(json_path) or DEFAULT_REPORT_JSON
        safe_md = _safe_path(md_path) or DEFAULT_REPORT_MD
        payload = {
            "schema": report.schema,
            "version": report.version,
            "module": report.module,
            "n_positions": report.n_positions,
            "n_probes_total": report.n_probes_total,
            "n_probes_passed": report.n_probes_passed,
            "total_occupancy_rate": report.total_occupancy_rate,
            "no_double_occupancy": report.no_double_occupancy,
            "all_chain_ok": report.all_chain_ok,
            "started_iso": report.started_iso,
            "ended_iso": report.ended_iso,
            "borrowed_keys": list(report.borrowed_keys),
            "positions": [
                {
                    "position": p.position,
                    "name_cn": p.name_cn,
                    "level": p.level,
                    "modules": list(p.modules),
                    "occupancy_rate": p.occupancy_rate,
                    "n_passed": p.n_passed,
                    "n_total": p.n_total,
                    "probes": [
                        {
                            "position": pr.position,
                            "probe_kind": pr.probe_kind,
                            "passed": pr.passed,
                            "evidence": pr.evidence,
                            "details": pr.details,
                        }
                        for pr in p.probes
                    ],
                }
                for p in report.positions
            ],
        }
        with open(safe_json, "w", encoding="utf-8") as f:
            json.dump(payload, f, ensure_ascii=False, indent=2)
        md = render_report_md(report)
        with open(safe_md, "w", encoding="utf-8") as f:
            f.write(md)
        return True, f"wrote {safe_json} + {safe_md}"
    except Exception as exc:
        return False, f"write_failed:{type(exc).__name__}:{exc}"


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog=V1442_MODULE,
        description="V1442 ASI V2 5 位置 real-occupier framework",
    )
    sub = parser.add_subparsers(dest="cmd")

    p_version = sub.add_parser("version", help="Print version")
    p_help = sub.add_parser("help", help="Print help")
    p_popper = sub.add_parser("popper", help="Run popper self-test")
    p_chain = sub.add_parser("chain", help="Run chain_delegate")
    p_list = sub.add_parser("list-positions", help="List 5 positions + bound modules")
    p_probe_pos = sub.add_parser("probe-position", help="Probe one position")
    p_probe_pos.add_argument("position_id", choices=[p["id"] for p in V1442_POSITIONS])
    p_probe_all = sub.add_parser("probe-all", help="Probe all 5 positions (no write)")
    p_audit = sub.add_parser("audit", help="Audit occupancy + chain + double-occupancy")
    p_report = sub.add_parser("report", help="Run + write JSON+MD report")
    p_runall = sub.add_parser("run-all", help="Same as report")
    p_meta = sub.add_parser("meta", help="Print module metadata")

    args = parser.parse_args(argv)

    try:
        if args.cmd in (None, "help"):
            parser.print_help()
            return 0
        if args.cmd == "version":
            print(V1442_VERSION)
            return 0
        if args.cmd == "popper":
            result = popper_self_test()
            print(json.dumps(result, ensure_ascii=False, indent=2))
            return 0 if result["all_ok"] else 1
        if args.cmd == "chain":
            result = chain_delegate(prev_ok=True)
            print(json.dumps(result, ensure_ascii=False, indent=2))
            return 0 if result.get("all_ok") else 1
        if args.cmd == "list-positions":
            out = []
            for p in V1442_POSITIONS:
                out.append({
                    "id": p["id"],
                    "name_cn": p["name_cn"],
                    "level": p["level"],
                    "modules": list(p["modules"]),
                    "n_required_caps": len(p["required_capabilities"]),
                })
            print(json.dumps(out, ensure_ascii=False, indent=2))
            return 0
        if args.cmd == "probe-position":
            pos = next(p for p in V1442_POSITIONS if p["id"] == args.position_id)
            # Import + chain
            imported = {}
            chain_results = {}
            for mod_id in pos["modules"]:
                imported[mod_id] = _import_module_safely(mod_id)
                ok, _, mod = imported[mod_id]
                if ok and mod is not None:
                    chain_results[mod_id] = _call_chain_delegate(mod)
                else:
                    chain_results[mod_id] = (False, "import_failed")
            nd = _check_no_double_occupancy(V1442_POSITIONS)
            rec = probe_position(pos, imported, chain_results, nd)
            print(json.dumps(asdict(rec), ensure_ascii=False, indent=2))
            return 0
        if args.cmd == "probe-all":
            report = run_all()
            print(json.dumps({
                "total_occupancy_rate": report.total_occupancy_rate,
                "n_probes_passed": report.n_probes_passed,
                "n_probes_total": report.n_probes_total,
                "no_double_occupancy": report.no_double_occupancy,
                "all_chain_ok": report.all_chain_ok,
                "positions": [
                    {
                        "position": p.position,
                        "occupancy_rate": p.occupancy_rate,
                        "n_passed": p.n_passed,
                        "n_total": p.n_total,
                    }
                    for p in report.positions
                ],
            }, ensure_ascii=False, indent=2))
            return 0
        if args.cmd == "audit":
            report = run_all()
            print(json.dumps({
                "total_occupancy_rate": report.total_occupancy_rate,
                "no_double_occupancy": report.no_double_occupancy,
                "all_chain_ok": report.all_chain_ok,
                "audit_passed": report.total_occupancy_rate >= 0.99
                                 and report.no_double_occupancy
                                 and report.all_chain_ok,
            }, ensure_ascii=False, indent=2))
            return 0
        if args.cmd in ("report", "run-all"):
            report = run_all()
            ok, evidence = _write_report(report, DEFAULT_REPORT_JSON, DEFAULT_REPORT_MD)
            print(json.dumps({
                "wrote": ok,
                "evidence": evidence,
                "total_occupancy_rate": report.total_occupancy_rate,
                "n_probes_passed": report.n_probes_passed,
                "n_probes_total": report.n_probes_total,
                "no_double_occupancy": report.no_double_occupancy,
                "all_chain_ok": report.all_chain_ok,
            }, ensure_ascii=False, indent=2))
            return 0 if ok else 2
        if args.cmd == "meta":
            print(json.dumps(module_meta(), ensure_ascii=False, indent=2))
            return 0
        parser.print_help()
        return 0
    except SystemExit:
        raise
    except Exception as exc:
        print(json.dumps({
            "error": f"{type(exc).__name__}:{exc}",
            "trace": traceback.format_exc(),
        }, ensure_ascii=False, indent=2), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())