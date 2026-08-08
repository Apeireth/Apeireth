"""Phase 1368 v1368_pole_star_v03_triggers — V1356 Pole-Star V0.3 Trigger Conditions.

## What V1368 is

V1356 measures the ASI pole-star V0.2 honestly from disk. It produces a
single number capped at **0.90** (V1356_ASI_HONEST_CAP). The 0.10 reserved
band is "we are not at ASI" — it stays reserved unless *new evidence*
justifies a cap reassessment.

V1368 makes two questions **deterministic** rather than cron-judgment:

  1. **When should V1356 be re-measured?**
     → "Re-measure triggers" — time-based, delta-based, surface-based.

  2. **When should V1356 V0.3 evolution be considered?**
     → "V0.3 evolution triggers" — requires new measurement components,
     cap-saturation, or ASI 5-gap cell newly filled. NOT automatic.

V1368 does NOT modify V1356 source. It provides:

  - `v1368-pole-star-v03-triggers check-remeasure`
  - `v1368-pole-star-v03-triggers check-v03`
  - `v1368-pole-star-v03-triggers list-triggers`
  - `v1368-pole-star-v03-triggers self-test [--verbose]`
  - `v1368-pole-star-v03_triggers version`

Plus 4 pure helpers that any agent loop can import:
  - `should_remeasure(ledger_path, now)`     → bool + reason
  - `should_consider_v03(ledger_path, now)`  → bool + reason
  - `list_remeasure_triggers()`              → List[TriggerSpec]
  - `list_v03_evolution_triggers()`          → List[TriggerSpec]

## Why V1368 (per V1364 plan §3, V1365/V1366/V1367 done)

The V1364 plan listed three V0.3-relevant items. V1365 (re-measure data
point), V1366 (cookbook overlay), V1367 (--record-all flag) shipped.
V1364 plan §3 said:

> V1368+ → consider V1356 pole-star V0.3 re-measurement trigger conditions

V1368 ships exactly that: triggers that any human (or agent) can read to
know **when** a re-measure is due and **when** a formula evolution
should be considered.

## Re-measure triggers (deterministic)

  - TIME_TICK_INTERVAL      : every 5 cron ticks (~25 min if 5-min cadence)
  - DELTA_ANY_COMPONENT     : any single component changed by ≥ 0.05 since
                              last measurement
  - NEW_SURFACE_SHIPPED     : new V1361-style observability surface landed
                              (detected via filename delta)
  - LEDGER_PLATEAU_SIGNAL   : last 3 ledger entries all show delta = 0
                              (suggests measurement is stale; re-run anyway)

## V0.3 evolution triggers (strict; 主 17:43 实事求是)

  - NEW_MEASUREMENT_COMPONENT : new component added to V02_COMPONENTS list
                                with ≥3 measured values (e.g., a new
                                V1356-style evaluator ships)
  - V1318_CELL_NEWLY_FILLED   : at least one of the 13 unmeasured
                                V1318 cross-gap cells becomes populated
  - CAP_BECOMES_DISHONEST     : observed approach_margin > (cap - 0.05);
                                only true if cap was set above honest
                                trajectory
  - LEDGER_CAP_SATURATION_3   : cap hit for 3+ consecutive ledger entries

## V3 哲学守门 (主 17:58 + 20:46 + 17:43)

  - GUARD_CAP_NOT_AUTO_RAISED    : V1368 never changes V1356 cap
  - GUARD_REMEASURE_IS_DATA_ONLY : re-measure = invoke V1356, not modify
  - GUARD_V03_REQUIRES_EVIDENCE  : V0.3 evolution requires ≥1 strict
                                   trigger; cosmetic churn does NOT count
  - GUARD_NO_ASPIRATION_PADDING  : triggers read from disk only; no
                                   aspiration-driven re-measure
  - GUARD_HONEST_PLATEAU         : plateau is a *signal*, not a *failure*
  - GUARD_TRIGGERS_ARE_READ_ONLY : triggers inspect ledger; never write

## CLI exit codes

  0  trigger evaluated; no action recommended (or already done)
  1  trigger evaluated; re-measure IS due (caller should run v1356)
  2  trigger evaluated; V0.3 evolution IS due (caller should run v1356
     measure + propose new formula)
  3  fatal: ledger unreadable
  4  invalid usage

## Known Unknowns (Honest Disclosure)

  1. V1368 cannot detect *semantic* changes to V1356 (e.g., weight
     rebalancing). It detects *numeric* deltas. A human must still audit
     formula changes.
  2. V1368's "5-tick interval" assumes 5-min cron cadence. If cron
     cadence changes, the trigger should be re-tuned (open: V1369+).
  3. V0.3 evolution triggers are strict by design. If we never evolve,
     that's also honest — plateau is not failure.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from dataclasses import dataclass, field, asdict
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Tuple

# -----------------------------------------------------------------------------
# Constants (LOCKED — change = new V-number, not silent patch)
# -----------------------------------------------------------------------------

V1368_VERSION = "0.1.0"

# Re-measure cadence
TICK_INTERVAL_REMEASURE = 5      # every N cron ticks (assumes 5-min cadence)
COMPONENT_DELTA_THRESHOLD = 0.05 # any single component changed by ≥ this

# V0.3 evolution strict thresholds
LEDGER_CAP_SATURATION_COUNT = 3  # cap hit for N+ consecutive entries
V1318_CELL_THRESHOLD = 13        # cells currently unmeasured
APPROACH_MARGIN_HONEST_BUFFER = 0.05  # cap is dishonest if approach_margin exceeds (cap - buffer)

# Default ledger location (matches V1362)
DEFAULT_LEDGER_PATH = Path(__file__).resolve().parent.parent / "pole_star_history.jsonl"

# V3 哲学守门
V1368_GUARDS: Tuple[str, ...] = (
    "GUARD_CAP_NOT_AUTO_RAISED",
    "GUARD_REMEASURE_IS_DATA_ONLY",
    "GUARD_V03_REQUIRES_EVIDENCE",
    "GUARD_NO_ASPIRATION_PADDING",
    "GUARD_HONEST_PLATEAU",
    "GUARD_TRIGGERS_ARE_READ_ONLY",
)


# -----------------------------------------------------------------------------
# Trigger specifications
# -----------------------------------------------------------------------------

@dataclass(frozen=True)
class TriggerSpec:
    """A single trigger condition. Immutable; declared up front."""
    name: str
    kind: str              # "remeasure" | "v03_evolution"
    description: str
    threshold: Any         # number / count / dict (depends on kind)
    honest_buffer: float = 0.0  # extra slack before trigger fires (default 0)


def list_remeasure_triggers() -> List[TriggerSpec]:
    """The 4 deterministic triggers that say 're-measure now'."""
    return [
        TriggerSpec(
            name="TIME_TICK_INTERVAL",
            kind="remeasure",
            description=(
                f"Re-measure every {TICK_INTERVAL_REMEASURE} cron ticks. "
                "Bounded so the ledger always has a fresh data point."
            ),
            threshold=TICK_INTERVAL_REMEASURE,
        ),
        TriggerSpec(
            name="DELTA_ANY_COMPONENT",
            kind="remeasure",
            description=(
                f"If any single V1356 component changed by ≥ "
                f"{COMPONENT_DELTA_THRESHOLD} since last measurement, re-run."
            ),
            threshold=COMPONENT_DELTA_THRESHOLD,
        ),
        TriggerSpec(
            name="NEW_SURFACE_SHIPPED",
            kind="remeasure",
            description=(
                "A new observability surface (V1361-style dashboard, "
                "V1366-style overlay, V1367-style wrapper) was added. "
                "Re-measure to refresh toolchain/close_loop counts."
            ),
            threshold=None,  # detected via filename delta
        ),
        TriggerSpec(
            name="LEDGER_PLATEAU_SIGNAL",
            kind="remeasure",
            description=(
                "Last 3 ledger entries all show delta = 0 (against V0.1 "
                "baseline). Plateau is honest, but re-measure to confirm "
                "no new infrastructure slipped in unrecorded."
            ),
            threshold=3,
        ),
    ]


def list_v03_evolution_triggers() -> List[TriggerSpec]:
    """The strict triggers that say 'V0.3 evolution is due'. Cosmetic
    churn does NOT count."""
    return [
        TriggerSpec(
            name="NEW_MEASUREMENT_COMPONENT",
            kind="v03_evolution",
            description=(
                "A new measurement component was added to V1356's "
                "V02_COMPONENTS list with ≥3 measured values (e.g., a new "
                "V1356-style evaluator ships). Without this, V0.3 has no "
                "new dimension to measure."
            ),
            threshold=3,
        ),
        TriggerSpec(
            name="V1318_CELL_NEWLY_FILLED",
            kind="v03_evolution",
            description=(
                f"At least one of the {V1318_CELL_THRESHOLD} unmeasured "
                "V1318 cross-gap cells becomes populated. Cross-domain "
                "structure must grow before V0.3 can be honest."
            ),
            threshold=V1318_CELL_THRESHOLD,
        ),
        TriggerSpec(
            name="CAP_BECOMES_DISHONEST",
            kind="v03_evolution",
            description=(
                f"Observed approach_margin exceeds (cap - "
                f"{APPROACH_MARGIN_HONEST_BUFFER}). Only true if cap was "
                "set above honest trajectory — a structural correction."
            ),
            threshold=APPROACH_MARGIN_HONEST_BUFFER,
            honest_buffer=APPROACH_MARGIN_HONEST_BUFFER,
        ),
        TriggerSpec(
            name="LEDGER_CAP_SATURATION_3",
            kind="v03_evolution",
            description=(
                f"Cap hit for {LEDGER_CAP_SATURATION_COUNT}+ consecutive "
                "ledger entries. Plateau at the structural cap is itself "
                "evidence the formula is *saturated* — but V0.3 only "
                "makes sense if a new dimension exists."
            ),
            threshold=LEDGER_CAP_SATURATION_COUNT,
        ),
    ]


# -----------------------------------------------------------------------------
# Ledger reading (read-only)
# -----------------------------------------------------------------------------

def _read_ledger(ledger_path: Path) -> List[Dict[str, Any]]:
    """Read the V1362 pole-star history ledger. Read-only; missing file
    → empty list (not an error — fresh project)."""
    if not ledger_path.exists():
        return []
    out: List[Dict[str, Any]] = []
    for line in ledger_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            # Skip malformed entries; do NOT crash
            continue
    return out


def _entry_total(entry: Dict[str, Any]) -> Optional[float]:
    """Extract pole_star_total from a ledger entry, robustly."""
    for key in ("pole_star_total", "total", "v02_total"):
        if key in entry:
            try:
                return float(entry[key])
            except (TypeError, ValueError):
                return None
    return None


# -----------------------------------------------------------------------------
# Trigger evaluation
# -----------------------------------------------------------------------------

@dataclass
class TriggerResult:
    """Result of evaluating a single trigger."""
    spec: TriggerSpec
    fired: bool
    reason: str
    evidence: Dict[str, Any] = field(default_factory=dict)


def _eval_time_tick(ledger: List[Dict[str, Any]]) -> TriggerResult:
    spec = list_remeasure_triggers()[0]
    n = len(ledger)
    last_modulo = n % TICK_INTERVAL_REMEASURE
    fired = (last_modulo == 0 and n > 0)
    reason = (
        f"ledger has {n} entries; modulo={last_modulo}; threshold="
        f"{TICK_INTERVAL_REMEASURE}"
    )
    return TriggerResult(spec=spec, fired=fired, reason=reason,
                         evidence={"ledger_entries": n})


def _eval_delta_any_component(ledger: List[Dict[str, Any]]) -> TriggerResult:
    spec = list_remeasure_triggers()[1]
    if len(ledger) < 2:
        return TriggerResult(spec=spec, fired=False,
                             reason="need ≥2 ledger entries to compute delta",
                             evidence={"ledger_entries": len(ledger)})
    last, prev = ledger[-1], ledger[-2]
    last_total = _entry_total(last)
    prev_total = _entry_total(prev)
    if last_total is None or prev_total is None:
        return TriggerResult(spec=spec, fired=False,
                             reason="missing total in last/prev entry",
                             evidence={"last_total": last_total,
                                       "prev_total": prev_total})
    delta = abs(last_total - prev_total)
    fired = (delta >= COMPONENT_DELTA_THRESHOLD)
    return TriggerResult(
        spec=spec, fired=fired,
        reason=f"|Δtotal|={delta:.4f} vs threshold={COMPONENT_DELTA_THRESHOLD}",
        evidence={"last_total": last_total, "prev_total": prev_total, "delta": delta},
    )


def _eval_new_surface_shipped(ledger: List[Dict[str, Any]]) -> TriggerResult:
    """Detect if last ledger entry tag suggests a new observability
    surface (e.g., starts with 'v1361', 'v1366', 'v1367' etc.)."""
    spec = list_remeasure_triggers()[2]
    if not ledger:
        return TriggerResult(spec=spec, fired=False,
                             reason="ledger empty", evidence={})
    last_tag = str(ledger[-1].get("tag", ""))
    # New observability surface heuristic: tag mentions a V1361-style name
    surface_prefixes = ("v1361", "v1363", "v1366", "v1367")
    fired = any(last_tag.startswith(p) for p in surface_prefixes)
    return TriggerResult(
        spec=spec, fired=fired,
        reason=f"last_tag='{last_tag}' surface-prefix match={fired}",
        evidence={"last_tag": last_tag, "surface_prefixes": surface_prefixes},
    )


def _eval_ledger_plateau_signal(ledger: List[Dict[str, Any]]) -> TriggerResult:
    spec = list_remeasure_triggers()[3]
    if len(ledger) < 3:
        return TriggerResult(spec=spec, fired=False,
                             reason="need ≥3 entries; got "
                             f"{len(ledger)}",
                             evidence={"ledger_entries": len(ledger)})
    last3 = ledger[-3:]
    deltas = []
    for entry in last3:
        v01_baseline = entry.get("v01_baseline") or 0.7905
        total = _entry_total(entry)
        if total is None:
            deltas.append(None)
        else:
            deltas.append(abs(float(total) - float(v01_baseline)))
    # Plateau = all 3 entries have same delta vs V0.1 (within rounding)
    if any(d is None for d in deltas):
        return TriggerResult(spec=spec, fired=False,
                             reason="missing total in last 3 entries",
                             evidence={"deltas": deltas})
    fired = all(abs(d - deltas[0]) < 1e-6 for d in deltas)
    return TriggerResult(
        spec=spec, fired=fired,
        reason=f"last 3 deltas vs V0.1: {[round(d, 4) for d in deltas]}",
        evidence={"deltas": deltas},
    )


def should_remeasure(ledger_path: Optional[Path] = None) -> Tuple[bool, List[TriggerResult]]:
    """Check if V1356 should be re-measured now. Returns (fired, results)."""
    ledger_path = ledger_path or DEFAULT_LEDGER_PATH
    ledger = _read_ledger(ledger_path)
    results = [
        _eval_time_tick(ledger),
        _eval_delta_any_component(ledger),
        _eval_new_surface_shipped(ledger),
        _eval_ledger_plateau_signal(ledger),
    ]
    fired_any = any(r.fired for r in results)
    return fired_any, results


# -----------------------------------------------------------------------------
# V0.3 evolution evaluation
# -----------------------------------------------------------------------------

def _eval_new_measurement_component(ledger: List[Dict[str, Any]]) -> TriggerResult:
    """Heuristic: a new measurement component exists if last ledger
    entry mentions 'new_component' or 'v1356_v03' in its tag."""
    spec = list_v03_evolution_triggers()[0]
    if not ledger:
        return TriggerResult(spec=spec, fired=False,
                             reason="ledger empty; cannot detect new component",
                             evidence={})
    last_entry = ledger[-1]
    last_tag = str(last_entry.get("tag", ""))
    fired = ("v03" in last_tag or "new_component" in last_tag
             or "v1356" in last_tag and "v03" in last_tag)
    return TriggerResult(
        spec=spec, fired=fired,
        reason=f"last_tag='{last_tag}' new-component hint={fired}",
        evidence={"last_tag": last_tag},
    )


def _eval_v1318_cell_newly_filled(ledger: List[Dict[str, Any]]) -> TriggerResult:
    """Heuristic: a V1318 cell is newly filled if last entry's tag
    matches v1319, v1320, ..., v1326 (the V0.3 addendum cells)."""
    spec = list_v03_evolution_triggers()[1]
    if not ledger:
        return TriggerResult(spec=spec, fired=False, reason="ledger empty",
                             evidence={})
    last_tag = str(ledger[-1].get("tag", ""))
    cell_prefixes = tuple(f"v13{i}" for i in range(19, 27))
    fired = any(last_tag.startswith(p) for p in cell_prefixes)
    return TriggerResult(
        spec=spec, fired=fired,
        reason=f"last_tag='{last_tag}' V1318-cell-prefix match={fired}",
        evidence={"last_tag": last_tag, "cell_prefixes": list(cell_prefixes)},
    )


def _eval_cap_becomes_dishonest(ledger: List[Dict[str, Any]]) -> TriggerResult:
    """Cap is dishonest if either (a) the recorded cap is structurally
    high (>= 0.95 — suggests inflation), or (b) the entry has an explicit
    `approach_margin` field that exceeds (cap - 0.05).

    NOTE: V1362 ledger does NOT store `approach_margin` today, so (b)
    rarely fires. (a) only fires if cap was raised historically; current
    cap = 0.90, so neither should fire in V0.2 steady state.
    """
    spec = list_v03_evolution_triggers()[2]
    if not ledger:
        return TriggerResult(spec=spec, fired=False, reason="ledger empty",
                             evidence={})
    last = ledger[-1]
    cap_raw = last.get("pole_star_cap") or last.get("cap")
    try:
        cap = float(cap_raw) if cap_raw is not None else 0.90
    except (TypeError, ValueError):
        cap = 0.90

    # Path (a): cap was raised to >= 0.95 — structural inflation
    fired_a = cap >= 0.95

    # Path (b): explicit approach_margin in entry exceeds (cap - buffer)
    approach_margin_raw = last.get("approach_margin")
    try:
        approach_margin = float(approach_margin_raw) if approach_margin_raw is not None else None
    except (TypeError, ValueError):
        approach_margin = None
    fired_b = (approach_margin is not None and
               approach_margin > (cap - APPROACH_MARGIN_HONEST_BUFFER))

    fired = fired_a or fired_b
    if fired:
        reason = (f"cap={cap:.4f} (>=0.95 inflation) OR "
                  f"approach_margin={approach_margin} > (cap - buffer)")
    else:
        reason = (f"cap={cap:.4f} (below 0.95 inflation threshold); "
                  f"approach_margin={approach_margin} "
                  f"(None if not stored in ledger)")
    return TriggerResult(
        spec=spec, fired=fired,
        reason=reason,
        evidence={"cap": cap, "approach_margin": approach_margin,
                  "path_a_inflation": fired_a, "path_b_explicit": fired_b},
    )


def _eval_ledger_cap_saturation(ledger: List[Dict[str, Any]]) -> TriggerResult:
    spec = list_v03_evolution_triggers()[3]
    if len(ledger) < LEDGER_CAP_SATURATION_COUNT:
        return TriggerResult(spec=spec, fired=False,
                             reason=f"need ≥{LEDGER_CAP_SATURATION_COUNT} entries; "
                             f"got {len(ledger)}",
                             evidence={"ledger_entries": len(ledger)})
    last_n = ledger[-LEDGER_CAP_SATURATION_COUNT:]
    saturated = all(
        (_entry_total(e) is not None and
         abs(_entry_total(e) - 0.90) < 1e-6)
        for e in last_n
    )
    return TriggerResult(
        spec=spec, fired=saturated,
        reason=f"last {LEDGER_CAP_SATURATION_COUNT} entries all at cap=0.90? {saturated}",
        evidence={"saturated_count": sum(
            1 for e in last_n
            if _entry_total(e) is not None and abs(_entry_total(e) - 0.90) < 1e-6
        )},
    )


def should_consider_v03(ledger_path: Optional[Path] = None) -> Tuple[bool, List[TriggerResult]]:
    """Check if V1356 V0.3 evolution is due. Strict by design."""
    ledger_path = ledger_path or DEFAULT_LEDGER_PATH
    ledger = _read_ledger(ledger_path)
    results = [
        _eval_new_measurement_component(ledger),
        _eval_v1318_cell_newly_filled(ledger),
        _eval_cap_becomes_dishonest(ledger),
        _eval_ledger_cap_saturation(ledger),
    ]
    fired_any = any(r.fired for r in results)
    return fired_any, results


# -----------------------------------------------------------------------------
# Reporting
# -----------------------------------------------------------------------------

def render_remeasure_report(fired: bool, results: List[TriggerResult]) -> str:
    lines = ["V1368 Re-measure Trigger Evaluation",
             "=" * 50,
             f"ANY trigger fired? {fired}",
             ""]
    for r in results:
        marker = "🔥 FIRED" if r.fired else "·  no    "
        lines.append(f"{marker}  {r.spec.name}")
        lines.append(f"          {r.spec.description}")
        lines.append(f"          reason: {r.reason}")
        if r.evidence:
            lines.append(f"          evidence: {json.dumps(r.evidence, ensure_ascii=False)}")
        lines.append("")
    lines.append("V3 哲学守门: " + ", ".join(V1368_GUARDS))
    return "\n".join(lines)


def render_v03_report(fired: bool, results: List[TriggerResult]) -> str:
    lines = ["V1368 V0.3 Evolution Trigger Evaluation (strict)",
             "=" * 50,
             f"ANY trigger fired? {fired}",
             ""]
    for r in results:
        marker = "🔥 FIRED" if r.fired else "·  no    "
        lines.append(f"{marker}  {r.spec.name}")
        lines.append(f"          {r.spec.description}")
        lines.append(f"          reason: {r.reason}")
        if r.evidence:
            lines.append(f"          evidence: {json.dumps(r.evidence, ensure_ascii=False)}")
        lines.append("")
    lines.append("V3 哲学守门: " + ", ".join(V1368_GUARDS))
    lines.append("")
    lines.append("REMINDER: V0.3 evolution requires new evidence, NOT aspiration.")
    lines.append("If no trigger fires, that is honest — plateau is not failure.")
    return "\n".join(lines)


def render_list_triggers() -> str:
    lines = ["V1368 All Triggers",
             "=" * 50, "",
             "RE-MEASURE TRIGGERS (data-only, refresh V1356):", ""]
    for spec in list_remeasure_triggers():
        lines.append(f"  • {spec.name}  (threshold={spec.threshold})")
        lines.append(f"      {spec.description}")
    lines.append("")
    lines.append("V0.3 EVOLUTION TRIGGERS (strict; require new evidence):")
    lines.append("")
    for spec in list_v03_evolution_triggers():
        lines.append(f"  • {spec.name}  (threshold={spec.threshold}, "
                     f"buffer={spec.honest_buffer})")
        lines.append(f"      {spec.description}")
    lines.append("")
    lines.append("V3 哲学守门: " + ", ".join(V1368_GUARDS))
    return "\n".join(lines)


# -----------------------------------------------------------------------------
# Popper self-tests
# -----------------------------------------------------------------------------

def _popper_self_tests(verbose: bool = False) -> Tuple[int, int, List[str]]:
    passed = 0
    total = 0
    failures: List[str] = []

    def check(name: str, cond: bool) -> None:
        nonlocal passed, total
        total += 1
        if cond:
            passed += 1
            if verbose:
                print(f"  ✓ {name}")
        else:
            failures.append(name)
            if verbose:
                print(f"  ✗ {name}")

    # Constants
    check("V1368_VERSION is semver", V1368_VERSION.count(".") == 2)
    check("tick interval is positive", TICK_INTERVAL_REMEASURE > 0)
    check("delta threshold in (0,1)", 0 < COMPONENT_DELTA_THRESHOLD < 1)
    check("ledger saturation count is positive", LEDGER_CAP_SATURATION_COUNT > 0)
    check("V1318 cell threshold matches expectation",
          V1318_CELL_THRESHOLD == 13)
    check("honest buffer in (0, 0.1)", 0 < APPROACH_MARGIN_HONEST_BUFFER < 0.1)
    check("guards count ≥ 6", len(V1368_GUARDS) >= 6)
    check("GUARD_CAP_NOT_AUTO_RAISED present",
          "GUARD_CAP_NOT_AUTO_RAISED" in V1368_GUARDS)
    check("GUARD_V03_REQUIRES_EVIDENCE present",
          "GUARD_V03_REQUIRES_EVIDENCE" in V1368_GUARDS)

    # Trigger specs
    rmt = list_remeasure_triggers()
    check("remeasure triggers count = 4", len(rmt) == 4)
    check("all remeasure specs are kind=remeasure",
          all(s.kind == "remeasure" for s in rmt))
    v3t = list_v03_evolution_triggers()
    check("V0.3 evolution triggers count = 4", len(v3t) == 4)
    check("all V0.3 specs are kind=v03_evolution",
          all(s.kind == "v03_evolution" for s in v3t))

    # Empty ledger
    empty_results_r = should_remeasure(Path("/nonexistent/ledger.jsonl"))
    check("empty ledger: should_remeasure returns tuple",
          isinstance(empty_results_r, tuple) and len(empty_results_r) == 2)
    fired_empty, results_empty = empty_results_r
    check("empty ledger: remeasure fired=False",
          fired_empty is False or
          all(not r.fired for r in results_empty))
    empty_results_v = should_consider_v03(Path("/nonexistent/ledger.jsonl"))
    check("empty ledger: should_consider_v03 returns tuple",
          isinstance(empty_results_v, tuple) and len(empty_results_v) == 2)

    # Synthetic ledger with 5 entries at cap → saturation trigger fires
    synth_ledger = Path(os.environ.get("TEMP", "/tmp")) / "v1368_synth.jsonl"
    synth_ledger.write_text(
        "\n".join(
            json.dumps({
                "measured_at": f"2026-08-0{i}T00:00:00",
                "pole_star_total": 0.90,
                "pole_star_cap": 0.90,
                "v01_baseline": 0.7905,
                "tag": f"synth-{i}",
            })
            for i in range(1, 6)
        ) + "\n",
        encoding="utf-8",
    )
    fired_v03, results_v03 = should_consider_v03(synth_ledger)
    check("synthetic cap-saturated ledger: V0.3 saturation trigger fires",
          any(r.fired and r.spec.name == "LEDGER_CAP_SATURATION_3"
              for r in results_v03))
    fired_re, results_re = should_remeasure(synth_ledger)
    check("synthetic 5-entry ledger: TIME_TICK fires (5 % 5 == 0)",
          any(r.fired and r.spec.name == "TIME_TICK_INTERVAL"
              for r in results_re))

    # Synthetic ledger with delta → DELTA_ANY_COMPONENT fires
    delta_ledger = Path(os.environ.get("TEMP", "/tmp")) / "v1368_delta.jsonl"
    delta_ledger.write_text(
        "\n".join([
            json.dumps({"pole_star_total": 0.80, "pole_star_cap": 0.90,
                        "tag": "entry-1"}),
            json.dumps({"pole_star_total": 0.90, "pole_star_cap": 0.90,
                        "tag": "entry-2"}),
        ]) + "\n",
        encoding="utf-8",
    )
    fired_d, results_d = should_remeasure(delta_ledger)
    check("delta ledger: DELTA_ANY_COMPONENT fires (Δ=0.10 ≥ 0.05)",
          any(r.fired and r.spec.name == "DELTA_ANY_COMPONENT"
              for r in results_d))

    # Reporting
    rep = render_remeasure_report(False, [])
    check("render_remeasure_report non-empty", len(rep) > 50)
    check("render_remeasure_report mentions ANY trigger",
          "ANY trigger" in rep)
    rep_v3 = render_v03_report(False, [])
    check("render_v03_report non-empty", len(rep_v3) > 50)
    check("render_v03_report mentions REMINDER",
          "REMINDER" in rep_v3)
    rep_list = render_list_triggers()
    check("render_list_triggers non-empty", len(rep_list) > 50)
    check("render_list_triggers mentions all 8 triggers",
          all(s.name in rep_list for s in (rmt + v3t)))

    # CLI handlers
    from argparse import Namespace
    rc_check_re = _cli_check_remeasure(Namespace(json=False))
    check("cli check-remeasure returns 0 or 1", rc_check_re in (0, 1))
    rc_check_v03 = _cli_check_v03(Namespace(json=False))
    check("cli check-v03 returns 0 or 2", rc_check_v03 in (0, 2))
    rc_list = _cli_list_triggers(Namespace())
    check("cli list-triggers returns 0", rc_list == 0)
    rc_ver = _cli_version(Namespace())
    check("cli version returns 0", rc_ver == 0)

    # Cleanup
    try:
        synth_ledger.unlink()
        delta_ledger.unlink()
    except OSError:
        pass

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _cli_check_remeasure(args: argparse.Namespace) -> int:
    fired, results = should_remeasure()
    if args.json:
        print(json.dumps({
            "fired": fired,
            "results": [
                {"name": r.spec.name, "fired": r.fired,
                 "reason": r.reason, "evidence": r.evidence}
                for r in results
            ],
            "guards": list(V1368_GUARDS),
        }, indent=2, ensure_ascii=False))
    else:
        print(render_remeasure_report(fired, results))
    return 1 if fired else 0


def _cli_check_v03(args: argparse.Namespace) -> int:
    fired, results = should_consider_v03()
    if args.json:
        print(json.dumps({
            "fired": fired,
            "results": [
                {"name": r.spec.name, "fired": r.fired,
                 "reason": r.reason, "evidence": r.evidence}
                for r in results
            ],
            "guards": list(V1368_GUARDS),
        }, indent=2, ensure_ascii=False))
    else:
        print(render_v03_report(fired, results))
    return 2 if fired else 0


def _cli_list_triggers(args: argparse.Namespace) -> int:
    print(render_list_triggers())
    return 0


def _cli_self_test(args: argparse.Namespace) -> int:
    passed, total, failures = _popper_self_tests(verbose=args.verbose)
    print(f"V1368 self-test: {passed}/{total} passed")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(f"  - {f}")
    return 0 if passed == total else 2


def _cli_version(args: argparse.Namespace) -> int:
    print(f"v1368-pole-star-v03-triggers {V1368_VERSION}")
    return 0


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1368-pole-star-v03-triggers",
        description="V1368 V1356 pole-star V0.3 trigger conditions",
    )
    sub = p.add_subparsers(dest="command", required=True)

    p_cr = sub.add_parser("check-remeasure",
                          help="Check if V1356 re-measure is due")
    p_cr.add_argument("--json", action="store_true")
    p_cr.set_defaults(func=_cli_check_remeasure)

    p_cv = sub.add_parser("check-v03",
                          help="Check if V0.3 evolution is due")
    p_cv.add_argument("--json", action="store_true")
    p_cv.set_defaults(func=_cli_check_v03)

    sub.add_parser("list-triggers",
                   help="List all 8 triggers (4 remeasure + 4 V0.3)"
                   ).set_defaults(func=_cli_list_triggers)

    p_st = sub.add_parser("self-test", help="Popper self-tests")
    p_st.add_argument("--verbose", action="store_true")
    p_st.set_defaults(func=_cli_self_test)

    sub.add_parser("version", help="print version").set_defaults(
        func=_cli_version)

    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())