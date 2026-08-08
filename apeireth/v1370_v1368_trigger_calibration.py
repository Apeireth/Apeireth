"""Phase 1370 v1370_v1368_trigger_calibration — Honest trigger calibration for V1368.

## What V1370 is

V1368 shipped 8 deterministic trigger conditions (4 remeasure + 4 V0.3
evolution). V1369 wired V1368 into the cron tick and recorded each
evaluation in a sidecar. After 7 evaluations over ~8 minutes, V1369's
sidecar revealed a problem: **triggers fire at 100% rate**.

Inspection showed three causes:

  1. **LEDGER_PLATEAU_SIGNAL**: spec says "last 3 ledger entries all
     show delta = 0", but the implementation fires when "all 3 entries
     have the same delta vs V0.1" — so any steady-state measurement
     (e.g., all at delta=0.1095 because V0.2 is the only measurement
     since V0.1) fires. False positive.

  2. **NEW_SURFACE_SHIPPED**: spec says "a new observability surface
     landed", but the implementation fires whenever the *last* entry's
     tag starts with `v1361`/`v1363`/`v1366`/`v1367` — so any time the
     *latest* measurement happens to be tagged with a known surface
     module, it fires. False positive.

  3. **LEDGER_CAP_SATURATION_3**: spec says "cap hit for 3+ consecutive
     ledger entries", but the implementation fires on any 3 consecutive
     entries all at cap — including 3 entries with the *same tag* (e.g.,
     3 back-to-back self-test runs). Technically correct, but noisy.

V1370 is the **honest calibration layer**: V1368 stays unchanged
(GUARD_NO_SOURCE_MUTATION), V1370 wraps each trigger with stricter
checks that filter out false positives. V1369 can be pointed at V1370
without changing V1368's source or its public API.

## Why V1370

  - V1368's *specs* are honest. V1368's *implementations* have known
    false-positive surfaces exposed by V1369's empirical sidecar.
  - Refactoring V1368 risks breaking the V1368 chain (29 self-tests +
    ~58 pytest in `test_v1368`). V1370 is *additive*: no V1368 mutation.
  - V3 philosophy: **honest plateau is signal, not failure** (master
    17:43 实事求是). V1369 firing 100% of the time would *de-sensitize*
    cron lanes to trigger signals. V1370 restores honest signal.

## What V1370 calibrates

  - **PLATEAU**: requires |Δ| < PLATEAU_DELTA_EPSILON (= 1e-4) for all
    3 consecutive entries — spec-faithful to "delta = 0".
  - **NEW_SURFACE**: requires the surface prefix to be absent from the
    prior NEW_SURFACE_LOOKBACK (= 10) entries — spec-faithful to "new".
  - **CAP_SATURATION_3**: requires the saturated entries to span at
    least CAP_SAT_MIN_DISTINCT_TAGS (= 2) distinct tags — filters
    duplicate-tag spam.

  - TIME_TICK_INTERVAL, DELTA_ANY_COMPONENT, NEW_MEASUREMENT_COMPONENT,
    V1318_CELL_NEWLY_FILLED, CAP_BECOMES_DISHONEST: pass through
    unchanged (already spec-faithful per V1368 self-tests).

## CLI

  - `python -m apeireth.v1370_v1368_trigger_calibration evaluate`
        → run V1368 + V1370 calibration; show side-by-side; exit 0/1/2
  - `python -m apeireth.v1370_v1368_trigger_calibration evaluate --json`
        → JSON output
  - `python -m apeireth.v1370_v1368_trigger_calibration compare`
        → side-by-side comparison table (v1368 vs v1370, no firing)
  - `python -m apeireth.v1370_v1368_trigger_calibration summary`
        → fire-rate comparison across current ledger
  - `python -m apeireth.v1370_v1368_trigger_calibration self-test [--verbose]`
        → Popper self-tests covering positive AND negative cases

## V3 哲学守门 (主 17:43 + 17:58 + 20:46 + 22:33)

  - GUARD_NO_SOURCE_MUTATION    : V1370 does NOT modify V1368 source
  - GUARD_CALIBRATION_NOT_LOOSENING : V1370 only tightens, never loosens
  - GUARD_PLATEAU_REQUIRES_ZERO_DELTA : PLATEAU_DELTA_EPSILON = 1e-4
  - GUARD_NEW_SURFACE_REQUIRES_DELTA  : lookback window = 10 entries
  - GUARD_DISHONEST_CAP_REQUIRES_DIVERSITY : distinct-tag count ≥ 2
  - GUARD_HONEST_PLATEAU             : plateau is signal, not failure

## Known Unknowns (Honest Disclosure)

  1. V1370's calibration constants (1e-4 / 10 / 2) are heuristic. They
     may need tuning after observing V1370's sidecar over multiple
     measurement cycles. V1370 is itself a hypothesis.
  2. V1368's spec language ("new surface", "delta = 0") is interpreted
     liberally here. A future V1371+ may tighten further if V1370 still
     fires too often.
  3. V1370 does NOT touch V1369's cron cadence or the ledger format.
     It is purely a trigger-evaluation wrapper.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
from collections import Counter
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# Import V1368 — read-only; V1370 does NOT mutate it.
from apeireth.v1368_pole_star_v03_triggers import (
    APPROACH_MARGIN_HONEST_BUFFER,
    DEFAULT_LEDGER_PATH,
    LEDGER_CAP_SATURATION_COUNT,
    TICK_INTERVAL_REMEASURE,
    V1368_GUARDS,
    V1368_VERSION as V1368_TRIGGERS_VERSION,
    list_remeasure_triggers,
    list_v03_evolution_triggers,
    should_consider_v03,
    should_remeasure,
)


# -----------------------------------------------------------------------------
# Version + constants
# -----------------------------------------------------------------------------

V1370_VERSION = "0.1.0"

# GUARD_PLATEAU_REQUIRES_ZERO_DELTA: |Δ| must be below this to count as
# "delta = 0" per spec. 1e-4 is well below the smallest meaningful
# measurement precision (1e-3) and well above IEEE-754 noise (1e-15).
PLATEAU_DELTA_EPSILON = 1e-4

# GUARD_NEW_SURFACE_REQUIRES_DELTA: a surface is "new" only if no entry
# in the prior NEW_SURFACE_LOOKBACK entries shares the same surface
# prefix. 10 entries matches V1362's "trend" overlay window.
NEW_SURFACE_LOOKBACK = 10

# GUARD_DISHONEST_CAP_REQUIRES_DIVERSITY: cap-saturation requires the
# saturated entries to span ≥ CAP_SAT_MIN_DISTINCT_TAGS distinct tags —
# filters out 3 back-to-back self-test runs at the same tag.
CAP_SAT_MIN_DISTINCT_TAGS = 2


V1370_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_SOURCE_MUTATION",
    "GUARD_CALIBRATION_NOT_LOOSENING",
    "GUARD_PLATEAU_REQUIRES_ZERO_DELTA",
    "GUARD_NEW_SURFACE_REQUIRES_DELTA",
    "GUARD_DISHONEST_CAP_REQUIRES_DIVERSITY",
    "GUARD_HONEST_PLATEAU",
)


# -----------------------------------------------------------------------------
# Data structures
# -----------------------------------------------------------------------------

@dataclass(frozen=True)
class CalibratedResult:
    """One trigger result, with both raw (V1368) and calibrated (V1370)
    states plus a reason explaining what changed (or didn't)."""
    name: str
    kind: str           # "remeasure" | "v03_evolution"
    raw_fired: bool     # V1368's evaluation
    calibrated_fired: bool  # V1370's calibrated evaluation
    raw_reason: str
    calibrated_reason: str
    suppressed: bool    # True iff raw=True and calibrated=False


@dataclass(frozen=True)
class CalibrationSummary:
    """Whole-pipeline summary for one evaluation."""
    timestamp: str
    ledger_path: str
    ledger_entries: int
    raw_remeasure_fired: bool
    raw_v03_fired: bool
    calibrated_remeasure_fired: bool
    calibrated_v03_fired: bool
    remeasure_results: List[CalibratedResult] = field(default_factory=list)
    v03_results: List[CalibratedResult] = field(default_factory=list)
    remeasure_suppressed_count: int = 0
    v03_suppressed_count: int = 0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "timestamp": self.timestamp,
            "ledger_path": self.ledger_path,
            "ledger_entries": self.ledger_entries,
            "raw_remeasure_fired": self.raw_remeasure_fired,
            "raw_v03_fired": self.raw_v03_fired,
            "calibrated_remeasure_fired": self.calibrated_remeasure_fired,
            "calibrated_v03_fired": self.calibrated_v03_fired,
            "remeasure_suppressed_count": self.remeasure_suppressed_count,
            "v03_suppressed_count": self.v03_suppressed_count,
            "remeasure_results": [
                {
                    "name": r.name, "kind": r.kind,
                    "raw_fired": r.raw_fired,
                    "calibrated_fired": r.calibrated_fired,
                    "raw_reason": r.raw_reason,
                    "calibrated_reason": r.calibrated_reason,
                    "suppressed": r.suppressed,
                }
                for r in self.remeasure_results
            ],
            "v03_results": [
                {
                    "name": r.name, "kind": r.kind,
                    "raw_fired": r.raw_fired,
                    "calibrated_fired": r.calibrated_fired,
                    "raw_reason": r.raw_reason,
                    "calibrated_reason": r.calibrated_reason,
                    "suppressed": r.suppressed,
                }
                for r in self.v03_results
            ],
        }


# -----------------------------------------------------------------------------
# Ledger reading (mirrors V1368; read-only)
# -----------------------------------------------------------------------------

def _read_ledger(ledger_path: Path) -> List[Dict[str, Any]]:
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
            continue
    return out


def _entry_total(entry: Dict[str, Any]) -> Optional[float]:
    """Mirror V1368's _entry_total — total field with multiple aliases."""
    for key in ("pole_star_total", "total", "score"):
        if key in entry and entry[key] is not None:
            try:
                return float(entry[key])
            except (TypeError, ValueError):
                continue
    return None


# -----------------------------------------------------------------------------
# Calibration filters (the actual fix)
# -----------------------------------------------------------------------------

def _calibrate_plateau(raw_fired: bool, ledger: List[Dict[str, Any]]) -> Tuple[bool, str]:
    """Apply GUARD_PLATEAU_REQUIRES_ZERO_DELTA.

    V1368 fires when last 3 entries have equal delta vs V0.1.
    V1370 only fires when all 3 have |Δ| < PLATEAU_DELTA_EPSILON
    (i.e., effectively delta = 0 per spec).
    """
    if not raw_fired or len(ledger) < 3:
        return raw_fired, "passthrough (raw did not fire or ledger too short)"
    last3 = ledger[-3:]
    deltas: List[float] = []
    for entry in last3:
        v01 = entry.get("v01_baseline") or 0.7905
        total = _entry_total(entry)
        if total is None:
            return False, "missing total in last 3 entries"
        deltas.append(abs(float(total) - float(v01)))
    all_zero = all(d < PLATEAU_DELTA_EPSILON for d in deltas)
    if all_zero:
        return True, (f"all 3 |Δ|<{PLATEAU_DELTA_EPSILON:.0e} "
                      f"({[round(d, 5) for d in deltas]})")
    return False, (f"V1368 fired on equal-delta, but V1370 requires |Δ|<"
                   f"{PLATEAU_DELTA_EPSILON:.0e}: "
                   f"{[round(d, 4) for d in deltas]}")


def _extract_surface_prefix(tag: str, prefixes: Tuple[str, ...]) -> Optional[str]:
    for p in prefixes:
        if tag.startswith(p):
            return p
    return None


def _calibrate_new_surface(raw_fired: bool, ledger: List[Dict[str, Any]]) -> Tuple[bool, str]:
    """Apply GUARD_NEW_SURFACE_REQUIRES_DELTA.

    V1368 fires whenever *last* tag starts with a known surface prefix.
    V1370 requires the prefix to be absent from the prior
    NEW_SURFACE_LOOKBACK entries — i.e., surface must be NEW relative to
    recent measurement history.
    """
    if not raw_fired or not ledger:
        return raw_fired, "passthrough (raw did not fire or ledger empty)"
    surface_prefixes = ("v1361", "v1363", "v1366", "v1367")
    last_tag = str(ledger[-1].get("tag", ""))
    last_prefix = _extract_surface_prefix(last_tag, surface_prefixes)
    if last_prefix is None:
        return raw_fired, "passthrough (no known surface prefix in last tag)"
    prior = ledger[-1 - NEW_SURFACE_LOOKBACK:-1] if len(ledger) > 1 else []
    prior_prefixes = {
        _extract_surface_prefix(str(e.get("tag", "")), surface_prefixes)
        for e in prior
    }
    prior_prefixes.discard(None)
    if last_prefix in prior_prefixes:
        return False, (f"surface '{last_prefix}' was seen in prior "
                       f"{len(prior)} entries; not new")
    return True, (f"surface '{last_prefix}' is new "
                  f"(absent from prior {len(prior)} entries)")


def _calibrate_cap_saturation(raw_fired: bool, ledger: List[Dict[str, Any]]) -> Tuple[bool, str]:
    """Apply GUARD_DISHONEST_CAP_REQUIRES_DIVERSITY.

    V1368 fires when last N entries all at cap.
    V1370 requires those N entries to span ≥ CAP_SAT_MIN_DISTINCT_TAGS
    distinct tags — filters out 3 back-to-back self-test runs at the
    same tag (which is a measurement-cycle artifact, not a plateau).
    """
    if not raw_fired or len(ledger) < LEDGER_CAP_SATURATION_COUNT:
        return raw_fired, "passthrough (raw did not fire or ledger too short)"
    last_n = ledger[-LEDGER_CAP_SATURATION_COUNT:]
    tags = [str(e.get("tag", "")) for e in last_n]
    distinct_tags = len(set(tags))
    if distinct_tags >= CAP_SAT_MIN_DISTINCT_TAGS:
        return True, (f"last {LEDGER_CAP_SATURATION_COUNT} entries at cap "
                      f"span {distinct_tags} distinct tags: "
                      f"{sorted(set(tags))[:5]}")
    return False, (f"V1368 fired on cap-saturation, but V1370 requires "
                   f"≥{CAP_SAT_MIN_DISTINCT_TAGS} distinct tags; got "
                   f"{distinct_tags}: {tags}")


# Map: trigger name → calibrator
_CALIBRATORS = {
    "LEDGER_PLATEAU_SIGNAL": _calibrate_plateau,
    "NEW_SURFACE_SHIPPED": _calibrate_new_surface,
    "LEDGER_CAP_SATURATION_3": _calibrate_cap_saturation,
}


# -----------------------------------------------------------------------------
# Main calibration entrypoint
# -----------------------------------------------------------------------------

def calibrate_remeasure(ledger_path: Optional[Path] = None) -> Tuple[bool, bool, List[CalibratedResult]]:
    """Run V1368's should_remeasure, then apply V1370 calibration.

    Returns (raw_fired, calibrated_fired, results).
    """
    ledger_path = ledger_path or DEFAULT_LEDGER_PATH
    ledger = _read_ledger(ledger_path)
    raw_fired, raw_results = should_remeasure(ledger_path)
    calibrated_results: List[CalibratedResult] = []
    for r in raw_results:
        calibrator = _CALIBRATORS.get(r.spec.name)
        if calibrator is None:
            # Pass through unchanged (TIME_TICK_INTERVAL, DELTA_ANY_COMPONENT)
            calibrated_results.append(CalibratedResult(
                name=r.spec.name, kind=r.spec.kind,
                raw_fired=r.fired, calibrated_fired=r.fired,
                raw_reason=r.reason, calibrated_reason="passthrough (no calibrator)",
                suppressed=False,
            ))
            continue
        cal_fired, cal_reason = calibrator(r.fired, ledger)
        calibrated_results.append(CalibratedResult(
            name=r.spec.name, kind=r.spec.kind,
            raw_fired=r.fired, calibrated_fired=cal_fired,
            raw_reason=r.reason, calibrated_reason=cal_reason,
            suppressed=(r.fired and not cal_fired),
        ))
    calibrated_fired = any(r.calibrated_fired for r in calibrated_results)
    return raw_fired, calibrated_fired, calibrated_results


def calibrate_v03(ledger_path: Optional[Path] = None) -> Tuple[bool, bool, List[CalibratedResult]]:
    """Run V1368's should_consider_v03, then apply V1370 calibration.

    Returns (raw_fired, calibrated_fired, results).
    """
    ledger_path = ledger_path or DEFAULT_LEDGER_PATH
    ledger = _read_ledger(ledger_path)
    raw_fired, raw_results = should_consider_v03(ledger_path)
    calibrated_results: List[CalibratedResult] = []
    for r in raw_results:
        calibrator = _CALIBRATORS.get(r.spec.name)
        if calibrator is None:
            calibrated_results.append(CalibratedResult(
                name=r.spec.name, kind=r.spec.kind,
                raw_fired=r.fired, calibrated_fired=r.fired,
                raw_reason=r.reason, calibrated_reason="passthrough (no calibrator)",
                suppressed=False,
            ))
            continue
        cal_fired, cal_reason = calibrator(r.fired, ledger)
        calibrated_results.append(CalibratedResult(
            name=r.spec.name, kind=r.spec.kind,
            raw_fired=r.fired, calibrated_fired=cal_fired,
            raw_reason=r.reason, calibrated_reason=cal_reason,
            suppressed=(r.fired and not cal_fired),
        ))
    calibrated_fired = any(r.calibrated_fired for r in calibrated_results)
    return raw_fired, calibrated_fired, calibrated_results


def evaluate(ledger_path: Optional[Path] = None) -> CalibrationSummary:
    """Run both pipelines; return a CalibrationSummary."""
    ledger_path = ledger_path or DEFAULT_LEDGER_PATH
    ledger = _read_ledger(ledger_path)
    raw_re, cal_re, re_results = calibrate_remeasure(ledger_path)
    raw_v3, cal_v3, v3_results = calibrate_v03(ledger_path)
    return CalibrationSummary(
        timestamp=datetime.now(timezone.utc).isoformat(),
        ledger_path=str(ledger_path),
        ledger_entries=len(ledger),
        raw_remeasure_fired=raw_re,
        raw_v03_fired=raw_v3,
        calibrated_remeasure_fired=cal_re,
        calibrated_v03_fired=cal_v3,
        remeasure_results=re_results,
        v03_results=v3_results,
        remeasure_suppressed_count=sum(1 for r in re_results if r.suppressed),
        v03_suppressed_count=sum(1 for r in v3_results if r.suppressed),
    )


# -----------------------------------------------------------------------------
# Reporting
# -----------------------------------------------------------------------------

def render_evaluate(summary: CalibrationSummary) -> str:
    lines = [
        "V1370 V1368 Trigger Calibration Evaluation",
        "=" * 60,
        f"timestamp:     {summary.timestamp}",
        f"ledger:        {summary.ledger_path}  ({summary.ledger_entries} entries)",
        "",
        f"V1368 RAW:  remeasure_fired={summary.raw_remeasure_fired}  "
        f"v03_fired={summary.raw_v03_fired}",
        f"V1370 CAL:  remeasure_fired={summary.calibrated_remeasure_fired}  "
        f"v03_fired={summary.calibrated_v03_fired}",
        f"suppressed:  remeasure={summary.remeasure_suppressed_count}  "
        f"v03={summary.v03_suppressed_count}",
        "",
        "REMEASURE TRIGGERS (raw → calibrated):",
        "-" * 60,
    ]
    for r in summary.remeasure_results:
        arrow = "→" if not r.suppressed else "🔥→✗"
        marker = "🔥" if r.calibrated_fired else "· "
        lines.append(f"  {marker} {r.name}")
        lines.append(f"      raw: fired={r.raw_fired}  {r.raw_reason}")
        lines.append(f"      cal: fired={r.calibrated_fired}  {r.calibrated_reason}")
        lines.append(f"      {arrow}  suppressed={r.suppressed}")
        lines.append("")
    lines.append("V0.3 EVOLUTION TRIGGERS (raw → calibrated):")
    lines.append("-" * 60)
    for r in summary.v03_results:
        marker = "🔥" if r.calibrated_fired else "· "
        lines.append(f"  {marker} {r.name}")
        lines.append(f"      raw: fired={r.raw_fired}  {r.raw_reason}")
        lines.append(f"      cal: fired={r.calibrated_fired}  {r.calibrated_reason}")
        lines.append(f"      suppressed={r.suppressed}")
        lines.append("")
    lines.append("V3 哲学守门 (V1370): " + ", ".join(V1370_GUARDS))
    lines.append("V3 哲学守门 (V1368): " + ", ".join(V1368_GUARDS))
    return "\n".join(lines)


def render_compare(summary: CalibrationSummary) -> str:
    """Compact side-by-side table."""
    lines = [
        "V1368 vs V1370 — Side-by-side",
        "=" * 60,
        f"{'TRIGGER':<28}  {'RAW':<6}  {'CAL':<6}  {'SUPPRESSED':<10}",
        "-" * 60,
    ]
    for r in summary.remeasure_results + summary.v03_results:
        raw = "FIRE" if r.raw_fired else "—"
        cal = "FIRE" if r.calibrated_fired else "—"
        sup = "yes" if r.suppressed else "no"
        lines.append(f"{r.name:<28}  {raw:<6}  {cal:<6}  {sup:<10}")
    lines.append("-" * 60)
    lines.append(f"summary: remeasure raw={summary.raw_remeasure_fired} "
                 f"cal={summary.calibrated_remeasure_fired} "
                 f"suppressed={summary.remeasure_suppressed_count}")
    lines.append(f"summary: v03        raw={summary.raw_v03_fired} "
                 f"cal={summary.calibrated_v03_fired} "
                 f"suppressed={summary.v03_suppressed_count}")
    return "\n".join(lines)


def render_summary(summary: CalibrationSummary) -> str:
    lines = [
        "V1370 Calibration Summary",
        "=" * 60,
        f"ledger entries:       {summary.ledger_entries}",
        f"V1368 remeasure fire: {summary.raw_remeasure_fired}",
        f"V1370 remeasure fire: {summary.calibrated_remeasure_fired}",
        f"V1368 V0.3 fire:      {summary.raw_v03_fired}",
        f"V1370 V0.3 fire:      {summary.calibrated_v03_fired}",
        f"triggers suppressed:  "
        f"remeasure={summary.remeasure_suppressed_count} "
        f"v03={summary.v03_suppressed_count}",
        "",
    ]
    if summary.remeasure_suppressed_count + summary.v03_suppressed_count > 0:
        lines.append("Calibration is doing work: false positives suppressed.")
    else:
        lines.append("No false positives suppressed this evaluation.")
        lines.append("(All raw-fired triggers passed V1370 calibration.)")
    return "\n".join(lines)


# -----------------------------------------------------------------------------
# Popper self-tests
# -----------------------------------------------------------------------------

def _write_synth_ledger(name: str, entries: List[Dict[str, Any]]) -> Path:
    """Write a synthetic ledger to a temp file. Returns the Path."""
    path = Path(os.environ.get("TEMP", "/tmp")) / f"v1370_{name}.jsonl"
    with path.open("w", encoding="utf-8") as f:
        for e in entries:
            f.write(json.dumps(e) + "\n")
    return path


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

    # Constants & guards
    check("V1370_VERSION is semver", V1370_VERSION.count(".") == 2)
    check("plateau epsilon is positive and small",
          0 < PLATEAU_DELTA_EPSILON < 1e-2)
    check("new surface lookback is positive",
          NEW_SURFACE_LOOKBACK > 0)
    check("cap-sat min distinct tags >= 1",
          CAP_SAT_MIN_DISTINCT_TAGS >= 1)
    check("V1370_GUARDS count >= 6", len(V1370_GUARDS) >= 6)
    check("GUARD_NO_SOURCE_MUTATION present",
          "GUARD_NO_SOURCE_MUTATION" in V1370_GUARDS)
    check("GUARD_PLATEAU_REQUIRES_ZERO_DELTA present",
          "GUARD_PLATEAU_REQUIRES_ZERO_DELTA" in V1370_GUARDS)
    check("GUARD_NEW_SURFACE_REQUIRES_DELTA present",
          "GUARD_NEW_SURFACE_REQUIRES_DELTA" in V1370_GUARDS)
    check("GUARD_DISHONEST_CAP_REQUIRES_DIVERSITY present",
          "GUARD_DISHONEST_CAP_REQUIRES_DIVERSITY" in V1370_GUARDS)

    # ---- PLATEAU: positive case (delta ≈ 0 fires) ----
    plateau_zero_ledger = _write_synth_ledger("plateau_zero", [
        {"pole_star_total": 0.7905, "v01_baseline": 0.7905, "tag": "p1"},
        {"pole_star_total": 0.7905, "v01_baseline": 0.7905, "tag": "p2"},
        {"pole_star_total": 0.7905, "v01_baseline": 0.7905, "tag": "p3"},
    ])
    raw_re, cal_re, results = calibrate_remeasure(plateau_zero_ledger)
    plateau_zero_results = [r for r in results if r.name == "LEDGER_PLATEAU_SIGNAL"]
    check("plateau_zero: V1368 raw fires", any(r.raw_fired for r in plateau_zero_results))
    check("plateau_zero: V1370 calibrated fires",
          any(r.calibrated_fired for r in plateau_zero_results))
    check("plateau_zero: not suppressed",
          not any(r.suppressed for r in plateau_zero_results))

    # ---- PLATEAU: negative case (equal delta but > 0 does NOT fire) ----
    plateau_steady_ledger = _write_synth_ledger("plateau_steady", [
        {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "s1"},
        {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "s2"},
        {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "s3"},
    ])
    _, _, results = calibrate_remeasure(plateau_steady_ledger)
    plateau_steady_results = [r for r in results if r.name == "LEDGER_PLATEAU_SIGNAL"]
    check("plateau_steady: V1368 raw fires (false positive)",
          any(r.raw_fired for r in plateau_steady_results))
    check("plateau_steady: V1370 calibrated does NOT fire (calibrated)",
          not any(r.calibrated_fired for r in plateau_steady_results))
    check("plateau_steady: suppressed=True (V1370 fixes V1368)",
          any(r.suppressed for r in plateau_steady_results))

    # ---- NEW_SURFACE: positive case (truly new surface fires) ----
    new_surface_ledger = _write_synth_ledger("new_surface", [
        {"pole_star_total": 0.80, "v01_baseline": 0.7905, "tag": f"entry-{i}"}
        for i in range(15)
    ] + [
        {"pole_star_total": 0.85, "v01_baseline": 0.7905, "tag": "v1361-fresh"},
    ])
    _, cal_re, results = calibrate_remeasure(new_surface_ledger)
    new_surface_results = [r for r in results if r.name == "NEW_SURFACE_SHIPPED"]
    check("new_surface: V1368 raw fires", any(r.raw_fired for r in new_surface_results))
    check("new_surface: V1370 calibrated fires (truly new)",
          any(r.calibrated_fired for r in new_surface_results))

    # ---- NEW_SURFACE: negative case (same surface repeated does NOT fire) ----
    repeat_surface_ledger = _write_synth_ledger("repeat_surface", [
        {"pole_star_total": 0.85, "v01_baseline": 0.7905, "tag": "v1361-dashboard"}
        for _ in range(5)
    ] + [
        {"pole_star_total": 0.85, "v01_baseline": 0.7905, "tag": "v1361-dashboard-2"},
    ])
    _, _, results = calibrate_remeasure(repeat_surface_ledger)
    repeat_results = [r for r in results if r.name == "NEW_SURFACE_SHIPPED"]
    check("repeat_surface: V1368 raw fires (false positive)",
          any(r.raw_fired for r in repeat_results))
    check("repeat_surface: V1370 calibrated does NOT fire",
          not any(r.calibrated_fired for r in repeat_results))
    check("repeat_surface: suppressed=True",
          any(r.suppressed for r in repeat_results))

    # ---- CAP_SATURATION: positive case (3 distinct tags at cap fires) ----
    cap_distinct_ledger = _write_synth_ledger("cap_distinct", [
        {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "tag-a"},
        {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "tag-b"},
        {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "tag-c"},
    ])
    _, _, results = calibrate_v03(cap_distinct_ledger)
    cap_results = [r for r in results if r.name == "LEDGER_CAP_SATURATION_3"]
    check("cap_distinct: V1368 raw fires", any(r.raw_fired for r in cap_results))
    check("cap_distinct: V1370 calibrated fires (diverse tags)",
          any(r.calibrated_fired for r in cap_results))

    # ---- CAP_SATURATION: negative case (3 same-tag entries does NOT fire) ----
    cap_same_ledger = _write_synth_ledger("cap_same", [
        {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "self-test"},
        {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "self-test"},
        {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "self-test"},
    ])
    _, _, results = calibrate_v03(cap_same_ledger)
    cap_same_results = [r for r in results if r.name == "LEDGER_CAP_SATURATION_3"]
    check("cap_same: V1368 raw fires (false positive)",
          any(r.raw_fired for r in cap_same_results))
    check("cap_same: V1370 calibrated does NOT fire",
          not any(r.calibrated_fired for r in cap_same_results))
    check("cap_same: suppressed=True",
          any(r.suppressed for r in cap_same_results))

    # ---- Empty ledger ----
    empty_path = Path(os.environ.get("TEMP", "/tmp")) / "v1370_empty.jsonl"
    empty_path.write_text("", encoding="utf-8")
    raw_re, cal_re, results = calibrate_remeasure(empty_path)
    check("empty ledger: V1368 + V1370 both raw=False",
          raw_re is False or all(not r.raw_fired for r in results))
    check("empty ledger: V1370 calibrated=False",
          cal_re is False)
    empty_path.unlink(missing_ok=True)

    # ---- Reporting ----
    summary = evaluate()
    rep = render_evaluate(summary)
    check("render_evaluate non-empty", len(rep) > 50)
    check("render_evaluate mentions V1368 RAW",
          "V1368 RAW" in rep)
    check("render_evaluate mentions V1370 CAL",
          "V1370 CAL" in rep)
    rep_c = render_compare(summary)
    check("render_compare non-empty", len(rep_c) > 50)
    check("render_compare has side-by-side header",
          "V1368 vs V1370" in rep_c)
    rep_s = render_summary(summary)
    check("render_summary non-empty", len(rep_s) > 30)

    # ---- CLI smoke (call underlying logic, not recursive CLI handlers) ----
    from argparse import Namespace
    rc_eval = _cli_evaluate(Namespace(json=False))
    check("cli evaluate returns 0/1/2", rc_eval in (0, 1, 2))
    rc_comp = _cli_compare(Namespace())
    check("cli compare returns 0", rc_comp == 0)
    rc_summ = _cli_summary(Namespace())
    check("cli summary returns 0", rc_summ == 0)
    rc_ver = _cli_version(Namespace())
    check("cli version returns 0", rc_ver == 0)

    # Cleanup
    for p in [plateau_zero_ledger, plateau_steady_ledger,
              new_surface_ledger, repeat_surface_ledger,
              cap_distinct_ledger, cap_same_ledger]:
        try:
            p.unlink()
        except OSError:
            pass

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _cli_evaluate(args: argparse.Namespace) -> int:
    summary = evaluate()
    if args.json:
        print(json.dumps(summary.to_dict(), indent=2, ensure_ascii=False))
    else:
        print(render_evaluate(summary))
    if summary.calibrated_remeasure_fired and summary.calibrated_v03_fired:
        return 2
    if summary.calibrated_remeasure_fired or summary.calibrated_v03_fired:
        return 1
    return 0


def _cli_compare(args: argparse.Namespace) -> int:
    summary = evaluate()
    print(render_compare(summary))
    return 0


def _cli_summary(args: argparse.Namespace) -> int:
    summary = evaluate()
    print(render_summary(summary))
    return 0


def _cli_self_test(args: argparse.Namespace) -> int:
    passed, total, failures = _popper_self_tests(verbose=args.verbose)
    print(f"V1370 self-test: {passed}/{total} passed")
    if failures:
        print("FAILURES:")
        for f in failures:
            print(f"  - {f}")
    return 0 if passed == total else 2


def _cli_version(args: argparse.Namespace) -> int:
    print(f"v1370-v1368-trigger-calibration {V1370_VERSION}")
    print(f"  v1368 baseline: {V1368_TRIGGERS_VERSION}")
    print(f"  constants: PLATEAU_DELTA_EPSILON={PLATEAU_DELTA_EPSILON}")
    print(f"             NEW_SURFACE_LOOKBACK={NEW_SURFACE_LOOKBACK}")
    print(f"             CAP_SAT_MIN_DISTINCT_TAGS={CAP_SAT_MIN_DISTINCT_TAGS}")
    return 0


def _build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1370-v1368-trigger-calibration",
        description="V1370 V1368 trigger calibration layer",
    )
    sub = p.add_subparsers(dest="command", required=True)

    p_e = sub.add_parser("evaluate",
                         help="Run V1368 + V1370 side-by-side; show calibration")
    p_e.add_argument("--json", action="store_true")
    p_e.set_defaults(func=_cli_evaluate)

    sub.add_parser("compare",
                   help="Compact side-by-side table").set_defaults(func=_cli_compare)
    sub.add_parser("summary",
                   help="Calibration summary").set_defaults(func=_cli_summary)

    p_st = sub.add_parser("self-test", help="Popper self-tests")
    p_st.add_argument("--verbose", action="store_true")
    p_st.set_defaults(func=_cli_self_test)

    sub.add_parser("version", help="print version").set_defaults(func=_cli_version)
    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    return int(args.func(args))


if __name__ == "__main__":
    sys.exit(main())
