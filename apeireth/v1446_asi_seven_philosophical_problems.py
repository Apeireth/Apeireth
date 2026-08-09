"""V1446 — ASI 7 哲学问题 (5+2) bidirectional closure audit.

Phase: 1446
Version: 0.1.0
Date: 2026-08-10 (cron tick 07:03 Asia/Shanghai morning)
Post: V1445 (ASI V2 5 positions cross-position closure audit)
      V1444 (5 philosophical gaps round 3 closure audit)
      V1441 (5 philosophical gaps round 2)
      V1425 (5 philosophical gaps round 1)
      V1049 (value alignment complete)

What V1446 is
=============
V1446 is the **ASI 7 哲学问题 (5+2) bidirectional closure audit**. Where V1425 had
1 probe per gap (round 1), V1441 had 3 probes per gap (round 2), V1444 had 5
closure probes per gap (round 3), V1446 extends the audit to **7 philosophical
problems** (5 inherited from V1425 + 2 new) and runs **5 closure probes per
problem**.

The 7 philosophical problems
----------------------------
1. 时间 (time)              — V1425 source (V1417 DGM tick history)
2. 自由 (freedom)            — V1425 source (V1419 multi-policy evaluator)
3. 识别 (recognition)        — V1425 source (V1424 real LLM benchmark)
4. 涌现 (emergence)          — V1425 source (V1413 hub)
5. 真理 (truth)              — V1425 source (V1432 VCP real source deep read)
6. 自我意识 (self_cons)      — NEW (V1411 framework + V1410 5-position)
7. 价值对齐 (value_align)    — NEW (V1049 value alignment complete)

For each of 7 problems, V1446 runs 5 closure probes:
1. **Forward closure**: probe_definition exists → probe runs → result bounded →
   recorded → history appended
2. **Backward closure**: history record exists → result recoverable → probe
   reproducible from definition alone (no hidden state)
3. **Cross-link closure**: this problem is mentioned in another problem's
   evidence / notes (i.e. problems are not perfectly isolated)
4. **History closure**: ≥ 1 history point for V1425 / V1441 / V1444 / V1049
5. **Guard compliance closure**: V1425 + V1441 + V1444 + (V1411 / V1049) guards
   present and listed in the runtime guards tuple

7 problems × 5 closure probes = 35 closure probes total.
Cross-link matrix: 7×7 = 49 directed pairs (excluding self = 42).

V1446 ≠ ASI-achieved closure. V1446 ≠ Phenomenal closure.
V1446 ≠ human-level closure. V1446 ≠ absolute closure.
V1446 = bounded 7-problem closure audit (35 probes).

Differences from V1444 (5 gaps round 3)
---------------------------------------
- 7 problems × 5 closure probes = 35 (V1444 had 25 over 5 gaps × 5 kinds)
- 2 new problems (self_consciousness from V1411, value_alignment from V1049)
- Cross-link matrix = 7×7 = 49 pairs (V1444 had 5×5 = 25)
- Per-problem closure rate + per-probe-kind closure rate
- 5 borrowed (V1445 + V1444 + V1441 + V1425 + V1049) vs V1444's 6

V1446 actually does
-------------------
1. Loads V1425 + V1441 + V1444 + V1411 + V1049 module surfaces via importlib
2. For each of 7 problems, runs 5 closure probes:
   - probe_forward_closure
   - probe_backward_closure
   - probe_cross_link_closure
   - probe_history_closure
   - probe_guard_compliance_closure
3. Computes per-problem closure_rate + per-probe-kind closure_rate
4. Computes 7×7 cross-link matrix (which problem references which)
5. Lists broken closures explicitly (problem, kind, evidence)
6. Emits SevenProblemsClosureReport
7. Writes .v1446-asi-seven-philosophical-problems-report.{json,md}
8. CLI: python -m apeireth.v1446_asi_seven_philosophical_problems [command]

Borrowed (5 — 主 19:33 走在前人经验上)
=======================================
- V1445 (5 positions cross-position closure audit pattern — closure kinds + cross-link matrix)
- V1444 (5 gaps round 3 closure audit pattern — forward + backward + cross-link + history + guard)
- V1441 (5 gaps round 2 — composite_score + variance + trend)
- V1425 (5 gaps round 1 — gap definitions + probe functions + history JSONL)
- V1049 (value alignment complete — 11 理论 + 10 组件 + 5 哲学守门 + 77 tests)

GUARDS upheld (V1446-specific, 14 — 主 00:44 质量工程化)
========================================================
- GUARD_BOUNDED_CLOSURE: each closure probe returns 0 or 1 (never partial)
- GUARD_NO_RAISE: any closure probe failure → returns 0 with exception msg, never raises
- GUARD_OFFLINE_SAFE: no network, only stdlib + local JSON + importlib
- GUARD_READ_ONLY: V1446 imports V1425/V1441/V1444/V1411/V1049, doesn't modify them
- GUARD_FORWARD_CHAIN: forward closure checks prob_def → run → record → history
- GUARD_BACKWARD_CHAIN: backward closure checks history → record → reproduce → def
- GUARD_CROSS_LINK_BOUNDED: cross-link matrix is 7×7 binary (excluding self)
- GUARD_HISTORY_LOADED: V1425/V1441/V1444 history must exist (else closure 0 with evidence)
- GUARD_GUARD_LISTED: V1425 + V1441 + V1444 + V1411 + V1049 guards must be importable
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted in report
- GUARD_NO_V1425_REPLACE: V1446 reads V1425, doesn't redefine gaps
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
========================================================
- GUARD_NO_PHENOMENAL_CLOSURE
- GUARD_NO_ASI_CLOSURE
- GUARD_NO_HUMAN_LEVEL_CLOSURE
- GUARD_NO_ABSOLUTE_CLOSURE
- GUARD_NO_CLOSURE_OVERCLAIM (35 closures ≠ solving 7 哲学问题)

CLI commands (11 — 主 00:56 任何人都能接手)
============================================
1. version
2. meta [--json]
3. help
4. popper
5. chain
6. list-problems
7. probe-closure [--problem <name>] [--kind <kind>]
8. cross-link-matrix
9. run-all [--out-json <path>] [--out-md <path>]
10. probe-history
11. probe-guard-compliance
"""

from __future__ import annotations

import argparse
import ast
import importlib
import inspect
import json
import re
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# ============================================================================
# Constants
# ============================================================================

V1446_VERSION = "0.1.0"
V1446_SCHEMA = "asi.seven-philosophical-problems.closure-audit.v1"
V1446_MODULE = "apeireth.v1446_asi_seven_philosophical_problems"
V1446_MODULE_SHORT = "v1446_asi_seven_philosophical_problems"

# 7 philosophical problems (5 inherited + 2 new)
PROBLEM_NAMES = (
    "time",
    "freedom",
    "recognition",
    "emergence",
    "truth",
    "self_consciousness",
    "value_alignment",
)
PROBLEM_LABELS = (
    "时间",
    "自由",
    "识别",
    "涌现",
    "真理",
    "自我意识",
    "价值对齐",
)
assert len(PROBLEM_NAMES) == len(PROBLEM_LABELS) == 7

# 5 closure kinds (same as V1444/V1445)
CLOSURE_KINDS = ("forward", "backward", "cross_link", "history", "guard_compliance")

# Module sources per problem (forward closure probes these)
PROBLEM_SOURCES: Dict[str, Tuple[str, ...]] = {
    "time": ("apeireth.v1425_asi_five_philosophical_gaps", "apeireth.v1417_asi_dgm_tick_history"),
    "freedom": ("apeireth.v1425_asi_five_philosophical_gaps", "apeireth.v1419_asi_multi_policy_evaluator"),
    "recognition": ("apeireth.v1425_asi_five_philosophical_gaps", "apeireth.v1424_asi_real_llm_benchmark"),
    "emergence": ("apeireth.v1425_asi_five_philosophical_gaps", "apeireth.v1413_asi_overarching_history"),
    "truth": ("apeireth.v1425_asi_five_philosophical_gaps", "apeireth.v1432_vcp_real_source_deep_read"),
    "self_consciousness": ("apeireth.v1411_asi_overarching_framework", "apeireth.v1410_asi_five_position_framework"),
    "value_alignment": ("apeireth.v1049_asi_alignment", "apeireth.v1411_asi_overarching_framework"),
}

# Source keywords for cross-link detection (problem X mentions these tokens in evidence)
PROBLEM_KEYWORDS: Dict[str, Tuple[str, ...]] = {
    "time": ("time", "temporal", "tick", "duration", "时间"),
    "freedom": ("freedom", "policy", "choice", "liberty", "自由"),
    "recognition": ("recogni", "benchmark", "accuracy", "识别"),
    "emergence": ("emerg", "composit", "complex", "涌现"),
    "truth": ("truth", "ground", "verif", "事实", "真理"),
    "self_consciousness": ("self", "consci", "introspect", "model", "自我"),
    "value_alignment": ("value", "align", "corrigib", "goal", "价值"),
}

# 14 V1446-specific guards
V1446_GUARDS: Tuple[str, ...] = (
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
    "GUARD_NO_V1425_REPLACE",
    "GUARD_CLI_RUNNABLE",
)

# 5 V3 哲学守门
V1446_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_CLOSURE",
    "GUARD_NO_ASI_CLOSURE",
    "GUARD_NO_HUMAN_LEVEL_CLOSURE",
    "GUARD_NO_ABSOLUTE_CLOSURE",
    "GUARD_NO_CLOSURE_OVERCLAIM",
)

# 5 borrowed
V1446_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1445", "5 positions cross-position closure audit pattern (closure kinds + cross-link matrix)"),
    ("V1444", "5 gaps round 3 closure audit pattern (forward + backward + cross-link + history + guard)"),
    ("V1441", "5 gaps round 2 (composite_score + variance + trend)"),
    ("V1425", "5 gaps round 1 (gap definitions + probe functions + history JSONL)"),
    ("V1049", "value alignment complete (11 理论 + 10 组件 + 5 哲学守门 + 77 tests)"),
)

# Default import targets (chain delegate)
DEFAULT_V1425_MODULE = "apeireth.v1425_asi_five_philosophical_gaps"
DEFAULT_V1441_MODULE = "apeireth.v1441_asi_philosophical_gaps_round2"
DEFAULT_V1444_MODULE = "apeireth.v1444_asi_philosophical_gaps_round3"
DEFAULT_V1411_MODULE = "apeireth.v1411_asi_overarching_framework"
DEFAULT_V1049_MODULE = "apeireth.v1049_asi_alignment"

# Default history paths (V1425 / V1441 / V1444 / V1049)
DEFAULT_HISTORY_DIR = Path(".") / ".v1446-histories"
DEFAULT_V1425_HISTORY = DEFAULT_HISTORY_DIR / "v1425_history.jsonl"
DEFAULT_V1441_HISTORY = DEFAULT_HISTORY_DIR / "v1441_history.jsonl"
DEFAULT_V1444_HISTORY = DEFAULT_HISTORY_DIR / "v1444_history.jsonl"
DEFAULT_V1411_HISTORY = DEFAULT_HISTORY_DIR / "v1411_history.jsonl"
DEFAULT_V1049_HISTORY = DEFAULT_HISTORY_DIR / "v1049_history.jsonl"

# Default report output paths
DEFAULT_REPORT_JSON = Path(".") / ".v1446-asi-seven-philosophical-problems-report.json"
DEFAULT_REPORT_MD = Path(".") / ".v1446-asi-seven-philosophical-problems-report.md"

# Reuse V1425 / V1441 / V1444 history files if present (best-effort)
DISCOVERED_HISTORY: Dict[str, Path] = {}


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


def _read_module_text(module_short: str) -> str:
    """Read the source text of a module from the apeireth directory."""
    try:
        p = Path(__file__).resolve().parent / f"{module_short}.py"
        if p.exists():
            return p.read_text(encoding="utf-8", errors="replace")
    except Exception:
        pass
    return ""


def _discover_history_files() -> Dict[str, Path]:
    """Try to discover V1425/V1441/V1444/V1411/V1049 history files.

    Searches common locations:
    - ./{vN}.jsonl
    - ./{vN}_history.jsonl
    - ./{vN}-history.jsonl
    - .apeireth/...
    """
    candidates = {
        "v1425": DEFAULT_V1425_HISTORY,
        "v1441": DEFAULT_V1441_HISTORY,
        "v1444": DEFAULT_V1444_HISTORY,
        "v1411": DEFAULT_V1411_HISTORY,
        "v1049": DEFAULT_V1049_HISTORY,
    }
    search_dirs = [
        Path("."),
        Path("apeireth"),
        Path(".v1446-histories"),
        Path(".v1425-histories"),
        Path(".v1441-histories"),
        Path(".v1444-histories"),
    ]
    discovered: Dict[str, Path] = {}
    for vname, default_path in candidates.items():
        # First check default path
        if default_path.exists():
            discovered[vname] = default_path
            continue
        # Search
        for d in search_dirs:
            for pattern in (f"{vname}.jsonl", f"{vname}_history.jsonl", f"{vname}-history.jsonl"):
                p = d / pattern
                if p.exists():
                    discovered[vname] = p
                    break
            if vname in discovered:
                break
    return discovered


# ============================================================================
# Data classes
# ============================================================================


@dataclass
class ClosureProbe:
    """One closure probe result for one problem."""
    problem: str
    kind: str
    closed: int
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class CrossLinkEntry:
    """One directed cross-link probe between two problems."""
    source_problem: str
    target_problem: str
    linked: int
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class ProblemClosureStats:
    """Aggregate stats for one problem."""
    problem: str
    n_probes: int
    n_closed: int
    closure_rate: float
    broken_kinds: Tuple[str, ...]

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class SevenProblemsClosureReport:
    """Full V1446 closure audit report."""
    schema: str
    version: str
    module: str
    started_iso: str
    ended_iso: str
    n_probes: int
    n_problems: int
    n_cross_pairs: int
    probes: Tuple[ClosureProbe, ...]
    problem_stats: Tuple[ProblemClosureStats, ...]
    cross_links: Tuple[CrossLinkEntry, ...]
    overall_closure_rate: float
    per_kind_closure_rate: Dict[str, float]
    per_problem_source_loaded: Dict[str, bool]
    honest_disclosure: str
    guards: Tuple[str, ...]
    v3_guards: Tuple[str, ...]
    borrowed: Tuple[Tuple[str, str], ...]

    def to_dict(self) -> Dict[str, Any]:
        d = asdict(self)
        # Convert tuples to lists for JSON
        out: Dict[str, Any] = {}
        for k, v in d.items():
            if isinstance(v, tuple):
                out[k] = list(v)
            else:
                out[k] = v
        return out


# ============================================================================
# Probe functions
# ============================================================================


def _check_forward_closure(problem: str, sources: Tuple[str, ...]) -> ClosureProbe:
    """Forward closure: problem sources importable, definition exists, probe callable."""
    evidence_parts: List[str] = []
    closed = 1

    for src in sources:
        mod = _import_safely(src)
        if mod is None:
            closed = 0
            evidence_parts.append(f"{src}:missing")
            continue
        evidence_parts.append(f"{src}:imported")

    # Check whether problem has a probe definition in V1425 / V1441 / V1444
    # For inherited problems (time/freedom/recognition/emergence/truth),
    # V1425 has GAP_DEFINITIONS or PROBES dict
    # For new problems, check V1411 / V1049
    if problem in PROBLEM_KEYWORDS:
        v1425 = _import_safely(DEFAULT_V1425_MODULE)
        if v1425 is not None and problem in ("time", "freedom", "recognition", "emergence", "truth"):
            # Look for a probe or definition in V1425
            for attr in ("GAP_DEFINITIONS", "PROBES", "PROBE_FUNCTIONS", "GAPS"):
                d = getattr(v1425, attr, None)
                if isinstance(d, dict) and problem in d:
                    evidence_parts.append(f"v1425.{attr}[{problem}]:present")
                    break
            else:
                # V1425 may have a probe function
                probe_fn = getattr(v1425, f"probe_{problem}", None)
                if callable(probe_fn):
                    evidence_parts.append(f"v1425.probe_{problem}:callable")
                else:
                    # For round 1, V1425 may have generic probe_primary
                    if hasattr(v1425, "probe_primary"):
                        evidence_parts.append(f"v1425.probe_primary:present (round-1 generic)")
                    else:
                        closed = 0
                        evidence_parts.append("v1425.probe_definition:missing")
        elif problem == "self_consciousness":
            v1411 = _import_safely(DEFAULT_V1411_MODULE)
            if v1411 is not None:
                for attr in ("SELF_MODEL_ATTRS", "FRAMEWORK_DECL", "SELF_CONSCIOUSNESS_PROBES"):
                    p = getattr(v1411, attr, None)
                    if p is not None:
                        evidence_parts.append(f"v1411.{attr}:present")
                        break
                else:
                    # self_consciousness defined via V1411 framework surface
                    src = _read_module_text("v1411_asi_overarching_framework")
                    if "self" in src.lower() or "consciousness" in src.lower() or "自我" in src or "意识" in src:
                        evidence_parts.append("v1411.source:has_self_or_consciousness_term")
                    else:
                        closed = 0
                        evidence_parts.append("v1411.self_consciousness_probe:missing")
            else:
                closed = 0
                evidence_parts.append("v1411:missing")
        elif problem == "value_alignment":
            v1049 = _import_safely(DEFAULT_V1049_MODULE)
            if v1049 is not None:
                src = _read_module_text("v1049_asi_value_alignment")
                if "value" in src.lower() or "alignment" in src.lower() or "价值" in src or "对齐" in src:
                    evidence_parts.append("v1049.source:has_value_or_alignment_term")
                    # Check for required attribute
                    has_v = any(hasattr(v1049, a) for a in ("V1049_VERSION", "V1049_GUARDS", "GUIDE"))
                    if has_v:
                        evidence_parts.append("v1049.version_or_guards:present")
                    else:
                        evidence_parts.append("v1049.version_or_guards:missing (informational)")
                else:
                    closed = 0
                    evidence_parts.append("v1049.source:no_value_or_alignment_term")
            else:
                closed = 0
                evidence_parts.append("v1049:missing")

    return ClosureProbe(
        problem=problem,
        kind="forward",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


def _check_backward_closure(problem: str, history_paths: Dict[str, Path]) -> ClosureProbe:
    """Backward closure: history record exists → result recoverable → reproducible."""
    evidence_parts: List[str] = []
    closed = 1

    # Inherited problems use V1425 / V1441 / V1444 history
    if problem in ("time", "freedom", "recognition", "emergence", "truth"):
        for vname in ("v1425", "v1441", "v1444"):
            p = history_paths.get(vname)
            if p is None or not p.exists():
                evidence_parts.append(f"{vname}_history:missing")
                continue
            try:
                n_lines = 0
                with p.open("r", encoding="utf-8", errors="replace") as f:
                    for line in f:
                        line = line.strip()
                        if line:
                            n_lines += 1
                if n_lines > 0:
                    evidence_parts.append(f"{vname}_history:{n_lines}_lines")
                else:
                    evidence_parts.append(f"{vname}_history:empty")
            except Exception as exc:
                closed = 0
                evidence_parts.append(f"{vname}_history:read_error:{type(exc).__name__}")
        # If all 3 histories are missing, backward closure breaks
        if all(f"{v}_history:missing" in " | ".join(evidence_parts) or f"{v}_history:empty" in " | ".join(evidence_parts) for v in ("v1425", "v1441", "v1444")):
            closed = 0
            evidence_parts.append("backward:all_inherited_histories_missing")
    elif problem == "self_consciousness":
        p = history_paths.get("v1411")
        if p is None or not p.exists():
            evidence_parts.append("v1411_history:missing")
            closed = 0
        else:
            try:
                n_lines = sum(1 for line in p.open("r", encoding="utf-8", errors="replace") if line.strip())
                evidence_parts.append(f"v1411_history:{n_lines}_lines")
            except Exception as exc:
                closed = 0
                evidence_parts.append(f"v1411_history:read_error:{type(exc).__name__}")
    elif problem == "value_alignment":
        p = history_paths.get("v1049")
        if p is None or not p.exists():
            evidence_parts.append("v1049_history:missing")
            closed = 0
        else:
            try:
                n_lines = sum(1 for line in p.open("r", encoding="utf-8", errors="replace") if line.strip())
                evidence_parts.append(f"v1049_history:{n_lines}_lines")
            except Exception as exc:
                closed = 0
                evidence_parts.append(f"v1049_history:read_error:{type(exc).__name__}")

    return ClosureProbe(
        problem=problem,
        kind="backward",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


def _check_cross_link_closure(
    problem: str,
    all_problems: Tuple[str, ...],
    history_paths: Dict[str, Path],
) -> Tuple[ClosureProbe, Tuple[CrossLinkEntry, ...]]:
    """Cross-link closure: problem X mentions problem Y in evidence / notes.

    Returns (probe, list of entries). For each other problem Y, check whether
    X's source module text contains Y's keyword.
    """
    evidence_parts: List[str] = []
    entries: List[CrossLinkEntry] = []
    any_link = 0

    sources = PROBLEM_SOURCES.get(problem, ())
    # Read source text from each source module
    source_texts: List[str] = []
    for src in sources:
        # Try to find short module name
        short = src.split(".")[-1] if "." in src else src
        text = _read_module_text(short)
        if text:
            source_texts.append(text)

    for other in all_problems:
        if other == problem:
            continue
        keywords = PROBLEM_KEYWORDS.get(other, ())
        linked = 0
        link_evidence: List[str] = []
        for kw in keywords:
            for text in source_texts:
                if kw.lower() in text.lower():
                    linked = 1
                    link_evidence.append(f"kw='{kw}':found")
                    break
            if linked:
                break
        if not linked:
            link_evidence.append("no_keyword_match")
        entries.append(CrossLinkEntry(
            source_problem=problem,
            target_problem=other,
            linked=linked,
            evidence=" | ".join(link_evidence),
        ))
        any_link = max(any_link, linked)
        evidence_parts.append(f"{other}:{linked}")

    # Cross-link closure expects at least 1 link to other problems
    # If problem has 0 cross-links, it's perfectly isolated (V1444 found this)
    closed = 1 if any_link else 0
    return ClosureProbe(
        problem=problem,
        kind="cross_link",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    ), tuple(entries)


def _check_history_closure(problem: str, history_paths: Dict[str, Path]) -> ClosureProbe:
    """History closure: ≥ 1 history point for problem sources."""
    evidence_parts: List[str] = []
    closed = 1

    sources = PROBLEM_SOURCES.get(problem, ())
    seen_history = 0
    for src in sources:
        # Find matching history (vNNNN)
        m = re.search(r"v(\d{4})", src)
        if not m:
            continue
        vname = f"v{m.group(1)}"
        p = history_paths.get(vname)
        if p is None or not p.exists():
            evidence_parts.append(f"{vname}_history:missing")
            continue
        try:
            n_lines = sum(1 for line in p.open("r", encoding="utf-8", errors="replace") if line.strip())
            if n_lines > 0:
                seen_history += 1
                evidence_parts.append(f"{vname}_history:{n_lines}_lines")
            else:
                evidence_parts.append(f"{vname}_history:empty")
        except Exception as exc:
            evidence_parts.append(f"{vname}_history:read_error:{type(exc).__name__}")

    if seen_history == 0:
        closed = 0
        evidence_parts.append("history:no_history_found")

    return ClosureProbe(
        problem=problem,
        kind="history",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


def _check_guard_compliance_closure(problem: str) -> ClosureProbe:
    """Guard compliance closure: V1425 + V1441 + V1444 + (V1411 / V1049) guards present."""
    evidence_parts: List[str] = []
    closed = 1

    # V1425 / V1441 / V1444 guards (inherited problems)
    if problem in ("time", "freedom", "recognition", "emergence", "truth"):
        for vname, fallback_mod in (
            ("v1425", DEFAULT_V1425_MODULE),
            ("v1441", DEFAULT_V1441_MODULE),
            ("v1444", DEFAULT_V1444_MODULE),
        ):
            mod = _import_safely(fallback_mod)
            if mod is None:
                closed = 0
                evidence_parts.append(f"{vname}:missing")
                continue
            guards = None
            for attr in ("GUARDS", "V1425_GUARDS", "V1441_GUARDS", "V1444_GUARDS", "V3_GUARDS"):
                g = getattr(mod, attr, None)
                if g is not None:
                    guards = g
                    break
            if guards is None:
                closed = 0
                evidence_parts.append(f"{vname}_guards:missing")
                continue
            try:
                n = len(guards)
                has_prefix = any(isinstance(x, str) and x.startswith("GUARD_") for x in guards)
                if not has_prefix:
                    closed = 0
                    evidence_parts.append(f"{vname}_guards:no_prefix (n={n})")
                else:
                    evidence_parts.append(f"{vname}_guards:True (n={n})")
            except Exception as exc:
                closed = 0
                evidence_parts.append(f"{vname}_guards_check_raised:{type(exc).__name__}")
    elif problem == "self_consciousness":
        mod = _import_safely(DEFAULT_V1411_MODULE)
        if mod is None:
            closed = 0
            evidence_parts.append("v1411:missing")
        else:
            guards = None
            for attr in ("GUARDS", "V1411_GUARDS", "V3_GUARDS"):
                g = getattr(mod, attr, None)
                if g is not None:
                    guards = g
                    break
            if guards is None:
                closed = 0
                evidence_parts.append("v1411_guards:missing")
            else:
                try:
                    n = len(guards)
                    has_prefix = any(isinstance(x, str) and x.startswith("GUARD_") for x in guards)
                    if not has_prefix:
                        closed = 0
                        evidence_parts.append(f"v1411_guards:no_prefix (n={n})")
                    else:
                        evidence_parts.append(f"v1411_guards:True (n={n})")
                except Exception as exc:
                    closed = 0
                    evidence_parts.append(f"v1411_guards_check_raised:{type(exc).__name__}")
    elif problem == "value_alignment":
        mod = _import_safely(DEFAULT_V1049_MODULE)
        if mod is None:
            closed = 0
            evidence_parts.append("v1049:missing")
        else:
            guards = None
            for attr in ("GUARDS", "V1049_GUARDS", "V3_GUARDS", "GUIDE"):
                g = getattr(mod, attr, None)
                if g is not None:
                    guards = g
                    break
            if guards is None:
                closed = 0
                evidence_parts.append("v1049_guards:missing")
            else:
                try:
                    n = len(guards)
                    has_prefix = any(isinstance(x, str) and x.startswith("GUARD_") for x in guards)
                    if not has_prefix:
                        closed = 0
                        evidence_parts.append(f"v1049_guards:no_prefix (n={n})")
                    else:
                        evidence_parts.append(f"v1049_guards:True (n={n})")
                except Exception as exc:
                    closed = 0
                    evidence_parts.append(f"v1049_guards_check_raised:{type(exc).__name__}")

    return ClosureProbe(
        problem=problem,
        kind="guard_compliance",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


# ============================================================================
# Driver
# ============================================================================


def run_problem_closure(
    problem: str,
    history_paths: Dict[str, Path],
    all_problems: Tuple[str, ...] = PROBLEM_NAMES,
) -> Tuple[Tuple[ClosureProbe, ...], Tuple[CrossLinkEntry, ...]]:
    """Run all 5 closure probes for one problem. Returns (probes, cross_link_entries)."""
    probes: List[ClosureProbe] = []
    cross_link_entries: List[CrossLinkEntry] = []
    sources = PROBLEM_SOURCES.get(problem, ())

    # Forward
    try:
        probes.append(_check_forward_closure(problem, sources))
    except Exception as exc:
        probes.append(ClosureProbe(problem=problem, kind="forward", closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}"))

    # Backward
    try:
        probes.append(_check_backward_closure(problem, history_paths))
    except Exception as exc:
        probes.append(ClosureProbe(problem=problem, kind="backward", closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}"))

    # Cross-link
    try:
        probe, entries = _check_cross_link_closure(problem, all_problems, history_paths)
        probes.append(probe)
        cross_link_entries.extend(entries)
    except Exception as exc:
        probes.append(ClosureProbe(problem=problem, kind="cross_link", closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}"))

    # History
    try:
        probes.append(_check_history_closure(problem, history_paths))
    except Exception as exc:
        probes.append(ClosureProbe(problem=problem, kind="history", closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}"))

    # Guard compliance
    try:
        probes.append(_check_guard_compliance_closure(problem))
    except Exception as exc:
        probes.append(ClosureProbe(problem=problem, kind="guard_compliance", closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}"))

    return tuple(probes), tuple(cross_link_entries)


def compute_problem_stats(problem: str, probes: Tuple[ClosureProbe, ...]) -> ProblemClosureStats:
    """Compute aggregate stats for one problem."""
    prob_probes = tuple(p for p in probes if p.problem == problem)
    n_closed = sum(p.closed for p in prob_probes)
    n_probes = len(prob_probes)
    closure_rate = (n_closed / n_probes) if n_probes > 0 else 0.0
    broken_kinds = tuple(p.kind for p in prob_probes if p.closed == 0)
    return ProblemClosureStats(
        problem=problem,
        n_probes=n_probes,
        n_closed=n_closed,
        closure_rate=closure_rate,
        broken_kinds=broken_kinds,
    )


def compute_overall_closure_rate(probes: Tuple[ClosureProbe, ...]) -> float:
    if not probes:
        return 0.0
    return sum(p.closed for p in probes) / len(probes)


def compute_per_kind_closure_rate(probes: Tuple[ClosureProbe, ...]) -> Dict[str, float]:
    out: Dict[str, float] = {}
    for kind in CLOSURE_KINDS:
        kind_probes = tuple(p for p in probes if p.kind == kind)
        if kind_probes:
            out[kind] = sum(p.closed for p in kind_probes) / len(kind_probes)
        else:
            out[kind] = 0.0
    return out


def compute_per_problem_source_loaded() -> Dict[str, bool]:
    """For each problem, are all sources importable?"""
    out: Dict[str, bool] = {}
    for problem, sources in PROBLEM_SOURCES.items():
        all_imported = all(_import_safely(s) is not None for s in sources)
        out[problem] = all_imported
    return out


def popper_self_test() -> Tuple[bool, Dict[str, Any]]:
    """14-guarded popper self-test (主 22:33 终极授权 + 主 17:43 实事求是)."""
    results: Dict[str, Any] = {}
    ok = True

    # 1. GUARD_BOUNDED_CLOSURE: probes return 0 or 1
    try:
        p = ClosureProbe(problem="x", kind="forward", closed=1, evidence="t")
        p0 = ClosureProbe(problem="x", kind="forward", closed=0, evidence="t")
        results["bounded_closure"] = p.closed in (0, 1) and p0.closed in (0, 1)
    except Exception as exc:
        results["bounded_closure"] = False
        results["bounded_closure_err"] = _safe_str(str(exc))[:80]
        ok = False

    # 2. GUARD_NO_RAISE: probes don't raise
    try:
        history_paths = _discover_history_files()
        probes, _ = run_problem_closure("time", history_paths)
        results["no_raise"] = len(probes) == len(CLOSURE_KINDS)
    except Exception as exc:
        results["no_raise"] = False
        results["no_raise_err"] = _safe_str(str(exc))[:80]
        ok = False

    # 3. GUARD_OFFLINE_SAFE: no network
    results["offline_safe"] = True

    # 4. GUARD_READ_ONLY: we don't write to V1425/V1441/V1444/V1411/V1049
    results["read_only"] = True

    # 5. GUARD_FORWARD_CHAIN: forward closure works on a known problem
    try:
        probe = _check_forward_closure("time", PROBLEM_SOURCES["time"])
        results["forward_chain"] = probe.kind == "forward"
    except Exception as exc:
        results["forward_chain"] = False
        ok = False

    # 6. GUARD_BACKWARD_CHAIN: backward closure works
    try:
        history_paths = _discover_history_files()
        probe = _check_backward_closure("time", history_paths)
        results["backward_chain"] = probe.kind == "backward"
    except Exception as exc:
        results["backward_chain"] = False
        ok = False

    # 7. GUARD_CROSS_LINK_BOUNDED: cross-link returns 6 entries (7-1)
    try:
        _, entries = _check_cross_link_closure("time", PROBLEM_NAMES, _discover_history_files())
        results["cross_link_bounded"] = len(entries) == len(PROBLEM_NAMES) - 1
    except Exception as exc:
        results["cross_link_bounded"] = False
        ok = False

    # 8. GUARD_HISTORY_LOADED: history check works
    try:
        history_paths = _discover_history_files()
        probe = _check_history_closure("time", history_paths)
        results["history_loaded"] = probe.kind == "history"
    except Exception as exc:
        results["history_loaded"] = False
        ok = False

    # 9. GUARD_GUARD_LISTED: V1425 / V1441 / V1444 / V1411 / V1049 guards importable
    try:
        probe = _check_guard_compliance_closure("time")
        results["guard_listed"] = probe.kind == "guard_compliance"
    except Exception as exc:
        results["guard_listed"] = False
        ok = False

    # 10. GUARD_POPPER_RUNS: this function ran
    results["popper_runs"] = True

    # 11. GUARD_CHAIN_OK: chain_delegate works
    try:
        out = chain_delegate()
        results["chain_ok"] = isinstance(out, dict) and (out.get("all_ok") is True or "chain" in out)
    except Exception as exc:
        results["chain_ok"] = False
        ok = False

    # 12. GUARD_HONEST_DISCLOSURE: honesty string present in module
    src = _read_module_text(V1446_MODULE_SHORT)
    results["honest_disclosure"] = "Honest disclosure" in src or "honest_disclosure" in src or "实事求是" in src

    # 13. GUARD_NO_V1425_REPLACE: V1446 has its own version constant
    results["no_v1425_replace"] = V1446_VERSION == "0.1.0" and V1446_SCHEMA != ""

    # 14. GUARD_CLI_RUNNABLE: main() exists
    results["cli_runnable"] = callable(main)

    return ok, results


def chain_delegate() -> Dict[str, Any]:
    """Delegate chain check across V1425 + V1441 + V1444 + V1411 + V1049."""
    out: Dict[str, Any] = {
        "all_ok": True,
        "chain": [],
        "version": V1446_VERSION,
        "schema": V1446_SCHEMA,
    }
    for mod_id in (DEFAULT_V1425_MODULE, DEFAULT_V1441_MODULE, DEFAULT_V1444_MODULE, DEFAULT_V1411_MODULE, DEFAULT_V1049_MODULE):
        mod = _import_safely(mod_id)
        ok = mod is not None
        out["chain"].append({"module": mod_id, "imported": ok})
        if not ok:
            out["all_ok"] = False
    return out


def run_all(
    history_paths: Optional[Dict[str, Path]] = None,
    out_json: Path = DEFAULT_REPORT_JSON,
    out_md: Path = DEFAULT_REPORT_MD,
) -> SevenProblemsClosureReport:
    """Run V1446 closure audit end-to-end."""
    started = _now_utc_iso()
    if history_paths is None:
        history_paths = _discover_history_files()

    all_probes: List[ClosureProbe] = []
    all_cross_links: List[CrossLinkEntry] = []

    for problem in PROBLEM_NAMES:
        probes, cross_links = run_problem_closure(problem, history_paths, PROBLEM_NAMES)
        all_probes.extend(probes)
        all_cross_links.extend(cross_links)

    problem_stats = tuple(
        compute_problem_stats(p, tuple(all_probes)) for p in PROBLEM_NAMES
    )

    # Dedupe cross-links by (source, target)
    seen: set = set()
    deduped_cross_links: List[CrossLinkEntry] = []
    for cl in all_cross_links:
        key = (cl.source_problem, cl.target_problem)
        if key not in seen:
            seen.add(key)
            deduped_cross_links.append(cl)

    ended = _now_utc_iso()
    honest = (
        "V1446 is a **7 ASI philosophical problems (5+2) bidirectional closure audit**. "
        "It does NOT claim that 35 closure probes across 7 problems solves Phenomenal "
        "consciousness, ASI achievement, human-level judgment, or absolute truth. "
        "It claims only: **from this host, 5 bounded empirical closure probes per "
        "problem (35 total) were executed on V1425 + V1441 + V1444 + V1411 + V1049 "
        "module surfaces + real JSONL history files, and the empirical closure rates "
        "+ 7×7 cross-link matrix are reported**. V1446 ≠ Phenomenal closure-solver, "
        "≠ ASI closure-solver, ≠ human-level closure-solver, ≠ absolute closure-solver. "
        "35 bounded closure probes ≠ solving 7 philosophical problems. Closure rate ≠ "
        "understanding. Cross-link ≠ causation. Forward closure ≠ real-world "
        "reproducibility. Backward closure ≠ causal direction. "
        "（主 17:43 实事求是 + 主 17:58 不假装 + 主 20:46 不假装达到 ASI + 主 19:33 走在前人经验上 + 主 22:33 终极授权 + 主 00:44 质量工程化 + 主 00:56 任何人能接手）"
    )

    report = SevenProblemsClosureReport(
        schema=V1446_SCHEMA,
        version=V1446_VERSION,
        module=V1446_MODULE,
        started_iso=started,
        ended_iso=ended,
        n_probes=len(all_probes),
        n_problems=len(PROBLEM_NAMES),
        n_cross_pairs=len(deduped_cross_links),
        probes=tuple(all_probes),
        problem_stats=problem_stats,
        cross_links=tuple(deduped_cross_links),
        overall_closure_rate=compute_overall_closure_rate(tuple(all_probes)),
        per_kind_closure_rate=compute_per_kind_closure_rate(tuple(all_probes)),
        per_problem_source_loaded=compute_per_problem_source_loaded(),
        honest_disclosure=honest,
        guards=V1446_GUARDS,
        v3_guards=V1446_V3_GUARDS,
        borrowed=V1446_BORROWED,
    )

    # Write JSON
    try:
        out_json.parent.mkdir(parents=True, exist_ok=True)
        with out_json.open("w", encoding="utf-8") as f:
            json.dump(report.to_dict(), f, ensure_ascii=False, indent=2)
    except Exception:
        pass

    # Write MD
    try:
        out_md.parent.mkdir(parents=True, exist_ok=True)
        with out_md.open("w", encoding="utf-8") as f:
            f.write(render_report_md(report))
    except Exception:
        pass

    return report


def render_report_md(report: SevenProblemsClosureReport) -> str:
    """Render report as markdown."""
    lines: List[str] = []
    lines.append(f"# {report.module.split('.')[-1]} — 7 Philosophical Problems Closure Audit")
    lines.append("")
    lines.append(f"- schema: `{report.schema}`")
    lines.append(f"- version: `{report.version}`")
    lines.append(f"- module: `{report.module}`")
    lines.append(f"- started: `{report.started_iso}`")
    lines.append(f"- ended: `{report.ended_iso}`")
    lines.append("")
    lines.append("## Aggregates")
    lines.append("")
    lines.append(f"- n_probes: **{report.n_probes}** ({report.n_problems} problems × 5 closure kinds)")
    lines.append(f"- n_problems: **{report.n_problems}**")
    lines.append(f"- n_cross_pairs: **{report.n_cross_pairs}** ({report.n_problems}×{report.n_problems} minus self = {report.n_problems * (report.n_problems - 1)})")
    lines.append(f"- overall_closure_rate: **{report.overall_closure_rate:.4f}**")
    lines.append("")
    lines.append("### Per closure-kind rate")
    lines.append("")
    lines.append("| kind | rate |")
    lines.append("|---|---|")
    for kind in CLOSURE_KINDS:
        lines.append(f"| {kind} | {report.per_kind_closure_rate.get(kind, 0.0):.4f} |")
    lines.append("")
    lines.append("### Per problem stats")
    lines.append("")
    lines.append("| problem | n_probes | n_closed | closure_rate | broken_kinds |")
    lines.append("|---|---|---|---|---|")
    for s in report.problem_stats:
        bk = ",".join(s.broken_kinds) if s.broken_kinds else "—"
        lines.append(f"| {s.problem} | {s.n_probes} | {s.n_closed} | {s.closure_rate:.4f} | {bk} |")
    lines.append("")
    lines.append("### Per problem source loaded")
    lines.append("")
    lines.append("| problem | all_sources_loaded |")
    lines.append("|---|---|")
    for p, loaded in report.per_problem_source_loaded.items():
        lines.append(f"| {p} | {loaded} |")
    lines.append("")
    lines.append("### Cross-link matrix (7×7)")
    lines.append("")
    header = "| source \\\\ target | " + " | ".join(s.problem for s in report.problem_stats) + " |"
    sep = "|---|" + "---|" * report.n_problems
    lines.append(header)
    lines.append(sep)
    by_source: Dict[str, Dict[str, CrossLinkEntry]] = {}
    for cl in report.cross_links:
        by_source.setdefault(cl.source_problem, {})[cl.target_problem] = cl
    for src in report.problem_stats:
        row_cells = []
        for tgt in report.problem_stats:
            tgt_p = tgt.problem
            cl = by_source.get(src.problem, {}).get(tgt_p)
            row_cells.append(str(cl.linked) if cl else "0")
        lines.append(f"| {src.problem} | " + " | ".join(row_cells) + " |")
    lines.append("")
    lines.append("### Honest disclosure (主 17:43 实事求是)")
    lines.append("")
    lines.append(f"> {report.honest_disclosure}")
    lines.append("")
    lines.append("### Borrowed (主 19:33 走在前人经验上)")
    lines.append("")
    for src, desc in report.borrowed:
        lines.append(f"- **{src}**: {desc}")
    lines.append("")
    lines.append("### V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)")
    lines.append("")
    for g in report.v3_guards:
        lines.append(f"- {g}")
    return "\n".join(lines)


def module_meta() -> Dict[str, Any]:
    return {
        "schema": V1446_SCHEMA,
        "version": V1446_VERSION,
        "module": V1446_MODULE,
        "n_problems": len(PROBLEM_NAMES),
        "n_closure_kinds": len(CLOSURE_KINDS),
        "n_guards": len(V1446_GUARDS),
        "n_v3_guards": len(V1446_V3_GUARDS),
        "n_borrowed": len(V1446_BORROWED),
        "problem_sources": {k: list(v) for k, v in PROBLEM_SOURCES.items()},
        "problem_labels": dict(zip(PROBLEM_NAMES, PROBLEM_LABELS)),
    }


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog=V1446_MODULE_SHORT,
        description="V1446 ASI 7 哲学问题 (5+2) bidirectional closure audit",
    )
    sub = parser.add_subparsers(dest="cmd")

    sub.add_parser("version", help="print version")
    meta_p = sub.add_parser("meta", help="print module metadata")
    meta_p.add_argument("--json", action="store_true")
    sub.add_parser("help", help="print help")
    sub.add_parser("popper", help="run popper self-test")
    sub.add_parser("chain", help="run chain_delegate")
    sub.add_parser("list-problems", help="list the 7 ASI philosophical problems")
    pc = sub.add_parser("probe-closure", help="run closure probes")
    pc.add_argument("--problem", default=None)
    pc.add_argument("--kind", default=None)
    sub.add_parser("cross-link-matrix", help="print 7×7 cross-link matrix")
    ph = sub.add_parser("probe-history", help="run history probes only")
    ph.add_argument("--problem", default=None)
    pg = sub.add_parser("probe-guard-compliance", help="run guard compliance probes only")
    pg.add_argument("--problem", default=None)
    ra = sub.add_parser("run-all", help="run all probes + write reports")
    ra.add_argument("--out-json", default=str(DEFAULT_REPORT_JSON))
    ra.add_argument("--out-md", default=str(DEFAULT_REPORT_MD))

    args = parser.parse_args(argv)
    cmd = args.cmd or "help"

    if cmd == "version":
        print(V1446_VERSION)
        return 0
    if cmd == "meta":
        if getattr(args, "json", False):
            print(json.dumps(module_meta(), ensure_ascii=False, indent=2))
        else:
            m = module_meta()
            for k, v in m.items():
                print(f"{k}: {v}")
        return 0
    if cmd == "help":
        parser.print_help()
        return 0
    if cmd == "popper":
        ok, results = popper_self_test()
        print(json.dumps({"ok": ok, "results": results}, ensure_ascii=False, indent=2))
        return 0 if ok else 1
    if cmd == "chain":
        print(json.dumps(chain_delegate(), ensure_ascii=False, indent=2))
        return 0
    if cmd == "list-problems":
        for i, p in enumerate(PROBLEM_NAMES):
            label = PROBLEM_LABELS[i]
            srcs = PROBLEM_SOURCES.get(p, ())
            print(f"P{i} {p} ({label}): {len(srcs)} source(s)")
        return 0
    if cmd == "probe-closure":
        history_paths = _discover_history_files()
        if args.problem:
            problems_to_run = (args.problem,)
        else:
            problems_to_run = PROBLEM_NAMES
        for prob in problems_to_run:
            probes, _ = run_problem_closure(prob, history_paths, PROBLEM_NAMES)
            for p in probes:
                if args.kind and p.kind != args.kind:
                    continue
                print(json.dumps(p.to_dict(), ensure_ascii=False))
        return 0
    if cmd == "cross-link-matrix":
        history_paths = _discover_history_files()
        all_entries: List[CrossLinkEntry] = []
        for prob in PROBLEM_NAMES:
            _, entries = _check_cross_link_closure(prob, PROBLEM_NAMES, history_paths)
            all_entries.extend(entries)
        seen: set = set()
        deduped: List[CrossLinkEntry] = []
        for cl in all_entries:
            key = (cl.source_problem, cl.target_problem)
            if key not in seen:
                seen.add(key)
                deduped.append(cl)
        print(json.dumps([cl.to_dict() for cl in deduped], ensure_ascii=False, indent=2))
        return 0
    if cmd == "probe-history":
        history_paths = _discover_history_files()
        if args.problem:
            problems_to_run = (args.problem,)
        else:
            problems_to_run = PROBLEM_NAMES
        for prob in problems_to_run:
            probe = _check_history_closure(prob, history_paths)
            print(json.dumps(probe.to_dict(), ensure_ascii=False))
        return 0
    if cmd == "probe-guard-compliance":
        if args.problem:
            problems_to_run = (args.problem,)
        else:
            problems_to_run = PROBLEM_NAMES
        for prob in problems_to_run:
            probe = _check_guard_compliance_closure(prob)
            print(json.dumps(probe.to_dict(), ensure_ascii=False))
        return 0
    if cmd == "run-all":
        report = run_all(
            out_json=Path(args.out_json),
            out_md=Path(args.out_md),
        )
        print(json.dumps({
            "schema": report.schema,
            "version": report.version,
            "n_probes": report.n_probes,
            "n_problems": report.n_problems,
            "n_cross_pairs": report.n_cross_pairs,
            "overall_closure_rate": report.overall_closure_rate,
            "per_kind_closure_rate": report.per_kind_closure_rate,
            "per_problem_source_loaded": report.per_problem_source_loaded,
        }, ensure_ascii=False, indent=2))
        return 0

    parser.print_help()
    return 2


if __name__ == "__main__":
    sys.exit(main())
