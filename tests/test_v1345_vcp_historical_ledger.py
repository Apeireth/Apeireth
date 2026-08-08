#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""test_v1345_vcp_historical_ledger.py — V1345 historical ledger pytest suite.

V1345 wraps V1344 CIGateResult as a persistent, queryable, drift-aware history.
This test suite verifies:
  - JSONL append-only persistence
  - SHA256 content-addressed record_id
  - history/latest/by_id/between queries
  - detect_regression drift alerts (coverage, tier, unclassified, pass→fail)
  - markdown/csv/json exporters
  - Popper self-tests in-module

All evidence is REAL: ledger written to tmp file, queries return real records.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "apeireth"))

import v1344_vcp_ci_gate as v1344  # noqa: E402
import v1345_vcp_historical_ledger as v1345  # noqa: E402


# --- Fixtures ---------------------------------------------------------------
@pytest.fixture
def tmp_ledger(tmp_path) -> Path:
    """Isolated JSONL ledger path per-test."""
    p = tmp_path / "ledger.jsonl"
    return p


@pytest.fixture
def perm_ledger() -> Path:
    """Permissive gate config that produces a passing result."""
    return v1344.CIGateConfig(
        tier_min="all",
        fail_on_coverage_loss=False,
        max_critical_failures=10,
        fail_on_unclassified=False,
    )


@pytest.fixture
def strict_ledger() -> Path:
    """Strict default gate config (matches V1344 default)."""
    return v1344.CIGateConfig()


# --- Tests: record() and persistence ----------------------------------------
def test_record_returns_ledger_record(tmp_ledger, perm_ledger):
    """record() must return a LedgerRecord with stable id."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    rec = v1345.record(gate, tmp_ledger)
    assert isinstance(rec, v1345.LedgerRecord)
    assert len(rec.record_id) == 16
    assert rec.ledger_hash == gate.ledger_hash


def test_record_appends_jsonl(tmp_ledger, perm_ledger):
    """JSONL file must contain exactly one line per record."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    v1345.record(gate, tmp_ledger)
    text = tmp_ledger.read_text(encoding="utf-8")
    lines = [ln for ln in text.splitlines() if ln.strip()]
    assert len(lines) == 1


def test_multiple_records(tmp_ledger, perm_ledger):
    """Three record() calls produce 3 lines."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    for _ in range(3):
        v1345.record(gate, tmp_ledger)
    hist = v1345.history(tmp_ledger)
    assert len(hist) == 3


# --- Tests: query API --------------------------------------------------------
def test_history_returns_records(tmp_ledger, perm_ledger):
    """history() returns the persisted records."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    r1 = v1345.record(gate, tmp_ledger)
    r2 = v1345.record(gate, tmp_ledger)
    hist = v1345.history(tmp_ledger)
    assert [r.record_id for r in hist] == [r1.record_id, r2.record_id]


def test_history_limit(tmp_ledger, perm_ledger):
    """history(limit=N) returns the most recent N records."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    for _ in range(5):
        v1345.record(gate, tmp_ledger)
    hist = v1345.history(tmp_ledger, limit=2)
    assert len(hist) == 2


def test_latest(tmp_ledger, perm_ledger):
    """latest() returns the most recent record."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    v1345.record(gate, tmp_ledger)
    v1345.record(gate, tmp_ledger)
    last = v1345.latest(tmp_ledger)
    hist = v1345.history(tmp_ledger)
    assert last is not None
    assert last.record_id == hist[-1].record_id


def test_latest_empty(tmp_ledger):
    """latest() on empty ledger returns None."""
    assert v1345.latest(tmp_ledger) is None


def test_by_id(tmp_ledger, perm_ledger):
    """by_id() finds the record with matching SHA256 id."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    rec = v1345.record(gate, tmp_ledger)
    found = v1345.by_id(rec.record_id, tmp_ledger)
    assert found is not None
    assert found.record_id == rec.record_id


def test_by_id_missing(tmp_ledger, perm_ledger):
    """by_id() returns None when id is not found."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    v1345.record(gate, tmp_ledger)
    assert v1345.by_id("0000000000000000", tmp_ledger) is None


def test_between(tmp_ledger, perm_ledger):
    """between() filters records by timestamp range."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    v1345.record(gate, tmp_ledger)
    recs = v1345.between("1970-01-01", "2999-12-31", tmp_ledger)
    assert len(recs) == 1
    recs_empty = v1345.between("1970-01-01", "1970-01-02", tmp_ledger)
    assert len(recs_empty) == 0


# --- Tests: drift detection -------------------------------------------------
def test_drift_no_change(tmp_ledger, perm_ledger):
    """detect_regression(baseline=current) returns no alerts."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    rec = v1345.record(gate, tmp_ledger)
    alerts = v1345.detect_regression(rec, rec)
    assert alerts == []


def test_drift_coverage_regression(tmp_ledger, perm_ledger):
    """Coverage drop > 1% triggers coverage-regression alert."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    base = v1345.record(gate, tmp_ledger)
    worse = v1345.LedgerRecord(
        record_id="0" * 16,
        ledger_hash=base.ledger_hash,
        timestamp=base.timestamp,
        passed=False,
        exit_code=1,
        coverage_current=base.coverage_current - 0.05,
        coverage_baseline=base.coverage_current,
        coverage_delta=-0.05,
        tier_breakdown=base.tier_breakdown,
        violations_count=base.violations_count,
        unclassified_count=base.unclassified_count,
        critical_failures=base.critical_failures,
        gate_config=base.gate_config,
    )
    alerts = v1345.detect_regression(base, worse)
    rule_ids = {a.ruleId for a in alerts}
    assert "coverage-regression" in rule_ids


def test_drift_high_tier_drop(tmp_ledger, perm_ledger):
    """HIGH tier count drop >= 5 triggers high-tier-count-drop alert."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    base = v1345.record(gate, tmp_ledger)
    worse_tb = dict(base.tier_breakdown)
    worse_tb["HIGH"] = worse_tb.get("HIGH", 0) - 10
    worse = v1345.LedgerRecord(
        record_id="0" * 16,
        ledger_hash=base.ledger_hash,
        timestamp=base.timestamp,
        passed=False,
        exit_code=1,
        coverage_current=base.coverage_current,
        coverage_baseline=base.coverage_current,
        coverage_delta=0.0,
        tier_breakdown=worse_tb,
        violations_count=base.violations_count,
        unclassified_count=base.unclassified_count,
        critical_failures=base.critical_failures,
        gate_config=base.gate_config,
    )
    alerts = v1345.detect_regression(base, worse)
    rule_ids = {a.ruleId for a in alerts}
    assert "high-tier-count-drop" in rule_ids


def test_drift_unclassified_growth(tmp_ledger, perm_ledger):
    """Unclassified growth > 5 triggers unclassified-growth alert."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    base = v1345.record(gate, tmp_ledger)
    worse = v1345.LedgerRecord(
        record_id="0" * 16,
        ledger_hash=base.ledger_hash,
        timestamp=base.timestamp,
        passed=False,
        exit_code=1,
        coverage_current=base.coverage_current,
        coverage_baseline=base.coverage_current,
        coverage_delta=0.0,
        tier_breakdown=base.tier_breakdown,
        violations_count=base.violations_count,
        unclassified_count=base.unclassified_count + 20,
        critical_failures=base.critical_failures,
        gate_config=base.gate_config,
    )
    alerts = v1345.detect_regression(base, worse)
    rule_ids = {a.ruleId for a in alerts}
    assert "unclassified-growth" in rule_ids


def test_drift_violation_growth(tmp_ledger, perm_ledger):
    """Violation growth > 1 triggers violation-growth alert."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    base = v1345.record(gate, tmp_ledger)
    worse = v1345.LedgerRecord(
        record_id="0" * 16,
        ledger_hash=base.ledger_hash,
        timestamp=base.timestamp,
        passed=False,
        exit_code=1,
        coverage_current=base.coverage_current,
        coverage_baseline=base.coverage_current,
        coverage_delta=0.0,
        tier_breakdown=base.tier_breakdown,
        violations_count=base.violations_count + 5,
        unclassified_count=base.unclassified_count,
        critical_failures=base.critical_failures,
        gate_config=base.gate_config,
    )
    alerts = v1345.detect_regression(base, worse)
    rule_ids = {a.ruleId for a in alerts}
    assert "violation-growth" in rule_ids


def test_drift_pass_to_fail(tmp_ledger, perm_ledger):
    """PASS → FAIL transition triggers pass-to-fail alert."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    base = v1345.record(gate, tmp_ledger)
    assert base.passed  # permissive config should pass
    worse = v1345.LedgerRecord(
        record_id="0" * 16,
        ledger_hash=base.ledger_hash,
        timestamp=base.timestamp,
        passed=False,
        exit_code=1,
        coverage_current=base.coverage_current,
        coverage_baseline=base.coverage_current,
        coverage_delta=0.0,
        tier_breakdown=base.tier_breakdown,
        violations_count=base.violations_count,
        unclassified_count=base.unclassified_count,
        critical_failures=base.critical_failures,
        gate_config=base.gate_config,
    )
    alerts = v1345.detect_regression(base, worse)
    rule_ids = {a.ruleId for a in alerts}
    assert "pass-to-fail" in rule_ids


def test_drift_summary(tmp_ledger, perm_ledger):
    """drift_summary() returns proper shape and structure."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    r1 = v1345.record(gate, tmp_ledger)
    r2 = v1345.record(gate, tmp_ledger)
    summary = v1345.drift_summary([r1, r2])
    assert summary["records"] == 2
    assert summary["first"] == r1.timestamp
    assert summary["last"] == r2.timestamp
    assert "alerts" in summary


# --- Tests: exporters --------------------------------------------------------
def test_to_markdown(tmp_ledger, perm_ledger):
    """to_markdown() renders valid markdown with all record fields."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    rec = v1345.record(gate, tmp_ledger)
    md = v1345.to_markdown([rec])
    assert "VCP Gate History" in md
    assert rec.record_id in md
    assert "passed" in md.lower() or "✓" in md or "✗" in md


def test_to_markdown_empty():
    """to_markdown([]) returns placeholder text."""
    assert "No ledger records" in v1345.to_markdown([])


def test_to_csv(tmp_ledger, perm_ledger):
    """to_csv() renders expected CSV header and rows."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    rec = v1345.record(gate, tmp_ledger)
    csv_out = v1345.to_csv([rec])
    assert "record_id,ledger_hash,timestamp" in csv_out
    assert rec.record_id in csv_out


def test_to_json(tmp_ledger, perm_ledger):
    """to_json() renders parseable JSON array."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    rec = v1345.record(gate, tmp_ledger)
    json_out = v1345.to_json([rec])
    parsed = json.loads(json_out)
    assert isinstance(parsed, list)
    assert len(parsed) == 1
    assert parsed[0]["record_id"] == rec.record_id


# --- Tests: JSONL round-trip ------------------------------------------------
def test_jsonl_roundtrip(tmp_ledger, perm_ledger):
    """JSONL serialization is lossless."""
    gate = v1344.lint_v1335_ledger_ci(perm_ledger)
    rec = v1345.record(gate, tmp_ledger)
    line = rec.to_jsonl()
    parsed = v1345.LedgerRecord.from_jsonl(line)
    assert parsed.record_id == rec.record_id
    assert parsed.ledger_hash == rec.ledger_hash
    assert parsed.tier_breakdown == rec.tier_breakdown


# --- Tests: integration with V1344 -----------------------------------------
def test_record_with_strict_gate(tmp_ledger, strict_ledger):
    """Recording a strict-gate result captures the FAIL state correctly."""
    gate = v1344.lint_v1335_ledger_ci(strict_ledger)
    rec = v1345.record(gate, tmp_ledger)
    assert rec.passed is False
    assert rec.exit_code == 1
    assert rec.violations_count > 0  # strict config produces violations


def test_real_v1344_then_v1345(tmp_ledger):
    """End-to-end: run real V1344 default gate, record, query."""
    gate = v1344.lint_v1335_ledger_ci()
    rec = v1345.record(gate, tmp_ledger)
    assert rec.coverage_current == gate.coverage.get("current", 0.0)
    assert rec.ledger_hash == gate.ledger_hash
    # Default gate should be FAIL (strict)
    assert rec.passed is False


# --- Tests: in-module Popper self-tests ------------------------------------
def test_v1345_self_tests():
    """In-module Popper self-tests must all pass."""
    results = v1345._self_test()
    assert all(results), f"V1345 self-tests failed: {results}"
    assert len(results) >= 12  # actual count is 13 (corrected from 12)


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))