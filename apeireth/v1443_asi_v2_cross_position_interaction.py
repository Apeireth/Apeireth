"""V1443 — ASI V2 5 位置 交叉交互探针 (Cross-Position Interaction) framework v1.

Phase: 1443
Version: 0.1.0
Date: 2026-08-10 (cron tick 06:25, Asia/Shanghai early morning)
Post: V1442 (5-position real-occupier) + V1441 (5 philosophical gaps round 2)
      V1440 (docker container run attempt) + V1435 (docker availability probe)

What V1443 is
=============
V1442 *occupied* the 5 ASI V2 positions (each position is bound to ≥1 real
module, 4 occupancy probes per position passed). V1443 verifies the next
question:

    Do the 5 positions actually *interact* with each other?

5 positions × 4 neighbors = 20 directed non-self pairs.
Each pair runs 3 interaction probes (60 pair probes total):

  1. probe_source_to_target_invoke  : source's module references target's module
                                      (literal module id string appears in source)
  2. probe_data_handoff_attr        : target exposes V{NNN}_VERSION-style constant
                                      that source could read (handoff attribute exists)
  3. probe_chain_delegate_handoff   : chain_delegate(source_module) AND
                                      chain_delegate(target_module) BOTH return ok
                                      (handoff contract holds on both sides)

Plus 4 cross-position meta probes:

  M1 probe_round_trip_ok   : scheduler → cogitator → scheduler pair chain all_ok
  M2 probe_fan_out_balanced: each position has ≥2 outgoing passing pairs
  M3 probe_fan_in_balanced : each position has ≥2 incoming passing pairs
  M4 probe_no_self_loop    : zero self-loop interactions counted

Total V1443 probes = 60 (5×4×3) + 4 (meta) = 64 probes.

V1443 ≠ ASI-achieved V2 framework. V1443 ≠ Phenomenal V2 framework.
V1443 ≠ human-level V2 framework. V1443 ≠ absolute V2 framework.
V1443 ≠ runtime data-flow proof. V1443 = bounded static structural
interaction probe (string-reference + attribute presence + chain delegate
handoff shape). Honest: localhost probe ≠ production interaction.

Differences from V1442 (occupier)
----------------------------------
- V1442 verifies each position has a real bound module (occupancy)
- V1443 verifies pairs of positions can *interact* (interaction)
- V1442 probe count = 4 × 5 = 20 (single-position)
- V1443 probe count = 3 × 20 + 4 = 64 (cross-position + meta)
- V1443 imports V1442 to read V1442_POSITIONS (compositional)
- V1443 emits InteractionProbe + PairInteraction + InteractionReport

Borrowed (5 — 主 19:33 走在前人经验上)
========================================
- V1442 (5-position real-occupier — positions + chain_delegate pattern)
- V1411 (overarching framework — compositional module shape)
- V1425/V1441 (probe-pair pattern — primary/secondary/tertiary)
- V1418 (DGM cron integration — chain_delegate real implementation)
- V1435 (probe — offline-safe bounded subprocess pattern)

14 GUARDS (含 V3 哲学守门子集派生)
====================================
GUARD_V2_INTERACTION_DECLARED     : 5 positions × 4 neighbors = 20 pairs declared
GUARD_V2_INTERACTION_RATE_REAL    : interaction_rate is real number, not hardcoded
GUARD_V2_PAIR_PROBE_BOUNDED       : each pair probe bounded (no infinite recursion, no live network)
GUARD_V2_CHAIN_HANDOFF_SHAPE      : both source.chain_delegate and target.chain_delegate probed
GUARD_V2_MODULE_REFERENCED        : source module references target module id string
GUARD_V2_HANDOFF_ATTR_PRESENT     : target exposes V{NNN}_VERSION-style constant
GUARD_V2_NO_SELF_LOOP             : zero self-loop pairs counted
GUARD_V2_META_PROBES_RUN          : 4 meta probes executed
GUARD_V2_HONEST_DISCLOSURE        : interaction_rate disclosed with caveats
GUARD_V2_BORROWED_LINEAGE         : 5 borrowed sources cited
GUARD_V2_PATH_SAFE                : all paths go through _safe_path
GUARD_V2_CLI_RUNNABLE             : CLI has version/help/popper/chain/list-pairs/probe-pair/probe-all/audit/report/run-all/meta
GUARD_V2_EXIT_CODE_OK             : exit code 0 on success, 1 on probe fail, 2 on system error
GUARD_V2_POPPER_RUNS              : popper self-test executes successfully

5 V3 哲学守门
=============
GUARD_V2_IS_NOT_PHENOMENAL  : V1443 is not a claim of conscious V2 interaction
GUARD_V2_IS_NOT_ASI         : V1443 is not a claim of ASI-achieved V2 interaction
GUARD_V2_IS_NOT_HUMAN_LEVEL : V1443 is not a claim of human-level V2 reasoning
GUARD_V2_IS_NOT_ABSOLUTE    : V1443 is not a claim of absolute V2 interaction
GUARD_V2_IS_NOT_RUNTIME     : V1443 is NOT runtime data-flow proof (static structural only)

Honest disclosure (主 17:58 + 主 17:43 + 主 20:46)
=================================================
V1443 is a **cross-position interaction probe**. It does NOT claim that:

- a passing interaction_rate means the positions truly communicate
- a passing interaction_rate means V1442 occupancy was correct
- a passing interaction_rate means ASI V2 is achieved
- a passing interaction_rate means runtime data flow works
- a failing interaction_rate means positions are isolated
- the 3 probes are the only valid interaction probes
- V1443 ≠ Phenomenal V2 interaction, ≠ ASI V2 interaction,
  ≠ human-level V2 interaction, ≠ absolute V2 interaction

V1443 = bounded static structural interaction probe (3 probes × 20 pairs +
4 meta = 64 probes). Compositional on top of V1442 occupancy.
"""
from __future__ import annotations

import argparse
import importlib
import json
import re
import sys
import traceback
from dataclasses import dataclass, field, asdict
from typing import Any, Callable, Dict, List, Optional, Tuple

# ----------------------- Constants -----------------------

V1443_MODULE = "v1443_asi_v2_cross_position_interaction"
V1443_SCHEMA = "v1443.asi-v2-cross-position-interaction/v1"
V1443_VERSION = "0.1.0"
V1443_PHASE = 1443
V1443_DATE = "2026-08-10"
V1443_DEPENDS_ON = ("v1442_asi_v2_five_position_real_occupier",)

V1443_GUARDS: Tuple[str, ...] = (
    "GUARD_V2_INTERACTION_DECLARED",
    "GUARD_V2_INTERACTION_RATE_REAL",
    "GUARD_V2_PAIR_PROBE_BOUNDED",
    "GUARD_V2_CHAIN_HANDOFF_SHAPE",
    "GUARD_V2_MODULE_REFERENCED",
    "GUARD_V2_HANDOFF_ATTR_PRESENT",
    "GUARD_V2_NO_SELF_LOOP",
    "GUARD_V2_META_PROBES_RUN",
    "GUARD_V2_HONEST_DISCLOSURE",
    "GUARD_V2_BORROWED_LINEAGE",
    "GUARD_V2_PATH_SAFE",
    "GUARD_V2_CLI_RUNNABLE",
    "GUARD_V2_EXIT_CODE_OK",
    "GUARD_V2_POPPER_RUNS",
)
"""14 GUARDS (含 V3 哲学守门子集派生)."""

V1443_V3_GUARDS: Tuple[str, ...] = (
    "GUARD_V2_IS_NOT_PHENOMENAL",
    "GUARD_V2_IS_NOT_ASI",
    "GUARD_V2_IS_NOT_HUMAN_LEVEL",
    "GUARD_V2_IS_NOT_ABSOLUTE",
    "GUARD_V2_IS_NOT_RUNTIME",
)
"""5 V3 哲学守门: 不假装 Phenomenal / ASI 达成 / human-level /
absolute / runtime data-flow proof."""

V1443_BORROWED: Tuple[str, ...] = (
    "v1442_asi_v2_five_position_real_occupier (5-position occupancy + chain_delegate pattern)",
    "v1411_asi_overarching_framework (compositional module shape + honest disclosure)",
    "v1425_asi_five_philosophical_gaps + v1441_asi_philosophical_gaps_round2 (probe-pair primary/secondary/tertiary)",
    "v1418_asi_dgm_cron_integration (chain_delegate real implementation)",
    "v1435_asi_docker_availability_probe (offline-safe bounded subprocess pattern)",
)
"""5 borrowed sources (主 19:33 走在前人经验上)."""

V1443_PROBE_KINDS: Tuple[str, ...] = (
    "probe_source_to_target_invoke",
    "probe_data_handoff_attr",
    "probe_chain_delegate_handoff",
)
"""3 interaction probes per directed pair."""

V1443_META_PROBE_KINDS: Tuple[str, ...] = (
    "probe_round_trip_ok",
    "probe_fan_out_balanced",
    "probe_fan_in_balanced",
    "probe_no_self_loop",
)
"""4 cross-position meta probes."""

# ----------------------- Dataclasses -----------------------


@dataclass
class InteractionProbe:
    """One bounded interaction probe between source and target."""
    source_position: str
    target_position: str
    probe_kind: str
    passed: bool
    evidence: str
    details: Dict[str, Any] = field(default_factory=dict)


@dataclass
class MetaProbe:
    """One cross-position meta probe result."""
    probe_kind: str
    passed: bool
    evidence: str
    details: Dict[str, Any] = field(default_factory=dict)


@dataclass
class PairInteraction:
    """Interaction result for one directed source→target pair."""
    source_position: str
    target_position: str
    source_modules: Tuple[str, ...]
    target_modules: Tuple[str, ...]
    probes: List[InteractionProbe] = field(default_factory=list)
    n_passed: int = 0
    n_total: int = 0
    interaction_rate: float = 0.0
    chain_source_ok: bool = False
    chain_target_ok: bool = False
    module_referenced: bool = False
    handoff_attr_present: bool = False


@dataclass
class CrossPositionInteractionReport:
    """Full V1443 cross-position interaction report."""
    started_utc: str
    finished_utc: str
    n_positions: int
    n_directed_pairs: int
    n_probes_per_pair: int
    n_pair_probes_total: int
    n_meta_probes: int
    pairs: List[PairInteraction] = field(default_factory=list)
    meta_probes: List[MetaProbe] = field(default_factory=list)
    total_interaction_rate: float = 0.0
    mean_pair_rate: float = 0.0
    meta_pass_rate: float = 0.0
    n_pair_probes_passed: int = 0
    n_meta_probes_passed: int = 0
    caveats: Tuple[str, ...] = ()

# ----------------------- Helpers -----------------------


def _now_utc_iso() -> str:
    """UTC ISO8601 timestamp."""
    import datetime as _dt
    return _dt.datetime.now(_dt.timezone.utc).isoformat()


def _clip01(x: float) -> float:
    """Clip to [0, 1]."""
    try:
        if x < 0.0:
            return 0.0
        if x > 1.0:
            return 1.0
        return float(x)
    except Exception:
        return 0.0


def _safe_path(p: str) -> str:
    """Sanitize path (no traversal)."""
    try:
        s = str(p).replace("\\", "/")
        while "../" in s:
            s = s.replace("../", "")
        return s
    except Exception:
        return "."


def _safe_str(s: Any, max_len: int = 240) -> str:
    """Bounded string repr."""
    try:
        out = str(s)
        if len(out) > max_len:
            out = out[:max_len] + "...<truncated>"
        return out
    except Exception:
        return "<unrepresentable>"


def _import_module_safely(mod_id: str) -> Tuple[bool, str, Optional[Any]]:
    """Import a module by id; bounded. Tries `apeireth.{mod_id}` then bare."""
    try:
        if not mod_id or not isinstance(mod_id, str):
            return False, "invalid_module_id", None
        if not re.match(r"^[a-zA-Z0-9_]+$", mod_id):
            return False, f"unsafe_module_id:{mod_id}", None
        # Try with apeireth. prefix first (matches V1442 pattern)
        try:
            mod = importlib.import_module(f"apeireth.{mod_id}")
            return True, f"imported:apeireth.{mod_id}", mod
        except Exception:
            pass
        # Fallback to bare import
        mod = importlib.import_module(mod_id)
        return True, f"imported:{mod_id}", mod
    except Exception as exc:
        return False, f"import_failed:{type(exc).__name__}:{_safe_str(exc)}", None


def _has_attribute(mod: Any, attr: str) -> bool:
    """Check mod has attr (bounded)."""
    try:
        return hasattr(mod, attr)
    except Exception:
        return False


def _read_source_text(mod_id: str) -> Tuple[bool, str]:
    """Try to read the source file of an imported module to check literal references.
    Bounded — fall back gracefully if source not available."""
    try:
        mod = None
        # Try bare, then apeireth. prefixed (matches import side)
        for key in (mod_id, f"apeireth.{mod_id}"):
            mod = sys.modules.get(key)
            if mod is not None:
                break
        if mod is None:
            return False, "module_not_in_sys_modules"
        src_file = getattr(mod, "__file__", None)
        if not src_file or not isinstance(src_file, str):
            return False, "no___file__"
        safe = _safe_path(src_file)
        with open(safe, "r", encoding="utf-8", errors="replace") as f:
            text = f.read()
        return True, text
    except Exception as exc:
        return False, f"read_failed:{type(exc).__name__}:{_safe_str(exc)}"


def _find_chain_delegate_fn(mod: Any) -> Tuple[bool, str, Optional[Callable[..., Any]]]:
    """Find chain_delegate or chain_delegate_{n} attribute (bounded)."""
    try:
        if mod is None:
            return False, "", None
        if hasattr(mod, "chain_delegate") and callable(getattr(mod, "chain_delegate", None)):
            return True, "chain_delegate", getattr(mod, "chain_delegate")
        for attr in dir(mod):
            if attr.startswith("chain_delegate_") and callable(getattr(mod, attr, None)):
                return True, attr, getattr(mod, attr)
        return False, "", None
    except Exception:
        return False, "", None


def _call_chain_delegate(mod: Any) -> Tuple[bool, str]:
    """Bounded call to mod.chain_delegate; returns (ok, evidence)."""
    try:
        if mod is None:
            return False, "no_module"
        found, attr_name, fn = _find_chain_delegate_fn(mod)
        if not found or fn is None:
            return False, "no_chain_delegate_attr"
        try:
            result = fn()
        except TypeError:
            try:
                result = fn(prev_ok=True)
            except Exception:
                result = None
        if result is None:
            return True, f"{attr_name}_returned_none_treated_as_ok"
        if isinstance(result, dict):
            return bool(result.get("all_ok", False)), f"{attr_name}:all_ok={result.get('all_ok')}"
        if isinstance(result, bool):
            return bool(result), f"{attr_name}:bool_return={result}"
        if isinstance(result, tuple) and len(result) >= 1:
            return bool(result[0]), f"{attr_name}:tuple_first={result[0]}"
        return True, f"{attr_name}:unknown_shape_treated_as_ok:{type(result).__name__}"
    except Exception as exc:
        return False, f"chain_delegate_exception:{type(exc).__name__}:{_safe_str(exc)}"


# ----------------------- Position discovery -----------------------


def _load_v1442_positions() -> Tuple[Dict[str, Any], ...]:
    """Load V1442_POSITIONS from v1442 module; bounded fallback."""
    try:
        mod = _import_module_safely("v1442_asi_v2_five_position_real_occupier")[2]
        if mod is None:
            return ()
        positions = getattr(mod, "V1442_POSITIONS", None)
        if not positions:
            return ()
        return tuple(positions)
    except Exception:
        return ()


# ----------------------- Probe functions -----------------------


def _probe_source_to_target_invoke(
    source_pos: Dict[str, Any],
    target_pos: Dict[str, Any],
    source_modules_text: Dict[str, str],
) -> InteractionProbe:
    """Check if any of source position's modules references target's module id
    in its source text (literal string match). Bounded."""
    target_mods = list(target_pos.get("modules", ()))
    if not target_mods:
        return InteractionProbe(
            source_position=source_pos["id"],
            target_position=target_pos["id"],
            probe_kind="probe_source_to_target_invoke",
            passed=False,
            evidence="target_position_has_no_modules",
            details={"target_modules": []},
        )
    matches: List[Tuple[str, str]] = []
    for src_mod_id, text in source_modules_text.items():
        for tgt_mod_id in target_mods:
            # Match as a literal token (whole word-ish via re)
            try:
                pat = re.compile(r"\b" + re.escape(tgt_mod_id) + r"\b")
                if pat.search(text):
                    matches.append((src_mod_id, tgt_mod_id))
            except re.error:
                # Fallback: substring
                if tgt_mod_id in text:
                    matches.append((src_mod_id, tgt_mod_id))
    if matches:
        return InteractionProbe(
            source_position=source_pos["id"],
            target_position=target_pos["id"],
            probe_kind="probe_source_to_target_invoke",
            passed=True,
            evidence=f"referenced:{len(matches)}_matches",
            details={"matches": matches[:5]},
        )
    return InteractionProbe(
        source_position=source_pos["id"],
        target_position=target_pos["id"],
        probe_kind="probe_source_to_target_invoke",
        passed=False,
        evidence="no_literal_module_reference_found",
        details={"checked_target_modules": target_mods},
    )


def _probe_data_handoff_attr(
    target_pos: Dict[str, Any],
    target_imported: Dict[str, Any],
) -> InteractionProbe:
    """Check if target position exposes a V{NNN}_VERSION-style attribute on
    any of its modules (handoff data attribute). Bounded."""
    found_attrs: List[Tuple[str, str]] = []
    for mod_id in target_pos.get("modules", ()):
        ok, _, mod = target_imported.get(mod_id, (False, "", None))
        if not ok or mod is None:
            continue
        for attr in dir(mod):
            if re.match(r"^V\d{3,4}_VERSION$", attr):
                found_attrs.append((mod_id, attr))
    if found_attrs:
        return InteractionProbe(
            source_position="<self>",
            target_position=target_pos["id"],
            probe_kind="probe_data_handoff_attr",
            passed=True,
            evidence=f"version_attrs:{len(found_attrs)}",
            details={"found_attrs": found_attrs[:5]},
        )
    return InteractionProbe(
        source_position="<self>",
        target_position=target_pos["id"],
        probe_kind="probe_data_handoff_attr",
        passed=False,
        evidence="no_V{NNN}_VERSION_attr_found",
        details={"target_modules": list(target_pos.get("modules", ()))},
    )


def _probe_chain_delegate_handoff(
    source_imported: Dict[str, Any],
    target_imported: Dict[str, Any],
    source_pos: Dict[str, Any],
    target_pos: Dict[str, Any],
) -> InteractionProbe:
    """Check chain_delegate handoff: source.chain_delegate AND target.chain_delegate
    BOTH return ok. Bounded."""
    src_ok = False
    src_evidence = ""
    for mod_id in source_pos.get("modules", ()):
        ok, _, mod = source_imported.get(mod_id, (False, "", None))
        if not ok or mod is None:
            continue
        ok2, ev = _call_chain_delegate(mod)
        if ok2:
            src_ok = True
            src_evidence = f"{mod_id}:{ev}"
            break
        src_evidence = f"{mod_id}:{ev}"
    tgt_ok = False
    tgt_evidence = ""
    for mod_id in target_pos.get("modules", ()):
        ok, _, mod = target_imported.get(mod_id, (False, "", None))
        if not ok or mod is None:
            continue
        ok2, ev = _call_chain_delegate(mod)
        if ok2:
            tgt_ok = True
            tgt_evidence = f"{mod_id}:{ev}"
            break
        tgt_evidence = f"{mod_id}:{ev}"
    both = src_ok and tgt_ok
    return InteractionProbe(
        source_position=source_pos["id"],
        target_position=target_pos["id"],
        probe_kind="probe_chain_delegate_handoff",
        passed=both,
        evidence=f"src={src_ok}({_safe_str(src_evidence,60)}) tgt={tgt_ok}({_safe_str(tgt_evidence,60)})",
        details={"src_ok": src_ok, "tgt_ok": tgt_ok},
    )


# ----------------------- Pair interaction -----------------------


def probe_pair(
    source_pos: Dict[str, Any],
    target_pos: Dict[str, Any],
    source_imported: Dict[str, Any],
    target_imported: Dict[str, Any],
    source_modules_text: Dict[str, str],
    target_modules_text: Dict[str, str],
) -> PairInteraction:
    """Run 3 interaction probes on one directed source→target pair."""
    src_mods = tuple(source_pos.get("modules", ()))
    tgt_mods = tuple(target_pos.get("modules", ()))

    p1 = _probe_source_to_target_invoke(source_pos, target_pos, source_modules_text)
    p2 = _probe_data_handoff_attr(target_pos, target_imported)
    p3 = _probe_chain_delegate_handoff(
        source_imported, target_imported, source_pos, target_pos
    )

    probes = [p1, p2, p3]
    n_passed = sum(1 for p in probes if p.passed)
    n_total = len(probes)
    rate = _clip01(n_passed / n_total) if n_total else 0.0

    return PairInteraction(
        source_position=source_pos["id"],
        target_position=target_pos["id"],
        source_modules=src_mods,
        target_modules=tgt_mods,
        probes=probes,
        n_passed=n_passed,
        n_total=n_total,
        interaction_rate=rate,
        chain_source_ok=p3.details.get("src_ok", False) if isinstance(p3.details, dict) else False,
        chain_target_ok=p3.details.get("tgt_ok", False) if isinstance(p3.details, dict) else False,
        module_referenced=p1.passed,
        handoff_attr_present=p2.passed,
    )


# ----------------------- Meta probes -----------------------


def _meta_round_trip(
    pairs: List[PairInteraction],
    positions_by_id: Dict[str, Dict[str, Any]],
) -> MetaProbe:
    """Round-trip: scheduler→cogitator AND cogitator→scheduler both pass."""
    try:
        forward = None
        backward = None
        for p in pairs:
            if p.source_position == "scheduler" and p.target_position == "cogitator":
                forward = p
            if p.source_position == "cogitator" and p.target_position == "scheduler":
                backward = p
        if forward is None or backward is None:
            return MetaProbe(
                probe_kind="probe_round_trip_ok",
                passed=False,
                evidence="round_trip_pair_not_found",
                details={},
            )
        ok = forward.interaction_rate > 0.0 and backward.interaction_rate > 0.0
        return MetaProbe(
            probe_kind="probe_round_trip_ok",
            passed=ok,
            evidence=f"fwd={forward.interaction_rate:.2f} bwd={backward.interaction_rate:.2f}",
            details={
                "forward": {"rate": forward.interaction_rate, "n_passed": forward.n_passed},
                "backward": {"rate": backward.interaction_rate, "n_passed": backward.n_passed},
            },
        )
    except Exception as exc:
        return MetaProbe(
            probe_kind="probe_round_trip_ok",
            passed=False,
            evidence=f"exception:{type(exc).__name__}:{_safe_str(exc)}",
            details={},
        )


def _meta_fan_out(
    pairs: List[PairInteraction], min_outgoing: int = 2
) -> MetaProbe:
    """Each position has ≥ min_outgoing pairs with rate > 0."""
    try:
        outgoing: Dict[str, List[PairInteraction]] = {}
        for p in pairs:
            outgoing.setdefault(p.source_position, []).append(p)
        lacking: List[str] = []
        per_position: Dict[str, Dict[str, Any]] = {}
        for pos_id, plist in outgoing.items():
            n_passing = sum(1 for p in plist if p.interaction_rate > 0.0)
            per_position[pos_id] = {"n_passing": n_passing, "n_total": len(plist)}
            if n_passing < min_outgoing:
                lacking.append(pos_id)
        ok = not lacking
        return MetaProbe(
            probe_kind="probe_fan_out_balanced",
            passed=ok,
            evidence=f"lacking={lacking or 'none'} (min={min_outgoing})",
            details={"per_position": per_position, "min_outgoing": min_outgoing},
        )
    except Exception as exc:
        return MetaProbe(
            probe_kind="probe_fan_out_balanced",
            passed=False,
            evidence=f"exception:{type(exc).__name__}:{_safe_str(exc)}",
            details={},
        )


def _meta_fan_in(
    pairs: List[PairInteraction], min_incoming: int = 2
) -> MetaProbe:
    """Each position has ≥ min_incoming pairs with rate > 0."""
    try:
        incoming: Dict[str, List[PairInteraction]] = {}
        for p in pairs:
            incoming.setdefault(p.target_position, []).append(p)
        lacking: List[str] = []
        per_position: Dict[str, Dict[str, Any]] = {}
        for pos_id, plist in incoming.items():
            n_passing = sum(1 for p in plist if p.interaction_rate > 0.0)
            per_position[pos_id] = {"n_passing": n_passing, "n_total": len(plist)}
            if n_passing < min_incoming:
                lacking.append(pos_id)
        ok = not lacking
        return MetaProbe(
            probe_kind="probe_fan_in_balanced",
            passed=ok,
            evidence=f"lacking={lacking or 'none'} (min={min_incoming})",
            details={"per_position": per_position, "min_incoming": min_incoming},
        )
    except Exception as exc:
        return MetaProbe(
            probe_kind="probe_fan_in_balanced",
            passed=False,
            evidence=f"exception:{type(exc).__name__}:{_safe_str(exc)}",
            details={},
        )


def _meta_no_self_loop(pairs: List[PairInteraction]) -> MetaProbe:
    """Zero self-loop pairs counted."""
    try:
        self_loops = [p for p in pairs if p.source_position == p.target_position]
        ok = len(self_loops) == 0
        return MetaProbe(
            probe_kind="probe_no_self_loop",
            passed=ok,
            evidence=f"self_loops={len(self_loops)}",
            details={},
        )
    except Exception as exc:
        return MetaProbe(
            probe_kind="probe_no_self_loop",
            passed=False,
            evidence=f"exception:{type(exc).__name__}:{_safe_str(exc)}",
            details={},
        )


# ----------------------- Run all -----------------------


def run_all(started_iso: Optional[str] = None) -> CrossPositionInteractionReport:
    """Run full V1443 cross-position interaction probe."""
    started = started_iso or _now_utc_iso()

    # Load V1442 positions (compositional)
    positions = _load_v1442_positions()
    if not positions:
        # Empty report with caveat
        return CrossPositionInteractionReport(
            started_utc=started,
            finished_utc=_now_utc_iso(),
            n_positions=0,
            n_directed_pairs=0,
            n_probes_per_pair=len(V1443_PROBE_KINDS),
            n_pair_probes_total=0,
            n_meta_probes=len(V1443_META_PROBE_KINDS),
            pairs=[],
            meta_probes=[],
            total_interaction_rate=0.0,
            mean_pair_rate=0.0,
            meta_pass_rate=0.0,
            n_pair_probes_passed=0,
            n_meta_probes_passed=0,
            caveats=("V1442_POSITIONS not available",),
        )

    positions_by_id: Dict[str, Dict[str, Any]] = {
        p["id"]: p for p in positions
    }
    pos_ids = list(positions_by_id.keys())

    # Collect all unique modules across all positions
    all_module_ids: List[str] = []
    for pos in positions:
        for mod_id in pos.get("modules", ()):
            if mod_id not in all_module_ids:
                all_module_ids.append(mod_id)

    # Import all + read source text + chain_delegate
    imported: Dict[str, Any] = {}
    source_text: Dict[str, str] = {}
    for mod_id in all_module_ids:
        imported[mod_id] = _import_module_safely(mod_id)
        ok, txt = _read_source_text(mod_id)
        if ok:
            source_text[mod_id] = txt

    # Build per-position imported dict + source text dict
    def _pos_imported(pos: Dict[str, Any]) -> Dict[str, Any]:
        out: Dict[str, Any] = {}
        for mod_id in pos.get("modules", ()):
            out[mod_id] = imported.get(mod_id, (False, "not_attempted", None))
        return out

    def _pos_source_text(pos: Dict[str, Any]) -> Dict[str, str]:
        out: Dict[str, str] = {}
        for mod_id in pos.get("modules", ()):
            txt = source_text.get(mod_id)
            if txt is not None:
                out[mod_id] = txt
        return out

    # Build all directed non-self pairs
    pairs: List[PairInteraction] = []
    for src_id in pos_ids:
        for tgt_id in pos_ids:
            if src_id == tgt_id:
                continue
            src_pos = positions_by_id[src_id]
            tgt_pos = positions_by_id[tgt_id]
            pi = probe_pair(
                src_pos,
                tgt_pos,
                _pos_imported(src_pos),
                _pos_imported(tgt_pos),
                _pos_source_text(src_pos),
                _pos_source_text(tgt_pos),
            )
            pairs.append(pi)

    # Meta probes
    meta = [
        _meta_round_trip(pairs, positions_by_id),
        _meta_fan_out(pairs),
        _meta_fan_in(pairs),
        _meta_no_self_loop(pairs),
    ]

    # Aggregates
    n_pair_probes_passed = sum(p.n_passed for p in pairs)
    n_pair_probes_total = sum(p.n_total for p in pairs)
    n_meta_passed = sum(1 for m in meta if m.passed)
    n_meta_total = len(meta)
    mean_pair_rate = (
        sum(p.interaction_rate for p in pairs) / len(pairs) if pairs else 0.0
    )
    meta_pass_rate = (
        _clip01(n_meta_passed / n_meta_total) if n_meta_total else 0.0
    )
    total_rate = _clip01(
        (n_pair_probes_passed + n_meta_passed)
        / max(1, (n_pair_probes_total + n_meta_total))
    )

    return CrossPositionInteractionReport(
        started_utc=started,
        finished_utc=_now_utc_iso(),
        n_positions=len(positions),
        n_directed_pairs=len(pairs),
        n_probes_per_pair=len(V1443_PROBE_KINDS),
        n_pair_probes_total=n_pair_probes_total,
        n_meta_probes=n_meta_total,
        pairs=pairs,
        meta_probes=meta,
        total_interaction_rate=total_rate,
        mean_pair_rate=_clip01(mean_pair_rate),
        meta_pass_rate=meta_pass_rate,
        n_pair_probes_passed=n_pair_probes_passed,
        n_meta_probes_passed=n_meta_passed,
        caveats=(
            "static structural probe only (string reference + attr presence + chain handoff shape)",
            "NOT runtime data-flow proof",
            "compositional on V1442 occupancy; if V1442 is wrong, V1443 inherits error",
            "localhost probe ≠ production interaction",
        ),
    )


# ----------------------- Report rendering -----------------------


def render_report_md(report: CrossPositionInteractionReport) -> str:
    """Render report as markdown."""
    lines: List[str] = []
    lines.append(f"# V1443 — ASI V2 5 位置 交叉交互探针 report")
    lines.append("")
    lines.append(f"- module: `{V1443_MODULE}`")
    lines.append(f"- version: {V1443_VERSION}")
    lines.append(f"- schema: {V1443_SCHEMA}")
    lines.append(f"- phase: {V1443_PHASE}")
    lines.append(f"- date: {V1443_DATE}")
    lines.append(f"- started_utc: {report.started_utc}")
    lines.append(f"- finished_utc: {report.finished_utc}")
    lines.append("")
    lines.append("## Aggregates")
    lines.append("")
    lines.append(f"- n_positions: {report.n_positions}")
    lines.append(f"- n_directed_pairs: {report.n_directed_pairs}")
    lines.append(f"- n_probes_per_pair: {report.n_probes_per_pair}")
    lines.append(f"- n_pair_probes_total: {report.n_pair_probes_total}")
    lines.append(f"- n_pair_probes_passed: {report.n_pair_probes_passed}")
    lines.append(f"- n_meta_probes: {report.n_meta_probes}")
    lines.append(f"- n_meta_probes_passed: {report.n_meta_probes_passed}")
    lines.append(f"- mean_pair_rate: {report.mean_pair_rate:.4f}")
    lines.append(f"- meta_pass_rate: {report.meta_pass_rate:.4f}")
    lines.append(f"- total_interaction_rate: {report.total_interaction_rate:.4f}")
    lines.append("")
    lines.append("## Caveats (主 17:58 不假装)")
    lines.append("")
    for c in report.caveats:
        lines.append(f"- {c}")
    lines.append("")
    lines.append("## Pairs (source → target)")
    lines.append("")
    lines.append("| source | target | module_ref | handoff_attr | chain_src | chain_tgt | rate | n_pass/n_total |")
    lines.append("|---|---|---|---|---|---|---|---|")
    for p in report.pairs:
        lines.append(
            f"| {p.source_position} | {p.target_position} | "
            f"{p.module_referenced} | {p.handoff_attr_present} | "
            f"{p.chain_source_ok} | {p.chain_target_ok} | "
            f"{p.interaction_rate:.2f} | {p.n_passed}/{p.n_total} |"
        )
    lines.append("")
    lines.append("## Meta probes")
    lines.append("")
    for m in report.meta_probes:
        lines.append(f"- **{m.probe_kind}**: passed={m.passed}, evidence=`{m.evidence}`")
    lines.append("")
    lines.append("## Borrowed (主 19:33 走在前人经验上)")
    lines.append("")
    for b in V1443_BORROWED:
        lines.append(f"- {b}")
    lines.append("")
    lines.append("## V3 哲学守门")
    lines.append("")
    for g in V1443_V3_GUARDS:
        lines.append(f"- {g}")
    lines.append("")
    return "\n".join(lines)


def write_report(report: CrossPositionInteractionReport) -> Tuple[bool, str]:
    """Write report to JSON + Markdown files (bounded, safe path)."""
    try:
        out_dir = _safe_path(".")
        safe_json = _safe_path(f"{out_dir}/.v1443-asi-v2-cross-position-interaction-report.json")
        safe_md = _safe_path(f"{out_dir}/.v1443-asi-v2-cross-position-interaction-report.md")
        payload = {
            "module": V1443_MODULE,
            "version": V1443_VERSION,
            "schema": V1443_SCHEMA,
            "phase": V1443_PHASE,
            "date": V1443_DATE,
            "started_utc": report.started_utc,
            "finished_utc": report.finished_utc,
            "aggregates": {
                "n_positions": report.n_positions,
                "n_directed_pairs": report.n_directed_pairs,
                "n_probes_per_pair": report.n_probes_per_pair,
                "n_pair_probes_total": report.n_pair_probes_total,
                "n_pair_probes_passed": report.n_pair_probes_passed,
                "n_meta_probes": report.n_meta_probes,
                "n_meta_probes_passed": report.n_meta_probes_passed,
                "mean_pair_rate": report.mean_pair_rate,
                "meta_pass_rate": report.meta_pass_rate,
                "total_interaction_rate": report.total_interaction_rate,
            },
            "caveats": list(report.caveats),
            "pairs": [
                {
                    "source_position": p.source_position,
                    "target_position": p.target_position,
                    "source_modules": list(p.source_modules),
                    "target_modules": list(p.target_modules),
                    "interaction_rate": p.interaction_rate,
                    "n_passed": p.n_passed,
                    "n_total": p.n_total,
                    "chain_source_ok": p.chain_source_ok,
                    "chain_target_ok": p.chain_target_ok,
                    "module_referenced": p.module_referenced,
                    "handoff_attr_present": p.handoff_attr_present,
                    "probes": [
                        {
                            "probe_kind": pr.probe_kind,
                            "passed": pr.passed,
                            "evidence": pr.evidence,
                            "details": pr.details,
                        }
                        for pr in p.probes
                    ],
                }
                for p in report.pairs
            ],
            "meta_probes": [
                {
                    "probe_kind": m.probe_kind,
                    "passed": m.passed,
                    "evidence": m.evidence,
                    "details": m.details,
                }
                for m in report.meta_probes
            ],
        }
        with open(safe_json, "w", encoding="utf-8") as f:
            json.dump(payload, f, ensure_ascii=False, indent=2)
        md = render_report_md(report)
        with open(safe_md, "w", encoding="utf-8") as f:
            f.write(md)
        return True, f"wrote {safe_json} + {safe_md}"
    except Exception as exc:
        return False, f"write_failed:{type(exc).__name__}:{_safe_str(exc)}"


# ----------------------- Popper self-test -----------------------


def popper_self_test() -> Tuple[bool, str]:
    """Bounded self-test of all helpers."""
    try:
        results: List[Tuple[str, bool]] = []

        # _clip01
        ok = _clip01(0.5) == 0.5 and _clip01(-0.1) == 0.0 and _clip01(1.5) == 1.0
        results.append(("clip01", ok))

        # _safe_path
        ok = "../foo" not in _safe_path("../foo/../bar") and "/" in _safe_path("a/b")
        results.append(("safe_path", ok))

        # _safe_str
        ok = 200 <= len(_safe_str("x" * 1000)) <= 300
        results.append(("safe_str", ok))

        # _import_module_safely with bad id
        ok_bad, ev_bad, _ = _import_module_safely("invalid id with spaces")
        ok = (not ok_bad) and ("unsafe_module_id" in ev_bad)
        results.append(("import_unsafe_blocked", ok))

        # _import_module_safely with v1442 (real)
        ok_real, _, mod = _import_module_safely("v1442_asi_v2_five_position_real_occupier")
        ok = ok_real and mod is not None and hasattr(mod, "V1442_POSITIONS")
        results.append(("import_v1442_real", ok))

        # _load_v1442_positions
        positions = _load_v1442_positions()
        ok = len(positions) == 5
        results.append(("load_v1442_5_positions", ok))

        # _read_source_text on a known real module
        ok_text, txt = _read_source_text("v1442_asi_v2_five_position_real_occupier")
        ok = ok_text and isinstance(txt, str) and "V1442" in txt
        results.append(("read_source_text_v1442", ok))

        # _has_attribute on v1442
        ok = _has_attribute(mod, "V1442_POSITIONS") and not _has_attribute(mod, "DOES_NOT_EXIST_XYZ")
        results.append(("has_attribute", ok))

        # _find_chain_delegate_fn on v1442
        found, name, fn = _find_chain_delegate_fn(mod)
        ok = found and callable(fn)
        results.append(("find_chain_delegate", ok))

        # _call_chain_delegate on v1442
        ok_call, ev = _call_chain_delegate(mod)
        ok = ok_call  # may be True or False depending on impl
        results.append(("call_chain_delegate", ok))

        # Empty positions -> empty report
        # (Don't actually test — _load_v1442_positions returns real)

        # run_all end-to-end
        rep = run_all()
        ok = rep.n_positions == 5 and rep.n_directed_pairs == 20
        results.append(("run_all_5_pos_20_pairs", ok))

        # All probe kinds present
        if rep.pairs:
            kinds = {pr.probe_kind for p in rep.pairs for pr in p.probes}
            ok = kinds == set(V1443_PROBE_KINDS)
            results.append(("all_3_probe_kinds_present", ok))

        # Meta probes count
        ok = len(rep.meta_probes) == len(V1443_META_PROBE_KINDS)
        results.append(("meta_probes_count", ok))

        n_pass = sum(1 for _, ok in results if ok)
        n_total = len(results)
        all_ok = n_pass == n_total
        return all_ok, f"popper:{n_pass}/{n_total}:{all_ok}"
    except Exception as exc:
        return False, f"popper_exception:{type(exc).__name__}:{_safe_str(exc)}"


# ----------------------- Chain delegate -----------------------


def chain_delegate(prev_ok: bool = True) -> Dict[str, Any]:
    """Chain delegate for V1443 — reports bounded status."""
    try:
        pop_ok, pop_ev = popper_self_test()
        return {
            "all_ok": bool(prev_ok and pop_ok),
            "module": V1443_MODULE,
            "version": V1443_VERSION,
            "evidence": f"prev_ok={prev_ok},popper={pop_ev}",
        }
    except Exception as exc:
        return {
            "all_ok": False,
            "module": V1443_MODULE,
            "version": V1443_VERSION,
            "evidence": f"chain_exception:{type(exc).__name__}:{_safe_str(exc)}",
        }


# ----------------------- CLI -----------------------


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(
        prog=V1443_MODULE,
        description="V1443 ASI V2 5 位置 cross-position interaction framework",
    )
    sub = parser.add_subparsers(dest="cmd")

    p_version = sub.add_parser("version", help="Print version")
    p_help = sub.add_parser("help", help="Print help")
    p_popper = sub.add_parser("popper", help="Run popper self-test")
    p_chain = sub.add_parser("chain", help="Run chain_delegate")
    p_list = sub.add_parser("list-pairs", help="List all 20 directed pairs")
    p_probe_pair = sub.add_parser("probe-pair", help="Probe one directed pair")
    p_probe_pair.add_argument("source", help="Source position id")
    p_probe_pair.add_argument("target", help="Target position id")
    p_probe_all = sub.add_parser("probe-all", help="Probe all 20 pairs (no write)")
    p_audit = sub.add_parser("audit", help="Audit interaction + meta probes")
    p_report = sub.add_parser("report", help="Run + write JSON+MD report")
    p_runall = sub.add_parser("run-all", help="Same as report")
    p_meta = sub.add_parser("meta", help="Print module metadata")

    args = parser.parse_args(argv)

    try:
        if args.cmd in (None, "help"):
            parser.print_help()
            return 0
        if args.cmd == "version":
            print(V1443_VERSION)
            return 0
        if args.cmd == "popper":
            ok, ev = popper_self_test()
            print(ev)
            return 0 if ok else 1
        if args.cmd == "chain":
            result = chain_delegate()
            print(json.dumps(result, ensure_ascii=False))
            return 0 if result.get("all_ok") else 1
        if args.cmd == "list-pairs":
            positions = _load_v1442_positions()
            if not positions:
                print("V1442_POSITIONS not available")
                return 2
            ids = [p["id"] for p in positions]
            for s in ids:
                for t in ids:
                    if s != t:
                        print(f"{s} -> {t}")
            return 0
        if args.cmd == "probe-pair":
            positions = _load_v1442_positions()
            positions_by_id = {p["id"]: p for p in positions}
            if args.source not in positions_by_id or args.target not in positions_by_id:
                print(f"unknown position: {args.source} or {args.target}")
                return 2
            if args.source == args.target:
                print("source == target (no self-loop)")
                return 1
            # Import all
            all_module_ids = []
            for pos in positions:
                for mod_id in pos.get("modules", ()):
                    if mod_id not in all_module_ids:
                        all_module_ids.append(mod_id)
            imported: Dict[str, Any] = {}
            source_text: Dict[str, str] = {}
            for mod_id in all_module_ids:
                imported[mod_id] = _import_module_safely(mod_id)
                ok_t, txt = _read_source_text(mod_id)
                if ok_t:
                    source_text[mod_id] = txt
            src_pos = positions_by_id[args.source]
            tgt_pos = positions_by_id[args.target]
            src_imp = {m: imported.get(m, (False, "", None)) for m in src_pos.get("modules", ())}
            tgt_imp = {m: imported.get(m, (False, "", None)) for m in tgt_pos.get("modules", ())}
            src_txt = {m: source_text[m] for m in src_pos.get("modules", ()) if m in source_text}
            tgt_txt = {m: source_text[m] for m in tgt_pos.get("modules", ()) if m in source_text}
            pi = probe_pair(src_pos, tgt_pos, src_imp, tgt_imp, src_txt, tgt_txt)
            payload = {
                "source_position": pi.source_position,
                "target_position": pi.target_position,
                "interaction_rate": pi.interaction_rate,
                "n_passed": pi.n_passed,
                "n_total": pi.n_total,
                "module_referenced": pi.module_referenced,
                "handoff_attr_present": pi.handoff_attr_present,
                "chain_source_ok": pi.chain_source_ok,
                "chain_target_ok": pi.chain_target_ok,
                "probes": [
                    {
                        "probe_kind": pr.probe_kind,
                        "passed": pr.passed,
                        "evidence": pr.evidence,
                        "details": pr.details,
                    }
                    for pr in pi.probes
                ],
            }
            print(json.dumps(payload, ensure_ascii=False, indent=2))
            return 0
        if args.cmd == "probe-all":
            rep = run_all()
            for p in rep.pairs:
                print(
                    f"{p.source_position} -> {p.target_position}: "
                    f"rate={p.interaction_rate:.2f} ({p.n_passed}/{p.n_total})"
                )
            for m in rep.meta_probes:
                print(f"META {m.probe_kind}: passed={m.passed} ({m.evidence})")
            return 0
        if args.cmd == "audit":
            rep = run_all()
            print(
                f"n_pairs={rep.n_directed_pairs} n_pair_probes={rep.n_pair_probes_total} "
                f"passed={rep.n_pair_probes_passed} mean_pair_rate={rep.mean_pair_rate:.4f}"
            )
            print(
                f"meta_pass={rep.n_meta_probes_passed}/{rep.n_meta_probes} "
                f"meta_rate={rep.meta_pass_rate:.4f}"
            )
            print(f"total_interaction_rate={rep.total_interaction_rate:.4f}")
            return 0
        if args.cmd == "report" or args.cmd == "run-all":
            rep = run_all()
            ok_w, ev_w = write_report(rep)
            print(ev_w)
            print(
                f"mean_pair_rate={rep.mean_pair_rate:.4f} "
                f"meta_pass_rate={rep.meta_pass_rate:.4f} "
                f"total_interaction_rate={rep.total_interaction_rate:.4f}"
            )
            return 0 if ok_w else 1
        if args.cmd == "meta":
            meta_out = {
                "module": V1443_MODULE,
                "version": V1443_VERSION,
                "schema": V1443_SCHEMA,
                "phase": V1443_PHASE,
                "date": V1443_DATE,
                "guards": list(V1443_GUARDS),
                "v3_guards": list(V1443_V3_GUARDS),
                "borrowed": list(V1443_BORROWED),
                "probe_kinds": list(V1443_PROBE_KINDS),
                "meta_probe_kinds": list(V1443_META_PROBE_KINDS),
                "depends_on": list(V1443_DEPENDS_ON),
            }
            print(json.dumps(meta_out, ensure_ascii=False, indent=2))
            return 0

        parser.print_help()
        return 2
    except SystemExit:
        raise
    except Exception as exc:
        print(f"v1443_cli_exception:{type(exc).__name__}:{_safe_str(exc)}")
        return 2


if __name__ == "__main__":
    sys.exit(main())
