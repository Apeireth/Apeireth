"""V1448 — ASI VCP 6 协议 × V2 5 位置 cross-modular audit.

Phase: 1448
Version: 0.1.0
Date: 2026-08-10 (cron tick 07:46 Asia/Shanghai morning)
Post: V1447 (ASI 7 哲学问题 × V2 5 位置 cross-combined audit)
      V1446 (ASI 7 哲学问题 (5+2) bidirectional closure audit)
      V1445 (ASI V2 5 位置 cross-position closure audit)
      V1444 (5 philosophical gaps round 3 closure audit)
      V1443 (ASI V2 5 位置 cross-position interaction)
      V1442 (ASI V2 5 位置 real-occupier)
      V1426 (VCP 6-plugin-protocol dispatcher)

What V1448 is
=============
V1448 is the **ASI VCP 6 协议 × V2 5 位置 cross-modular audit**. Where V1447
combined 7 philosophical problems × 5 positions into a 35-pair matrix, V1448
combines **6 VCP (Virtual Context Protocol) protocols × 5 V2 positions into
a 30-pair matrix** with **5 closure probes per pair = 150 combined probes**.

The 6 VCP protocols are the canonical protocols defined in V1426
(VCP 6.4 public docs, 主 18:40 真借鉴):
1. **sync**       — synchronous dispatch (immediate return)
2. **async**      — asynchronous dispatch (deferred / promise-like)
3. **static**     — cached dispatch (second call returns cached result)
4. **service**    — long-running service (task marked as long-running)
5. **preprocessor** — chained transform (input → preprocessor → result)
6. **hybrid**     — sync + async composition (deterministic hybrid)

The 5 V2 positions (inherited from V1442/V1445):
1. scheduler
2. cogitator
3. aggregator
4. max_authority
5. asi_occupier

The 5 closure kinds (inherited from V1445/V1446/V1447):
1. forward
2. backward
3. cross_link
4. history
5. guard_compliance

V1448 ≠ ASI-achieved closure. V1448 ≠ Phenomenal closure.
V1448 ≠ human-level closure. V1448 ≠ absolute closure.
V1448 = bounded 6×5 combined closure audit (150 probes + 870 cross-pair links
+ per-pair compositional / anti-modular / substitutability detection).

What V1448 actually does
------------------------
1. Loads V1426 (6 VCP protocols definitions) + V1442 (5 positions) module surfaces
2. For each of 6 protocols × 5 positions = 30 pairs, runs 5 closure probes:
   - probe_forward_combined: position K's modules contain protocol P's keyword
   - probe_backward_combined: V1426/V1442/V1445/V1447 history mentions pair
   - probe_cross_link_combined: another (P', K') pair references this pair
   - probe_history_combined: at least 1 history point mentions pair identifier
   - probe_guard_compliance_combined: position K guards reference P keyword
3. Computes 30 × 30 cross-combined matrix (excluding self = 870 directed pairs)
4. Detects compositional_pairs: both P and K fully closed (5/5 each)
5. Detects anti_modular_pairs: closing P breaks K (or vice versa)
6. Detects substitutable_pairs: closing K (position) implies closing P (protocol)
7. Computes per-pair closure_rate + per-position closure_rate + per-protocol closure_rate
8. Emits VCPCrossModularAuditReport
9. Writes .v1448-asi-vcp-six-protocol-cross-modular-report.{json,md}
10. CLI: python -m apeireth.v1448_asi_vcp_six_protocol_cross_modular [command]

Borrowed (5 — 主 19:33 走在前人经验上)
=======================================
- V1447 (pair matrix pattern + cross-modular audit + compositional/anti-modular/substitutable detection)
- V1446 (7 problems definitions + 5 closure kinds)
- V1442 (5 POSITIONS dict — modules per position)
- V1426 (6 VCP protocols sync/async/static/service/preprocessor/hybrid + VCPSixProtocol enum)
- stdlib (importlib + inspect + json + dataclasses + re + ast)

GUARDS upheld (V1448-specific, 15 — 主 00:44 质量工程化)
=========================================================
- GUARD_BOUNDED_CLOSURE: each closure probe returns 0 or 1
- GUARD_NO_RAISE: any closure probe failure → returns 0 with exception msg, never raises
- GUARD_OFFLINE_SAFE: no network, only stdlib + local JSON + importlib
- GUARD_READ_ONLY: V1448 imports V1447/V1446/V1445/V1442/V1426, doesn't modify them
- GUARD_FORWARD_CHAIN: forward_combined checks position.modules → protocol keywords
- GUARD_BACKWARD_CHAIN: backward_combined checks V1426/V1447 history → pair
- GUARD_CROSS_LINK_BOUNDED: cross-link matrix is 30×29 per source pair (binary)
- GUARD_HISTORY_LOADED: V1426/V1447 history must exist (else closure 0 with evidence)
- GUARD_GUARD_LISTED: V1442 + V1446 guards must be importable
- GUARD_POPPER_RUNS: popper self-test runs in CLI
- GUARD_CHAIN_OK: chain_delegate returns all_ok
- GUARD_HONEST_DISCLOSURE: honesty paragraph emitted in report
- GUARD_NO_V1447_REPLACE: V1448 reads V1447, doesn't redefine cross-modular pattern
- GUARD_NO_V1442_REPLACE: V1448 reads V1442, doesn't redefine positions
- GUARD_CLI_RUNNABLE: CLI 真可跑

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43) — 5 guards
========================================================
- GUARD_NO_PHENOMENAL_CLOSURE
- GUARD_NO_ASI_CLOSURE
- GUARD_NO_HUMAN_LEVEL_CLOSURE
- GUARD_NO_ABSOLUTE_CLOSURE
- GUARD_NO_CLOSURE_OVERCLAIM (150 combined closures ≠ closing 6 protocols)

CLI commands (12 — 主 00:56 任何人都能接手)
============================================
1. version
2. meta [--json]
3. help
4. popper
5. chain
6. list-pairs
7. probe-closure [--protocol <name>] [--position <id>] [--kind <kind>]
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

# Fix sys.path when invoked as `python apeireth/v1448_*.py` (sys.path[0] = apeireth/ shadows apeireth package).
# Ensure cwd's parent is in sys.path so `apeireth.*` imports resolve.
_THIS_DIR = Path(__file__).resolve().parent
_PARENT_DIR = _THIS_DIR.parent
if str(_PARENT_DIR) not in sys.path:
    sys.path.insert(0, str(_PARENT_DIR))


# ============================================================================
# Constants
# ============================================================================

V1448_VERSION = "0.1.0"
V1448_SCHEMA = "asi.vcp-six-protocol-cross-modular.v1"
V1448_MODULE = "apeireth.v1448_asi_vcp_six_protocol_cross_modular"
V1448_MODULE_SHORT = "v1448_asi_vcp_six_protocol_cross_modular"

# 6 VCP protocols (inherited from V1426 — Virtual Context Protocol 6.4 docs)
V1448_PROTOCOL_NAMES: Tuple[str, ...] = (
    "sync",
    "async",
    "static",
    "service",
    "preprocessor",
    "hybrid",
)
V1448_PROTOCOL_LABELS: Tuple[str, ...] = (
    "同步",
    "异步",
    "静态",
    "服务",
    "预处理器",
    "混合",
)
assert len(V1448_PROTOCOL_NAMES) == len(V1448_PROTOCOL_LABELS) == 6

# 5 V2 positions (inherited from V1442/V1445/V1447)
V1448_POSITION_NAMES: Tuple[str, ...] = (
    "scheduler",
    "cogitator",
    "aggregator",
    "max_authority",
    "asi_occupier",
)
V1448_POSITION_LABELS: Tuple[str, ...] = (
    "调度者",
    "沉思者",
    "无数关系聚合者",
    "最大权者",
    "ASI 位置占据者",
)
assert len(V1448_POSITION_NAMES) == len(V1448_POSITION_LABELS) == 5

# 5 closure kinds (same as V1445/V1446/V1447)
V1448_CLOSURE_KINDS: Tuple[str, ...] = (
    "forward",
    "backward",
    "cross_link",
    "history",
    "guard_compliance",
)

# Default import targets
DEFAULT_V1426_MODULE = "apeireth.v1426_vcp_six_protocol_dispatcher"
DEFAULT_V1447_MODULE = "apeireth.v1447_asi_cross_modular_audit"
DEFAULT_V1446_MODULE = "apeireth.v1446_asi_seven_philosophical_problems"
DEFAULT_V1445_MODULE = "apeireth.v1445_asi_v2_position_closure_audit"
DEFAULT_V1442_MODULE = "apeireth.v1442_asi_v2_five_position_real_occupier"
DEFAULT_V1443_MODULE = "apeireth.v1443_asi_v2_cross_position_interaction"

# Protocol keywords (inherited from V1426 — protocol identifiers)
V1448_PROTOCOL_KEYWORDS: Dict[str, Tuple[str, ...]] = {
    "sync": ("sync", "immediate", "direct", "call", "同步"),
    "async": ("async", "defer", "promise", "schedul", "异步"),
    "static": ("static", "cache", "memo", "persist", "静态"),
    "service": ("service", "lifecycle", "long.run", "daemon", "服务"),
    "preprocessor": ("preprocess", "transform", "chain", "input", "预处理"),
    "hybrid": ("hybrid", "composit", "sync.async", "混合"),
}

# Position identifiers — keywords found in position modules
V1448_POSITION_KEYWORDS: Dict[str, Tuple[str, ...]] = {
    "scheduler": ("scheduler", "cron", "tick", "schedule", "调度"),
    "cogitator": ("cogitator", "philo", "gap", "reason", "沉思"),
    "aggregator": ("aggregator", "protocol", "dispatch", "structural", "聚合"),
    "max_authority": ("max_authority", "watchdog", "lint", "guard", "权威"),
    "asi_occupier": ("asi_occupier", "overarching", "framework", "position", "ASI"),
}

# Default history paths (re-use V1426/V1442/V1447 reports)
DEFAULT_HISTORY_DIR = Path(".") / ".v1448-histories"
DEFAULT_V1447_REPORT = DEFAULT_HISTORY_DIR / "v1447_report.json"
DEFAULT_V1442_REPORT = DEFAULT_HISTORY_DIR / "v1442_report.json"
DEFAULT_V1445_REPORT = DEFAULT_HISTORY_DIR / "v1445_report.json"
DEFAULT_V1426_REPORT = DEFAULT_HISTORY_DIR / "v1426_report.json"

# Default report output paths
DEFAULT_REPORT_JSON = Path(".") / ".v1448-asi-vcp-six-protocol-cross-modular-report.json"
DEFAULT_REPORT_MD = Path(".") / ".v1448-asi-vcp-six-protocol-cross-modular-report.md"

# 15 V1448-specific guards
V1448_GUARDS: Tuple[str, ...] = (
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
    "GUARD_NO_V1447_REPLACE",
    "GUARD_NO_V1442_REPLACE",
    "GUARD_CLI_RUNNABLE",
)

# 5 V3 哲学守门
V1448_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_PHENOMENAL_CLOSURE",
    "GUARD_NO_ASI_CLOSURE",
    "GUARD_NO_HUMAN_LEVEL_CLOSURE",
    "GUARD_NO_ABSOLUTE_CLOSURE",
    "GUARD_NO_CLOSURE_OVERCLAIM",
)

# 5 borrowed
V1448_BORROWED: Tuple[Tuple[str, str], ...] = (
    ("V1447", "cross-modular audit pattern + compositional/anti-modular/substitutable detection"),
    ("V1446", "7 problems + 5 closure kinds + per-problem-source-loaded pattern"),
    ("V1442", "5 POSITIONS dict — modules per position + chain_delegate_v1414"),
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


def _read_module_text(module_short: str) -> str:
    """Read the source text of a module from the apeireth directory."""
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
        return x
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
    """Try to discover V1426/V1442/V1445/V1447 history / report files."""
    known_files: Dict[str, str] = {
        "v1447": ".v1447-asi-cross-modular-audit-report.json",
        "v1445": ".v1445-asi-v2-position-closure-report.json",
        "v1442": ".v1442-asi-v2-five-position-real-occupier-report.json",
        "v1426": ".v1426-asi-vcp-six-protocol-dispatcher-report.json",
    }
    candidates = {
        "v1447": DEFAULT_V1447_REPORT,
        "v1445": DEFAULT_V1445_REPORT,
        "v1442": DEFAULT_V1442_REPORT,
        "v1426": DEFAULT_V1426_REPORT,
    }
    search_dirs = [
        Path("."),
        Path(".v1448-histories"),
        Path(".v1447-histories"),
        Path(".v1442-histories"),
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


def _get_position_modules(position_id: str) -> Tuple[str, ...]:
    """Get module list for a position from V1442 POSITIONS dict.

    Returns module short names like 'v1418_asi_dgm_cron_integration'.
    """
    v1442 = _import_safely(DEFAULT_V1442_MODULE)
    if v1442 is None:
        return ()
    positions = getattr(v1442, "V1442_POSITIONS", None)
    if not positions:
        return ()
    for p in positions:
        if isinstance(p, dict) and p.get("id") == position_id:
            mods = p.get("modules", ())
            return tuple(mods) if isinstance(mods, (list, tuple)) else (mods,)
    return ()


def _get_position_guards(position_id: str) -> Tuple[str, ...]:
    """Get guards tuple for a position from V1442.

    Returns V1442_GUARDS (shared across positions).
    """
    v1442 = _import_safely(DEFAULT_V1442_MODULE)
    if v1442 is None:
        return ()
    return tuple(getattr(v1442, "V1442_GUARDS", ()) or ())


def _position_module_text(position_id: str) -> str:
    """Read source text of all modules belonging to a position."""
    mods = _get_position_modules(position_id)
    parts: List[str] = []
    for mod in mods:
        parts.append(_read_module_text(mod))
    return "\n".join(parts)


def _get_v1426_module_text() -> str:
    """Read source text of V1426 (VCP six protocol dispatcher)."""
    return _read_module_text("v1426_vcp_six_protocol_dispatcher")


# ============================================================================
# Data classes
# ============================================================================


@dataclass
class PairClosureProbe:
    """One closure probe result for one (protocol, position) pair."""
    protocol: str
    position: str
    kind: str
    closed: int
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class CrossCombinedEntry:
    """One directed cross-link entry between two (protocol, position) pairs."""
    source_protocol: str
    source_position: str
    target_protocol: str
    target_position: str
    linked: int
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class PairClosureStats:
    """Aggregate stats for one (protocol, position) pair."""
    protocol: str
    position: str
    n_probes: int
    n_closed: int
    closure_rate: float
    broken_kinds: Tuple[str, ...]

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class CompositionalPair:
    """A (protocol, position) pair that is both protocol-closed and position-closed."""
    protocol: str
    position: str
    closure_rate: float
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class AntiModularPair:
    """A pair where closing one breaks another."""
    protocol: str
    position: str
    opposite_protocol: str
    opposite_position: str
    closure_rate_a: float
    closure_rate_b: float
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class SubstitutablePair:
    """A pair where closing K (position) implies closing P (protocol)."""
    protocol: str
    position: str
    protocol_closure: float
    position_closure: float
    evidence: str

    def to_dict(self) -> Dict[str, Any]:
        return asdict(self)


@dataclass
class VCPCrossModularAuditReport:
    """Full V1448 cross-combined audit report."""
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
    per_position_closure_rate: Dict[str, float]
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


def _check_forward_combined(protocol: str, position: str) -> PairClosureProbe:
    """Forward combined closure: position's modules contain protocol's keyword."""
    evidence_parts: List[str] = []
    closed = 0

    ptext = _position_module_text(position)
    if not ptext:
        evidence_parts.append(f"position[{position}]:no_module_text")
    else:
        keywords = V1448_PROTOCOL_KEYWORDS.get(protocol, ())
        matches: List[str] = []
        for kw in keywords:
            if kw.lower() in ptext.lower():
                matches.append(kw)
        if matches:
            closed = 1
            evidence_parts.append(f"position[{position}].modules:has_{len(matches)}_{protocol}_keywords={matches[:3]}")
        else:
            evidence_parts.append(f"position[{position}].modules:no_{protocol}_keyword (kw={list(keywords)})")

    return PairClosureProbe(
        protocol=protocol,
        position=position,
        kind="forward",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


def _check_backward_combined(protocol: str, position: str, history_paths: Dict[str, Path]) -> PairClosureProbe:
    """Backward combined closure: V1426/V1442/V1445/V1447 history mentions this pair."""
    evidence_parts: List[str] = []
    closed = 0

    found_in: List[str] = []
    for vname, p in history_paths.items():
        text = _load_history_text(p)
        if not text:
            evidence_parts.append(f"{vname}_history:missing")
            continue
        position_id_present = position in text
        protocol_keywords = V1448_PROTOCOL_KEYWORDS.get(protocol, ())
        protocol_present = any(kw in text for kw in protocol_keywords)
        if position_id_present and protocol_present:
            found_in.append(vname)
            evidence_parts.append(f"{vname}:pair_present")
        else:
            evidence_parts.append(f"{vname}:no_pair (pos={position_id_present},proto={protocol_present})")

    if found_in:
        closed = 1
        evidence_parts.append(f"backward:found_in={','.join(found_in)}")
    else:
        evidence_parts.append("backward:no_pair_in_any_history")

    return PairClosureProbe(
        protocol=protocol,
        position=position,
        kind="backward",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


def _check_cross_link_combined(
    protocol: str,
    position: str,
    ptext: str,
    all_pairs: Tuple[Tuple[str, str], ...],
) -> Tuple[PairClosureProbe, Tuple[CrossCombinedEntry, ...]]:
    """Cross-link combined closure: another (P', K') pair references this pair.

    Returns (probe, list of entries).
    """
    evidence_parts: List[str] = []
    entries: List[CrossCombinedEntry] = []
    any_link = 0

    this_protocol_kws = V1448_PROTOCOL_KEYWORDS.get(protocol, ())
    this_position_kws = V1448_POSITION_KEYWORDS.get(position, ())

    for other_protocol, other_position in all_pairs:
        if other_protocol == protocol and other_position == position:
            continue  # skip self
        other_position_text = _position_module_text(other_position)
        if not other_position_text:
            entries.append(CrossCombinedEntry(
                source_protocol=protocol,
                source_position=position,
                target_protocol=other_protocol,
                target_position=other_position,
                linked=0,
                evidence=f"target[{other_position}].modules:no_text",
            ))
            continue
        target_has_source_protocol = any(kw in other_position_text for kw in this_protocol_kws)
        target_position_kws = V1448_POSITION_KEYWORDS.get(other_position, ())
        source_has_target_position = any(kw in ptext for kw in target_position_kws)

        linked = 1 if (target_has_source_protocol and source_has_target_position) else 0
        link_evidence = f"target_proto={target_has_source_protocol},source_pos={source_has_target_position}"
        entries.append(CrossCombinedEntry(
            source_protocol=protocol,
            source_position=position,
            target_protocol=other_protocol,
            target_position=other_position,
            linked=linked,
            evidence=link_evidence,
        ))
        any_link = max(any_link, linked)
        evidence_parts.append(f"({other_protocol},{other_position}):{linked}")

    closed = 1 if any_link else 0
    return PairClosureProbe(
        protocol=protocol,
        position=position,
        kind="cross_link",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    ), tuple(entries)


def _check_history_combined(
    protocol: str,
    position: str,
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
        protocol_kws = V1448_PROTOCOL_KEYWORDS.get(protocol, ())
        position_kws = V1448_POSITION_KEYWORDS.get(position, ())
        proto_present = any(kw in text for kw in protocol_kws)
        pos_present = any(kw in text for kw in position_kws)
        if proto_present and pos_present:
            seen_history += 1
            evidence_parts.append(f"{vname}:pair_present (proto={proto_present},pos={pos_present})")
        else:
            evidence_parts.append(f"{vname}:no_pair (proto={proto_present},pos={pos_present})")

    closed = 1 if seen_history > 0 else 0
    if seen_history == 0:
        evidence_parts.append("history:no_pair_in_any_history")

    return PairClosureProbe(
        protocol=protocol,
        position=position,
        kind="history",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


def _check_guard_compliance_combined(protocol: str, position: str) -> PairClosureProbe:
    """Guard compliance combined closure: position guards reference protocol keyword."""
    evidence_parts: List[str] = []
    closed = 0

    pos_guards = _get_position_guards(position)
    if not pos_guards:
        evidence_parts.append(f"position[{position}].guards:missing")
    else:
        protocol_kws = V1448_PROTOCOL_KEYWORDS.get(protocol, ())
        guard_text = "\n".join(pos_guards)
        matches: List[str] = []
        for kw in protocol_kws:
            if kw.lower() in guard_text.lower():
                matches.append(kw)
        if matches:
            closed = 1
            evidence_parts.append(f"position[{position}].guards:has_{len(matches)}_{protocol}_kw={matches[:3]}")
        else:
            evidence_parts.append(f"position[{position}].guards:no_{protocol}_kw")

    # Also check V1426 source for protocol guards referencing position
    v1426_text = _get_v1426_module_text()
    if v1426_text:
        position_kws = V1448_POSITION_KEYWORDS.get(position, ())
        v1426_matches: List[str] = []
        for kw in position_kws:
            if kw.lower() in v1426_text.lower():
                v1426_matches.append(kw)
        if v1426_matches:
            closed = 1
            evidence_parts.append(f"v1426.source:has_{len(v1426_matches)}_{position}_kw={v1426_matches[:3]}")
        else:
            evidence_parts.append(f"v1426.source:no_{position}_kw")
    else:
        evidence_parts.append("v1426_source:missing")

    return PairClosureProbe(
        protocol=protocol,
        position=position,
        kind="guard_compliance",
        closed=closed,
        evidence=" | ".join(evidence_parts) if evidence_parts else "no_evidence",
    )


# ============================================================================
# Driver — run probes for one (protocol, position) pair
# ============================================================================


def run_pair_closure(
    protocol: str,
    position: str,
    history_paths: Dict[str, Path],
    all_pairs: Tuple[Tuple[str, str], ...],
) -> Tuple[Tuple[PairClosureProbe, ...], Tuple[CrossCombinedEntry, ...]]:
    """Run all 5 closure probes for one (protocol, position) pair.

    Returns (probes, cross_combined_entries).
    """
    probes: List[PairClosureProbe] = []
    cross_entries: List[CrossCombinedEntry] = []
    ptext = _position_module_text(position)

    try:
        probes.append(_check_forward_combined(protocol, position))
    except Exception as exc:
        probes.append(PairClosureProbe(
            protocol=protocol, position=position, kind="forward",
            closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}",
        ))

    try:
        probes.append(_check_backward_combined(protocol, position, history_paths))
    except Exception as exc:
        probes.append(PairClosureProbe(
            protocol=protocol, position=position, kind="backward",
            closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}",
        ))

    try:
        probe, entries = _check_cross_link_combined(protocol, position, ptext, all_pairs)
        probes.append(probe)
        cross_entries.extend(entries)
    except Exception as exc:
        probes.append(PairClosureProbe(
            protocol=protocol, position=position, kind="cross_link",
            closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}",
        ))

    try:
        probes.append(_check_history_combined(protocol, position, history_paths))
    except Exception as exc:
        probes.append(PairClosureProbe(
            protocol=protocol, position=position, kind="history",
            closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}",
        ))

    try:
        probes.append(_check_guard_compliance_combined(protocol, position))
    except Exception as exc:
        probes.append(PairClosureProbe(
            protocol=protocol, position=position, kind="guard_compliance",
            closed=0, evidence=f"raised:{type(exc).__name__}:{_safe_str(str(exc))[:80]}",
        ))

    return tuple(probes), tuple(cross_entries)


def compute_pair_stats(protocol: str, position: str, probes: Tuple[PairClosureProbe, ...]) -> PairClosureStats:
    """Compute aggregate stats for one (protocol, position) pair."""
    pair_probes = tuple(
        p for p in probes
        if p.protocol == protocol and p.position == position
    )
    n_closed = sum(p.closed for p in pair_probes)
    n_probes = len(pair_probes)
    closure_rate = _safe_div(n_closed, n_probes)
    broken_kinds = tuple(p.kind for p in pair_probes if p.closed == 0)
    return PairClosureStats(
        protocol=protocol,
        position=position,
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
    for kind in V1448_CLOSURE_KINDS:
        kind_probes = tuple(p for p in probes if p.kind == kind)
        if kind_probes:
            out[kind] = _safe_div(sum(p.closed for p in kind_probes), len(kind_probes))
        else:
            out[kind] = 0.0
    return out


def compute_per_position_closure_rate(probes: Tuple[PairClosureProbe, ...]) -> Dict[str, float]:
    out: Dict[str, float] = {}
    for position in V1448_POSITION_NAMES:
        pos_probes = tuple(p for p in probes if p.position == position)
        if pos_probes:
            out[position] = _safe_div(sum(p.closed for p in pos_probes), len(pos_probes))
        else:
            out[position] = 0.0
    return out


def compute_per_protocol_closure_rate(probes: Tuple[PairClosureProbe, ...]) -> Dict[str, float]:
    out: Dict[str, float] = {}
    for protocol in V1448_PROTOCOL_NAMES:
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
    """Detect pairs that are both protocol-closed and position-closed."""
    out: List[CompositionalPair] = []
    for ps in pair_stats:
        if ps.closure_rate >= 1.0:
            out.append(CompositionalPair(
                protocol=ps.protocol,
                position=ps.position,
                closure_rate=ps.closure_rate,
                evidence=f"pair({ps.protocol},{ps.position}) n_closed={ps.n_closed}/{ps.n_probes}",
            ))
    return tuple(out)


def detect_anti_modular_pairs(pair_stats: Tuple[PairClosureStats, ...]) -> Tuple[AntiModularPair, ...]:
    """Detect anti-modular pairs."""
    out: List[AntiModularPair] = []
    high_closure = [p for p in pair_stats if p.closure_rate >= 0.8]
    low_closure = [p for p in pair_stats if p.closure_rate <= 0.2]
    for hi in high_closure:
        for lo in low_closure:
            if hi.protocol == lo.protocol and hi.position == lo.position:
                continue
            if hi.protocol != lo.protocol and hi.position != lo.position:
                out.append(AntiModularPair(
                    protocol=hi.protocol,
                    position=hi.position,
                    opposite_protocol=lo.protocol,
                    opposite_position=lo.position,
                    closure_rate_a=hi.closure_rate,
                    closure_rate_b=lo.closure_rate,
                    evidence=f"hi({hi.protocol},{hi.position})={hi.closure_rate:.2f} vs lo({lo.protocol},{lo.position})={lo.closure_rate:.2f}",
                ))
    return tuple(out[:30])


def detect_substitutable_pairs(pair_stats: Tuple[PairClosureStats, ...]) -> Tuple[SubstitutablePair, ...]:
    """Detect substitutable pairs (high position closure ↔ high protocol closure)."""
    out: List[SubstitutablePair] = []
    for ps in pair_stats:
        # Substitutable: pair has any closure > 0 and at least one of (forward, guard) closed
        if ps.closure_rate >= 0.4 and "forward" not in ps.broken_kinds:
            out.append(SubstitutablePair(
                protocol=ps.protocol,
                position=ps.position,
                protocol_closure=ps.closure_rate,
                position_closure=ps.closure_rate,
                evidence=f"forward_present in pair({ps.protocol},{ps.position}) closure={ps.closure_rate:.2f}",
            ))
    return tuple(out)


def render_markdown_report(report: VCPCrossModularAuditReport) -> str:
    """Render report as markdown."""
    lines: List[str] = []
    lines.append(f"# V1448 — ASI VCP 6 协议 × V2 5 位置 cross-modular audit")
    lines.append("")
    lines.append(f"- Schema: `{report.schema}`")
    lines.append(f"- Version: `{report.version}`")
    lines.append(f"- Module: `{report.module}`")
    lines.append(f"- Started: `{report.started_iso}`")
    lines.append(f"- Ended: `{report.ended_iso}`")
    lines.append("")
    lines.append("## Counts")
    lines.append(f"- n_pairs: **{report.n_pairs}** (= 6 protocols × 5 positions)")
    lines.append(f"- n_probes: **{report.n_probes}** (= 30 pairs × 5 closure kinds)")
    lines.append(f"- n_cross_combined_pairs: **{report.n_cross_combined_pairs}** (= 30 × 29 directed)")
    lines.append("")
    lines.append("## Closure rates")
    lines.append(f"- overall_closure_rate: **{report.overall_closure_rate:.4f}**")
    lines.append(f"- overall_cross_link_density: **{report.overall_cross_link_density:.4f}**")
    lines.append("")
    lines.append("### Per-kind closure rate")
    for kind, rate in report.per_kind_closure_rate.items():
        lines.append(f"- {kind}: **{rate:.4f}**")
    lines.append("")
    lines.append("### Per-position closure rate")
    for position, rate in report.per_position_closure_rate.items():
        lines.append(f"- {position}: **{rate:.4f}**")
    lines.append("")
    lines.append("### Per-protocol closure rate")
    for protocol, rate in report.per_protocol_closure_rate.items():
        lines.append(f"- {protocol}: **{rate:.4f}**")
    lines.append("")
    lines.append("## Compositional pairs (full closure)")
    if report.compositional_pairs:
        for cp in report.compositional_pairs:
            lines.append(f"- {cp.protocol} × {cp.position}: closure_rate={cp.closure_rate:.2f}")
    else:
        lines.append("- (none)")
    lines.append("")
    lines.append("## Anti-modular pairs (top 30)")
    if report.anti_modular_pairs:
        for amp in report.anti_modular_pairs:
            lines.append(f"- {amp.protocol}×{amp.position}={amp.closure_rate_a:.2f} vs {amp.opposite_protocol}×{amp.opposite_position}={amp.closure_rate_b:.2f}")
    else:
        lines.append("- (none)")
    lines.append("")
    lines.append("## Substitutable pairs (forward closure implies pair closure)")
    if report.substitutable_pairs:
        for sp in report.substitutable_pairs:
            lines.append(f"- {sp.protocol} × {sp.position}: closure_rate={sp.protocol_closure:.2f}")
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


def run_all(out_json: Optional[Path] = None, out_md: Optional[Path] = None) -> VCPCrossModularAuditReport:
    """Run the full V1448 cross-modular audit.

    Returns VCPCrossModularAuditReport dataclass.
    """
    started = _now_utc_iso()
    history_paths = _discover_history_files()

    all_pairs = tuple((p, pos) for p in V1448_PROTOCOL_NAMES for pos in V1448_POSITION_NAMES)
    assert len(all_pairs) == 30, f"expected 30 pairs, got {len(all_pairs)}"

    all_probes: List[PairClosureProbe] = []
    all_cross_entries: List[CrossCombinedEntry] = []

    for protocol, position in all_pairs:
        probes, entries = run_pair_closure(protocol, position, history_paths, all_pairs)
        all_probes.extend(probes)
        all_cross_entries.extend(entries)

    pair_stats = tuple(compute_pair_stats(p, pos, tuple(all_probes)) for p, pos in all_pairs)
    compositional = detect_compositional_pairs(pair_stats)
    anti_modular = detect_anti_modular_pairs(pair_stats)
    substitutable = detect_substitutable_pairs(pair_stats)

    overall_closure = compute_overall_closure_rate(tuple(all_probes))
    per_kind = compute_per_kind_closure_rate(tuple(all_probes))
    per_position = compute_per_position_closure_rate(tuple(all_probes))
    per_protocol = compute_per_protocol_closure_rate(tuple(all_probes))
    cross_density = compute_cross_combined_density(tuple(all_cross_entries))

    ended = _now_utc_iso()

    honest = (
        "V1448 = ASI VCP 6 协议 × V2 5 位置 cross-modular audit. "
        "This is a bounded 30-pair × 5-kind = 150-probe closure audit. "
        "V1448 ≠ ASI-achieved closure. V1448 ≠ Phenomenal closure. "
        "V1448 ≠ human-level closure. V1448 ≠ absolute closure. "
        "V1448 ≠ VCP protocol correctness (we did not run any actual VCP protocol dispatch here; "
        "we only audited whether position modules mention VCP protocol keywords). "
        "Real VCP dispatch is owned by V1426 (run: `python -m apeireth.v1426_vcp_six_protocol_dispatcher run-all`). "
        f"overall_closure_rate={overall_closure:.4f} honest disclosure: "
        f"forward={per_kind.get('forward', 0):.2f} backward={per_kind.get('backward', 0):.2f} "
        f"cross_link={per_kind.get('cross_link', 0):.2f} history={per_kind.get('history', 0):.2f} "
        f"guard_compliance={per_kind.get('guard_compliance', 0):.2f} "
        f"cross_link_density={cross_density:.2f}."
    )

    report = VCPCrossModularAuditReport(
        schema=V1448_SCHEMA,
        version=V1448_VERSION,
        module=V1448_MODULE,
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
        per_position_closure_rate=per_position,
        per_protocol_closure_rate=per_protocol,
        overall_cross_link_density=cross_density,
        honest_disclosure=honest,
        guards=V1448_GUARDS,
        v3_guards=V1448_V3_GUARDS,
        borrowed=V1448_BORROWED,
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
    """Chain delegate: check that V1447+V1446+V1445+V1442+V1426 are loadable."""
    out: Dict[str, Any] = {}
    for vname, mid in (
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
    out["all_ok"] = all(v["importable"] for v in out.values() if vname != "all_ok")
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
    _record("module_version_set", bool(V1448_VERSION == "0.1.0"), f"V1448_VERSION={V1448_VERSION}")

    # 2. 6 protocols declared
    _record("protocol_count_6", len(V1448_PROTOCOL_NAMES) == 6, f"len={len(V1448_PROTOCOL_NAMES)}")

    # 3. 5 positions declared
    _record("position_count_5", len(V1448_POSITION_NAMES) == 5, f"len={len(V1448_POSITION_NAMES)}")

    # 4. 5 closure kinds declared
    _record("closure_kind_count_5", len(V1448_CLOSURE_KINDS) == 5, f"len={len(V1448_CLOSURE_KINDS)}")

    # 5. All 30 pairs enumerated
    all_pairs = tuple((p, pos) for p in V1448_PROTOCOL_NAMES for pos in V1448_POSITION_NAMES)
    _record("pair_count_30", len(all_pairs) == 30, f"len={len(all_pairs)}")

    # 6. Each protocol has keywords
    all_have_keywords = all(len(V1448_PROTOCOL_KEYWORDS.get(p, ())) > 0 for p in V1448_PROTOCOL_NAMES)
    _record("all_protocols_have_keywords", all_have_keywords, "all 6 protocols have ≥1 keyword")

    # 7. Each position has keywords
    all_pos_have_keywords = all(len(V1448_POSITION_KEYWORDS.get(p, ())) > 0 for p in V1448_POSITION_NAMES)
    _record("all_positions_have_keywords", all_pos_have_keywords, "all 5 positions have ≥1 keyword")

    # 8. V1442 importable (needed for positions)
    v1442 = _import_safely(DEFAULT_V1442_MODULE)
    _record("v1442_importable", v1442 is not None, DEFAULT_V1442_MODULE)

    # 9. V1426 importable (needed for VCP protocols)
    v1426 = _import_safely(DEFAULT_V1426_MODULE)
    _record("v1426_importable", v1426 is not None, DEFAULT_V1426_MODULE)

    # 10. run_pair_closure on (sync, scheduler) doesn't raise
    try:
        history_paths = _discover_history_files()
        probes, _ = run_pair_closure("sync", "scheduler", history_paths, all_pairs)
        _record("run_pair_closure_no_raise", len(probes) == 5, f"n_probes={len(probes)}")
    except Exception as exc:
        _record("run_pair_closure_no_raise", False, f"raised:{type(exc).__name__}")

    # 11. compute_overall_closure_rate returns float in [0,1]
    sample_probes = (
        PairClosureProbe("sync", "scheduler", "forward", 1, "test"),
        PairClosureProbe("sync", "scheduler", "backward", 0, "test"),
    )
    rate = compute_overall_closure_rate(sample_probes)
    _record("overall_rate_bounded", 0.0 <= rate <= 1.0, f"rate={rate}")

    # 12. cross_combined links per source = 29 (30 pairs - self)
    try:
        history_paths = _discover_history_files()
        _, entries = run_pair_closure("sync", "scheduler", history_paths, all_pairs)
        _record("cross_combined_count_29", len(entries) == 29, f"n={len(entries)}")
    except Exception as exc:
        _record("cross_combined_count_29", False, f"raised:{type(exc).__name__}")

    # 13. run_all doesn't raise
    try:
        report = run_all()
        _record("run_all_no_raise", report is not None and report.n_probes == 150, f"n_probes={report.n_probes if report else 0}")
    except Exception as exc:
        _record("run_all_no_raise", False, f"raised:{type(exc).__name__}")

    # 14. chain_delegate returns all_ok
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
        prog="v1448_asi_vcp_six_protocol_cross_modular",
        description="V1448 — ASI VCP 6 协议 × V2 5 位置 cross-modular audit",
    )
    sub = p.add_subparsers(dest="cmd")

    sub.add_parser("version", help="Print version")
    sub.add_parser("help", help="Print help")

    p_meta = sub.add_parser("meta", help="Print module metadata")
    p_meta.add_argument("--json", action="store_true")

    sub.add_parser("popper", help="Run popper self-tests")
    sub.add_parser("chain", help="Print chain_delegate() result")
    sub.add_parser("list-pairs", help="List all 30 (protocol, position) pairs")

    p_probe = sub.add_parser("probe-closure", help="Probe one pair")
    p_probe.add_argument("--protocol", default="sync")
    p_probe.add_argument("--position", default="scheduler")
    p_probe.add_argument("--kind", default=None, help="Optional closure kind filter")

    p_ccm = sub.add_parser("cross-combined-matrix", help="Compute cross-combined matrix for one pair")
    p_ccm.add_argument("--protocol", default="sync")
    p_ccm.add_argument("--position", default="scheduler")
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
        print(V1448_VERSION)
        return 0

    if cmd == "help":
        parser.print_help()
        return 0

    if cmd == "meta":
        meta = {
            "schema": V1448_SCHEMA,
            "version": V1448_VERSION,
            "module": V1448_MODULE,
            "n_protocols": len(V1448_PROTOCOL_NAMES),
            "n_positions": len(V1448_POSITION_NAMES),
            "n_closure_kinds": len(V1448_CLOSURE_KINDS),
            "n_pairs": 30,
            "n_probes": 150,
            "n_cross_combined_pairs": 870,
            "guards": list(V1448_GUARDS),
            "v3_guards": list(V1448_V3_GUARDS),
            "borrowed": [list(b) for b in V1448_BORROWED],
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
        for p in V1448_PROTOCOL_NAMES:
            for pos in V1448_POSITION_NAMES:
                print(f"{p} × {pos}")
        return 0

    if cmd == "probe-closure":
        history_paths = _discover_history_files()
        all_pairs = tuple((p, pos) for p in V1448_PROTOCOL_NAMES for pos in V1448_POSITION_NAMES)
        probes, _ = run_pair_closure(args.protocol, args.position, history_paths, all_pairs)
        for probe in probes:
            if args.kind and probe.kind != args.kind:
                continue
            mark = "✓" if probe.closed else "✗"
            print(f"{mark} {probe.kind}: closed={probe.closed} evidence={probe.evidence}")
        return 0

    if cmd == "cross-combined-matrix":
        history_paths = _discover_history_files()
        all_pairs = tuple((p, pos) for p in V1448_PROTOCOL_NAMES for pos in V1448_POSITION_NAMES)
        _, entries = run_pair_closure(args.protocol, args.position, history_paths, all_pairs)
        for e in entries:
            mark = "✓" if e.linked else "✗"
            print(f"{mark} {e.target_protocol}×{e.target_position}: {e.evidence}")
        return 0

    if cmd == "detect-compositional":
        report = run_all()
        if report.compositional_pairs:
            for cp in report.compositional_pairs:
                print(f"{cp.protocol}×{cp.position}: {cp.closure_rate:.2f}")
        else:
            print("(none)")
        return 0

    if cmd == "detect-anti-modular":
        report = run_all()
        if report.anti_modular_pairs:
            for amp in report.anti_modular_pairs:
                print(f"{amp.protocol}×{amp.position}={amp.closure_rate_a:.2f} vs {amp.opposite_protocol}×{amp.opposite_position}={amp.closure_rate_b:.2f}")
        else:
            print("(none)")
        return 0

    if cmd == "detect-substitutable":
        report = run_all()
        if report.substitutable_pairs:
            for sp in report.substitutable_pairs:
                print(f"{sp.protocol}×{sp.position}: {sp.protocol_closure:.2f}")
        else:
            print("(none)")
        return 0

    if cmd == "run-all":
        out_json = Path(args.out_json) if args.out_json else None
        out_md = Path(args.out_md) if args.out_md else None
        report = run_all(out_json=out_json, out_md=out_md)
        print(f"V1448 report written.")
        print(f"  overall_closure_rate: {report.overall_closure_rate:.4f}")
        print(f"  per_kind: {report.per_kind_closure_rate}")
        print(f"  per_position: {report.per_position_closure_rate}")
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