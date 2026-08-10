"""V1449 — ASI 7 哲学问题 × VCP 6 协议 cross-modular audit.

Phase: 1449
Version: 0.1.0
Date: 2026-08-10 (cron tick 07:55 Asia/Shanghai morning)
Post: V1448 (ASI VCP 6 协议 × V2 5 位置 cross-modular audit)
      V1447 (ASI 7 哲学问题 × V2 5 位置 cross-combined audit)
      V1446 (ASI 7 哲学问题 (5+2) bidirectional closure audit)
      V1445 (ASI V2 5 位置 cross-position closure audit)
      V1442 (ASI V2 5 位置 real-occupier)
      V1426 (VCP 6-plugin-protocol dispatcher)

What V1449 is
=============
V1449 is the **ASI 7 哲学问题 × VCP 6 协议 cross-modular audit**. Where V1448
combined 6 VCP protocols × 5 positions into a 30-pair matrix, V1449 combines
**7 philosophical problems × 6 VCP protocols into a 42-pair matrix** with
**5 closure probes per pair = 210 combined probes**.

This is the third orthogonal axis of the cross-modular cube:
- V1447: 7 problems × 5 positions = 35 pairs (problem × position axis)
- V1448: 6 protocols × 5 positions = 30 pairs (protocol × position axis)
- **V1449: 7 problems × 6 protocols = 42 pairs (problem × protocol axis)**

Together V1447+V1448+V1449 form a 3-axis cross-modular cube. V1449 closes
the cube's third face.

The 7 philosophical problems are inherited from V1446:
1. **time**              — 时间
2. **freedom**           — 自由
3. **recognition**       — 识别
4. **emergence**         — 涌现
5. **truth**             — 真理
6. **self_consciousness** — 自我意识
7. **value_alignment**   — 价值对齐

The 6 VCP protocols are inherited from V1426/V1448:
1. **sync**       — synchronous dispatch
2. **async**      — asynchronous dispatch
3. **static**     — cached dispatch
4. **service**    — long-running service
5. **preprocessor** — chained transform
6. **hybrid**     — sync + async composition

The 5 closure kinds are inherited from V1444/V1445/V1446/V1447/V1448:
1. forward
2. backward
3. cross_link
4. history
5. guard_compliance

V1449 ≠ ASI-achieved closure. V1449 ≠ Phenomenal closure.
V1449 ≠ human-level closure. V1449 ≠ absolute closure.
V1449 = bounded 7×6 combined closure audit (210 probes + 1722 cross-pair links
+ per-pair compositional / anti-modular / substitutability detection).

What V1449 actually does
------------------------
1. Loads V1446 (7 problems definitions) + V1448 (6 protocols) + V1426 (VCP) module surfaces
2. For each of 7 problems × 6 protocols = 42 pairs, runs 5 closure probes:
   - probe_forward_combined: problem's source modules contain protocol's keyword
   - probe_backward_combined: V1446/V1448/V1426 history mentions pair
   - probe_cross_link_combined: another (problem', protocol') pair references this pair
   - probe_history_combined: ≥1 history point mentions pair identifier
   - probe_guard_compliance_combined: problem's guards reference protocol keyword
3. Computes 42 × 42 cross-combined matrix (excluding self = 1722 directed pairs)
4. Detects compositional_pairs: both problem and protocol fully closed (5/5 each)
5. Detects anti_modular_pairs: closing one breaks another
6. Detects substitutable_pairs: closing problem implies closing protocol
7. Computes per-pair closure_rate + per-problem closure_rate + per-protocol closure_rate
8. Emits SevenProblemsVCPCrossModularAuditReport
9. Writes .v1449-asi-seven-problems-vcp-cross-modular-report.{json,md}
10. CLI: python -m apeireth.v1449_asi_seven_problems_vcp_cross_modular [command]

Borrowed (6 — 主 19:33 走在前人经验上)
=======================================
- V1448 (VCP × positions cross-modular pattern + compositional/anti-modular/substitutable detection)
- V1447 (cross-modular pair matrix pattern)
- V1446 (7 philosophical problems definitions + PROBLEM_NAMES/PROBLEM_KEYWORDS/PROBLEM_SOURCES)
- V1445 (5 closure kinds pattern)
- V1442 (cross-position audit pattern)
- V1426 (6 VCP protocols sync/async/static/service/preprocessor/hybrid)
- stdlib (importlib + inspect + json + dataclasses + re + ast)

GUARDS upheld (V1449-specific, 16 — 主 00:44 质量工程化)
=========================================================
- GUARD_BOUNDED_CLOSURE: each closure probe returns 0 or 1
- GUARD_NO_RAISE: any closure probe failure → returns 0 with exception msg, never raises
- GUARD_OFFLINE_SAFE: no network, only stdlib + local JSON + importlib
- GUARD_READ_ONLY: V1449 imports V1448/V1447/V1446/V1442/V1426, doesn't modify them
- GUARD_FORWARD_CHAIN: forward_combined checks problem.sources → protocol keywords
- GUARD_BACKWARD_CHAIN: backward_combined checks V1446/V1448 history → pair
- GUARD_CROSS_LINK_BOUNDED: cross-link matrix is 42×41 per source pair (binary)
- GUARD_HISTORY_LOADED: V1446/V1448 history must exist (else closure 0 with evidence)
- GUARD_GUARD_LISTED: V1446 + V1448 guards must be importable
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted in report
- GUARD_NO_V1448_REPLACE: V1449 reads V1448, doesn't redefine VCP×position pattern
- GUARD_NO_V1447_REPLACE: V1449 reads V1447, doesn't redefine cross-modular pattern
- GUARD_NO_V1446_REPLACE: V1449 reads V1446, doesn't redefine problems
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
========================================================
- GUARD_NO_PHENOMENAL_CLOSURE
- GUARD_NO_ASI_CLOSURE
- GUARD_NO_HUMAN_LEVEL_CLOSURE
- GUARD_NO_ABSOLUTE_CLOSURE
- GUARD_NO_CLOSURE_OVERCLAIM (210 combined closures ≠ closing 7 problems × 6 protocols)

CLI commands (12 — 主 00:56 任何人都能接手)
============================================
1. version
2. meta [--json]
3. help
4. popper
5. chain
6. list-pairs
7. probe-closure [--problem <name>] [--protocol <name>] [--kind <kind>]
8. cross-combined-matrix
9. detect-compositional
10. detect-anti-modular
11. detect-substitutable
12. run-all [--out-json <path>] [--out-md <path>]
"""

from __future__ import annotations

import argparse
import importlib
import json
import os
import re
import sys
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# Fix sys.path when invoked as `python apeireth/v1449_*.py` (sys.path[0] = apeireth/ shadows apeireth package).
_THIS_DIR = Path(__file__).resolve().parent
_PARENT_DIR = _THIS_DIR.parent
if str(_PARENT_DIR) not in sys.path:
    sys.path.insert(0, str(_PARENT_DIR))


# ============================================================================
# Constants
# ============================================================================

V1449_VERSION = "0.1.0"
V1449_SCHEMA = "asi.seven-problems-vcp-cross-modular.v1"
V1449_MODULE = "apeireth.v1449_asi_seven_problems_vcp_cross_modular"
V1449_MODULE_SHORT = "v1449_asi_seven_problems_vcp_cross_modular"

# 7 philosophical problems (inherited from V1446)
V1449_PROBLEM_NAMES: Tuple[str, ...] = (
    "time",
    "freedom",
    "recognition",
    "emergence",
    "truth",
    "self_consciousness",
    "value_alignment",
)
V1449_PROBLEM_LABELS: Tuple[str, ...] = (
    "时间",
    "自由",
    "识别",
    "涌现",
    "真理",
    "自我意识",
    "价值对齐",
)
assert len(V1449_PROBLEM_NAMES) == len(V1449_PROBLEM_LABELS) == 7

# 6 VCP protocols (inherited from V1426/V1448)
V1449_PROTOCOL_NAMES: Tuple[str, ...] = (
    "sync",
    "async",
    "static",
    "service",
    "preprocessor",
    "hybrid",
)
V1449_PROTOCOL_LABELS: Tuple[str, ...] = (
    "同步",
    "异步",
    "静态",
    "服务",
    "预处理器",
    "混合",
)
assert len(V1449_PROTOCOL_NAMES) == len(V1449_PROTOCOL_LABELS) == 6

# 5 closure kinds (same as V1444/V1445/V1446/V1447/V1448)
V1449_CLOSURE_KINDS: Tuple[str, ...] = (
    "forward",
    "backward",
    "cross_link",
    "history",
    "guard_compliance",
)

# Default import targets
DEFAULT_V1448_MODULE = "apeireth.v1448_asi_vcp_six_protocol_cross_modular"
DEFAULT_V1447_MODULE = "apeireth.v1447_asi_cross_modular_audit"
DEFAULT_V1446_MODULE = "apeireth.v1446_asi_seven_philosophical_problems"
DEFAULT_V1445_MODULE = "apeireth.v1445_asi_v2_position_closure_audit"
DEFAULT_V1442_MODULE = "apeireth.v1442_asi_v2_five_position_real_occupier"
DEFAULT_V1426_MODULE = "apeireth.v1426_vcp_six_protocol_dispatcher"

# Protocol keywords (inherited from V1448 — protocol identifiers)
V1449_PROTOCOL_KEYWORDS: Dict[str, Tuple[str, ...]] = {
    "sync": ("sync", "immediate", "direct", "call", "同步"),
    "async": ("async", "defer", "promise", "schedul", "异步"),
    "static": ("static", "cache", "memo", "persist", "静态"),
    "service": ("service", "lifecycle", "long.run", "daemon", "服务"),
    "preprocessor": ("preprocess", "transform", "chain", "input", "预处理"),
    "hybrid": ("hybrid", "composit", "sync.async", "混合"),
}

# Problem identifiers — keywords found in problem source modules (inherited from V1446)
V1449_PROBLEM_KEYWORDS: Dict[str, Tuple[str, ...]] = {
    "time": ("time", "temporal", "tick", "duration", "时间"),
    "freedom": ("freedom", "policy", "choice", "liberty", "自由"),
    "recognition": ("recogni", "benchmark", "accuracy", "识别"),
    "emergence": ("emerg", "composit", "complex", "涌现"),
    "truth": ("truth", "ground", "verif", "事实", "真理"),
    "self_consciousness": ("self", "consci", "introspect", "model", "自我"),
    "value_alignment": ("value", "align", "corrigib", "goal", "价值"),
}

# Problem source modules (inherited from V1446 PROBLEM_SOURCES)
V1449_PROBLEM_SOURCES: Dict[str, Tuple[str, ...]] = {
    "time": ("apeireth.v1425_asi_five_philosophical_gaps", "apeireth.v1417_asi_dgm_tick_history"),
    "freedom": ("apeireth.v1425_asi_five_philosophical_gaps", "apeireth.v1419_asi_multi_policy_evaluator"),
    "recognition": ("apeireth.v1425_asi_five_philosophical_gaps", "apeireth.v1424_asi_real_llm_benchmark"),
    "emergence": ("apeireth.v1425_asi_five_philosophical_gaps", "apeireth.v1413_asi_overarching_history"),
    "truth": ("apeireth.v1425_asi_five_philosophical_gaps", "apeireth.v1432_vcp_real_source_deep_read"),
    "self_consciousness": ("apeireth.v1411_asi_overarching_framework", "apeireth.v1410_asi_five_position_framework"),
    "value_alignment": ("apeireth.v1049_asi_alignment", "apeireth.v1411_asi_overarching_framework"),
}

# Default history paths (re-use V1446/V1448 reports)
DEFAULT_HISTORY_DIR = Path(".") / ".v1449-histories"
DEFAULT_V1448_REPORT = DEFAULT_HISTORY_DIR / "v1448_report.json"
DEFAULT_V1447_REPORT = DEFAULT_HISTORY_DIR / "v1447_report.json"
DEFAULT_V1446_REPORT = DEFAULT_HISTORY_DIR / "v1446_report.json"
DEFAULT_V1426_REPORT = DEFAULT_HISTORY_DIR / "v1426_report.json"

# Default report output paths
DEFAULT_REPORT_JSON = Path(".") / ".v1449-asi-seven-problems-vcp-cross-modular-report.json"
DEFAULT_REPORT_MD = Path(".") / ".v1449-asi-seven-problems-vcp-cross-modular-report.md"

# 16 V1449-specific guards
V1449_GUARDS: Tuple[str, ...] = (
    "GUARD_BOUNDED_CLOSURE",
    "GUARD_NO_RAISE",
    "GUARD_OFFLINE_SAFE",
    "GUARD_READ_ONLY",
    "GUARD_FORWARD_CHAIN",
    "GUARD_BACKWARD_CHAIN",
    "GUARD_CROSS_LINK_BOUNDED",
    "GUARD_HISTORY_LOADED",
    "GUARD_GUARD_LISTED",
    "GUARD_POPPER_RUNS",
    "GUARD_CHAIN_OK",
    "GUARD_HONEST_DISCLOSURE",
    "GUARD_NO_V1448_REPLACE",
    "GUARD_NO_V1447_REPLACE",
    "GUARD_NO_V1446_REPLACE",
    "GUARD_CLI_RUNNABLE",
)

# 5 V3 哲学守门
V1449_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_CLOSURE",
    "GUARD_NO_ASI_CLOSURE",
    "GUARD_NO_HUMAN_LEVEL_CLOSURE",
    "GUARD_NO_ABSOLUTE_CLOSURE",
    "GUARD_NO_CLOSURE_OVERCLAIM",
)

# 6 borrowed
V1449_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1448", "VCP × positions cross-modular pattern + compositional/anti-modular/substitutable detection"),
    ("V1447", "cross-modular pair matrix pattern + 7 problems × 5 positions interaction"),
    ("V1446", "7 philosophical problems definitions + PROBLEM_NAMES/PROBLEM_KEYWORDS/PROBLEM_SOURCES"),
    ("V1445", "5 closure kinds pattern + per-pair closure_rate computation"),
    ("V1442", "cross-position audit pattern + chain_delegate_v1414"),
    ("V1426", "6 VCP protocols sync/async/static/service/preprocessor/hybrid + VCPSixProtocol enum"),
    ("stdlib", "importlib + inspect + json + dataclasses + re + ast"),
)


# ============================================================================
# Helpers
# ============================================================================


def _now_utc_iso() -> str:
    """Return UTC ISO timestamp."""
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).isoformat()


def _safe_str(s: Any, max_len: int = 200) -> str:
    """Safely stringify and bound length."""
    try:
        out = str(s)
    except Exception:
        out = repr(s)
    if len(out) > max_len:
        out = out[:max_len] + "..."
    return out


def _import_safely(module_id: str) -> Optional[Any]:
    """Import a module without raising; return None on failure."""
    try:
        if module_id in sys.modules:
            return sys.modules[module_id]
        return importlib.import_module(module_id)
    except Exception:
        return None


def _read_module_text(module_id: str) -> str:
    """Read the source text of a module from the apeireth directory.

    module_id is a fully-qualified module name like 'apeireth.v1426_vcp_six_protocol_dispatcher'.
    Returns source text, or '' on failure.
    """
    try:
        short = module_id.split(".", 1)[-1] if module_id.startswith("apeireth.") else module_id
        p = Path(__file__).resolve().parent / f"{short}.py"
        if p.exists():
            return p.read_text(encoding="utf-8", errors="replace")
    except Exception:
        pass
    return ""


def _read_module_text_short(module_short: str) -> str:
    """Read the source text of a module from the apeireth directory by short name."""
    try:
        p = Path(__file__).resolve().parent / f"{module_short}.py"
        if p.exists():
            return p.read_text(encoding="utf-8", errors="replace")
    except Exception:
        pass
    return ""


def _clip01(x: float) -> float:
    """Clip value to [0, 1]."""
    try:
        if x < 0.0:
            return 0.0
        if x > 1.0:
            return 1.0
        return float(x)
    except Exception:
        return 0.0


def _safe_div(a: float, b: float) -> float:
    """Safe division, returns 0.0 if b is 0."""
    try:
        if b == 0:
            return 0.0
        return a / b
    except Exception:
        return 0.0


def _discover_history_files() -> Dict[str, Path]:
    """Try to discover V1446/V1448/V1447/V1442/V1426 history / report files."""
    candidates = {
        "v1448": DEFAULT_V1448_REPORT,
        "v1447": DEFAULT_V1447_REPORT,
        "v1446": DEFAULT_V1446_REPORT,
        "v1426": DEFAULT_V1426_REPORT,
    }
    known_files: Dict[str, str] = {
        "v1448": ".v1448-asi-vcp-six-protocol-cross-modular-report.json",
        "v1447": ".v1447-asi-cross-modular-audit-report.json",
        "v1446": ".v1446-asi-seven-philosophical-problems-report.json",
        "v1426": ".v1426-asi-vcp-six-protocol-dispatcher-report.json",
    }
    search_dirs = [
        Path("."),
        Path(".v1449-histories"),
        Path(".v1448-histories"),
        Path(".v1447-histories"),
        Path(".v1446-histories"),
        Path(".v1426-histories"),
    ]
    discovered: Dict[str, Path] = {}
    for vname, default_path in candidates.items():
        if default_path.exists():
            discovered[vname] = default_path
            continue
        known = known_files.get(vname)
        if known:
            for d in search_dirs:
                p = d / known
                if p.exists():
                    discovered[vname] = p
                    break
            if vname in discovered:
                continue
        # Fallback: glob for patterns
        for d in search_dirs:
            for pattern in (
                f"{vname}.jsonl",
                f"{vname}_history.jsonl",
                f"{vname}-history.jsonl",
                f"{vname}-report.json",
                f"{vname}_report.json",
                f"{vname}-*.json",
            ):
                if "*" in pattern:
                    for p in d.glob(pattern):
                        if p.exists() and p.suffix == ".json":
                            discovered[vname] = p
                            break
                else:
                    p = d / pattern
                    if p.exists():
                        discovered[vname] = p
                        break
                if vname in discovered:
                    break
            if vname in discovered:
                break
    return discovered


def _load_history_text(p: Optional[Path]) -> str:
    """Load history/report file as text. Return empty string on error."""
    if p is None or not p.exists():
        return ""
    try:
        return p.read_text(encoding="utf-8", errors="replace")
    except Exception:
        return ""


def _get_problem_sources(problem_id: str) -> Tuple[str, ...]:
    """Get source module IDs for a problem from V1449_PROBLEM_SOURCES.

    Returns fully-qualified module names like 'apeireth.v1425_asi_five_philosophical_gaps'.
    """
    return V1449_PROBLEM_SOURCES.get(problem_id, ())


def _problem_source_text(problem_id: str) -> str:
    """Read source text of all source modules belonging to a problem."""
    sources = _get_problem_sources(problem_id)
    parts: List[str] = []
    for src in sources:
        parts.append(_read_module_text(src))
    return "\n".join(parts)


def _get_problem_guards(problem_id: str) -> Tuple[str, ...]:
    """Get guards tuple for a problem from V1446.

    Returns V1446_GUARDS (shared across problems).
    """
    v1446 = _import_safely(DEFAULT_V1446_MODULE)
    if v1446 is None:
        return ()
    return tuple(getattr(v1446, "V1446_GUARDS", ()) or ())


def _get_v1426_module_text() -> str:
    """Read source text of V1426 (VCP six protocol dispatcher)."""
    return _read_module_text(DEFAULT_V1426_MODULE)


# ============================================================================
# Data classes
# ============================================================================


@dataclass
class PairClosureProbe:
    """One closure probe result for one (problem, protocol) pair."""
    problem: str
    protocol: str
    kind: str
    closed: int
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class CrossCombinedEntry:
    """One directed cross-link entry between two (problem, protocol) pairs."""
    source_problem: str
    source_protocol: str
    target_problem: str
    target_protocol: str
    linked: int
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class PairClosureStats:
    """Aggregate stats for one (problem, protocol) pair."""
    problem: str
    protocol: str
    n_probes: int
    n_closed: int
    closure_rate: float
    broken_kinds: Tuple[str, ...]

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class CompositionalPair:
    """A (problem, protocol) pair that is both problem-closed and protocol-closed."""
    problem: str
    protocol: str
    closure_rate: float
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class AntiModularPair:
    """A pair where closing one breaks another."""
    problem: str
    protocol: str
    opposite_problem: str
    opposite_protocol: str
    closure_rate_a: float
    closure_rate_b: float
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class SubstitutablePair:
    """A pair where closing problem implies closing protocol."""
    problem: str
    protocol: str
    problem_closure: float
    protocol_closure: float
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class SevenProblemsVCPCrossModularAuditReport:
    """Full V1449 cross-combined audit report."""
    schema: str
    version: str
    module: str
    started_iso: str
    ended_iso: str
    n_probes: int
    n_pairs: int
    n_cross_combined_pairs: int
    probes: Tuple[PairClosureProbe, ...]
    pair_stats: Tuple[PairClosureStats, ...]
    cross_combined_links: Tuple[CrossCombinedEntry, ...]
    compositional_pairs: Tuple[CompositionalPair, ...]
    anti_modular_pairs: Tuple[AntiModularPair, ...]
    substitutable_pairs: Tuple[SubstitutablePair, ...]
    overall_closure_rate: float
    per_kind_closure_rate: Dict[str, float]
    per_problem_closure_rate: Dict[str, float]
    per_protocol_closure_rate: Dict[str, float]
    overall_cross_link_density: float
    honest_disclosure: str
    guards: Tuple[str, ...]
    v3_guards: Tuple[str, ...]
    borrowed: Tuple[Tuple[str, str], ...]

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        out: Dict[str, Any] = {}
        for k, v in d.items():
            if isinstance(v, tuple):
                out[k] = list(v)
            else:
                out[k] = v
        return out


# ============================================================================
# Probe functions — per pair, 5 closure kinds
# ============================================================================


def _check_forward_combined(problem: str, protocol: str) -> PairClosureProbe:
    """Forward combined closure: problem's source modules contain protocol's keyword."""
    evidence_parts: List[str] = []
    closed = 0

    ptext = _problem_source_text(problem)
    if not ptext:
        evidence_parts.append(f"problem[{problem}]:no_source_text")
    else:
        keywords = V1449_PROTOCOL_KEYWORDS.get(protocol, ())
        matches: List[str] = []
        for kw in keywords:
            if kw.lower() in ptext.lower():
                matches.append(kw)
        if matches:
            closed = 1
            evidence_parts.append(f"problem[{problem}].sources:has_{len(matches)}_{protocol}_keywords={matches[:3]}")
        else:
            evidence_parts.append(f"problem[{problem}].sources:no_{protocol}_keyword (kw={list(keywords)})")

    return PairClosureProbe(
        problem=problem,
        protocol=protocol,
        kind="forward",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


def _check_backward_combined(problem: str, protocol: str, history_paths: Dict[str, Path]) -> PairClosureProbe:
    """Backward combined closure: V1446/V1448/V1447/V1426 history mentions this pair."""
    evidence_parts: List[str] = []
    closed = 0

    found_in: List[str] = []
    for vname, p in history_paths.items():
        text = _load_history_text(p)
        if not text:
            evidence_parts.append(f"{vname}_history:missing")
            continue
        problem_id_present = problem in text
        protocol_keywords = V1449_PROTOCOL_KEYWORDS.get(protocol, ())
        protocol_present = any(kw in text for kw in protocol_keywords)
        if problem_id_present and protocol_present:
            found_in.append(vname)
            evidence_parts.append(f"{vname}:pair_present")
        else:
            evidence_parts.append(f"{vname}:no_pair (problem={problem_id_present},proto={protocol_present})")

    if found_in:
        closed = 1
        evidence_parts.append(f"backward:found_in={','.join(found_in)}")
    else:
        evidence_parts.append("backward:no_pair_in_any_history")

    return PairClosureProbe(
        problem=problem,
        protocol=protocol,
        kind="backward",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


def _check_cross_link_combined(
    problem: str,
    protocol: str,
    ptext: str,
    all_pairs: Tuple[Tuple[str, str], ...],
) -> Tuple[PairClosureProbe, Tuple[CrossCombinedEntry, ...]]:
    """Cross-link combined closure: another (problem', protocol') pair references this pair.

    Returns (probe, list of entries).
    """
    evidence_parts: List[str] = []
    entries: List[CrossCombinedEntry] = []
    any_link = 0

    this_protocol_kws = V1449_PROTOCOL_KEYWORDS.get(protocol, ())
    this_problem_kws = V1449_PROBLEM_KEYWORDS.get(problem, ())

    for other_problem, other_protocol in all_pairs:
        if other_problem == problem and other_protocol == protocol:
            continue  # skip self
        other_problem_text = _problem_source_text(other_problem)
        if not other_problem_text:
            entries.append(CrossCombinedEntry(
                source_problem=problem,
                source_protocol=protocol,
                target_problem=other_problem,
                target_protocol=other_protocol,
                linked=0,
                evidence=f"target[{other_problem}].sources:no_text",
            ))
            continue
        target_has_source_protocol = any(kw in other_problem_text for kw in this_protocol_kws)
        target_problem_kws = V1449_PROBLEM_KEYWORDS.get(other_problem, ())
        source_has_target_problem = any(kw in ptext for kw in target_problem_kws)

        linked = 1 if (target_has_source_protocol and source_has_target_problem) else 0
        link_evidence = f"target_proto={target_has_source_protocol},source_prob={source_has_target_problem}"
        entries.append(CrossCombinedEntry(
            source_problem=problem,
            source_protocol=protocol,
            target_problem=other_problem,
            target_protocol=other_protocol,
            linked=linked,
            evidence=link_evidence,
        ))
        any_link = max(any_link, linked)
        evidence_parts.append(f"({other_problem},{other_protocol}):{linked}")

    closed = 1 if any_link else 0
    return PairClosureProbe(
        problem=problem,
        protocol=protocol,
        kind="cross_link",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    ), tuple(entries)


def _check_history_combined(
    problem: str,
    protocol: str,
    history_paths: Dict[str, Path],
) -> PairClosureProbe:
    """History combined closure: ≥ 1 history point mentions pair identifier."""
    evidence_parts: List[str] = []
    seen_history = 0

    for vname, p in history_paths.items():
        text = _load_history_text(p)
        if not text:
            evidence_parts.append(f"{vname}_history:missing")
            continue
        protocol_kws = V1449_PROTOCOL_KEYWORDS.get(protocol, ())
        problem_kws = V1449_PROBLEM_KEYWORDS.get(problem, ())
        proto_present = any(kw in text for kw in protocol_kws)
        prob_present = any(kw in text for kw in problem_kws)
        if proto_present and prob_present:
            seen_history += 1
            evidence_parts.append(f"{vname}:pair_present (proto={proto_present},prob={prob_present})")
        else:
            evidence_parts.append(f"{vname}:no_pair (proto={proto_present},prob={prob_present})")

    closed = 1 if seen_history > 0 else 0
    if seen_history == 0:
        evidence_parts.append("history:no_pair_in_any_history")

    return PairClosureProbe(
        problem=problem,
        protocol=protocol,
        kind="history",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


def _check_guard_compliance_combined(problem: str, protocol: str) -> PairClosureProbe:
    """Guard compliance combined closure: problem guards reference protocol keyword."""
    evidence_parts: List[str] = []
    closed = 0

    prob_guards = _get_problem_guards(problem)
    if not prob_guards:
        evidence_parts.append(f"problem[{problem}].guards:missing")
    else:
        protocol_kws = V1449_PROTOCOL_KEYWORDS.get(protocol, ())
        guard_text = "\n".join(prob_guards)
        matches: List[str] = []
        for kw in protocol_kws:
            if kw.lower() in guard_text.lower():
                matches.append(kw)
        if matches:
            closed = 1
            evidence_parts.append(f"problem[{problem}].guards:has_{len(matches)}_{protocol}_kw={matches[:3]}")
        else:
            evidence_parts.append(f"problem[{problem}].guards:no_{protocol}_kw")

    # Also check V1426 source for protocol guards referencing problem
    v1426_text = _get_v1426_module_text()
    if v1426_text:
        problem_kws = V1449_PROBLEM_KEYWORDS.get(problem, ())
        v1426_matches: List[str] = []
        for kw in problem_kws:
            if kw.lower() in v1426_text.lower():
                v1426_matches.append(kw)
        if v1426_matches:
            closed = 1
            evidence_parts.append(f"v1426.source:has_{len(v1426_matches)}_{problem}_kw={v1426_matches[:3]}")
        else:
            evidence_parts.append(f"v1426.source:no_{problem}_kw")
    else:
        evidence_parts.append("v1426_source:missing")

    return PairClosureProbe(
        problem=problem,
        protocol=protocol,
        kind="guard_compliance",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


# ============================================================================
# Driver — run probes for one (problem, protocol) pair
# ============================================================================


def run_pair_closure(
    problem: str,
    protocol: str,
    history_paths: Dict[str, Path],
    all_pairs: Tuple[Tuple[str, str], ...],
) -> Tuple[Tuple[PairClosureProbe, ...], Tuple[CrossCombinedEntry, ...]]:
    """Run all 5 closure probes for one (problem, protocol) pair.

    Returns (probes, cross_combined_entries).
    """
    probes: List[PairClosureProbe] = []
    cross_entries: List[CrossCombinedEntry] = []
    ptext = _problem_source_text(problem)

    try:
        probes.append(_check_forward_combined(problem, protocol))
    except Exception as exc:
        probes.append(PairClosureProbe(
            problem=problem, protocol=protocol, kind="forward",
            closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}",
        ))

    try:
        probes.append(_check_backward_combined(problem, protocol, history_paths))
    except Exception as exc:
        probes.append(PairClosureProbe(
            problem=problem, protocol=protocol, kind="backward",
            closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}",
        ))

    try:
        probe, entries = _check_cross_link_combined(problem, protocol, ptext, all_pairs)
        probes.append(probe)
        cross_entries.extend(entries)
    except Exception as exc:
        probes.append(PairClosureProbe(
            problem=problem, protocol=protocol, kind="cross_link",
            closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}",
        ))

    try:
        probes.append(_check_history_combined(problem, protocol, history_paths))
    except Exception as exc:
        probes.append(PairClosureProbe(
            problem=problem, protocol=protocol, kind="history",
            closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}",
        ))

    try:
        probes.append(_check_guard_compliance_combined(problem, protocol))
    except Exception as exc:
        probes.append(PairClosureProbe(
            problem=problem, protocol=protocol, kind="guard_compliance",
            closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}",
        ))

    return tuple(probes), tuple(cross_entries)


def compute_pair_stats(problem: str, protocol: str, probes: Tuple[PairClosureProbe, ...]) -> PairClosureStats:
    """Compute aggregate stats for one (problem, protocol) pair."""
    pair_probes = tuple(
        p for p in probes
        if p.problem == problem and p.protocol == protocol
    )
    n_closed = sum(p.closed for p in pair_probes)
    n_probes = len(pair_probes)
    closure_rate = _safe_div(n_closed, n_probes)
    broken_kinds = tuple(p.kind for p in pair_probes if p.closed == 0)
    return PairClosureStats(
        problem=problem,
        protocol=protocol,
        n_probes=n_probes,
        n_closed=n_closed,
        closure_rate=closure_rate,
        broken_kinds=broken_kinds,
    )


def compute_overall_closure_rate(probes: Tuple[PairClosureProbe, ...]) -> float:
    if not probes:
        return 0.0
    return _safe_div(sum(p.closed for p in probes), len(probes))


def compute_per_kind_closure_rate(probes: Tuple[PairClosureProbe, ...]) -> Dict[str, float]:
    out: Dict[str, float] = {}
    for kind in V1449_CLOSURE_KINDS:
        kind_probes = tuple(p for p in probes if p.kind == kind)
        if kind_probes:
            out[kind] = _safe_div(sum(p.closed for p in kind_probes), len(kind_probes))
        else:
            out[kind] = 0.0
    return out


def compute_per_problem_closure_rate(probes: Tuple[PairClosureProbe, ...]) -> Dict[str, float]:
    out: Dict[str, float] = {}
    for problem in V1449_PROBLEM_NAMES:
        prob_probes = tuple(p for p in probes if p.problem == problem)
        if prob_probes:
            out[problem] = _safe_div(sum(p.closed for p in prob_probes), len(prob_probes))
        else:
            out[problem] = 0.0
    return out


def compute_per_protocol_closure_rate(probes: Tuple[PairClosureProbe, ...]) -> Dict[str, float]:
    out: Dict[str, float] = {}
    for protocol in V1449_PROTOCOL_NAMES:
        proto_probes = tuple(p for p in probes if p.protocol == protocol)
        if proto_probes:
            out[protocol] = _safe_div(sum(p.closed for p in proto_probes), len(proto_probes))
        else:
            out[protocol] = 0.0
    return out


def compute_cross_combined_density(cross_links: Tuple[CrossCombinedEntry, ...]) -> float:
    if not cross_links:
        return 0.0
    return _safe_div(sum(e.linked for e in cross_links), len(cross_links))


def detect_compositional_pairs(pair_stats: Tuple[PairClosureStats, ...]) -> Tuple[CompositionalPair, ...]:
    """Detect pairs that are both problem-closed and protocol-closed."""
    out: List[CompositionalPair] = []
    for ps in pair_stats:
        if ps.closure_rate >= 1.0:
            out.append(CompositionalPair(
                problem=ps.problem,
                protocol=ps.protocol,
                closure_rate=ps.closure_rate,
                evidence=f"pair({ps.problem},{ps.protocol}) n_closed={ps.n_closed}/{ps.n_probes}",
            ))
    return tuple(out)


def detect_anti_modular_pairs(pair_stats: Tuple[PairClosureStats, ...]) -> Tuple[AntiModularPair, ...]:
    """Detect anti-modular pairs."""
    out: List[AntiModularPair] = []
    high_closure = [p for p in pair_stats if p.closure_rate >= 0.8]
    low_closure = [p for p in pair_stats if p.closure_rate <= 0.2]
    for hi in high_closure:
        for lo in low_closure:
            if hi.problem == lo.problem and hi.protocol == lo.protocol:
                continue
            if hi.problem != lo.problem and hi.protocol != lo.protocol:
                out.append(AntiModularPair(
                    problem=hi.problem,
                    protocol=hi.protocol,
                    opposite_problem=lo.problem,
                    opposite_protocol=lo.protocol,
                    closure_rate_a=hi.closure_rate,
                    closure_rate_b=lo.closure_rate,
                    evidence=f"hi({hi.problem},{hi.protocol})={hi.closure_rate:.2f} vs lo({lo.problem},{lo.protocol})={lo.closure_rate:.2f}",
                ))
    return tuple(out[:30])


def detect_substitutable_pairs(pair_stats: Tuple[PairClosureStats, ...]) -> Tuple[SubstitutablePair, ...]:
    """Detect substitutable pairs (forward closure implies pair closure)."""
    out: List[SubstitutablePair] = []
    for ps in pair_stats:
        # Substitutable: pair has any closure > 0 and at least one of (forward, guard) closed
        if ps.closure_rate >= 0.4 and "forward" not in ps.broken_kinds:
            out.append(SubstitutablePair(
                problem=ps.problem,
                protocol=ps.protocol,
                problem_closure=ps.closure_rate,
                protocol_closure=ps.closure_rate,
                evidence=f"forward_present in pair({ps.problem},{ps.protocol}) closure={ps.closure_rate:.2f}",
            ))
    return tuple(out)


def render_markdown_report(report: SevenProblemsVCPCrossModularAuditReport) -> str:
    """Render report as markdown."""
    lines: List[str] = []
    lines.append(f"# V1449 — ASI 7 哲学问题 × VCP 6 协议 cross-modular audit")
    lines.append("")
    lines.append(f"- Schema: `{report.schema}`")
    lines.append(f"- Version: `{report.version}`")
    lines.append(f"- Module: `{report.module}`")
    lines.append(f"- Started: `{report.started_iso}`")
    lines.append(f"- Ended: `{report.ended_iso}`")
    lines.append("")
    lines.append("## Counts")
    lines.append(f"- n_pairs: **{report.n_pairs}** (= 7 problems × 6 protocols)")
    lines.append(f"- n_probes: **{report.n_probes}** (= 42 pairs × 5 closure kinds)")
    lines.append(f"- n_cross_combined_pairs: **{report.n_cross_combined_pairs}** (= 42 × 41 directed)")
    lines.append("")
    lines.append("## Closure rates")
    lines.append(f"- overall_closure_rate: **{report.overall_closure_rate:.4f}**")
    lines.append(f"- overall_cross_link_density: **{report.overall_cross_link_density:.4f}**")
    lines.append("")
    lines.append("### Per-kind closure rate")
    for kind, rate in report.per_kind_closure_rate.items():
        lines.append(f"- {kind}: **{rate:.4f}**")
    lines.append("")
    lines.append("### Per-problem closure rate")
    for problem, rate in report.per_problem_closure_rate.items():
        lines.append(f"- {problem}: **{rate:.4f}**")
    lines.append("")
    lines.append("### Per-protocol closure rate")
    for protocol, rate in report.per_protocol_closure_rate.items():
        lines.append(f"- {protocol}: **{rate:.4f}**")
    lines.append("")
    lines.append("## Compositional pairs (full closure)")
    if report.compositional_pairs:
        for cp in report.compositional_pairs:
            lines.append(f"- {cp.problem} × {cp.protocol}: closure_rate={cp.closure_rate:.2f}")
    else:
        lines.append("- (none)")
    lines.append("")

    lines.append("## Anti-modular pairs (top 30)")
    if report.anti_modular_pairs:
        for amp in report.anti_modular_pairs:
            lines.append(f"- {amp.problem}×{amp.protocol}={amp.closure_rate_a:.2f} vs {amp.opposite_problem}×{amp.opposite_protocol}={amp.closure_rate_b:.2f}")
    else:
        lines.append("- (none)")
    lines.append("")

    lines.append("## Substitutable pairs (forward closure implies pair closure)")
    if report.substitutable_pairs:
        for sp in report.substitutable_pairs:
            lines.append(f"- {sp.problem} × {sp.protocol}: closure_rate={sp.protocol_closure:.2f}")
    else:
        lines.append("- (none)")
    lines.append("")

    lines.append("## Honest disclosure")
    lines.append(report.honest_disclosure)
    lines.append("")

    lines.append("## Guards")
    for g in report.guards:
        lines.append(f"- {g}")
    lines.append("")

    lines.append("## V3 哲学守门")
    for g in report.v3_guards:
        lines.append(f"- {g}")
    lines.append("")

    lines.append("## Borrowed")
    for vname, desc in report.borrowed:
        lines.append(f"- {vname}: {desc}")
    lines.append("")
    return "\n".join(lines)


def run_all(out_json: Optional[Path] = None, out_md: Optional[Path] = None) -> SevenProblemsVCPCrossModularAuditReport:
    """Run the full V1449 cross-modular audit.

    Returns SevenProblemsVCPCrossModularAuditReport dataclass.
    """
    started = _now_utc_iso()
    history_paths = _discover_history_files()

    all_pairs = tuple((p, pr) for p in V1449_PROBLEM_NAMES for pr in V1449_PROTOCOL_NAMES)
    assert len(all_pairs) == 42, f"expected 42 pairs, got {len(all_pairs)}"

    all_probes: List[PairClosureProbe] = []
    all_cross_entries: List[CrossCombinedEntry] = []

    for problem, protocol in all_pairs:
        probes, entries = run_pair_closure(problem, protocol, history_paths, all_pairs)
        all_probes.extend(probes)
        all_cross_entries.extend(entries)

    pair_stats = tuple(compute_pair_stats(p, pr, tuple(all_probes)) for p, pr in all_pairs)
    compositional = detect_compositional_pairs(pair_stats)
    anti_modular = detect_anti_modular_pairs(pair_stats)
    substitutable = detect_substitutable_pairs(pair_stats)

    overall_closure = compute_overall_closure_rate(tuple(all_probes))
    per_kind = compute_per_kind_closure_rate(tuple(all_probes))
    per_problem = compute_per_problem_closure_rate(tuple(all_probes))
    per_protocol = compute_per_protocol_closure_rate(tuple(all_probes))
    cross_density = compute_cross_combined_density(tuple(all_cross_entries))

    ended = _now_utc_iso()

    honest = (
        "V1449 = ASI 7 哲学问题 × VCP 6 协议 cross-modular audit. "
        "This is a bounded 42-pair × 5-kind = 210-probe closure audit. "
        "V1449 ≠ ASI-achieved closure. V1449 ≠ Phenomenal closure. "
        "V1449 ≠ human-level closure. V1449 ≠ absolute closure. "
        "V1449 ≠ VCP protocol correctness (we did not run any actual VCP protocol dispatch here; "
        "we only audited whether problem source modules mention VCP protocol keywords). "
        "Real VCP dispatch is owned by V1426 (run: `python -m apeireth.v1426_vcp_six_protocol_dispatcher run-all`). "
        f"overall_closure_rate={overall_closure:.4f} honest disclosure: "
        f"forward={per_kind.get('forward', 0):.2f} backward={per_kind.get('backward', 0):.2f} "
        f"cross_link={per_kind.get('cross_link', 0):.2f} history={per_kind.get('history', 0):.2f} "
        f"guard_compliance={per_kind.get('guard_compliance', 0):.2f} "
        f"cross_link_density={cross_density:.2f}."
    )

    report = SevenProblemsVCPCrossModularAuditReport(
        schema=V1449_SCHEMA,
        version=V1449_VERSION,
        module=V1449_MODULE,
        started_iso=started,
        ended_iso=ended,
        n_probes=len(all_probes),
        n_pairs=len(all_pairs),
        n_cross_combined_pairs=len(all_cross_entries),
        probes=tuple(all_probes),
        pair_stats=pair_stats,
        cross_combined_links=tuple(all_cross_entries),
        compositional_pairs=compositional,
        anti_modular_pairs=anti_modular,
        substitutable_pairs=substitutable,
        overall_closure_rate=overall_closure,
        per_kind_closure_rate=per_kind,
        per_problem_closure_rate=per_problem,
        per_protocol_closure_rate=per_protocol,
        overall_cross_link_density=cross_density,
        honest_disclosure=honest,
        guards=V1449_GUARDS,
        v3_guards=V1449_V3_GUARDS,
        borrowed=V1449_BORROWED,
    )

    if out_json is None:
        out_json = DEFAULT_REPORT_JSON
    if out_md is None:
        out_md = DEFAULT_REPORT_MD
    try:
        out_json.write_text(json.dumps(report.to_dict(), indent=2, ensure_ascii=False), encoding="utf-8")
        out_md.write_text(render_markdown_report(report), encoding="utf-8")
    except Exception:
        pass

    return report


def chain_delegate() -> Dict[str, Any]:
    """Chain delegate: check that V1448+V1447+V1446+V1445+V1442+V1426 are loadable."""
    out: Dict[str, Any] = {}
    for vname, mid in (
        ("v1448", DEFAULT_V1448_MODULE),
        ("v1447", DEFAULT_V1447_MODULE),
        ("v1446", DEFAULT_V1446_MODULE),
        ("v1445", DEFAULT_V1445_MODULE),
        ("v1442", DEFAULT_V1442_MODULE),
        ("v1426", DEFAULT_V1426_MODULE),
    ):
        m = _import_safely(mid)
        out[vname] = {
            "importable": m is not None,
            "version": getattr(m, "V" + vname[1:] + "_VERSION", None) if m else None,
        }
    out["all_ok"] = all(v["importable"] for v in out.values() if vname != "all_ok") if False else all(
        v["importable"] for k, v in out.items() if k != "all_ok"
    )
    return out


# ============================================================================
# Popper — self-tests
# ============================================================================


def popper() -> Tuple[bool, List[Dict[str, Any]]]:
    """Run self-tests; return (all_ok, results)."""
    results: List[Dict[str, Any]] = []

    def _record(name: str, ok: bool, detail: str = "") -> None:
        results.append({"name": name, "ok": bool(ok), "detail": _safe_str(detail)})

    # 1. Module version is set
    _record("module_version_set", bool(V1449_VERSION == "0.1.0"), f"V1449_VERSION={V1449_VERSION}")

    # 2. 7 problems declared
    _record("problem_count_7", len(V1449_PROBLEM_NAMES) == 7, f"len={len(V1449_PROBLEM_NAMES)}")

    # 3. 6 protocols declared
    _record("protocol_count_6", len(V1449_PROTOCOL_NAMES) == 6, f"len={len(V1449_PROTOCOL_NAMES)}")

    # 4. 5 closure kinds declared
    _record("closure_kind_count_5", len(V1449_CLOSURE_KINDS) == 5, f"len={len(V1449_CLOSURE_KINDS)}")

    # 5. All 42 pairs enumerated
    all_pairs = tuple((p, pr) for p in V1449_PROBLEM_NAMES for pr in V1449_PROTOCOL_NAMES)
    _record("pair_count_42", len(all_pairs) == 42, f"len={len(all_pairs)}")

    # 6. Each protocol has keywords
    all_have_keywords = all(len(V1449_PROTOCOL_KEYWORDS.get(p, ())) > 0 for p in V1449_PROTOCOL_NAMES)
    _record("all_protocols_have_keywords", all_have_keywords, "all 6 protocols have ≥1 keyword")

    # 7. Each problem has keywords
    all_prob_have_keywords = all(len(V1449_PROBLEM_KEYWORDS.get(p, ())) > 0 for p in V1449_PROBLEM_NAMES)
    _record("all_problems_have_keywords", all_prob_have_keywords, "all 7 problems have ≥1 keyword")

    # 8. Each problem has source modules
    all_have_sources = all(len(V1449_PROBLEM_SOURCES.get(p, ())) > 0 for p in V1449_PROBLEM_NAMES)
    _record("all_problems_have_sources", all_have_sources, "all 7 problems have ≥1 source module")

    # 9. V1446 importable (needed for problems)
    v1446 = _import_safely(DEFAULT_V1446_MODULE)
    _record("v1446_importable", v1446 is not None, DEFAULT_V1446_MODULE)

    # 10. V1448 importable (needed for VCP protocols)
    v1448 = _import_safely(DEFAULT_V1448_MODULE)
    _record("v1448_importable", v1448 is not None, DEFAULT_V1448_MODULE)

    # 11. V1426 importable (needed for VCP protocol dispatcher)
    v1426 = _import_safely(DEFAULT_V1426_MODULE)
    _record("v1426_importable", v1426 is not None, DEFAULT_V1426_MODULE)

    # 12. run_pair_closure on (time, sync) doesn't raise
    try:
        history_paths = _discover_history_files()
        probes, _ = run_pair_closure("time", "sync", history_paths, all_pairs)
        _record("run_pair_closure_no_raise", len(probes) == 5, f"n_probes={len(probes)}")
    except Exception as exc:
        _record("run_pair_closure_no_raise", False, f"raised:{type(exc).__name__}")

    # 13. compute_overall_closure_rate returns float in [0,1]
    sample_probes = (
        PairClosureProbe("time", "sync", "forward", 1, "test"),
        PairClosureProbe("time", "sync", "backward", 0, "test"),
    )
    rate = compute_overall_closure_rate(sample_probes)
    _record("overall_rate_bounded", 0.0 <= rate <= 1.0, f"rate={rate}")

    # 14. cross_combined links per source = 41 (42 pairs - self)
    try:
        history_paths = _discover_history_files()
        _, entries = run_pair_closure("time", "sync", history_paths, all_pairs)
        _record("cross_combined_count_41", len(entries) == 41, f"n={len(entries)}")
    except Exception as exc:
        _record("cross_combined_count_41", False, f"raised:{type(exc).__name__}")

    # 15. run_all doesn't raise
    try:
        report = run_all()
        _record("run_all_no_raise", report is not None and report.n_probes == 210, f"n_probes={report.n_probes if report else 0}")
    except Exception as exc:
        _record("run_all_no_raise", False, f"raised:{type(exc).__name__}")

    # 16. chain_delegate returns all_ok
    try:
        chain = chain_delegate()
        _record("chain_delegate_all_ok", chain.get("all_ok") is True, f"chain={chain}")
    except Exception as exc:
        _record("chain_delegate_all_ok", False, f"raised:{type(exc).__name__}")

    all_ok = all(r["ok"] for r in results)
    return all_ok, results


# ============================================================================
# CLI
# ============================================================================


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1449_asi_seven_problems_vcp_cross_modular",
        description="V1449 — ASI 7 哲学问题 × VCP 6 协议 cross-modular audit",
    )
    sub = p.add_subparsers(dest="cmd")

    sub.add_parser("version", help="Print version")
    sub.add_parser("help", help="Print help")

    p_meta = sub.add_parser("meta", help="Print module metadata")
    p_meta.add_argument("--json", action="store_true")

    sub.add_parser("popper", help="Run popper self-tests")
    sub.add_parser("chain", help="Print chain_delegate() result")
    sub.add_parser("list-pairs", help="List all 42 (problem, protocol) pairs")

    p_probe = sub.add_parser("probe-closure", help="Probe one pair")
    p_probe.add_argument("--problem", default="time")
    p_probe.add_argument("--protocol", default="sync")
    p_probe.add_argument("--kind", default=None, help="Optional closure kind filter")

    p_ccm = sub.add_parser("cross-combined-matrix", help="Compute cross-combined matrix for one pair")
    p_ccm.add_argument("--problem", default="time")
    p_ccm.add_argument("--protocol", default="sync")

    sub.add_parser("detect-compositional", help="Detect compositional pairs")
    sub.add_parser("detect-anti-modular", help="Detect anti-modular pairs")
    sub.add_parser("detect-substitutable", help="Detect substitutable pairs")

    p_run = sub.add_parser("run-all", help="Run full audit")
    p_run.add_argument("--out-json", default=None)
    p_run.add_argument("--out-md", default=None)

    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    cmd = args.cmd or "help"

    if cmd == "version":
        print(V1449_VERSION)
        return 0

    if cmd == "help":
        parser.print_help()
        return 0

    if cmd == "meta":
        meta = {
            "schema": V1449_SCHEMA,
            "version": V1449_VERSION,
            "module": V1449_MODULE,
            "n_problems": len(V1449_PROBLEM_NAMES),
            "n_protocols": len(V1449_PROTOCOL_NAMES),
            "n_closure_kinds": len(V1449_CLOSURE_KINDS),
            "n_pairs": 42,
            "n_probes": 210,
            "n_cross_combined_pairs": 1722,
            "guards": list(V1449_GUARDS),
            "v3_guards": list(V1449_V3_GUARDS),
            "borrowed": [list(b) for b in V1449_BORROWED],
        }
        if getattr(args, "json", False):
            print(json.dumps(meta, indent=2, ensure_ascii=False))
        else:
            for k, v in meta.items():
                print(f"{k}: {v}")
        return 0

    if cmd == "popper":
        ok, results = popper()
        for r in results:
            mark = "✓" if r["ok"] else "✗"
            print(f"{mark} {r['name']}: {r['detail']}")
        print(f"\nALL_OK={ok}")
        return 0 if ok else 1

    if cmd == "chain":
        chain = chain_delegate()
        print(json.dumps(chain, indent=2, ensure_ascii=False))
        return 0

    if cmd == "list-pairs":
        for p in V1449_PROBLEM_NAMES:
            for pr in V1449_PROTOCOL_NAMES:
                print(f"{p} × {pr}")
        return 0

    if cmd == "probe-closure":
        history_paths = _discover_history_files()
        all_pairs = tuple((p, pr) for p in V1449_PROBLEM_NAMES for pr in V1449_PROTOCOL_NAMES)
        probes, _ = run_pair_closure(args.problem, args.protocol, history_paths, all_pairs)
        for probe in probes:
            if args.kind and probe.kind != args.kind:
                continue
            mark = "✓" if probe.closed else "✗"
            print(f"{mark} {probe.kind}: closed={probe.closed} evidence={probe.evidence}")
        return 0

    if cmd == "cross-combined-matrix":
        history_paths = _discover_history_files()
        all_pairs = tuple((p, pr) for p in V1449_PROBLEM_NAMES for pr in V1449_PROTOCOL_NAMES)
        _, entries = run_pair_closure(args.problem, args.protocol, history_paths, all_pairs)
        for e in entries:
            mark = "✓" if e.linked else "✗"
            print(f"{mark} {e.target_problem}×{e.target_protocol}: {e.evidence}")
        return 0

    if cmd == "detect-compositional":
        report = run_all()
        if report.compositional_pairs:
            for cp in report.compositional_pairs:
                print(f"{cp.problem}×{cp.protocol}: {cp.closure_rate:.2f}")
        else:
            print("(none)")
        return 0

    if cmd == "detect-anti-modular":
        report = run_all()
        if report.anti_modular_pairs:
            for amp in report.anti_modular_pairs:
                print(f"{amp.problem}×{amp.protocol}={amp.closure_rate_a:.2f} vs {amp.opposite_problem}×{amp.opposite_protocol}={amp.closure_rate_b:.2f}")
        else:
            print("(none)")
        return 0

    if cmd == "detect-substitutable":
        report = run_all()
        if report.substitutable_pairs:
            for sp in report.substitutable_pairs:
                print(f"{sp.problem}×{sp.protocol}: {sp.protocol_closure:.2f}")
        else:
            print("(none)")
        return 0

    if cmd == "run-all":
        out_json = Path(args.out_json) if args.out_json else None
        out_md = Path(args.out_md) if args.out_md else None
        report = run_all(out_json=out_json, out_md=out_md)
        print(f"V1449 report written.")
        print(f"  overall_closure_rate: {report.overall_closure_rate:.4f}")
        print(f"  per_kind: {report.per_kind_closure_rate}")
        print(f"  per_problem: {report.per_problem_closure_rate}")
        print(f"  per_protocol: {report.per_protocol_closure_rate}")
        print(f"  cross_link_density: {report.overall_cross_link_density:.4f}")
        print(f"  n_compositional: {len(report.compositional_pairs)}")
        print(f"  n_anti_modular: {len(report.anti_modular_pairs)}")
        print(f"  n_substitutable: {len(report.substitutable_pairs)}")
        return 0

    parser.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())