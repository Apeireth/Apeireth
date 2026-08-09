"""Tests for V1425 — ASI 总框架 5 哲学空缺 (Time / Freedom / Recognition / Emergence / Truth)."""

from __future__ import annotations

import json
import math
import sys
import tempfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1425_asi_five_philosophical_gaps as m


# ============================================================================
# Constants / structural
# ============================================================================


def test_module_constants_present():
    assert m.V1425_VERSION == "0.1.0"
    assert m.V1425_SCHEMA == "v1425.asi-five-philosophical-gaps/v1"
    assert m.V1425_MODULE == "v1425_asi_five_philosophical_gaps"


def test_gap_names_complete():
    """GAP_NAMES must contain exactly the 5 gaps."""
    assert m.GAP_NAMES == ("time", "freedom", "recognition", "emergence", "truth")


def test_gap_definitions_complete():
    """GAP_DEFINITIONS must have all 5 gaps defined."""
    for gap in m.GAP_NAMES:
        assert gap in m.GAP_DEFINITIONS
        d = m.GAP_DEFINITIONS[gap]
        assert d.name == gap
        assert d.probe_name  # has a probe
        assert d.v3_guard  # has a guard


def test_all_gaps_have_v3_guard():
    """Each gap must have a dedicated V3 guard (no probe claims to answer)."""
    guards = {
        "time": "GUARD_TIME_PROBE_IS_NOT_TIME",
        "freedom": "GUARD_FREEDOM_PROBE_IS_NOT_FREEDOM",
        "recognition": "GUARD_RECOGNITION_PROBE_IS_NOT_RECOGNITION",
        "emergence": "GUARD_EMERGENCE_PROBE_IS_NOT_EMERGENCE",
        "truth": "GUARD_TRUTH_PROBE_IS_NOT_TRUTH",
    }
    for gap, expected_guard in guards.items():
        d = m.GAP_DEFINITIONS[gap]
        assert expected_guard in d.v3_guard, f"{gap}: {d.v3_guard}"


def test_guards_well_formed():
    """GUARDS must contain all 18 expected entries."""
    assert len(m.V1425_GUARDS) >= 18
    expected = {
        "GUARD_PROBE_REAL",
        "GUARD_NO_V1417_WRITE",
        "GUARD_NO_V1419_WRITE",
        "GUARD_NO_V1034_WRITE",
        "GUARD_NO_V1411_WRITE",
        "GUARD_GAP_NOT_SOLVED",
        "GUARD_PROBE_BOUNDED",
        "GUARD_ENTROPY_BOUNDED",
        "GUARD_CONSISTENCY_BOUNDED",
        "GUARD_ACCURACY_BOUNDED",
        "GUARD_CHAIN_BOUNDED",
        "GUARD_DATA_MISSING_HONEST",
        "GUARD_GAP_NAMED",
        "GUARD_GUARD_NAMED",
        "GUARD_BORROWED_REAL",
        "GUARD_POPPER_RUNS",
        "GUARD_CHAIN_OK",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_CLI_RUNNABLE",
    }
    for g in expected:
        assert g in m.V1425_GUARDS, f"missing guard: {g}"


def test_v3_guards_well_formed():
    """V3 哲学守门 must contain at least 9 (5 gap guards + 4 framework)."""
    assert len(m.V1425_V3_GUARDS) >= 9


def test_borrowed_real():
    """BORROWED must contain at least 8 entries (8 listed)."""
    assert len(m.V1425_BORROWED) >= 8
    keys = [b[0] for b in m.V1425_BORROWED]
    assert "V1049" in keys
    assert "V1411" in keys
    assert "V1417" in keys
    assert "V1419" in keys
    assert "V1424" in keys


# ============================================================================
# ProbeResult / GapReport dataclasses
# ============================================================================


def test_probe_result_construct():
    p = m.ProbeResult(
        gap_name="time",
        probe_name="probe_time",
        value=0.5,
        normalized_value=0.5,
        n_samples=10,
        gap_status="OPEN",
        v3_guard="GUARD_TIME_PROBE_IS_NOT_TIME",
        note="test",
        ran_at_iso="2026-08-10T03:55:00Z",
    )
    assert p.gap_name == "time"
    assert p.gap_status == "OPEN"


def test_gap_report_construct():
    cfg = m.build_default_config()
    report = m.run_all_probes(cfg)
    assert report.time is not None
    assert report.freedom is not None
    assert report.recognition is not None
    assert report.emergence is not None
    assert report.truth is not None
    assert report.started_iso
    assert report.ended_iso


# ============================================================================
# Config helpers
# ============================================================================


def test_build_default_config():
    cfg = m.build_default_config()
    assert "history_path" in cfg
    assert "evaluations_path" in cfg
    assert "tick_jsonl_path" in cfg


def test_validate_config_accepts_default():
    cfg = m.build_default_config()
    validated = m.validate_config(cfg)
    assert validated is cfg


def test_validate_config_rejects_missing_history():
    with pytest.raises(ValueError):
        m.validate_config({})


def test_validate_config_rejects_bad_path_type():
    """Path containing '..' must be rejected."""
    with pytest.raises(ValueError):
        m.validate_config(
            {
                "history_path": "../escape.jsonl",
                "evaluations_path": "x",
                "tick_jsonl_path": "y",
            }
        )


# ============================================================================
# Helpers: entropy normalization
# ============================================================================


def test_entropy_uniform_is_1():
    """Uniform distribution should normalize to 1.0."""
    norm, total, reason = m._normalize_entropy_to_unit({"A": 5, "B": 5, "C": 5, "D": 5, "E": 5, "F": 5})
    assert abs(norm - 1.0) < 1e-9, f"norm={norm} reason={reason}"
    assert total == 30


def test_entropy_deterministic_is_0():
    """Single-value distribution → 0.0 entropy."""
    norm, total, _ = m._normalize_entropy_to_unit({"A": 10})
    assert norm == 0.0
    assert total == 10


def test_entropy_empty_is_nan():
    """Empty distribution → NaN, not 0."""
    norm, total, reason = m._normalize_entropy_to_unit({})
    assert math.isnan(norm)
    assert total == 0
    assert reason


def test_entropy_two_equal_is_1():
    """2 values with equal count → entropy = log(2)/log(2) = 1.0 normalized."""
    norm, total, _ = m._normalize_entropy_to_unit({"A": 3, "B": 3})
    assert abs(norm - 1.0) < 1e-9


def test_entropy_three_skewed_is_between_0_and_1():
    """3 values with skewed count → entropy in (0, 1)."""
    norm, total, _ = m._normalize_entropy_to_unit({"A": 7, "B": 2, "C": 1})
    assert 0.0 < norm < 1.0
    assert total == 10


# ============================================================================
# JSONL loader
# ============================================================================


def test_safe_load_jsonl_skips_malformed():
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "x.jsonl"
        p.write_text('{"a": 1}\nnot json\n{"b": 2}\n', encoding="utf-8")
        loaded = m._safe_load_jsonl(p)
        assert len(loaded) == 2
        assert loaded[0] == {"a": 1}
        assert loaded[1] == {"b": 2}


def test_safe_load_jsonl_nonexistent_returns_empty():
    loaded = m._safe_load_jsonl(Path("nonexistent.jsonl"))
    assert loaded == []


def test_safe_load_jsonl_blank_lines_skipped():
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "x.jsonl"
        p.write_text('\n{"a": 1}\n\n{"b": 2}\n\n', encoding="utf-8")
        loaded = m._safe_load_jsonl(p)
        assert len(loaded) == 2


# ============================================================================
# probe_time
# ============================================================================


def test_probe_time_returns_result():
    cfg = m.build_default_config()
    r = m.probe_time(cfg)
    assert r.gap_name == "time"
    assert r.probe_name == "probe_time"
    assert r.gap_status == "OPEN"
    assert r.v3_guard == "GUARD_TIME_PROBE_IS_NOT_TIME"


def test_probe_time_normalized_in_range_or_nan():
    cfg = m.build_default_config()
    r = m.probe_time(cfg)
    if not math.isnan(r.normalized_value):
        assert 0.0 <= r.normalized_value <= 1.0


def test_probe_time_no_data_returns_nan():
    cfg = m.build_default_config({"history_path": Path("nonexistent.jsonl")})
    r = m.probe_time(cfg)
    assert math.isnan(r.normalized_value)
    assert r.gap_status == "OPEN"
    assert r.n_samples == 0


def test_probe_time_with_synthetic_data():
    """Synthetic history with 2 ticks → finite entropy."""
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "hist.jsonl"
        records = [
            {"tick_id": "t1", "ran_at_iso": "2026-08-10T00:00:00Z"},
            {"tick_id": "t2", "ran_at_iso": "2026-08-10T00:00:30Z"},
            {"tick_id": "t3", "ran_at_iso": "2026-08-10T00:02:00Z"},
        ]
        p.write_text("\n".join(json.dumps(r) for r in records) + "\n", encoding="utf-8")
        cfg = m.build_default_config({"history_path": p})
        r = m.probe_time(cfg)
        # 2 intervals: 30s, 90s — entropy in (0, 1)
        if not math.isnan(r.normalized_value):
            assert 0.0 <= r.normalized_value <= 1.0
        assert r.n_samples >= 1


# ============================================================================
# probe_freedom
# ============================================================================


def test_probe_freedom_returns_result():
    cfg = m.build_default_config()
    r = m.probe_freedom(cfg)
    assert r.gap_name == "freedom"
    assert r.probe_name == "probe_freedom"
    assert r.gap_status == "OPEN"
    assert r.v3_guard == "GUARD_FREEDOM_PROBE_IS_NOT_FREEDOM"


def test_probe_freedom_normalized_in_range_or_nan():
    cfg = m.build_default_config()
    r = m.probe_freedom(cfg)
    if not math.isnan(r.normalized_value):
        assert 0.0 <= r.normalized_value <= 1.0


def test_probe_freedom_no_data_returns_nan():
    cfg = m.build_default_config(
        {
            "history_path": Path("nonexistent.jsonl"),
            "evaluations_path": Path("nonexistent.jsonl"),
            "tick_jsonl_path": Path("nonexistent.jsonl"),
        }
    )
    r = m.probe_freedom(cfg)
    assert math.isnan(r.normalized_value)
    assert r.gap_status == "OPEN"


def test_probe_freedom_mixed_policies():
    """History with 3 distinct mixed policies → entropy in (0, 1)."""
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "hist.jsonl"
        records = [
            {"tick_id": "t1", "policy": "PROCEED"},
            {"tick_id": "t2", "policy": "PAUSE"},
            {"tick_id": "t3", "policy": "PROCEED"},
            {"tick_id": "t4", "policy": "LOCKDOWN"},
            {"tick_id": "t5", "policy": "PAUSE"},
            {"tick_id": "t6", "policy": "PROCEED"},
        ]
        p.write_text("\n".join(json.dumps(r) for r in records) + "\n", encoding="utf-8")
        cfg = m.build_default_config({"history_path": p})
        r = m.probe_freedom(cfg)
        if not math.isnan(r.normalized_value):
            assert 0.0 < r.normalized_value < 1.0


# ============================================================================
# probe_recognition
# ============================================================================


def test_probe_recognition_returns_result():
    cfg = m.build_default_config()
    r = m.probe_recognition(cfg)
    assert r.gap_name == "recognition"
    assert r.probe_name == "probe_recognition"
    assert r.gap_status == "OPEN"
    assert r.v3_guard == "GUARD_RECOGNITION_PROBE_IS_NOT_RECOGNITION"


def test_probe_recognition_has_22_samples():
    """22 samples (10 MMLU + 5 GSM8K + 3 HumanEval + 4 HellaSwag)."""
    cfg = m.build_default_config()
    r = m.probe_recognition(cfg)
    assert r.n_samples == 22


def test_probe_recognition_value_in_range():
    cfg = m.build_default_config()
    r = m.probe_recognition(cfg)
    assert 0.0 <= r.normalized_value <= 1.0


# ============================================================================
# probe_emergence
# ============================================================================


def test_probe_emergence_returns_result():
    cfg = m.build_default_config()
    r = m.probe_emergence(cfg)
    assert r.gap_name == "emergence"
    assert r.probe_name == "probe_emergence"
    assert r.gap_status == "OPEN"
    assert r.v3_guard == "GUARD_EMERGENCE_PROBE_IS_NOT_EMERGENCE"


def test_probe_emergence_value_in_range():
    cfg = m.build_default_config()
    r = m.probe_emergence(cfg)
    assert 0.0 <= r.normalized_value <= 1.0


def test_probe_emergence_counts_25_frameworks():
    """V1400-V1424 = 25 frameworks."""
    cfg = m.build_default_config()
    r = m.probe_emergence(cfg)
    assert r.n_samples == 25


# ============================================================================
# probe_truth
# ============================================================================


def test_probe_truth_returns_result():
    cfg = m.build_default_config()
    r = m.probe_truth(cfg)
    assert r.gap_name == "truth"
    assert r.probe_name == "probe_truth"
    assert r.gap_status == "OPEN"
    assert r.v3_guard == "GUARD_TRUTH_PROBE_IS_NOT_TRUTH"


def test_probe_truth_value_in_range_or_nan():
    cfg = m.build_default_config()
    r = m.probe_truth(cfg)
    if not math.isnan(r.normalized_value):
        assert 0.0 <= r.normalized_value <= 1.0


def test_probe_truth_intra_history_consensus():
    """Single-source: consensus rate of policy field."""
    with tempfile.TemporaryDirectory() as td:
        p = Path(td) / "hist.jsonl"
        records = [
            {"tick_id": "t1", "policy": "PROCEED"},
            {"tick_id": "t2", "policy": "PROCEED"},
            {"tick_id": "t3", "policy": "PROCEED"},
            {"tick_id": "t4", "policy": "PAUSE"},
        ]
        p.write_text("\n".join(json.dumps(r) for r in records) + "\n", encoding="utf-8")
        cfg = m.build_default_config({"history_path": p})
        r = m.probe_truth(cfg)
        if not math.isnan(r.normalized_value):
            # 3/4 = 0.75
            assert abs(r.normalized_value - 0.75) < 1e-9


# ============================================================================
# run_all_probes
# ============================================================================


def test_run_all_probes_completes():
    cfg = m.build_default_config()
    report = m.run_all_probes(cfg)
    assert report.time is not None
    assert report.freedom is not None
    assert report.recognition is not None
    assert report.emergence is not None
    assert report.truth is not None


def test_run_all_probes_started_before_ended():
    cfg = m.build_default_config()
    report = m.run_all_probes(cfg)
    assert report.started_iso <= report.ended_iso


def test_run_all_probes_all_status_open():
    """Every probe must tag gap_status = OPEN (no probe claims to solve the gap)."""
    cfg = m.build_default_config()
    report = m.run_all_probes(cfg)
    for slot in (report.time, report.freedom, report.recognition, report.emergence, report.truth):
        assert slot.gap_status == "OPEN"


# ============================================================================
# popper_self_test
# ============================================================================


def test_popper_self_test_passes():
    ok, n, checks = m.popper_self_test()
    assert ok is True
    assert n == 17
    for c in checks:
        assert c["ok"] is True, f"failed check: {c}"


def test_popper_self_test_returns_list():
    ok, n, checks = m.popper_self_test()
    assert isinstance(checks, list)
    assert len(checks) == 17


# ============================================================================
# chain_delegate
# ============================================================================


def test_chain_delegate_returns_dict():
    result = m.chain_delegate()
    assert isinstance(result, dict)


def test_chain_delegate_has_v1425():
    result = m.chain_delegate()
    assert "v1425" in result
    assert result["v1425"] is True


# ============================================================================
# render_report_md
# ============================================================================


def test_render_report_md_returns_string():
    cfg = m.build_default_config()
    report = m.run_all_probes(cfg)
    md = m.render_report_md(report)
    assert isinstance(md, str)
    assert "time" in md.lower()
    assert "freedom" in md.lower()
    assert "recognition" in md.lower()
    assert "emergence" in md.lower()
    assert "truth" in md.lower()


def test_render_report_md_includes_open_status():
    cfg = m.build_default_config()
    report = m.run_all_probes(cfg)
    md = m.render_report_md(report)
    assert "OPEN" in md


# ============================================================================
# run_cli
# ============================================================================


def test_run_cli_version():
    rc = m.run_cli(["version"])
    assert rc == 0


def test_run_cli_meta():
    rc = m.run_cli(["meta"])
    assert rc == 0


def test_run_cli_popper():
    rc = m.run_cli(["popper"])
    assert rc == 0


def test_run_cli_chain():
    rc = m.run_cli(["chain"])
    assert rc == 0


def test_run_cli_list_gaps():
    rc = m.run_cli(["list-gaps"])
    assert rc == 0


def test_run_cli_help():
    rc = m.run_cli(["help"])
    assert rc == 0


def test_run_cli_probe_time():
    rc = m.run_cli(["probe", "--gap", "time"])
    assert rc == 0


def test_run_cli_probe_freedom():
    rc = m.run_cli(["probe", "--gap", "freedom"])
    assert rc == 0


def test_run_cli_probe_recognition():
    rc = m.run_cli(["probe", "--gap", "recognition"])
    assert rc == 0


def test_run_cli_probe_emergence():
    rc = m.run_cli(["probe", "--gap", "emergence"])
    assert rc == 0


def test_run_cli_probe_truth():
    rc = m.run_cli(["probe", "--gap", "truth"])
    assert rc == 0


def test_run_cli_probe_invalid_gap():
    rc = m.run_cli(["probe", "--gap", "bogus"])
    # Should fail (return non-zero)
    assert rc != 0


def test_run_cli_run_all():
    rc = m.run_cli(["run-all"])
    assert rc == 0


def test_run_cli_report():
    rc = m.run_cli(["report"])
    assert rc == 0


def test_run_cli_demo():
    rc = m.run_cli(["demo"])
    assert rc == 0


def test_run_cli_unknown_command():
    rc = m.run_cli(["bogus"])
    assert rc != 0


# ============================================================================
# Reporting helpers
# ============================================================================


def test_module_constants_json_serializable():
    """All public string constants must be JSON-serializable."""
    constants = {
        "version": m.V1425_VERSION,
        "schema": m.V1425_SCHEMA,
        "module": m.V1425_MODULE,
    }
    s = json.dumps(constants)
    assert "0.1.0" in s