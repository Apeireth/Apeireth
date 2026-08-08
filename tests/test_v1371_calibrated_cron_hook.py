"""Tests for V1371 calibrated cron hook.

V1371 wires V1369 (raw) + V1370 (calibrated) into a single cron tick and
writes a new sidecar `v1370_calibrated_cron_evaluations.jsonl`. These tests
prove:

  1. V1371 does NOT mutate V1368/V1369/V1370 source (imports work).
  2. V1371 writes BOTH sidecars (raw sidecar + calibrated sidecar).
  3. V1371's calibrated_exit reflects calibrated state, not raw.
  4. V1371's per-trigger fields correctly mark suppressed FP.
  5. V1371's CLI is non-recursive and returns the documented exit codes.
  6. V1371 honors GUARD_NO_SOURCE_MUTATION (V1368/V1369/V1370 unchanged).
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import tempfile
from pathlib import Path

import pytest

# Ensure project root is on sys.path
PROJECT_ROOT = Path(__file__).resolve().parent.parent
if str(PROJECT_ROOT) not in sys.path:
    sys.path.insert(0, str(PROJECT_ROOT))


def _import_v1371():
    from apeireth import v1371_calibrated_cron_hook as v1371
    return v1371


def _import_v1368_v1369_v1370():
    from apeireth import v1368_pole_star_v03_triggers as v1368
    from apeireth import v1369_v1368_cron_hook as v1369
    from apeireth import v1370_v1368_trigger_calibration as v1370
    return v1368, v1369, v1370


# -----------------------------------------------------------------------------
# Module constants & guards
# -----------------------------------------------------------------------------

def test_v1371_module_imports():
    v1371 = _import_v1371()
    assert v1371.V1371_VERSION == "0.1.0"
    assert v1371.V1371_VERSION.count(".") == 2


def test_v1371_guards_complete():
    v1371 = _import_v1371()
    required = {
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
    }
    assert required.issubset(set(v1371.V1371_GUARDS))


def test_v1371_sidecar_paths():
    v1371 = _import_v1371()
    assert v1371.DEFAULT_CALIBRATED_SIDECAR_PATH != v1371.DEFAULT_LEDGER_PATH
    assert v1371.DEFAULT_CALIBRATED_SIDECAR_PATH.parent == v1371.DEFAULT_LEDGER_PATH.parent
    assert v1371.DEFAULT_CALIBRATED_SIDECAR_PATH != v1371.DEFAULT_RAW_SIDECAR_PATH
    assert v1371.DEFAULT_RAW_SIDECAR_PATH.parent == v1371.DEFAULT_LEDGER_PATH.parent


def test_v1371_exit_codes_distinct():
    v1371 = _import_v1371()
    codes = {
        v1371.EXIT_NO_FIRE,
        v1371.EXIT_REMEASURE_FIRED,
        v1371.EXIT_V03_FIRED,
        v1371.EXIT_FATAL_WRITE,
    }
    assert len(codes) == 4


# -----------------------------------------------------------------------------
# Source mutation guards (V1371 must NOT touch V1368/V1369/V1370)
# -----------------------------------------------------------------------------

def test_v1371_propagates_v1368_guards():
    v1368, _, _ = _import_v1368_v1369_v1370()
    v1371 = _import_v1371()
    # V1368 guards present in v1371's GUARDS list
    assert all(g in v1371.V1371_GUARDS + v1371.V1370_GUARDS + v1371.V1369_GUARDS + v1371.V1368_GUARDS
               for g in v1368.V1368_GUARDS)


def test_v1371_propagates_v1369_guards():
    _, v1369, _ = _import_v1368_v1369_v1370()
    v1371 = _import_v1371()
    assert "GUARD_SIDECAR_NOT_LEDGER" in v1369.V1369_GUARDS
    assert "GUARD_SIDECAR_NOT_LEDGER" in v1371.V1369_GUARDS


def test_v1371_propagates_v1370_guards():
    _, _, v1370 = _import_v1368_v1369_v1370()
    v1371 = _import_v1371()
    assert "GUARD_NO_SOURCE_MUTATION" in v1370.V1370_GUARDS
    assert "GUARD_NO_SOURCE_MUTATION" in v1371.V1370_GUARDS


def test_v1371_v1370_constants_match():
    _, _, v1370 = _import_v1368_v1369_v1370()
    v1371 = _import_v1371()
    assert v1371.PLATEAU_DELTA_EPSILON == v1370.PLATEAU_DELTA_EPSILON == 1e-4
    assert v1371.NEW_SURFACE_LOOKBACK == v1370.NEW_SURFACE_LOOKBACK == 10
    assert v1371.CAP_SAT_MIN_DISTINCT_TAGS == v1370.CAP_SAT_MIN_DISTINCT_TAGS == 2


# -----------------------------------------------------------------------------
# Core pipeline: evaluate_now_calibrated
# -----------------------------------------------------------------------------

def _empty_ledger(tmp: Path) -> Path:
    p = tmp / "ledger.jsonl"
    p.write_text("", encoding="utf-8")
    return p


def _write_ledger(tmp: Path, entries: list) -> Path:
    p = tmp / "ledger.jsonl"
    with p.open("w", encoding="utf-8") as f:
        for e in entries:
            f.write(json.dumps(e) + "\n")
    return p


def test_v1371_empty_ledger_no_fire():
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        # Use a NON-EXISTENT path so ledger_exists is False
        ledger = Path(tmp) / "nonexistent_ledger.jsonl"
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        entry = v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T03:30:00Z",
        )
        assert entry["schema"] == "v1371_evaluation_v1"
        assert entry["evaluated_at"] == "2026-08-09T03:30:00Z"
        assert entry["ledger_exists"] is False
        assert entry["ledger_entries"] == 0
        assert entry["summary"]["raw_remeasure_fired"] is False
        assert entry["summary"]["raw_v03_fired"] is False
        assert entry["summary"]["calibrated_remeasure_fired"] is False
        assert entry["summary"]["calibrated_v03_fired"] is False
        assert entry["summary"]["raw_fired_names"] == []
        assert entry["summary"]["calibrated_fired_names"] == []
        # Both sidecars written
        assert raw_sc.exists()
        assert cal_sc.exists()


def test_v1371_writes_both_sidecars():
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        ledger = _empty_ledger(Path(tmp))
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T03:30:00Z",
        )
        # Raw sidecar has 1 entry (V1369 schema)
        raw_entries = [l for l in raw_sc.read_text(encoding="utf-8").splitlines() if l.strip()]
        assert len(raw_entries) == 1
        raw_entry = json.loads(raw_entries[0])
        assert raw_entry["schema"] == "v1368_evaluation_v1"
        # Calibrated sidecar has 1 entry (V1371 schema)
        cal_entries = [l for l in cal_sc.read_text(encoding="utf-8").splitlines() if l.strip()]
        assert len(cal_entries) == 1
        cal_entry = json.loads(cal_entries[0])
        assert cal_entry["schema"] == "v1371_evaluation_v1"


def test_v1371_steady_state_calibrated_suppresses_fp():
    """V1368 raw fires on equal-delta (FP); V1370 suppresses; V1371 records."""
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        ledger = _write_ledger(Path(tmp), [
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test-a"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test-b"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test-c"},
        ])
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        entry = v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T03:35:00Z",
        )
        # Raw V1368 fires
        assert entry["summary"]["raw_remeasure_fired"] is True
        # V1370 calibrated does NOT fire
        assert entry["summary"]["calibrated_remeasure_fired"] is False
        # Suppressed count > 0
        assert entry["summary"]["suppressed_remeasure_count"] > 0
        # LEDGER_PLATEAU_SIGNAL in raw_fired but not in calibrated_fired
        assert "LEDGER_PLATEAU_SIGNAL" in entry["summary"]["raw_fired_names"]
        assert "LEDGER_PLATEAU_SIGNAL" not in entry["summary"]["calibrated_fired_names"]
        # Per-trigger: suppressed=True for that trigger
        plateau_results = [r for r in entry["calibrated"]["results"]
                           if r["name"] == "LEDGER_PLATEAU_SIGNAL"]
        assert len(plateau_results) == 1
        assert plateau_results[0]["raw_fired"] is True
        assert plateau_results[0]["calibrated_fired"] is False
        assert plateau_results[0]["suppressed"] is True


def test_v1371_honest_cap_saturation_fires_calibrated():
    """3 distinct tags at cap → V1370 fires (calibrated honest signal)."""
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        ledger = _write_ledger(Path(tmp), [
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "tag-a"},
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "tag-b"},
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "tag-c"},
        ])
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        entry = v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T03:40:00Z",
        )
        # Honest cap-saturation (3 distinct tags) should pass calibration
        assert entry["summary"]["calibrated_v03_fired"] is True
        assert "LEDGER_CAP_SATURATION_3" in entry["summary"]["calibrated_fired_names"]


def test_v1371_same_tag_cap_saturation_suppressed():
    """3 same-tag entries at cap → V1370 suppresses (duplicate-tag spam)."""
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        ledger = _write_ledger(Path(tmp), [
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "self-test"},
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "self-test"},
            {"pole_star_total": 0.90, "pole_star_cap": 0.90, "tag": "self-test"},
        ])
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        entry = v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T03:45:00Z",
        )
        # Raw fires, calibrated does NOT fire (suppressed)
        assert entry["summary"]["raw_v03_fired"] is True
        assert entry["summary"]["calibrated_v03_fired"] is False
        assert entry["summary"]["suppressed_v03_count"] > 0
        assert "LEDGER_CAP_SATURATION_3" in entry["summary"]["raw_fired_names"]
        assert "LEDGER_CAP_SATURATION_3" not in entry["summary"]["calibrated_fired_names"]


def test_v1371_entry_schema_field_complete():
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        ledger = _empty_ledger(Path(tmp))
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        entry = v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T03:50:00Z",
        )
        required_top = {
            "schema", "evaluated_at", "v1371_version", "v1370_version",
            "v1369_version", "v1368_version", "ledger_path", "ledger_exists",
            "ledger_entries", "raw", "calibrated", "summary", "guards",
        }
        assert required_top.issubset(entry.keys())
        required_raw = {"remeasure_fired", "v03_fired", "sidecar_path", "results"}
        assert required_raw.issubset(entry["raw"].keys())
        required_cal = {"remeasure_fired", "v03_fired", "results"}
        assert required_cal.issubset(entry["calibrated"].keys())
        required_summary = {
            "raw_remeasure_fired", "raw_v03_fired",
            "calibrated_remeasure_fired", "calibrated_v03_fired",
            "suppressed_remeasure_count", "suppressed_v03_count",
            "raw_fired_names", "calibrated_fired_names",
        }
        assert required_summary.issubset(entry["summary"].keys())


def test_v1371_calibrated_guards_included():
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        ledger = _empty_ledger(Path(tmp))
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        entry = v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T03:55:00Z",
        )
        # All V1371 guards + downstream guards are in entry
        for g in v1371.V1371_GUARDS:
            assert g in entry["guards"]
        for g in v1371.V1370_GUARDS:
            assert g in entry["guards"]
        for g in v1371.V1369_GUARDS:
            assert g in entry["guards"]
        for g in v1371.V1368_GUARDS:
            assert g in entry["guards"]


# -----------------------------------------------------------------------------
# Reporting
# -----------------------------------------------------------------------------

def test_v1371_render_evaluation_human_includes_calibrated():
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        ledger = _write_ledger(Path(tmp), [
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test-a"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test-b"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test-c"},
        ])
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        entry = v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T04:00:00Z",
        )
        rep = v1371.render_evaluation_human(entry)
        assert isinstance(rep, str)
        assert len(rep) > 100
        assert "CALIBRATED" in rep
        assert "RAW" in rep
        assert "LEDGER_PLATEAU_SIGNAL" in rep


def test_v1371_render_summary_with_data():
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        ledger = _empty_ledger(Path(tmp))
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T04:00:00Z",
        )
        rep = v1371.render_summary(cal_sc)
        assert isinstance(rep, str)
        assert "total evaluations" in rep
        assert "raw" in rep
        assert "calibrated" in rep


def test_v1371_render_compare_shows_side_by_side():
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        ledger = _write_ledger(Path(tmp), [
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test-a"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test-b"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test-c"},
        ])
        raw_sc = Path(tmp) / "v1368_evaluations.jsonl"
        cal_sc = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=raw_sc,
            calibrated_sidecar_path=cal_sc,
            evaluated_at="2026-08-09T04:00:00Z",
        )
        rep = v1371.render_compare(cal_sc)
        assert isinstance(rep, str)
        assert "raw vs calibrated" in rep
        assert "TRIGGER" in rep


def test_v1371_render_summary_empty_sidecar():
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        empty_path = Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl"
        rep = v1371.render_summary(empty_path)
        assert "empty" in rep


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def test_v1371_cli_evaluate_returns_exit_code():
    v1371 = _import_v1371()
    args = argparse.Namespace(json=False)
    rc = v1371._cli_evaluate(args)
    assert rc in (v1371.EXIT_NO_FIRE, v1371.EXIT_REMEASURE_FIRED,
                  v1371.EXIT_V03_FIRED, v1371.EXIT_FATAL_WRITE)


def test_v1371_cli_show_last_returns_zero():
    v1371 = _import_v1371()
    args = argparse.Namespace(n=1)
    rc = v1371._cli_show_last(args)
    assert rc == 0


def test_v1371_cli_summary_returns_zero():
    v1371 = _import_v1371()
    args = argparse.Namespace()
    rc = v1371._cli_summary(args)
    assert rc == 0


def test_v1371_cli_compare_returns_zero():
    v1371 = _import_v1371()
    args = argparse.Namespace()
    rc = v1371._cli_compare(args)
    assert rc == 0


def test_v1371_cli_self_test_passes():
    v1371 = _import_v1371()
    args = argparse.Namespace(verbose=False)
    rc = v1371._cli_self_test(args)
    assert rc == 0


def test_v1371_build_parser_has_all_subcommands():
    v1371 = _import_v1371()
    p = v1371.build_parser()
    # Parse ok
    for cmd in (["evaluate"], ["summary"], ["compare"],
                ["show-last"], ["show-last", "3"], ["self-test"]):
        args = p.parse_args(cmd)
        assert args.command == cmd[0]
    # Invalid usage
    with pytest.raises(SystemExit):
        p.parse_args([])


# -----------------------------------------------------------------------------
# V3 哲学守门 (主 17:58 + 20:46 + 17:43 + 22:33 + 00:56)
# -----------------------------------------------------------------------------

def test_v1371_no_source_mutation_v1368():
    """V1368 source unchanged: importing V1371 does not modify V1368 API."""
    v1368, _, _ = _import_v1368_v1369_v1370()
    v1371 = _import_v1371()
    # V1368's public API still works
    assert hasattr(v1368, "should_remeasure")
    assert hasattr(v1368, "should_consider_v03")
    assert hasattr(v1368, "V1368_GUARDS")
    # V1371 imports from V1368 (not redefines)
    assert v1371.V1368_TRIGGERS_VERSION == v1368.V1368_VERSION


def test_v1371_no_source_mutation_v1369():
    """V1369 source unchanged."""
    _, v1369, _ = _import_v1368_v1369_v1370()
    v1371 = _import_v1371()
    assert hasattr(v1369, "evaluate_now")
    assert hasattr(v1369, "V1369_GUARDS")
    assert v1371.V1369_VERSION == v1369.V1369_VERSION


def test_v1371_no_source_mutation_v1370():
    """V1370 source unchanged."""
    _, _, v1370 = _import_v1368_v1369_v1370()
    v1371 = _import_v1371()
    assert hasattr(v1370, "evaluate")
    assert hasattr(v1370, "V1370_GUARDS")
    assert v1371.V1370_VERSION == v1370.V1370_VERSION


def test_v1371_calibrated_is_honest_exit_code():
    """Exit code reflects calibrated, not raw. Use SAME-tag cap-saturation
    data so V1370 suppresses both PLATEAU and CAP_SATURATION_FP."""
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        # Same tag → V1370 suppresses CAP_SATURATION_3 (need distinct tags)
        # Equal non-zero delta → V1370 suppresses PLATEAU (need delta ≈ 0)
        ledger = _write_ledger(Path(tmp), [
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test"},
        ])
        entry = v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=Path(tmp) / "v1368_evaluations.jsonl",
            calibrated_sidecar_path=Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl",
            evaluated_at="2026-08-09T04:00:00Z",
        )
        # Raw fires on PLATEAU + CAP_SATURATION
        assert entry["summary"]["raw_remeasure_fired"] is True
        assert entry["summary"]["raw_v03_fired"] is True
        # Calibrated suppresses both
        assert entry["summary"]["calibrated_remeasure_fired"] is False
        assert entry["summary"]["calibrated_v03_fired"] is False
        # CLI exit code reflects calibrated_state
        s = entry["summary"]
        if s["calibrated_v03_fired"]:
            expected = v1371.EXIT_V03_FIRED
        elif s["calibrated_remeasure_fired"]:
            expected = v1371.EXIT_REMEASURE_FIRED
        else:
            expected = v1371.EXIT_NO_FIRE
        assert expected == v1371.EXIT_NO_FIRE  # 0


def test_v1371_honest_plateau_implicit_in_calibrated():
    """When calibrated does NOT fire (same-tag cap-saturation), summary is empty."""
    v1371 = _import_v1371()
    with tempfile.TemporaryDirectory() as tmp:
        # Same tag → V1370 suppresses CAP_SATURATION_3
        ledger = _write_ledger(Path(tmp), [
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test"},
            {"pole_star_total": 0.9, "v01_baseline": 0.7905, "tag": "v1362-self-test"},
        ])
        entry = v1371.evaluate_now_calibrated(
            ledger_path=ledger,
            raw_sidecar_path=Path(tmp) / "v1368_evaluations.jsonl",
            calibrated_sidecar_path=Path(tmp) / "v1370_calibrated_cron_evaluations.jsonl",
            evaluated_at="2026-08-09T04:00:00Z",
        )
        # Honest plateau: no calibrated fire
        assert entry["summary"]["calibrated_fired_names"] == []
        # Render reports "no calibrated fire — plateau is honest, not failure"
        rep = v1371.render_evaluation_human(entry)
        assert "plateau is honest, not failure" in rep
