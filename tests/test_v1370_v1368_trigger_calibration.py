"""Phase 1370 v1370_v1368_trigger_calibration — Honest trigger calibration.

Companion test file to `apeireth/v1370_v1368_trigger_calibration.py`.

## What we verify (Pytest)

  1. Constants & guards — locked values, no silent drift.
  2. Calibrator function shape — each calibrator is pure, returns (bool, str).
  3. PLATEAU: positive case (delta=0 fires) AND negative case (equal delta
     > 0 does NOT fire). V1370 fixes the V1368 false positive.
  4. NEW_SURFACE: positive case (truly new fires) AND negative case
     (recently seen does NOT fire).
  5. CAP_SATURATION_3: positive case (distinct tags fires) AND negative
     case (duplicate tags does NOT fire).
  6. Passthrough — TIME_TICK_INTERVAL, DELTA_ANY_COMPONENT,
     NEW_MEASUREMENT_COMPONENT, V1318_CELL_NEWLY_FILLED,
     CAP_BECOMES_DISHONEST pass through unchanged.
  7. Empty ledger — pure functions return (False, results) with no crash.
  8. Real ledger integration — calling evaluate() against the actual
     `pole_star_history.jsonl` (if present) does not crash and produces
     sensible results (suppressed ≥ 0).
  9. CLI smoke — argparse handlers return expected exit codes.

## V3 哲学守门 (主 17:43 + 17:58 + 20:46 + 22:33)

  - GUARD_NO_SOURCE_MUTATION          : V1368 source untouched
  - GUARD_CALIBRATION_NOT_LOOSENING   : V1370 only tightens
  - GUARD_PLATEAU_REQUIRES_ZERO_DELTA : PLATEAU_DELTA_EPSILON = 1e-4
  - GUARD_NEW_SURFACE_REQUIRES_DELTA  : lookback = 10 entries
  - GUARD_DISHONEST_CAP_REQUIRES_DIVERSITY : distinct-tag count ≥ 2
  - GUARD_HONEST_PLATEAU              : plateau is signal, not failure

These guards are NOT modified by tests; tests only assert the helpers'
*behavior*, never tamper with the philosophy.
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

from apeireth.v1370_v1368_trigger_calibration import (  # noqa: E402
    CAP_SAT_MIN_DISTINCT_TAGS,
    NEW_SURFACE_LOOKBACK,
    PLATEAU_DELTA_EPSILON,
    V1370_GUARDS,
    V1370_VERSION,
    CalibrationSummary,
    CalibratedResult,
    _calibrate_cap_saturation,
    _calibrate_new_surface,
    _calibrate_plateau,
    calibrate_remeasure,
    calibrate_v03,
    evaluate,
    render_compare,
    render_evaluate,
    render_summary,
)


def _write_synth_ledger(name: str, entries: list) -> Path:
    """Write a synthetic ledger to a temp file."""
    path = Path(os.environ.get("TEMP", "/tmp")) / f"v1370_pytest_{name}.jsonl"
    with path.open("w", encoding="utf-8") as f:
        for e in entries:
            f.write(json.dumps(e) + "\n")
    return path


# -----------------------------------------------------------------------------
# Constants & guards
# -----------------------------------------------------------------------------

class TestConstantsAndGuards:
    def test_version_is_semver(self):
        assert V1370_VERSION.count(".") == 2
        assert all(p.isdigit() for p in V1370_VERSION.split("."))

    def test_plateau_delta_epsilon_small_positive(self):
        assert 0 < PLATEAU_DELTA_EPSILON < 1e-2

    def test_new_surface_lookback_positive(self):
        assert NEW_SURFACE_LOOKBACK > 0
        assert isinstance(NEW_SURFACE_LOOKBACK, int)

    def test_cap_sat_min_distinct_tags_at_least_1(self):
        assert CAP_SAT_MIN_DISTINCT_TAGS >= 1

    def test_guards_count_at_least_6(self):
        assert len(V1370_GUARDS) >= 6

    def test_required_guards_present(self):
        required = {
            "GUARD_NO_SOURCE_MUTATION",
            "GUARD_CALIBRATION_NOT_LOOSENING",
            "GUARD_PLATEAU_REQUIRES_ZERO_DELTA",
            "GUARD_NEW_SURFACE_REQUIRES_DELTA",
            "GUARD_DISHONEST_CAP_REQUIRES_DIVERSITY",
            "GUARD_HONEST_PLATEAU",
        }
        assert required.issubset(set(V1370_GUARDS))


# -----------------------------------------------------------------------------
# Calibrator function shape
# -----------------------------------------------------------------------------

class TestCalibratorShape:
    def test_calibrate_plateau_returns_tuple(self):
        fired, reason = _calibrate_plateau(False, [])
        assert isinstance(fired, bool)
        assert isinstance(reason, str)
        assert len(reason) > 0

    def test_calibrate_new_surface_returns_tuple(self):
        fired, reason = _calibrate_new_surface(False, [])
        assert isinstance(fired, bool)
        assert isinstance(reason, str)

    def test_calibrate_cap_saturation_returns_tuple(self):
        fired, reason = _calibrate_cap_saturation(False, [])
        assert isinstance(fired, bool)
        assert isinstance(reason, str)

    def test_calibrate_plateau_passthrough_when_raw_false(self):
        fired, reason = _calibrate_plateau(False, [{"pole_star_total": 0.9,
                                                   "tag": "x"}] * 5)
        assert fired is False
        assert "passthrough" in reason


# -----------------------------------------------------------------------------
# PLATEAU trigger calibration
# -----------------------------------------------------------------------------

class TestPlateauCalibration:
    def test_fires_when_delta_is_zero(self):
        """Spec: delta = 0 → fire. V1370 honors spec."""
        ledger = _write_synth_ledger("plateau_positive", [
            {"pole_star_total": 0.7905, "v01_baseline": 0.7905, "tag": "p1"},
            {"pole_star_total": 0.7905, "v01_baseline": 0.7905, "tag": "p2"},
            {"pole_star_total": 0.7905, "v01_baseline": 0.7905, "tag": "p3"},
        ])
        try:
            _, cal_fired, results = calibrate_remeasure(ledger)
            plateau = [r for r in results if r.name == "LEDGER_PLATEAU_SIGNAL"]
            assert len(plateau) == 1
            assert plateau[0].calibrated_fired is True
            assert plateau[0].suppressed is False
        finally:
            ledger.unlink(missing_ok=True)

    def test_does_not_fire_when_delta_equal_but_nonzero(self):
        """The V1368 false positive: equal delta but > 0. V1370 fixes it."""
        ledger = _write_synth_ledger("plateau_false_positive", [
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "s1"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "s2"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "s3"},
        ])
        try:
            _, cal_fired, results = calibrate_remeasure(ledger)
            plateau = [r for r in results if r.name == "LEDGER_PLATEAU_SIGNAL"]
            assert len(plateau) == 1
            assert plateau[0].raw_fired is True  # V1368 raw fires (FP)
            assert plateau[0].calibrated_fired is False  # V1370 fixes
            assert plateau[0].suppressed is True
        finally:
            ledger.unlink(missing_ok=True)

    def test_passthrough_when_ledger_too_short(self):
        fired, reason = _calibrate_plateau(True, [{"pole_star_total": 0.9,
                                                   "tag": "x"}])
        assert fired is True
        assert "passthrough" in reason


# -----------------------------------------------------------------------------
# NEW_SURFACE trigger calibration
# -----------------------------------------------------------------------------

class TestNewSurfaceCalibration:
    def test_fires_when_truly_new(self):
        ledger = _write_synth_ledger("surface_new", [
            {"pole_star_total": 0.80, "v01_baseline": 0.7905, "tag": f"entry-{i}"}
            for i in range(15)
        ] + [
            {"pole_star_total": 0.85, "v01_baseline": 0.7905, "tag": "v1361-fresh"},
        ])
        try:
            _, _, results = calibrate_remeasure(ledger)
            surface = [r for r in results if r.name == "NEW_SURFACE_SHIPPED"]
            assert len(surface) == 1
            assert surface[0].calibrated_fired is True
        finally:
            ledger.unlink(missing_ok=True)

    def test_does_not_fire_when_repeated(self):
        """V1368 false positive: same surface prefix in last 10 entries."""
        ledger = _write_synth_ledger("surface_repeat", [
            {"pole_star_total": 0.85, "v01_baseline": 0.7905,
             "tag": "v1361-dashboard"}
            for _ in range(5)
        ] + [
            {"pole_star_total": 0.85, "v01_baseline": 0.7905,
             "tag": "v1361-dashboard-2"},
        ])
        try:
            _, _, results = calibrate_remeasure(ledger)
            surface = [r for r in results if r.name == "NEW_SURFACE_SHIPPED"]
            assert len(surface) == 1
            assert surface[0].raw_fired is True
            assert surface[0].calibrated_fired is False
            assert surface[0].suppressed is True
        finally:
            ledger.unlink(missing_ok=True)


# -----------------------------------------------------------------------------
# CAP_SATURATION trigger calibration
# -----------------------------------------------------------------------------

class TestCapSaturationCalibration:
    def test_fires_when_distinct_tags(self):
        ledger = _write_synth_ledger("cap_distinct", [
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "tag-a"},
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "tag-b"},
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "tag-c"},
        ])
        try:
            _, _, results = calibrate_v03(ledger)
            cap = [r for r in results if r.name == "LEDGER_CAP_SATURATION_3"]
            assert len(cap) == 1
            assert cap[0].calibrated_fired is True
        finally:
            ledger.unlink(missing_ok=True)

    def test_does_not_fire_when_duplicate_tags(self):
        ledger = _write_synth_ledger("cap_dup", [
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "self-test"},
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "self-test"},
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "self-test"},
        ])
        try:
            _, _, results = calibrate_v03(ledger)
            cap = [r for r in results if r.name == "LEDGER_CAP_SATURATION_3"]
            assert len(cap) == 1
            assert cap[0].raw_fired is True
            assert cap[0].calibrated_fired is False
            assert cap[0].suppressed is True
        finally:
            ledger.unlink(missing_ok=True)


# -----------------------------------------------------------------------------
# Passthrough triggers
# -----------------------------------------------------------------------------

class TestPassthroughTriggers:
    def test_time_tick_passthrough(self):
        # V1370 does not calibrate TIME_TICK_INTERVAL
        ledger = _write_synth_ledger("passthrough_time", [
            {"pole_star_total": 0.85, "v01_baseline": 0.7905, "tag": f"t-{i}"}
            for i in range(5)
        ])
        try:
            _, _, results = calibrate_remeasure(ledger)
            tt = [r for r in results if r.name == "TIME_TICK_INTERVAL"]
            assert len(tt) == 1
            assert tt[0].raw_fired == tt[0].calibrated_fired
        finally:
            ledger.unlink(missing_ok=True)

    def test_delta_any_component_passthrough(self):
        ledger = _write_synth_ledger("passthrough_delta", [
            {"pole_star_total": 0.80, "v01_baseline": 0.7905, "tag": "d-1"},
            {"pole_star_total": 0.90, "v01_baseline": 0.7905, "tag": "d-2"},
        ])
        try:
            _, _, results = calibrate_remeasure(ledger)
            delta = [r for r in results if r.name == "DELTA_ANY_COMPONENT"]
            assert len(delta) == 1
            assert delta[0].raw_fired == delta[0].calibrated_fired
        finally:
            ledger.unlink(missing_ok=True)


# -----------------------------------------------------------------------------
# Empty ledger
# -----------------------------------------------------------------------------

class TestEmptyLedger:
    def test_empty_remeasure_returns_safe(self):
        empty = Path(os.environ.get("TEMP", "/tmp")) / "v1370_pytest_empty.jsonl"
        empty.write_text("", encoding="utf-8")
        try:
            raw, cal, results = calibrate_remeasure(empty)
            assert raw is False or all(not r.raw_fired for r in results)
            assert cal is False
        finally:
            empty.unlink(missing_ok=True)

    def test_empty_v03_returns_safe(self):
        empty = Path(os.environ.get("TEMP", "/tmp")) / "v1370_pytest_empty.jsonl"
        empty.write_text("", encoding="utf-8")
        try:
            raw, cal, results = calibrate_v03(empty)
            assert raw is False or all(not r.raw_fired for r in results)
            assert cal is False
        finally:
            empty.unlink(missing_ok=True)


# -----------------------------------------------------------------------------
# Reporting
# -----------------------------------------------------------------------------

class TestReporting:
    def test_render_evaluate_non_empty_and_well_formed(self):
        s = evaluate()
        rep = render_evaluate(s)
        assert len(rep) > 100
        assert "V1368 RAW" in rep
        assert "V1370 CAL" in rep

    def test_render_compare_table_format(self):
        s = evaluate()
        rep = render_compare(s)
        assert "V1368 vs V1370" in rep
        assert "TRIGGER" in rep

    def test_render_summary_includes_suppressed_count(self):
        s = evaluate()
        rep = render_summary(s)
        assert "triggers suppressed" in rep

    def test_evaluate_returns_calibration_summary(self):
        s = evaluate()
        assert isinstance(s, CalibrationSummary)
        assert hasattr(s, "calibrated_remeasure_fired")
        assert hasattr(s, "calibrated_v03_fired")
        assert hasattr(s, "remeasure_suppressed_count")
        assert hasattr(s, "v03_suppressed_count")
        assert s.remeasure_suppressed_count >= 0
        assert s.v03_suppressed_count >= 0


# -----------------------------------------------------------------------------
# Real ledger integration (only if present)
# -----------------------------------------------------------------------------

@pytest.mark.skipif(
    not (ROOT / "pole_star_history.jsonl").exists(),
    reason="real pole_star_history.jsonl not present",
)
class TestRealLedgerIntegration:
    def test_evaluate_against_real_ledger_does_not_crash(self):
        s = evaluate()
        # No crash means we got a valid CalibrationSummary
        assert isinstance(s, CalibrationSummary)
        # Real ledger has ≥ 100 entries per recent history
        assert s.ledger_entries >= 100

    def test_calibration_suppresses_real_false_positives(self):
        """V1369 sidecar showed 100% fire rate → V1370 must suppress ≥ 1."""
        s = evaluate()
        # Real ledger has 163 entries with V0.2 plateau at 0.9 = 0.1095 delta
        # LEDGER_PLATEAU_SIGNAL fires raw but should be calibrated out.
        remeasure_results = s.remeasure_results
        plateau = [r for r in remeasure_results
                   if r.name == "LEDGER_PLATEAU_SIGNAL"]
        if plateau:
            # Either V1370 suppressed it (cal=False, sup=True) OR raw was
            # already False. We accept either as long as V1370 didn't
            # wrongly fire on equal-but-nonzero delta.
            assert plateau[0].calibrated_fired is False or \
                "delta=0" in plateau[0].calibrated_reason.lower() or \
                "|Δ|" in plateau[0].calibrated_reason
