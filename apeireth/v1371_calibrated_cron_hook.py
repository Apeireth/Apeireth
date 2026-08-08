"""Phase 1371 v1371_calibrated_cron_hook — V1370 calibrated cron hook.

## What V1371 is

V1369 wires V1368 into the cron tick and writes a raw sidecar
(`v1368_evaluations.jsonl`). V1370 is the calibration layer that suppresses
the 3 false-positive surfaces V1368 ships with. V1371 is the **integration
layer** that wires V1370 into the cron tick and writes a calibrated sidecar
(`v1370_calibrated_cron_evaluations.jsonl`).

So the cron pipeline becomes:

    cron tick (5 min)
      → V1369 evaluate_now()            writes raw V1368 sidecar
      → V1370 evaluate()                reads ledger, applies calibration
      → V1371 evaluate_now_calibrated()  writes combined sidecar
      → exit code reflects CALIBRATED fire (not raw)

This means cron lanes only react to honest signal. V1368's known false
positives no longer cause cron spam.

## Why V1371

V1370 fixed the trigger logic; V1371 makes the fix *visible* to the cron
lane. Without V1371, the raw sidecar still says "fire", and downstream
lanes (or future agents waking to the sidecar) would re-derive the same
false alarm. V1371 records the calibrated state as a first-class field.

## What V1371 does NOT do

  - Does NOT modify V1368 / V1369 / V1370 source (GUARD_NO_SOURCE_MUTATION).
  - Does NOT touch the ledger (V1362's append-only invariant holds).
  - Does NOT raise the cap (V1356 source unchanged).
  - Does NOT auto-re-measure: it records calibrated state; the cron lane
    decides whether to act on the calibrated fire.
  - Does NOT aspire to V0.3: it just records honest, calibrated state.

## Sidecar schema (v1371_evaluation_v1)

{
  "schema": "v1371_evaluation_v1",
  "evaluated_at": "2026-08-09T03:30:00Z",
  "v1371_version": "0.1.0",
  "v1370_version": "0.1.0",
  "v1369_version": "0.1.0",
  "v1368_version": "0.1.0",
  "ledger_path": "...",
  "ledger_exists": true,
  "ledger_entries": 175,
  "raw": {
    "remeasure_fired": true,
    "v03_fired": true,
    "sidecar_path": "v1368_evaluations.jsonl",
    "results": [{ name, kind, fired, reason, evidence }, ...]
  },
  "calibrated": {
    "remeasure_fired": false,   # calibrator suppressed 2 FP
    "v03_fired": false,
    "results": [{
      name, kind,
      raw_fired, calibrated_fired,
      raw_reason, calibrated_reason,
      suppressed,            # raw=True && calibrated=False
    }, ...]
  },
  "summary": {
    "raw_remeasure_fired": true,
    "raw_v03_fired": true,
    "calibrated_remeasure_fired": false,
    "calibrated_v03_fired": false,
    "suppressed_remeasure_count": 1,
    "suppressed_v03_count": 1,
    "raw_fired_names": [...],
    "calibrated_fired_names": [],
  },
  "guards": [V1371_GUARDS + V1370_GUARDS + V1369_GUARDS + V1368_GUARDS]
}

## CLI

  - `python -m apeireth.v1371_calibrated_cron_hook evaluate`
        → run V1369 + V1370 + V1371; write both sidecars; exit 0/1/2/3
  - `python -m apeireth.v1371_calibrated_cron_hook evaluate --json`
        → JSON output instead of human-readable
  - `python -m apeireth.v1371_calibrated_cron_hook show-last`
        → print most recent V1371 sidecar entry
  - `python -m apeireth.v1371_calibrated_cron_hook show-last N`
        → print last N V1371 sidecar entries
  - `python -m apeireth.v1371_calibrated_cron_hook summary`
        → aggregate stats (raw vs calibrated fire rates)
  - `python -m apeireth.v1371_calibrated_cron_hook compare`
        → per-trigger raw vs calibrated fire-rate side-by-side
  - `python -m apeireth.v1371_calibrated_cron_hook self-test [--verbose]`
        → Popper self-tests

## Exit codes

  0  evaluated; calibrated no fire
  1  evaluated; calibrated remeasure trigger fired
  2  evaluated; calibrated V0.3 trigger fired
  3  fatal: cannot write sidecar
  4  invalid usage

## V3 哲学守门 (主 17:58 + 20:46 + 17:43 + 22:33 + 00:56)

  - GUARD_NO_SOURCE_MUTATION    : V1371 does NOT modify V1368/V1369/V1370
  - GUARD_SIDECAR_NOT_LEDGER    : V1371 sidecar is *separate* from ledger
  - GUARD_CALIBRATED_IS_HONEST  : exit code reflects calibrated, not raw
  - GUARD_RAW_SIDECAR_STILL_WRITTEN : V1369 sidecar continues to receive
                                       raw entries (back-compat)
  - GUARD_EVALUATION_IS_HONEST  : records actual state, no aspiration
  - GUARD_NO_AUTO_REMEASURE     : V1371 records; never invokes V1356
  - GUARD_CAP_NOT_AUTO_RAISED   : V1371 never touches cap
  - GUARD_READ_ONLY_LEDGER      : V1371 only reads ledger
  - GUARD_BOTH_SIDECARS_RECORDED: raw sidecar (V1369) + calibrated (V1371)
  - GUARD_HONEST_PLATEAU        : plateau is signal, not failure
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

# Import V1368 (read-only, V1371 does NOT mutate it)
from apeireth.v1368_pole_star_v03_triggers import (
    DEFAULT_LEDGER_PATH,
    V1368_GUARDS,
    V1368_VERSION as V1368_TRIGGERS_VERSION,
)

# Import V1369 (read-only, V1371 does NOT mutate it)
from apeireth.v1369_v1368_cron_hook import (
    DEFAULT_SIDECAR_PATH as DEFAULT_RAW_SIDECAR_PATH,
    V1369_VERSION,
    V1369_GUARDS,
    evaluate_now as v1369_evaluate_now,
)

# Import V1370 (read-only, V1371 does NOT mutate it)
from apeireth.v1370_v1368_trigger_calibration import (
    PLATEAU_DELTA_EPSILON,
    NEW_SURFACE_LOOKBACK,
    CAP_SAT_MIN_DISTINCT_TAGS,
    V1370_VERSION,
    V1370_GUARDS,
    evaluate as v1370_evaluate,
    CalibrationSummary,
    CalibratedResult,
)


# -----------------------------------------------------------------------------
# Version + constants
# -----------------------------------------------------------------------------

V1371_VERSION = "0.1.0"

# Sidecar path: SIBLING of pole_star_history.jsonl + SIBLING of raw V1369 sidecar.
# NOT a child of ledger dir. Distinct from V1369's sidecar (raw).
DEFAULT_CALIBRATED_SIDECAR_PATH = (
    DEFAULT_LEDGER_PATH.parent / "v1370_calibrated_cron_evaluations.jsonl"
)

# V3 哲学守门 (V1371-specific)
V1371_GUARDS: Tuple[str, ...] = (
    "GUARD_NO_SOURCE_MUTATION",
    "GUARD_SIDECAR_NOT_LEDGER",
    "GUARD_CALIBRATED_IS_HONEST",
    "GUARD_RAW_SIDECAR_STILL_WRITTEN",
    "GUARD_EVALUATION_IS_HONEST",
    "GUARD_NO_AUTO_REMEASURE",
    "GUARD_CAP_NOT_AUTO_RAISED",
    "GUARD_READ_ONLY_LEDGER",
    "GUARD_BOTH_SIDECARS_RECORDED",
    "GUARD_HONEST_PLATEAU",
)

# Exit code per outcome (calibrated state)
EXIT_NO_FIRE = 0
EXIT_REMEASURE_FIRED = 1
EXIT_V03_FIRED = 2
EXIT_FATAL_WRITE = 3


# -----------------------------------------------------------------------------
# Sidecar I/O
# -----------------------------------------------------------------------------

def _now_iso() -> str:
    """UTC ISO timestamp for evaluation entry."""
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _read_sidecar(path: Optional[Path] = None) -> List[Dict[str, Any]]:
    path = path or DEFAULT_CALIBRATED_SIDECAR_PATH
    if not path.exists():
        return []
    out: List[Dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            out.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return out


def _run_v1369_evaluate(
    ledger_path: Path,
    raw_sidecar_path: Path,
    evaluated_at: str,
) -> Dict[str, Any]:
    """Call V1369's evaluate_now; returns the sidecar entry (also written)."""
    return v1369_evaluate_now(
        ledger_path=ledger_path,
        sidecar_path=raw_sidecar_path,
        evaluate_at=evaluated_at,
    )


def _run_v1370_evaluate(ledger_path: Path) -> CalibrationSummary:
    """Call V1370's evaluate; returns the calibration summary."""
    return v1370_evaluate(ledger_path)


# -----------------------------------------------------------------------------
# Main pipeline entrypoint
# -----------------------------------------------------------------------------

def evaluate_now_calibrated(
    ledger_path: Optional[Path] = None,
    raw_sidecar_path: Optional[Path] = None,
    calibrated_sidecar_path: Optional[Path] = None,
    evaluated_at: Optional[str] = None,
) -> Dict[str, Any]:
    """Run V1369 + V1370; write raw sidecar (V1369) + calibrated sidecar (V1371).

    Args:
        ledger_path: ledger to read (default: DEFAULT_LEDGER_PATH).
        raw_sidecar_path: where V1369 writes raw sidecar (default: sibling).
        calibrated_sidecar_path: where V1371 writes calibrated sidecar (default: sibling).
        evaluated_at: ISO timestamp for the entry (default: now UTC).

    Returns:
        Dict with schema v1371_evaluation_v1, containing both raw and
        calibrated trigger states.
    """
    ledger_path = ledger_path or DEFAULT_LEDGER_PATH
    raw_sidecar_path = raw_sidecar_path or DEFAULT_RAW_SIDECAR_PATH
    calibrated_sidecar_path = (
        calibrated_sidecar_path or DEFAULT_CALIBRATED_SIDECAR_PATH
    )
    evaluated_at = evaluated_at or _now_iso()

    # Step 1: V1369 writes raw sidecar
    raw_entry = _run_v1369_evaluate(
        ledger_path=ledger_path,
        raw_sidecar_path=raw_sidecar_path,
        evaluated_at=evaluated_at,
    )

    # Step 2: V1370 evaluates calibrated state
    cal_summary = _run_v1370_evaluate(ledger_path)

    # Step 3: Combine into V1371 entry
    entry = _build_entry(
        evaluated_at=evaluated_at,
        ledger_path=ledger_path,
        raw_entry=raw_entry,
        cal_summary=cal_summary,
        raw_sidecar_path=raw_sidecar_path,
        calibrated_sidecar_path=calibrated_sidecar_path,
    )

    # Step 4: Write V1371 calibrated sidecar (append-only JSONL)
    calibrated_sidecar_path.parent.mkdir(parents=True, exist_ok=True)
    with calibrated_sidecar_path.open("a", encoding="utf-8") as f:
        f.write(json.dumps(entry, ensure_ascii=False) + "\n")

    return entry


def _build_entry(
    *,
    evaluated_at: str,
    ledger_path: Path,
    raw_entry: Dict[str, Any],
    cal_summary: CalibrationSummary,
    raw_sidecar_path: Path,
    calibrated_sidecar_path: Path,
) -> Dict[str, Any]:
    """Combine V1369 raw + V1370 calibrated into a single V1371 entry."""
    raw_results = raw_entry["remeasure"]["results"] + raw_entry["v03_evolution"]["results"]
    raw_fired_names = [
        r["name"] for r in raw_results if r["fired"]
    ]

    cal_results = cal_summary.remeasure_results + cal_summary.v03_results
    cal_fired_names = [
        r.name for r in cal_results if r.calibrated_fired
    ]

    # Serialize calibrated results
    cal_ser: List[Dict[str, Any]] = []
    for r in cal_results:
        cal_ser.append({
            "name": r.name,
            "kind": r.kind,
            "raw_fired": r.raw_fired,
            "calibrated_fired": r.calibrated_fired,
            "raw_reason": r.raw_reason,
            "calibrated_reason": r.calibrated_reason,
            "suppressed": r.suppressed,
        })

    return {
        "schema": "v1371_evaluation_v1",
        "evaluated_at": evaluated_at,
        "v1371_version": V1371_VERSION,
        "v1370_version": V1370_VERSION,
        "v1369_version": V1369_VERSION,
        "v1368_version": V1368_TRIGGERS_VERSION,
        "ledger_path": str(ledger_path),
        "ledger_exists": ledger_path.exists(),
        "ledger_entries": len(_read_ledger_safely(ledger_path)),
        "raw": {
            "remeasure_fired": raw_entry["remeasure"]["fired"],
            "v03_fired": raw_entry["v03_evolution"]["fired"],
            "sidecar_path": str(raw_sidecar_path),
            "results": raw_results,
        },
        "calibrated": {
            "remeasure_fired": cal_summary.calibrated_remeasure_fired,
            "v03_fired": cal_summary.calibrated_v03_fired,
            "results": cal_ser,
        },
        "summary": {
            "raw_remeasure_fired": raw_entry["remeasure"]["fired"],
            "raw_v03_fired": raw_entry["v03_evolution"]["fired"],
            "calibrated_remeasure_fired": cal_summary.calibrated_remeasure_fired,
            "calibrated_v03_fired": cal_summary.calibrated_v03_fired,
            "suppressed_remeasure_count": cal_summary.remeasure_suppressed_count,
            "suppressed_v03_count": cal_summary.v03_suppressed_count,
            "raw_fired_names": raw_fired_names,
            "calibrated_fired_names": cal_fired_names,
        },
        "guards": list(V1371_GUARDS) + list(V1370_GUARDS) + list(V1369_GUARDS) + list(V1368_GUARDS),
    }


def _read_ledger_safely(ledger_path: Path) -> List[Dict[str, Any]]:
    """Read ledger entries; returns empty list on missing/corrupt file."""
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


# -----------------------------------------------------------------------------
# Reporting
# -----------------------------------------------------------------------------

def render_evaluation_human(entry: Dict[str, Any]) -> str:
    """Render a single V1371 evaluation as human-readable text."""
    lines = [
        f"V1371 Calibrated Evaluation @ {entry['evaluated_at']}",
        "=" * 60,
        f"ledger: {entry['ledger_path']}  (entries: {entry['ledger_entries']})",
        f"versions: v1371={entry['v1371_version']} v1370={entry['v1370_version']} "
        f"v1369={entry['v1369_version']} v1368={entry['v1368_version']}",
        "",
        "RAW (V1368/V1369, uncalibrated):",
        f"  remeasure_fired: {entry['raw']['remeasure_fired']}",
        f"  v03_fired:       {entry['raw']['v03_fired']}",
        f"  sidecar:         {entry['raw']['sidecar_path']}",
        "",
        "CALIBRATED (V1370, post-correction):",
        f"  remeasure_fired: {entry['calibrated']['remeasure_fired']}",
        f"  v03_fired:       {entry['calibrated']['v03_fired']}",
        "",
        "PER-TRIGGER (raw → calibrated):",
        "-" * 60,
    ]
    for r in entry["calibrated"]["results"]:
        marker = "?? FIRE" if r["calibrated_fired"] else "·  no  "
        arrow = "→" if not r["suppressed"] else "??→?"
        lines.append(f"  {marker}  {r['name']:<28}  {arrow}")
        lines.append(f"      raw:    fired={r['raw_fired']}  {r['raw_reason']}")
        lines.append(f"      cal:    fired={r['calibrated_fired']}  {r['calibrated_reason']}")
        lines.append(f"      suppressed: {r['suppressed']}")
        lines.append("")
    s = entry["summary"]
    lines.append("SUMMARY:")
    lines.append(f"  raw fired:        {sorted(s['raw_fired_names']) or '—'}")
    lines.append(f"  calibrated fired: {sorted(s['calibrated_fired_names']) or '—'}")
    lines.append(f"  suppressed:       remeasure={s['suppressed_remeasure_count']} "
                 f"v03={s['suppressed_v03_count']}")
    if not s["calibrated_fired_names"]:
        lines.append("")
        lines.append("  (no calibrated fire — plateau is honest, not failure)")
    return "\n".join(lines)


def render_summary(sidecar_path: Optional[Path] = None) -> str:
    """Aggregate stats over the V1371 sidecar."""
    sidecar_path = sidecar_path or DEFAULT_CALIBRATED_SIDECAR_PATH
    entries = _read_sidecar(sidecar_path)
    n = len(entries)
    if n == 0:
        return f"V1371 calibrated sidecar at {sidecar_path}: empty"
    n_raw_re = sum(1 for e in entries if e["summary"]["raw_remeasure_fired"])
    n_raw_v3 = sum(1 for e in entries if e["summary"]["raw_v03_fired"])
    n_cal_re = sum(1 for e in entries if e["summary"]["calibrated_remeasure_fired"])
    n_cal_v3 = sum(1 for e in entries if e["summary"]["calibrated_v03_fired"])
    n_supp_re = sum(1 for e in entries if e["summary"]["suppressed_remeasure_count"] > 0)
    n_supp_v3 = sum(1 for e in entries if e["summary"]["suppressed_v03_count"] > 0)
    raw_name_counter: Counter = Counter()
    cal_name_counter: Counter = Counter()
    for e in entries:
        for name in e["summary"]["raw_fired_names"]:
            raw_name_counter[name] += 1
        for name in e["summary"]["calibrated_fired_names"]:
            cal_name_counter[name] += 1
    lines = [
        f"V1371 calibrated sidecar at {sidecar_path}",
        "=" * 60,
        f"total evaluations:        {n}",
        f"raw remeasure fires:      {n_raw_re} ({n_raw_re / n * 100:.1f}%)",
        f"raw V0.3 fires:           {n_raw_v3} ({n_raw_v3 / n * 100:.1f}%)",
        f"cal remeasure fires:      {n_cal_re} ({n_cal_re / n * 100:.1f}%)",
        f"cal V0.3 fires:           {n_cal_v3} ({n_cal_v3 / n * 100:.1f}%)",
        f"eval with remeasure FP:   {n_supp_re} ({n_supp_re / n * 100:.1f}%)",
        f"eval with V0.3 FP:        {n_supp_v3} ({n_supp_v3 / n * 100:.1f}%)",
        "",
        "raw fires by trigger:",
    ]
    if raw_name_counter:
        for name, count in sorted(raw_name_counter.items(), key=lambda kv: -kv[1]):
            lines.append(f"  {count:4d}  {name}")
    else:
        lines.append("  (none)")
    lines.append("")
    lines.append("calibrated fires by trigger:")
    if cal_name_counter:
        for name, count in sorted(cal_name_counter.items(), key=lambda kv: -kv[1]):
            lines.append(f"  {count:4d}  {name}")
    else:
        lines.append("  (none — V1370 calibration is doing real work)")
    if entries:
        first_ts = entries[0]["evaluated_at"]
        last_ts = entries[-1]["evaluated_at"]
        lines.append("")
        lines.append(f"first evaluation: {first_ts}")
        lines.append(f"last  evaluation: {last_ts}")
    return "\n".join(lines)


def render_compare(sidecar_path: Optional[Path] = None) -> str:
    """Per-trigger raw vs calibrated fire-rate side-by-side."""
    sidecar_path = sidecar_path or DEFAULT_CALIBRATED_SIDECAR_PATH
    entries = _read_sidecar(sidecar_path)
    n = len(entries)
    if n == 0:
        return f"V1371 sidecar at {sidecar_path}: empty (no comparison)"
    raw_by_name: Counter = Counter()
    cal_by_name: Counter = Counter()
    for e in entries:
        for r in e["calibrated"]["results"]:
            if r["raw_fired"]:
                raw_by_name[r["name"]] += 1
            if r["calibrated_fired"]:
                cal_by_name[r["name"]] += 1
    all_names = sorted(set(raw_by_name) | set(cal_by_name))
    lines = [
        "V1371 — Per-trigger raw vs calibrated fire-rate",
        "=" * 60,
        f"{'TRIGGER':<28}  {'RAW':<8}  {'CAL':<8}  {'SUPPRESSED':<10}",
        "-" * 60,
    ]
    for name in all_names:
        raw_n = raw_by_name.get(name, 0)
        cal_n = cal_by_name.get(name, 0)
        sup_pct = ((raw_n - cal_n) / n * 100) if raw_n > cal_n else 0.0
        lines.append(
            f"{name:<28}  {raw_n:>4d}/{n:<3d}  {cal_n:>4d}/{n:<3d}  "
            f"{sup_pct:>5.1f}%"
        )
    lines.append("-" * 60)
    lines.append(f"raw    remeasure fires: {sum(1 for e in entries if e['summary']['raw_remeasure_fired'])}/{n}")
    lines.append(f"cal    remeasure fires: {sum(1 for e in entries if e['summary']['calibrated_remeasure_fired'])}/{n}")
    lines.append(f"raw    V0.3 fires:      {sum(1 for e in entries if e['summary']['raw_v03_fired'])}/{n}")
    lines.append(f"cal    V0.3 fires:      {sum(1 for e in entries if e['summary']['calibrated_v03_fired'])}/{n}")
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
                print(f"  ? {name}")
        else:
            failures.append(name)
            if verbose:
                print(f"  ? {name}")

    # Constants & guards
    check("V1371_VERSION is semver", V1371_VERSION.count(".") == 2)
    check("V1371 guards count >= 10", len(V1371_GUARDS) >= 10)
    check("GUARD_NO_SOURCE_MUTATION present",
          "GUARD_NO_SOURCE_MUTATION" in V1371_GUARDS)
    check("GUARD_SIDECAR_NOT_LEDGER present",
          "GUARD_SIDECAR_NOT_LEDGER" in V1371_GUARDS)
    check("GUARD_CALIBRATED_IS_HONEST present",
          "GUARD_CALIBRATED_IS_HONEST" in V1371_GUARDS)
    check("GUARD_RAW_SIDECAR_STILL_WRITTEN present",
          "GUARD_RAW_SIDECAR_STILL_WRITTEN" in V1371_GUARDS)
    check("GUARD_BOTH_SIDECARS_RECORDED present",
          "GUARD_BOTH_SIDECARS_RECORDED" in V1371_GUARDS)
    check("V1370 guards propagated", len(V1370_GUARDS) >= 6)
    check("V1369 guards propagated", len(V1369_GUARDS) >= 5)
    check("V1368 guards propagated", len(V1368_GUARDS) >= 5)

    # Sidecar paths
    check("calibrated sidecar is NOT ledger",
          DEFAULT_CALIBRATED_SIDECAR_PATH != DEFAULT_LEDGER_PATH)
    check("calibrated sidecar is sibling of ledger",
          DEFAULT_CALIBRATED_SIDECAR_PATH.parent == DEFAULT_LEDGER_PATH.parent)
    check("calibrated sidecar is NOT raw V1369 sidecar",
          DEFAULT_CALIBRATED_SIDECAR_PATH != DEFAULT_RAW_SIDECAR_PATH)
    check("raw V1369 sidecar is sibling of ledger",
          DEFAULT_RAW_SIDECAR_PATH.parent == DEFAULT_LEDGER_PATH.parent)

    # Exit codes
    check("exit codes are distinct", len({EXIT_NO_FIRE, EXIT_REMEASURE_FIRED,
                                          EXIT_V03_FIRED, EXIT_FATAL_WRITE}) == 4)

    # Round-trip via tmp
    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        ledger = Path(tmp) / "ledger.jsonl"
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        ts = "2026-08-09T03:30:00Z"

        # Empty ledger → both raw and calibrated should not fire
        entry = evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at=ts,
        )
        check("entry schema is v1371_evaluation_v1",
              entry["schema"] == "v1371_evaluation_v1")
        check("entry evaluated_at matches", entry["evaluated_at"] == ts)
        check("entry ledger_exists is False", entry["ledger_exists"] is False)
        check("entry ledger_entries is 0", entry["ledger_entries"] == 0)
        check("entry raw_remeasure_fired is False",
              entry["summary"]["raw_remeasure_fired"] is False)
        check("entry calibrated_remeasure_fired is False",
              entry["summary"]["calibrated_remeasure_fired"] is False)
        check("raw sidecar was written", raw_sc.exists())
        check("calibrated sidecar was written", cal_sc.exists())
        check("raw sidecar has 1 entry",
              len([l for l in raw_sc.read_text(encoding="utf-8").splitlines() if l.strip()]) == 1)
        check("calibrated sidecar has 1 entry",
              len([l for l in cal_sc.read_text(encoding="utf-8").splitlines() if l.strip()]) == 1)

        # Steady-state ledger (delta=0.1095 pattern) → raw fires, calibrated suppresses
        with ledger.open("w", encoding="utf-8") as f:
            for i in range(3):
                f.write(json.dumps({
                    "pole_star_total": 0.9,
                    "v01_baseline": 0.7905,
                    "tag": f"v1362-self-test-{i}",
                }) + "\n")
        entry = evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T03:35:00Z",
        )
        check("steady-state: raw remeasure fires",
              entry["summary"]["raw_remeasure_fired"] is True)
        check("steady-state: calibrated remeasure does NOT fire (FP suppressed)",
              entry["summary"]["calibrated_remeasure_fired"] is False)
        check("steady-state: has suppressed_remeasure_count > 0",
              entry["summary"]["suppressed_remeasure_count"] > 0)
        # Per-trigger: LEDGER_PLATEAU_SIGNAL should be in raw_fired_names but not calibrated_fired_names
        check("steady-state: LEDGER_PLATEAU_SIGNAL in raw_fired",
              "LEDGER_PLATEAU_SIGNAL" in entry["summary"]["raw_fired_names"])
        check("steady-state: LEDGER_PLATEAU_SIGNAL NOT in calibrated_fired",
              "LEDGER_PLATEAU_SIGNAL" not in entry["summary"]["calibrated_fired_names"])

        # Honest cap-saturation ledger (3 distinct tags at cap) → V1370 fires
        with ledger.open("w", encoding="utf-8") as f:
            for tag in ["tag-a", "tag-b", "tag-c"]:
                f.write(json.dumps({
                    "pole_star_total": 0.90,
                    "pole_star_cap": 0.90,
                    "tag": tag,
                }) + "\n")
        entry = evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T03:40:00Z",
        )
        check("cap_distinct: calibrated V0.3 fires (honest cap-saturation)",
              entry["summary"]["calibrated_v03_fired"] is True)
        check("cap_distinct: LEDGER_CAP_SATURATION_3 in calibrated_fired",
              "LEDGER_CAP_SATURATION_3" in entry["summary"]["calibrated_fired_names"])

        # Reporting smoke
        rep = render_evaluation_human(entry)
        check("render_evaluation_human non-empty", len(rep) > 100)
        check("render_evaluation_human mentions 'CALIBRATED'",
              "CALIBRATED" in rep)
        rep_s = render_summary(cal_sc)
        check("render_summary non-empty", len(rep_s) > 50)
        check("render_summary mentions 'total evaluations'",
              "total evaluations" in rep_s)
        rep_c = render_compare(cal_sc)
        check("render_compare non-empty", len(rep_c) > 50)
        check("render_compare has side-by-side header",
              "raw vs calibrated" in rep_c)

        # CLI smoke
        from argparse import Namespace
        rc_eval = _cli_evaluate(Namespace(json=False))
        check("cli evaluate returns 0/1/2/3", rc_eval in (0, 1, 2, 3))
        rc_show = _cli_show_last(Namespace(n=1))
        check("cli show-last returns 0", rc_show == 0)
        rc_sum = _cli_summary(Namespace())
        check("cli summary returns 0", rc_sum == 0)
        rc_comp = _cli_compare(Namespace())
        check("cli compare returns 0", rc_comp == 0)

    # ---- Constants sanity vs V1370 (proves propagation, not copy) ----
    check("V1370 constants exposed (PLATEAU_DELTA_EPSILON)",
          PLATEAU_DELTA_EPSILON == 1e-4)
    check("V1370 constants exposed (NEW_SURFACE_LOOKBACK)",
          NEW_SURFACE_LOOKBACK == 10)
    check("V1370 constants exposed (CAP_SAT_MIN_DISTINCT_TAGS)",
          CAP_SAT_MIN_DISTINCT_TAGS == 2)

    # ---- Schema field check ----
    with tempfile.TemporaryDirectory() as tmp:
        ledger = Path(tmp) / "ledger.jsonl"
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        entry = evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T03:45:00Z",
        )
        required_top = {
            "schema", "evaluated_at", "v1371_version", "v1370_version",
            "v1369_version", "v1368_version", "ledger_path", "ledger_exists",
            "ledger_entries", "raw", "calibrated", "summary", "guards",
        }
        check("entry has all required top-level fields",
              required_top.issubset(entry.keys()))
        required_raw = {"remeasure_fired", "v03_fired", "sidecar_path", "results"}
        check("raw has all required fields",
              required_raw.issubset(entry["raw"].keys()))
        required_cal = {"remeasure_fired", "v03_fired", "results"}
        check("calibrated has all required fields",
              required_cal.issubset(entry["calibrated"].keys()))
        required_summary = {
            "raw_remeasure_fired", "raw_v03_fired",
            "calibrated_remeasure_fired", "calibrated_v03_fired",
            "suppressed_remeasure_count", "suppressed_v03_count",
            "raw_fired_names", "calibrated_fired_names",
        }
        check("summary has all required fields",
              required_summary.issubset(entry["summary"].keys()))

    return passed, total, failures


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def _cli_evaluate(args: argparse.Namespace) -> int:
    entry = evaluate_now_calibrated()
    if args.json:
        print(json.dumps(entry, ensure_ascii=False, indent=2))
    else:
        print(render_evaluation_human(entry))
    if entry["summary"]["calibrated_v03_fired"]:
        return EXIT_V03_FIRED
    if entry["summary"]["calibrated_remeasure_fired"]:
        return EXIT_REMEASURE_FIRED
    return EXIT_NO_FIRE


def _cli_show_last(args: argparse.Namespace) -> int:
    n = max(1, args.n)
    entries = _read_sidecar()
    if not entries:
        print(f"V1371 sidecar at {DEFAULT_CALIBRATED_SIDECAR_PATH}: empty")
        return 0
    last = entries[-n:]
    for e in last:
        print(render_evaluation_human(e))
        print()
    return 0


def _cli_summary(args: argparse.Namespace) -> int:
    print(render_summary())
    return 0


def _cli_compare(args: argparse.Namespace) -> int:
    print(render_compare())
    return 0


def _cli_self_test(args: argparse.Namespace) -> int:
    passed, total, failures = _popper_self_tests(verbose=args.verbose)
    print(f"V1371 self-test: {passed}/{total} passed")
    if failures:
        print("FAILED:")
        for f in failures:
            print(f"  - {f}")
        return 1
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="v1371_calibrated_cron_hook",
        description="V1371 calibrated cron hook (V1370 + V1369 integration)",
    )
    sub = p.add_subparsers(dest="command", required=True)

    p_eval = sub.add_parser("evaluate", help="run V1369 + V1370 + V1371 now")
    p_eval.add_argument("--json", action="store_true",
                        help="JSON output instead of human-readable")
    p_eval.set_defaults(func=_cli_evaluate)

    p_show = sub.add_parser("show-last", help="print most recent sidecar entries")
    p_show.add_argument("n", nargs="?", type=int, default=1,
                        help="number of entries (default: 1)")
    p_show.set_defaults(func=_cli_show_last)

    p_sum = sub.add_parser("summary", help="aggregate stats over sidecar")
    p_sum.set_defaults(func=_cli_summary)

    p_comp = sub.add_parser("compare",
                            help="per-trigger raw vs calibrated fire-rate")
    p_comp.set_defaults(func=_cli_compare)

    p_st = sub.add_parser("self-test", help="Popper self-tests")
    p_st.add_argument("--verbose", action="store_true")
    p_st.set_defaults(func=_cli_self_test)

    return p


def main(argv: Optional[List[str]] = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        return args.func(args)
    except Exception as e:
        print(f"fatal: {e}", file=sys.stderr)
        return EXIT_FATAL_WRITE


if __name__ == "__main__":
    sys.exit(main())
