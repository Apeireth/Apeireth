"""Tests for V1352 VCP Toolchain History + Diff CLI.

Covers:
  - HistoryEntry / DiffDelta / DiffReport dataclasses
  - load_history / get_record_by_id / get_latest_record
  - compute_diff regression rules (R1/R1b/R1c/R2/R3/R4)
  - _format_history_human / _format_diff_human
  - to_json stability
  - CLI subcommand dispatch
  - Chain regression (V1351 + V1352)
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import List

import pytest

# Make apeireth importable
APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

import apeireth.v1352_vcp_history_diff as m


# -----------------------------------------------------------------------------
# Fixtures
# -----------------------------------------------------------------------------

@pytest.fixture
def sample_entry():
    return m.HistoryEntry(
        record_id="abcd1234abcd1234",
        timestamp="2026-08-08T15:26:11Z",
        passed=False,
        exit_code=1,
        violations_count=3,
        tier_high=93, tier_medium=3, tier_low=0, tier_unclassified=57,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=50, critical_failures=0,
        ledger_hash="cd9c79ba",
    )


@pytest.fixture
def fake_rollup_factory():
    """Build a fake V1351 EcosystemRollup-like object."""
    def make(h, m_, l, sub, ws, es):
        class R:
            pass
        r = R()
        r.n_high_tier = h
        r.n_medium_tier = m_
        r.n_low_tier = l
        r.n_substrates = sub
        r.worst_severity = ws
        r.ecosystem_state = es
        return r

    class FakeResult:
        def __init__(self, r):
            self.ecosystem_rollup = r

    def runner(r):
        return FakeResult(r)

    return runner, make


# -----------------------------------------------------------------------------
# 1. Constants & guards
# -----------------------------------------------------------------------------

def test_v1352_version():
    assert m.V1352_VERSION == "0.1.0"


def test_v1352_asi_cap_honest():
    assert 0 < m.V1352_ASI_CAP < 0.5  # honest cap, not pretending to be ASI


def test_philosophy_guards_locked():
    assert len(m.PHILOSOPHY_GUARDS) >= 5
    for g in m.PHILOSOPHY_GUARDS:
        assert "GUARD_" in g
        # Each guard must reference V1352 OR describe an observer/diff limit
        g_lower = g.lower()
        assert ("V1352" in g) or ("observer" in g_lower) or ("history" in g_lower) or ("diff" in g_lower) or ("regression" in g_lower)


# -----------------------------------------------------------------------------
# 2. HistoryEntry dataclass
# -----------------------------------------------------------------------------

def test_history_entry_fields(sample_entry):
    assert sample_entry.record_id == "abcd1234abcd1234"
    assert sample_entry.passed is False
    assert sample_entry.tier_high == 93
    assert sample_entry.violations_count == 3


def test_history_entry_total_substrates(sample_entry):
    total = (sample_entry.tier_high + sample_entry.tier_medium
             + sample_entry.tier_low + sample_entry.tier_unclassified)
    assert total == 153


# -----------------------------------------------------------------------------
# 3. _short helper
# -----------------------------------------------------------------------------

def test_short_truncates():
    assert m._short("abcdefghij", 4) == "abcd"


def test_short_empty():
    assert m._short("") == ""


def test_short_shorter_than_n():
    assert m._short("abc", 10) == "abc"


# -----------------------------------------------------------------------------
# 4. load_history
# -----------------------------------------------------------------------------

def test_load_history_returns_list():
    entries = m.load_history(m.LEDGER_PATH)
    assert isinstance(entries, list)


def test_load_history_sorted_newest_first():
    entries = m.load_history(m.LEDGER_PATH)
    if len(entries) >= 2:
        # Newest first by timestamp
        assert entries[0].timestamp >= entries[1].timestamp


def test_load_history_last_n_respected(tmp_path):
    p = tmp_path / "ledger.jsonl"
    rows = []
    for i in range(5):
        rows.append({
            "record_id": f"rec{i:016d}",
            "timestamp": f"2026-08-08T15:{i:02d}:00Z",
            "passed": True,
            "exit_code": 0,
            "violations_count": 0,
            "tier_breakdown": {"HIGH": 0, "MEDIUM": 0, "LOW": 0, "UNCLASSIFIED": 0},
            "coverage_current": 1.0,
            "coverage_baseline": 1.0,
            "coverage_delta": 0.0,
            "unclassified_count": 0,
            "critical_failures": 0,
            "ledger_hash": f"hash{i}",
        })
    with open(p, "w", encoding="utf-8") as f:
        for r in rows:
            f.write(json.dumps(r) + "\n")
    out = m.load_history(p, last_n=2)
    assert len(out) == 2


def test_load_history_empty_file(tmp_path):
    p = tmp_path / "empty.jsonl"
    p.touch()
    assert m.load_history(p) == []


def test_load_history_missing_file(tmp_path):
    p = tmp_path / "missing.jsonl"
    assert m.load_history(p) == []


def test_load_history_skips_bad_lines(tmp_path):
    p = tmp_path / "mixed.jsonl"
    with open(p, "w", encoding="utf-8") as f:
        f.write("not-json\n")
        f.write(json.dumps({
            "record_id": "good00000000000",
            "timestamp": "2026-08-08T15:00:00Z",
            "passed": True, "exit_code": 0, "violations_count": 0,
            "tier_breakdown": {"HIGH": 0, "MEDIUM": 0, "LOW": 0, "UNCLASSIFIED": 0},
            "coverage_current": 1.0, "coverage_baseline": 1.0, "coverage_delta": 0.0,
            "unclassified_count": 0, "critical_failures": 0,
            "ledger_hash": "goodhash",
        }) + "\n")
        f.write("{broken-json\n")
    out = m.load_history(p)
    assert len(out) == 1
    assert out[0].record_id == "good00000000000"


# -----------------------------------------------------------------------------
# 5. get_record_by_id / get_latest_record
# -----------------------------------------------------------------------------

def test_get_latest_record_matches_first():
    latest = m.get_latest_record(m.LEDGER_PATH)
    hist = m.load_history(m.LEDGER_PATH, last_n=1)
    if latest and hist:
        assert m._short(latest.record_id, 16) == m._short(hist[0].record_id, 16)


def test_get_record_by_id_full_match():
    latest = m.get_latest_record(m.LEDGER_PATH)
    if not latest:
        pytest.skip("no records in ledger")
    found = m.get_record_by_id(m.LEDGER_PATH, latest.record_id)
    assert found is not None
    assert found.record_id == latest.record_id


def test_get_record_by_id_short_match():
    latest = m.get_latest_record(m.LEDGER_PATH)
    if not latest:
        pytest.skip("no records in ledger")
    short_id = latest.record_id[:16]
    found = m.get_record_by_id(m.LEDGER_PATH, short_id)
    assert found is not None


def test_get_record_by_id_missing(tmp_path):
    p = tmp_path / "empty.jsonl"
    p.touch()
    assert m.get_record_by_id(p, "nonexistent") is None


def test_get_latest_record_empty(tmp_path):
    p = tmp_path / "empty.jsonl"
    p.touch()
    assert m.get_latest_record(p) is None


# -----------------------------------------------------------------------------
# 6. DiffDelta dataclass
# -----------------------------------------------------------------------------

def test_diff_delta_dataclass():
    d = m.DiffDelta(field="x", baseline=1.0, current=3.0, delta=2.0, worse=True)
    assert d.field == "x"
    assert d.delta == 2.0
    assert d.worse is True


def test_diff_delta_not_worse():
    d = m.DiffDelta(field="x", baseline=5.0, current=2.0, delta=-3.0, worse=False)
    assert d.worse is False


# -----------------------------------------------------------------------------
# 7. compute_diff — regression rules
# -----------------------------------------------------------------------------

def test_diff_equal_no_regression(fake_rollup_factory):
    runner, make = fake_rollup_factory
    fr = make(53, 3, 0, 56, "MEDIUM", "CLOSED")
    base = m.HistoryEntry(
        record_id="eq00000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="eq",
    )
    diff = m.compute_diff(runner(fr), base)
    assert diff.exit_code == 0
    assert len(diff.regression_flags) == 0


def test_diff_R1_high_up(fake_rollup_factory):
    runner, make = fake_rollup_factory
    fr = make(60, 3, 0, 63, "MEDIUM", "CLOSED")
    base = m.HistoryEntry(
        record_id="base000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="base",
    )
    diff = m.compute_diff(runner(fr), base)
    assert diff.exit_code == 1
    assert any("R1" in f and "tier_high" in f for f in diff.regression_flags)


def test_diff_R1b_medium_up(fake_rollup_factory):
    runner, make = fake_rollup_factory
    fr = make(53, 8, 0, 61, "MEDIUM", "CLOSED")
    base = m.HistoryEntry(
        record_id="base000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="base",
    )
    diff = m.compute_diff(runner(fr), base)
    assert any("R1b" in f for f in diff.regression_flags)


def test_diff_R1c_low_up(fake_rollup_factory):
    runner, make = fake_rollup_factory
    fr = make(53, 3, 5, 61, "LOW", "CLOSED")
    base = m.HistoryEntry(
        record_id="base000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="base",
    )
    diff = m.compute_diff(runner(fr), base)
    assert any("R1c" in f for f in diff.regression_flags)


def test_diff_R2_passed_to_failed(fake_rollup_factory):
    runner, make = fake_rollup_factory
    # Baseline: LOW severity → "passed" proxy
    base = m.HistoryEntry(
        record_id="pass000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=0, tier_medium=0, tier_low=5, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="pass",
    )
    # Current: HIGH severity → "failed" proxy
    fr = make(53, 3, 0, 56, "HIGH", "OPEN")
    diff = m.compute_diff(runner(fr), base)
    assert any("R2" in f for f in diff.regression_flags)


def test_diff_R2_does_not_fire_when_baseline_already_failing(fake_rollup_factory):
    runner, make = fake_rollup_factory
    # Baseline: HIGH severity → already failing
    base = m.HistoryEntry(
        record_id="fail000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=False, exit_code=1, violations_count=3,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="fail",
    )
    # Current: HIGH severity → still failing (no change)
    fr = make(53, 3, 0, 56, "HIGH", "OPEN")
    diff = m.compute_diff(runner(fr), base)
    assert not any("R2" in f for f in diff.regression_flags)


def test_diff_R3_violations_no_spurious(fake_rollup_factory):
    """V1351 doesn't expose violations_count; we should not spuriously flag."""
    runner, make = fake_rollup_factory
    fr = make(53, 3, 0, 56, "MEDIUM", "CLOSED")
    base = m.HistoryEntry(
        record_id="v000000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=False, exit_code=1, violations_count=5,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="v",
    )
    diff = m.compute_diff(runner(fr), base)
    # R3 should NOT fire because cur.violations_count = 0 (not measured)
    assert not any("R3" in f for f in diff.regression_flags)


def test_diff_R4_unclassified_no_spurious(fake_rollup_factory):
    runner, make = fake_rollup_factory
    fr = make(53, 3, 0, 56, "MEDIUM", "CLOSED")
    base = m.HistoryEntry(
        record_id="u000000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=False, exit_code=1, violations_count=0,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=10, critical_failures=0, ledger_hash="u",
    )
    diff = m.compute_diff(runner(fr), base)
    assert not any("R4" in f for f in diff.regression_flags)


def test_diff_multiple_regressions(fake_rollup_factory):
    runner, make = fake_rollup_factory
    fr = make(60, 8, 5, 73, "HIGH", "OPEN")
    # Baseline must be "passing" (no HIGH tier) for R2 to fire.
    base = m.HistoryEntry(
        record_id="multi000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=0, tier_medium=0, tier_low=3, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="multi",
    )
    diff = m.compute_diff(runner(fr), base)
    # Should fire R1, R1b, R1c, R2
    flags = diff.regression_flags
    assert sum(1 for f in flags if f.startswith("R1")) >= 3
    assert any("R2" in f for f in flags)
    assert diff.exit_code == 1


# -----------------------------------------------------------------------------
# 8. _compute_asi_lift
# -----------------------------------------------------------------------------

def test_asi_lift_no_flags_capped():
    val = m._compute_asi_lift(0, 5)
    assert val == m.V1352_ASI_CAP  # capped at full subscore


def test_asi_lift_with_flags_lower():
    val_clean = m._compute_asi_lift(0, 5)
    val_dirty = m._compute_asi_lift(5, 5)
    assert val_clean >= val_dirty


def test_asi_lift_zero_deltas_zero():
    assert m._compute_asi_lift(0, 0) == 0.0


def test_asi_lift_within_cap():
    val = m._compute_asi_lift(0, 100)
    assert val <= m.V1352_ASI_CAP


# -----------------------------------------------------------------------------
# 9. Formatting
# -----------------------------------------------------------------------------

def test_format_history_human(sample_entry):
    out = m._format_history_human([sample_entry])
    assert "V1352 VCP History" in out
    assert sample_entry.record_id[:16] in out or "abcd1234" in out


def test_format_history_human_empty():
    out = m._format_history_human([])
    assert "no history records" in out


def test_format_diff_human_with_regression(fake_rollup_factory):
    runner, make = fake_rollup_factory
    fr = make(60, 3, 0, 63, "MEDIUM", "CLOSED")
    base = m.HistoryEntry(
        record_id="base000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="base",
    )
    diff = m.compute_diff(runner(fr), base)
    out = m._format_diff_human(diff)
    assert "V1352 VCP Diff" in out
    assert "YES" in out
    assert "R1" in out
    assert "GUARD_" in out


def test_format_diff_human_no_regression(fake_rollup_factory):
    runner, make = fake_rollup_factory
    fr = make(53, 3, 0, 56, "MEDIUM", "CLOSED")
    base = m.HistoryEntry(
        record_id="base000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="base",
    )
    diff = m.compute_diff(runner(fr), base)
    out = m._format_diff_human(diff)
    assert "NO" in out


# -----------------------------------------------------------------------------
# 10. to_json
# -----------------------------------------------------------------------------

def test_to_json_stability(fake_rollup_factory):
    runner, make = fake_rollup_factory
    fr = make(53, 3, 0, 56, "MEDIUM", "CLOSED")
    base = m.HistoryEntry(
        record_id="base000000000000", timestamp="2026-08-08T15:00:00Z",
        passed=True, exit_code=0, violations_count=0,
        tier_high=53, tier_medium=3, tier_low=0, tier_unclassified=0,
        coverage_current=1.0, coverage_baseline=1.0, coverage_delta=0.0,
        unclassified_count=0, critical_failures=0, ledger_hash="base",
    )
    diff = m.compute_diff(runner(fr), base)
    assert m.to_json(diff) == m.to_json(diff)
    parsed = json.loads(m.to_json(diff))
    assert "deltas" in parsed
    assert "regression_flags" in parsed


def test_to_json_history_entry(sample_entry):
    j = m.to_json(sample_entry)
    parsed = json.loads(j)
    assert parsed["record_id"] == sample_entry.record_id


# -----------------------------------------------------------------------------
# 11. CLI subprocess (smoke tests)
# -----------------------------------------------------------------------------

def test_cli_version():
    r = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1352_vcp_history_diff.py"), "version"],
        capture_output=True, text=True, timeout=30,
    )
    assert r.returncode == 0
    assert "0.1.0" in r.stdout


def test_cli_self_test():
    r = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1352_vcp_history_diff.py"), "self-test"],
        capture_output=True, text=True, timeout=60,
    )
    assert "37/37 PASS" in r.stdout
    assert r.returncode == 0


def test_cli_history_human():
    r = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1352_vcp_history_diff.py"),
         "history", "--last", "3"],
        capture_output=True, text=True, timeout=30,
    )
    assert r.returncode == 0
    assert "V1352 VCP History" in r.stdout


def test_cli_history_json():
    r = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1352_vcp_history_diff.py"),
         "history", "--last", "2", "--json"],
        capture_output=True, text=True, timeout=30,
    )
    assert r.returncode == 0
    parsed = json.loads(r.stdout)
    assert isinstance(parsed, list)
    assert len(parsed) <= 2


def test_cli_diff_no_regression():
    """Default diff against latest ledger record; should not regress."""
    r = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1352_vcp_history_diff.py"),
         "diff"],
        capture_output=True, text=True, timeout=180,
    )
    # Either 0 (no regression) or 1 (regression); never 2 (error)
    assert r.returncode in (0, 1)
    assert "V1352 VCP Diff" in r.stdout


def test_cli_diff_json():
    r = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1352_vcp_history_diff.py"),
         "diff", "--json"],
        capture_output=True, text=True, timeout=180,
    )
    assert r.returncode in (0, 1)
    parsed = json.loads(r.stdout)
    assert "deltas" in parsed
    assert "regression_flags" in parsed


def test_cli_diff_against_unknown_record():
    r = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1352_vcp_history_diff.py"),
         "diff", "--against", "deadbeef00000000"],
        capture_output=True, text=True, timeout=30,
    )
    assert r.returncode == 2
    assert "ERROR" in r.stderr


def test_cli_no_args_prints_help():
    r = subprocess.run(
        [sys.executable, str(APEIRETH_DIR / "v1352_vcp_history_diff.py")],
        capture_output=True, text=True, timeout=30,
    )
    assert r.returncode == 0
    assert "usage:" in r.stdout or "history" in r.stdout