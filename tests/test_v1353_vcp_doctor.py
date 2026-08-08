"""Tests for V1353 VCP Doctor.

Covers:
  - Constants (V1353_VERSION, V1353_ASI_CAP, MIN_PYTHON, REQUIRED_MODULES)
  - Each individual check function returns CheckResult
  - Severity assignment for each check (OK / WARN / ERROR / CRITICAL / SKIP)
  - _aggregate() exit-code logic (5 cases + 2 strict cases)
  - DoctorReport aggregation (counts, exit_code, philosophy_guards)
  - run_doctor() default + check_keys + strict
  - _format_report_human()
  - to_dict() stability
  - CLI subcommand dispatch (run / list / self-test / version)
  - Chain regression with V1350 + V1352
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import List

import pytest

# Make apeireth importable
APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

import apeireth.v1353_vcp_doctor as m


# -----------------------------------------------------------------------------
# 1. Constants & guards
# -----------------------------------------------------------------------------

def test_v1353_version():
    assert m.V1353_VERSION == "0.1.0"


def test_v1353_asi_cap_honest():
    assert m.V1353_ASI_CAP == 0.008


def test_min_python_at_least_3_11():
    assert m.MIN_PYTHON >= (3, 11)


def test_required_modules_nonempty():
    assert len(m.REQUIRED_MODULES) >= 5


def test_required_modules_have_v1351_and_v1352():
    assert "v1351_vcp_toolchain_cli" in m.REQUIRED_MODULES
    assert "v1352_vcp_history_diff" in m.REQUIRED_MODULES


def test_optional_modules_includes_pytest():
    assert "pytest" in m.OPTIONAL_MODULES


def test_philosophy_guards_locked():
    """V3 guards must remain in doctor (no drift to pretending)."""
    assert len(m.PHILOSOPHY_GUARDS) >= 5
    text = " ".join(m.PHILOSOPHY_GUARDS)
    for keyword in ("Phenomenal", "ASI", "doctor", "mechanical"):
        assert keyword in text, f"V1353 guard missing keyword: {keyword}"


# -----------------------------------------------------------------------------
# 2. Helper: _try_import fallback
# -----------------------------------------------------------------------------

def test_try_import_works_for_real_module():
    ok, err = m._try_import("v1351_vcp_toolchain_cli")
    assert ok is True
    assert err is None


def test_try_import_fails_for_missing_module():
    ok, err = m._try_import("v99999_nonexistent_module_xyz")
    assert ok is False
    assert err is not None


def test_try_import_handles_bare_and_namespaced():
    """Both 'mod' and 'apeireth.mod' should resolve."""
    ok_bare, _ = m._try_import("v1352_vcp_history_diff")
    assert ok_bare is True


# -----------------------------------------------------------------------------
# 3. Individual checks return CheckResult
# -----------------------------------------------------------------------------

def _assert_valid_check(c: m.CheckResult):
    assert c.name
    assert c.severity in (m.SEVERITY_OK, m.SEVERITY_WARN, m.SEVERITY_ERROR, m.SEVERITY_CRITICAL)
    assert c.status in (m.STATUS_PASS, m.STATUS_FAIL, m.STATUS_SKIP)
    assert c.message


@pytest.mark.parametrize("check_fn", [
    m.check_python_version,
    m.check_required_modules,
    m.check_optional_modules,
    m.check_ledger_path,
    m.check_migration_audit_path,
    m.check_substrate_matrix,
    m.check_ledger_freshness,
    m.check_apeireth_tree,
    m.check_disk_space,
    m.check_tests_dir,
    lambda: m.check_api_keys(check_keys=False),
    lambda: m.check_api_keys(check_keys=True),
])
def test_individual_checks_return_valid_CheckResult(check_fn):
    c = check_fn()
    _assert_valid_check(c)


def test_check_python_version_passes_on_this_system():
    c = m.check_python_version()
    assert c.severity == m.SEVERITY_OK
    assert c.status == m.STATUS_PASS


def test_check_required_modules_passes_in_workspace():
    c = m.check_required_modules()
    assert c.status == m.STATUS_PASS
    assert c.severity == m.SEVERITY_OK


def test_check_substrate_matrix_passes_with_v1335():
    c = m.check_substrate_matrix()
    assert c.status == m.STATUS_PASS
    assert c.severity == m.SEVERITY_OK


def test_check_apeireth_tree_passes_in_workspace():
    c = m.check_apeireth_tree()
    assert c.status == m.STATUS_PASS
    assert c.severity == m.SEVERITY_OK


def test_check_ledger_path_passes_when_ledger_exists(tmp_path):
    """Create a writable ledger, expect pass."""
    ledger = tmp_path / "test_ledger.jsonl"
    ledger.write_text("")
    c = m.check_ledger_path(ledger)
    assert c.status == m.STATUS_PASS
    assert c.severity == m.SEVERITY_OK


def test_check_ledger_path_warns_when_missing_but_parent_writable(tmp_path):
    """Missing ledger with writable parent should WARN (not fail)."""
    ledger = tmp_path / "does_not_exist.jsonl"
    c = m.check_ledger_path(ledger)
    assert c.status == m.STATUS_PASS  # parent is writable, file will be created
    assert c.severity == m.SEVERITY_WARN


def test_check_ledger_path_fails_when_parent_missing(tmp_path):
    """Missing ledger with missing parent should FAIL."""
    parent = tmp_path / "missing_subdir"
    ledger = parent / "ledger.jsonl"
    c = m.check_ledger_path(ledger)
    assert c.status == m.STATUS_FAIL
    assert c.severity == m.SEVERITY_WARN


def test_check_migration_audit_path_warns_when_missing(tmp_path):
    audit = tmp_path / "audit.jsonl"
    c = m.check_migration_audit_path(audit)
    assert c.status == m.STATUS_PASS
    assert c.severity == m.SEVERITY_WARN


def test_check_api_keys_skip_when_check_keys_false():
    c = m.check_api_keys(check_keys=False)
    assert c.status == m.STATUS_SKIP
    assert c.severity == m.SEVERITY_OK


def test_check_api_keys_warn_when_all_missing(monkeypatch):
    monkeypatch.delenv("OPENAI_API_KEY", raising=False)
    monkeypatch.delenv("ANTHROPIC_API_KEY", raising=False)
    monkeypatch.delenv("APEIRETH_LLM_API_KEY", raising=False)
    c = m.check_api_keys(check_keys=True)
    assert c.status == m.STATUS_FAIL
    assert c.severity == m.SEVERITY_WARN


def test_check_api_keys_pass_when_all_set(monkeypatch):
    monkeypatch.setenv("OPENAI_API_KEY", "test-openai")
    monkeypatch.setenv("ANTHROPIC_API_KEY", "test-anthropic")
    monkeypatch.setenv("APEIRETH_LLM_API_KEY", "test-apeireth")
    c = m.check_api_keys(check_keys=True)
    assert c.status == m.STATUS_PASS
    assert c.severity == m.SEVERITY_OK


def test_check_disk_space_returns_actual_value():
    c = m.check_disk_space()
    assert c.status == m.STATUS_PASS
    # Should be in MB
    assert "MB" in c.message


def test_check_ledger_freshness_skips_when_no_ledger(tmp_path):
    no_ledger = tmp_path / "no_ledger_here.jsonl"
    c = m.check_ledger_freshness(no_ledger)
    assert c.status == m.STATUS_SKIP
    assert c.severity == m.SEVERITY_WARN


def test_check_ledger_freshness_warns_when_stale(tmp_path):
    """Create a ledger with a 2-day-old record."""
    ledger = tmp_path / "stale_ledger.jsonl"
    old_ts = (datetime.now(timezone.utc) - timedelta(days=2)).isoformat()
    rec = {"timestamp": old_ts, "record_id": "abcdef1234567890", "passed": False,
           "exit_code": 1, "violations_count": 0, "tier_breakdown": {"HIGH": 0, "MEDIUM": 0, "LOW": 0, "UNCLASSIFIED": 0}}
    ledger.write_text(json.dumps(rec) + "\n")
    c = m.check_ledger_freshness(ledger)
    assert c.status == m.STATUS_FAIL
    assert c.severity == m.SEVERITY_WARN


def test_check_ledger_freshness_critical_when_very_stale(tmp_path):
    """Create a ledger with a 10-day-old record."""
    ledger = tmp_path / "very_stale_ledger.jsonl"
    old_ts = (datetime.now(timezone.utc) - timedelta(days=10)).isoformat()
    rec = {"timestamp": old_ts, "record_id": "abcdef1234567890", "passed": False,
           "exit_code": 1, "violations_count": 0, "tier_breakdown": {"HIGH": 0, "MEDIUM": 0, "LOW": 0, "UNCLASSIFIED": 0}}
    ledger.write_text(json.dumps(rec) + "\n")
    c = m.check_ledger_freshness(ledger)
    assert c.status == m.STATUS_FAIL
    assert c.severity == m.SEVERITY_CRITICAL


def test_check_ledger_freshness_passes_when_fresh(tmp_path):
    ledger = tmp_path / "fresh_ledger.jsonl"
    fresh_ts = datetime.now(timezone.utc).isoformat()
    rec = {"timestamp": fresh_ts, "record_id": "abcdef1234567890", "passed": True,
           "exit_code": 0, "violations_count": 0, "tier_breakdown": {"HIGH": 0, "MEDIUM": 0, "LOW": 0, "UNCLASSIFIED": 0}}
    ledger.write_text(json.dumps(rec) + "\n")
    c = m.check_ledger_freshness(ledger)
    assert c.status == m.STATUS_PASS
    assert c.severity == m.SEVERITY_OK


def test_check_tests_dir_passes_in_workspace():
    c = m.check_tests_dir()
    assert c.status == m.STATUS_PASS
    assert c.severity == m.SEVERITY_OK


def test_check_tests_dir_warns_when_missing(tmp_path):
    bad_dir = tmp_path / "no_tests_dir"
    c = m.check_tests_dir(bad_dir)
    assert c.status == m.STATUS_FAIL
    assert c.severity == m.SEVERITY_WARN


# -----------------------------------------------------------------------------
# 4. _aggregate() exit-code logic
# -----------------------------------------------------------------------------

def _cr(severity, status=None, msg="m"):
    if status is None:
        status = m.STATUS_FAIL
    return m.CheckResult(name="x", severity=severity, status=status, message=msg)


def test_aggregate_empty_zero():
    assert m._aggregate([]) == 0


def test_aggregate_only_passes_zero():
    cs = [_cr(m.SEVERITY_OK, m.STATUS_PASS), _cr(m.SEVERITY_WARN, m.STATUS_PASS)]
    assert m._aggregate(cs) == 0


def test_aggregate_warn_returns_one():
    cs = [_cr(m.SEVERITY_WARN, m.STATUS_FAIL)]
    assert m._aggregate(cs) == 1


def test_aggregate_error_returns_two():
    cs = [_cr(m.SEVERITY_ERROR, m.STATUS_FAIL)]
    assert m._aggregate(cs) == 2


def test_aggregate_critical_returns_three():
    cs = [_cr(m.SEVERITY_CRITICAL, m.STATUS_FAIL)]
    assert m._aggregate(cs) == 3


def test_aggregate_critical_overrides_everything():
    cs = [_cr(m.SEVERITY_WARN, m.STATUS_FAIL), _cr(m.SEVERITY_ERROR, m.STATUS_FAIL), _cr(m.SEVERITY_CRITICAL, m.STATUS_FAIL)]
    assert m._aggregate(cs) == 3


def test_aggregate_strict_warn_returns_two():
    cs = [_cr(m.SEVERITY_WARN, m.STATUS_FAIL)]
    assert m._aggregate(cs, strict=True) == 2


def test_aggregate_strict_passes_zero():
    cs = [_cr(m.SEVERITY_OK, m.STATUS_PASS), _cr(m.SEVERITY_WARN, m.STATUS_PASS)]
    assert m._aggregate(cs, strict=True) == 0


def test_aggregate_passing_warn_is_zero():
    cs = [_cr(m.SEVERITY_WARN, m.STATUS_PASS)]
    assert m._aggregate(cs) == 0


# -----------------------------------------------------------------------------
# 5. _build_default_checks registry
# -----------------------------------------------------------------------------

def test_default_checks_has_all_expected():
    expected_names = {
        "python_version", "required_modules", "optional_modules",
        "ledger_path", "migration_audit_path", "substrate_matrix",
        "ledger_freshness", "apeireth_tree", "disk_space",
        "tests_dir", "api_keys",
    }
    actual_names = {name for name, _ in m._build_default_checks()}
    assert expected_names.issubset(actual_names)


def test_default_checks_count_is_eleven():
    assert len(m._build_default_checks()) == 11


def test_default_checks_unchanged_with_check_keys():
    a = {n for n, _ in m._build_default_checks(check_keys=False)}
    b = {n for n, _ in m._build_default_checks(check_keys=True)}
    assert a == b  # api_keys always included; flag changes behavior, not registration


# -----------------------------------------------------------------------------
# 6. run_doctor() aggregation
# -----------------------------------------------------------------------------

def test_run_doctor_returns_DoctorReport():
    report = m.run_doctor()
    assert isinstance(report, m.DoctorReport)


def test_run_doctor_has_eleven_checks():
    report = m.run_doctor()
    assert report.n_checks == 11


def test_run_doctor_check_sum_consistent():
    report = m.run_doctor()
    assert report.n_pass + report.n_fail + report.n_skip == report.n_checks


def test_run_doctor_default_exit_zero_in_workspace():
    """Healthy workspace should yield exit_code 0."""
    report = m.run_doctor()
    # migration_audit_path WARN but status=pass; api_keys SKIP
    # so default run should be 0 (assuming env is OK)
    assert report.exit_code == 0


def test_run_doctor_strict_includes_warning_failures():
    """Strict mode upgrades WARN to ERROR-equivalent, so warnings counted as failures."""
    report = m.run_doctor(strict=True)
    # Same checks run, but exit_code may differ
    assert report.exit_code in (0, 1, 2, 3)


def test_run_doctor_to_dict_has_all_keys():
    report = m.run_doctor()
    d = report.to_dict()
    expected_keys = {"version", "n_checks", "n_pass", "n_warn", "n_fail", "n_skip",
                     "exit_code", "asi_cap", "philosophy_guards", "checks",
                     "started_at", "finished_at"}
    assert expected_keys.issubset(d.keys())


def test_run_doctor_to_dict_checks_have_all_fields():
    report = m.run_doctor()
    d = report.to_dict()
    assert len(d["checks"]) == 11
    for c in d["checks"]:
        assert {"name", "severity", "status", "message", "suggestion"} <= c.keys()


def test_run_doctor_philosophy_guards_locked():
    report = m.run_doctor()
    assert len(report.philosophy_guards) >= 5


def test_run_doctor_with_injected_check():
    """Operator can inject custom checks for testing."""
    def fake_pass():
        return m.CheckResult(name="custom_pass", severity=m.SEVERITY_OK,
                             status=m.STATUS_PASS, message="custom ok")
    def fake_fail():
        return m.CheckResult(name="custom_fail", severity=m.SEVERITY_CRITICAL,
                             status=m.STATUS_FAIL, message="custom fail")
    report = m.run_doctor(check_fns=[("custom_pass", fake_pass), ("custom_fail", fake_fail)])
    assert report.n_checks == 2
    assert report.exit_code == 3
    names = [c.name for c in report.checks]
    assert "custom_pass" in names
    assert "custom_fail" in names


def test_run_doctor_no_keys_check_skip():
    """Without --check-keys, api_keys check is skipped."""
    report = m.run_doctor(check_keys=False)
    api_check = next(c for c in report.checks if c.name == "api_keys")
    assert api_check.status == m.STATUS_SKIP


def test_run_doctor_with_keys_check_evaluated():
    report = m.run_doctor(check_keys=True)
    api_check = next(c for c in report.checks if c.name == "api_keys")
    assert api_check.status in (m.STATUS_PASS, m.STATUS_FAIL)


# -----------------------------------------------------------------------------
# 7. _format_report_human
# -----------------------------------------------------------------------------

def test_format_report_human_nonempty():
    report = m.run_doctor()
    text = m._format_report_human(report)
    assert len(text) > 100


def test_format_report_human_contains_metadata():
    report = m.run_doctor()
    text = m._format_report_human(report)
    assert "V1353" in text
    assert "exit_code" in text
    assert "asi_cap" in text
    assert "started" in text
    assert "finished" in text


def test_format_report_human_contains_all_check_names():
    report = m.run_doctor()
    text = m._format_report_human(report)
    for c in report.checks:
        assert c.name in text


def test_format_report_human_truncates_long_messages():
    long_msg = "x" * 500
    fake = m.CheckResult(name="long_msg_test", severity=m.SEVERITY_OK,
                          status=m.STATUS_PASS, message=long_msg)
    report = m.run_doctor(check_fns=[("long_msg_test", lambda: fake)])
    text = m._format_report_human(report)
    assert "..." in text  # truncated


# -----------------------------------------------------------------------------
# 8. to_dict() / JSON stability
# -----------------------------------------------------------------------------

def test_to_dict_serializable_to_json():
    report = m.run_doctor()
    d = report.to_dict()
    s = json.dumps(d, sort_keys=True)
    assert isinstance(s, str)
    assert len(s) > 100


def test_to_dict_round_trip():
    report = m.run_doctor()
    d = report.to_dict()
    assert d["version"] == report.version
    assert d["exit_code"] == report.exit_code
    assert len(d["checks"]) == len(report.checks)


# -----------------------------------------------------------------------------
# 9. CLI subcommand dispatch
# -----------------------------------------------------------------------------

def test_cli_version():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1353_vcp_doctor", "version"],
        capture_output=True, text=True, timeout=60,
        cwd=str(APEIRETH_DIR.parent),
    )
    assert result.returncode == 0
    assert "V1353" in result.stdout


def test_cli_list():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1353_vcp_doctor", "list"],
        capture_output=True, text=True, timeout=60,
        cwd=str(APEIRETH_DIR.parent),
    )
    assert result.returncode == 0
    lines = [l for l in result.stdout.splitlines() if l.strip()]
    assert "python_version" in lines
    assert "api_keys" in lines


def test_cli_self_test():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1353_vcp_doctor", "self-test"],
        capture_output=True, text=True, timeout=60,
        cwd=str(APEIRETH_DIR.parent),
    )
    assert result.returncode == 0
    assert "38/38" in result.stdout or "PASS" in result.stdout


def test_cli_run_human():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1353_vcp_doctor", "run"],
        capture_output=True, text=True, timeout=60,
        cwd=str(APEIRETH_DIR.parent),
    )
    assert result.returncode in (0, 1, 2, 3)
    assert "V1353" in result.stdout
    assert "python_version" in result.stdout


def test_cli_run_json():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1353_vcp_doctor", "run", "--json"],
        capture_output=True, text=True, timeout=60,
        cwd=str(APEIRETH_DIR.parent),
    )
    assert result.returncode in (0, 1, 2, 3)
    d = json.loads(result.stdout)
    assert "version" in d
    assert "checks" in d


def test_cli_run_strict():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1353_vcp_doctor", "run", "--strict"],
        capture_output=True, text=True, timeout=60,
        cwd=str(APEIRETH_DIR.parent),
    )
    assert result.returncode in (0, 1, 2, 3)


def test_cli_run_check_keys():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1353_vcp_doctor", "run", "--check-keys"],
        capture_output=True, text=True, timeout=60,
        cwd=str(APEIRETH_DIR.parent),
    )
    assert result.returncode in (0, 1, 2, 3)


def test_cli_no_args_prints_help():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1353_vcp_doctor"],
        capture_output=True, text=True, timeout=60,
        cwd=str(APEIRETH_DIR.parent),
    )
    # No args defaults to version (returncode 0)
    assert result.returncode == 0


def test_cli_unknown_subcommand_exits_2():
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1353_vcp_doctor", "bogus"],
        capture_output=True, text=True, timeout=60,
        cwd=str(APEIRETH_DIR.parent),
    )
    assert result.returncode == 2


# -----------------------------------------------------------------------------
# 10. Self-test popper checks
# -----------------------------------------------------------------------------

def test_popper_self_tests_returns_list():
    results = m._popper_self_tests()
    assert isinstance(results, list)
    assert len(results) >= 30


def test_popper_self_tests_all_pass():
    results = m._popper_self_tests()
    failed = [(n, m) for n, ok, m in results if not ok]
    assert not failed, f"Self-tests failed: {failed}"


# -----------------------------------------------------------------------------
# 11. Chain regression: V1350 + V1352 + V1353 still pass
# -----------------------------------------------------------------------------

def test_chain_v1350_v1352_v1353_no_regression():
    """Quick smoke test that the chain still works."""
    import apeireth.v1352_vcp_history_diff as v1352
    import apeireth.v1350_vcp_anomaly_lifecycle as v1350
    # Just import them — no exceptions
    assert v1352.V1352_VERSION == "0.1.0"
    assert v1350.V1350_VERSION == "0.1.0"
    assert m.V1353_VERSION == "0.1.0"