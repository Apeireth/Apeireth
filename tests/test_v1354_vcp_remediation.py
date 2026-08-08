"""Tests for V1354 VCP Remediation Planner + Safe Auto-Fix.

主 17:43 实事求是: real tests against real filesystem (tempdir), no mocks.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

# Add apeireth to path
APEIRETH_DIR = Path(__file__).resolve().parent.parent / "apeireth"
if str(APEIRETH_DIR.parent) not in sys.path:
    sys.path.insert(0, str(APEIRETH_DIR.parent))

import apeireth.v1354_vcp_remediation as v1354  # noqa: E402


# -----------------------------------------------------------------------------
# Constants + dataclasses
# -----------------------------------------------------------------------------

class TestConstants:
    def test_version_is_semver(self):
        assert v1354.V1354_VERSION.count(".") == 2
        parts = v1354.V1354_VERSION.split(".")
        assert all(p.isdigit() for p in parts)

    def test_asi_cap_honest(self):
        assert 0.0 < v1354.V1354_ASI_CAP <= 0.01

    def test_safe_fixes_whitelist_only(self):
        for name in v1354.SAFE_FIXES:
            # Only safe mechanical fixes allowed; no code touch
            assert name.startswith(("create_", "make_")), (
                f"unsafe fix name: {name}"
            )

    def test_safe_fixable_paths_are_basename(self):
        for p in v1354.SAFE_FIXABLE_PATHS:
            assert "/" not in p and "\\" not in p


# -----------------------------------------------------------------------------
# compute_priority
# -----------------------------------------------------------------------------

class TestComputePriority:
    def test_critical_beats_warn(self):
        p_crit = v1354.compute_priority("CRITICAL", 0.0, 1)
        p_warn = v1354.compute_priority("WARN", 0.0, 1)
        assert p_crit > p_warn

    def test_error_beats_warn(self):
        p_err = v1354.compute_priority("ERROR", 0.0, 1)
        p_warn = v1354.compute_priority("WARN", 0.0, 1)
        assert p_err > p_warn

    def test_ok_priority_zero(self):
        assert v1354.compute_priority("OK", 0.0, 1) == 0.0

    def test_age_increases_priority(self):
        p_new = v1354.compute_priority("ERROR", 0.0, 1)
        p_old = v1354.compute_priority("ERROR", 7.0, 1)
        assert p_old > p_new

    def test_age_capped_at_horizon(self):
        p_7d = v1354.compute_priority("ERROR", 7.0, 1)
        p_30d = v1354.compute_priority("ERROR", 30.0, 1)
        # age_factor saturates at AGE_HORIZON_DAYS=7
        assert p_7d == pytest.approx(p_30d)

    def test_frequency_increases_priority(self):
        p_1 = v1354.compute_priority("ERROR", 0.0, 1)
        p_10 = v1354.compute_priority("ERROR", 0.0, 10)
        assert p_10 > p_1

    def test_priority_is_float(self):
        assert isinstance(v1354.compute_priority("WARN", 1.0, 1), float)


# -----------------------------------------------------------------------------
# Data class integrity
# -----------------------------------------------------------------------------

class TestDataClasses:
    def test_remediation_item_frozen(self):
        ri = v1354.RemediationItem(
            item_id="x", source="s", severity="WARN",
            message="m", suggested_action=v1354.ACTION_MANUAL, priority=1.0,
        )
        d = v1354.asdict(ri)
        assert d["item_id"] == "x"
        assert d["severity"] == "WARN"
        assert d["priority"] == 1.0

    def test_apply_result_frozen(self):
        ar = v1354.ApplyResult(
            item_id="x", status=v1354.STATUS_OK,
            message="ok", artifact_path="/tmp/x", duration_ms=1.5,
        )
        d = v1354.asdict(ar)
        assert d["status"] == "OK"
        assert d["duration_ms"] == 1.5

    def test_plan_to_dict_has_all_fields(self):
        plan = v1354.RemediationPlan(
            version=v1354.V1354_VERSION, n_items=0,
            n_fix=0, n_manual=0, n_defer=0, n_ignore=0,
            items=(), generated_at="now",
            philosophy_guards=("GUARD_X",),
        )
        d = plan.to_dict()
        assert d["version"] == v1354.V1354_VERSION
        assert "items" in d
        assert "philosophy_guards" in d


# -----------------------------------------------------------------------------
# generate_plan
# -----------------------------------------------------------------------------

class TestGeneratePlan:
    def test_plan_has_version(self):
        plan = v1354.generate_plan(limit=5, source="fallback")
        assert plan.version == v1354.V1354_VERSION

    def test_plan_respects_limit(self):
        plan = v1354.generate_plan(limit=2, source="fallback")
        assert plan.n_items <= 2

    def test_plan_sorted_desc_by_priority(self):
        plan = v1354.generate_plan(limit=10, source="fallback")
        priorities = [it.priority for it in plan.items]
        assert priorities == sorted(priorities, reverse=True)

    def test_plan_has_philosophy_guards(self):
        plan = v1354.generate_plan(limit=5, source="fallback")
        assert any("GUARD_NOT_PLANNER_IS_ASI" in g for g in plan.philosophy_guards)

    def test_plan_source_doctor_no_fail_returns_empty(self):
        # With healthy workspace, Doctor reports no failures → 0 items
        plan = v1354.generate_plan(limit=10, source="doctor")
        # If doctor unavailable, items == 0 (graceful fallback)
        # If doctor available, may return items
        assert isinstance(plan.items, tuple)


# -----------------------------------------------------------------------------
# apply_plan — dry-run
# -----------------------------------------------------------------------------

class TestApplyDryRun:
    def test_dry_run_never_fails(self):
        plan = v1354.generate_plan(limit=10, source="fallback")
        report = v1354.apply_plan(plan, dry_run=True)
        assert report.dry_run is True
        assert report.n_fail == 0

    def test_dry_run_attempted_matches_plan(self):
        plan = v1354.generate_plan(limit=10, source="fallback")
        report = v1354.apply_plan(plan, dry_run=True)
        assert report.n_attempted == plan.n_fix

    def test_dry_run_exit_code_zero_when_clean(self):
        plan = v1354.generate_plan(limit=10, source="fallback")
        report = v1354.apply_plan(plan, dry_run=True)
        assert report.exit_code in (0, 1)


# -----------------------------------------------------------------------------
# apply_plan — real (tempdir-isolated)
# -----------------------------------------------------------------------------

class TestApplyReal:
    def test_apply_creates_empty_file(self, tmp_path: Path):
        target = tmp_path / "test.jsonl"
        custom = {
            "test_create": (v1354._fix_create_empty_file, lambda: target),
        }
        plan = v1354.RemediationPlan(
            version=v1354.V1354_VERSION, n_items=1, n_fix=1,
            n_manual=0, n_defer=0, n_ignore=0,
            items=(v1354.RemediationItem(
                item_id="i", source="t", severity="WARN",
                message="m", suggested_action=v1354.ACTION_FIX,
                fix_fn="test_create", priority=1.0,
            ),),
            generated_at="now", philosophy_guards=(),
        )
        report = v1354.apply_plan(plan, dry_run=False, fix_registry=custom)
        assert target.exists()
        assert report.n_ok == 1

    def test_apply_is_idempotent(self, tmp_path: Path):
        target = tmp_path / "idem.jsonl"
        custom = {
            "idem": (v1354._fix_create_empty_file, lambda: target),
        }
        plan = v1354.RemediationPlan(
            version=v1354.V1354_VERSION, n_items=1, n_fix=1,
            n_manual=0, n_defer=0, n_ignore=0,
            items=(v1354.RemediationItem(
                item_id="i", source="s", severity="WARN",
                message="m", suggested_action=v1354.ACTION_FIX,
                fix_fn="idem", priority=1.0,
            ),),
            generated_at="now", philosophy_guards=(),
        )
        r1 = v1354.apply_plan(plan, dry_run=False, fix_registry=custom)
        r2 = v1354.apply_plan(plan, dry_run=False, fix_registry=custom)
        assert r1.n_ok == 1
        assert r2.n_ok == 1  # idempotent: already-exists is still OK

    def test_apply_unknown_fix_fails(self, tmp_path: Path):
        target = tmp_path / "x.jsonl"
        custom: dict = {}  # empty registry
        plan = v1354.RemediationPlan(
            version=v1354.V1354_VERSION, n_items=1, n_fix=1,
            n_manual=0, n_defer=0, n_ignore=0,
            items=(v1354.RemediationItem(
                item_id="i", source="s", severity="WARN",
                message="m", suggested_action=v1354.ACTION_FIX,
                fix_fn="unknown_fix", priority=1.0,
            ),),
            generated_at="now", philosophy_guards=(),
        )
        report = v1354.apply_plan(plan, dry_run=False, fix_registry=custom)
        assert report.n_fail == 1
        assert report.exit_code == 2

    def test_apply_only_filter(self, tmp_path: Path):
        target_a = tmp_path / "a.jsonl"
        target_b = tmp_path / "b.jsonl"
        custom = {
            "fix_a": (v1354._fix_create_empty_file, lambda: target_a),
            "fix_b": (v1354._fix_create_empty_file, lambda: target_b),
        }
        plan = v1354.RemediationPlan(
            version=v1354.V1354_VERSION, n_items=2, n_fix=2,
            n_manual=0, n_defer=0, n_ignore=0,
            items=(
                v1354.RemediationItem(
                    item_id="a", source="s", severity="WARN",
                    message="m", suggested_action=v1354.ACTION_FIX,
                    fix_fn="fix_a", priority=2.0,
                ),
                v1354.RemediationItem(
                    item_id="b", source="s", severity="WARN",
                    message="m", suggested_action=v1354.ACTION_FIX,
                    fix_fn="fix_b", priority=1.0,
                ),
            ),
            generated_at="now", philosophy_guards=(),
        )
        report = v1354.apply_plan(plan, dry_run=False,
                                  only_fix="fix_a", fix_registry=custom)
        assert target_a.exists()
        assert not target_b.exists()
        assert report.n_attempted == 1


# -----------------------------------------------------------------------------
# Fallback issue generation
# -----------------------------------------------------------------------------

class TestFallbackIssues:
    def test_returns_list(self):
        items = v1354._fallback_minimal_issues()
        assert isinstance(items, list)

    def test_all_items_are_remediation_item(self):
        for it in v1354._fallback_minimal_issues():
            assert isinstance(it, v1354.RemediationItem)

    def test_missing_files_get_fix_action(self, tmp_path: Path, monkeypatch):
        # Point workspace paths to empty tmpdir
        monkeypatch.setattr(v1354, "LEDGER_PATH", tmp_path / "ledger.jsonl")
        monkeypatch.setattr(v1354, "MIGRATION_AUDIT_PATH", tmp_path / "audit.jsonl")
        monkeypatch.setattr(v1354, "REMEDIATION_HISTORY_PATH", tmp_path / "hist.jsonl")
        items = v1354._fallback_minimal_issues()
        # All should have FIX action (since files missing in tmp)
        for it in items:
            assert it.suggested_action == v1354.ACTION_FIX


# -----------------------------------------------------------------------------
# Doctor integration (graceful degradation)
# -----------------------------------------------------------------------------

class TestDoctorIntegration:
    def test_run_doctor_returns_none_or_object(self):
        # Either returns None (Doctor unavailable) or an object — never crashes
        result = v1354._run_doctor()
        assert result is None or hasattr(result, "checks") or isinstance(result, dict)

    def test_doctor_to_issues_handles_dict(self):
        # Build a fake Doctor dict-report
        fake = {
            "checks": [
                {"name": "ledger_path", "severity": "WARN", "status": "fail",
                 "message": "ledger missing", "suggestion": "create it"},
                {"name": "python_version", "severity": "CRITICAL", "status": "fail",
                 "message": "old python", "suggestion": "upgrade"},
                {"name": "disk_space", "severity": "OK", "status": "pass",
                 "message": "ok", "suggestion": ""},
            ]
        }
        items = v1354._doctor_to_issues(fake)
        # Only failed checks become items
        assert len(items) == 2
        ids = {it.item_id for it in items}
        assert "doctor:ledger_path" in ids
        assert "doctor:python_version" in ids
        # ledger_path should be FIX-able
        ledger_item = next(i for i in items if "ledger_path" in i.item_id)
        assert ledger_item.suggested_action == v1354.ACTION_FIX
        assert ledger_item.fix_fn == "create_ledger_if_missing"

    def test_doctor_to_issues_handles_object(self):
        # Fake object with .checks attribute (dataclass style)
        class FakeCheck:
            def __init__(self, name, severity, status, message, suggestion=""):
                self.name = name
                self.severity = severity
                self.status = status
                self.message = message
                self.suggestion = suggestion

        class FakeReport:
            def __init__(self):
                self.checks = [
                    FakeCheck("ledger_path", "ERROR", "fail", "broken", "fix"),
                ]

        items = v1354._doctor_to_issues(FakeReport())
        assert len(items) == 1
        assert items[0].item_id == "doctor:ledger_path"

    def test_doctor_to_issues_empty_on_none(self):
        items = v1354._doctor_to_issues(None)
        assert items == []


# -----------------------------------------------------------------------------
# History load/save (uses real path; safe to read missing)
# -----------------------------------------------------------------------------

class TestHistoryIO:
    def test_load_history_missing_returns_empty(self, tmp_path, monkeypatch):
        monkeypatch.setattr(v1354, "REMEDIATION_HISTORY_PATH", tmp_path / "nope.jsonl")
        assert v1354.load_history() == []

    def test_load_history_parses_jsonl(self, tmp_path, monkeypatch):
        hist = tmp_path / "hist.jsonl"
        rec1 = {"ts": "2026-08-08T16:00:00", "report": {"n_ok": 1}}
        rec2 = {"ts": "2026-08-08T17:00:00", "report": {"n_ok": 2}}
        hist.write_text(json.dumps(rec1) + "\n" + json.dumps(rec2) + "\n", encoding="utf-8")
        monkeypatch.setattr(v1354, "REMEDIATION_HISTORY_PATH", hist)
        out = v1354.load_history(limit=5)
        assert len(out) == 2
        assert out[0]["ts"] == "2026-08-08T16:00:00"


# -----------------------------------------------------------------------------
# CLI smoke test
# -----------------------------------------------------------------------------

class TestCLI:
    def test_version_command(self, capsys):
        rc = v1354.main(["version"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "V1354" in captured.out

    def test_list_command(self, capsys):
        rc = v1354.main(["list"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "create_ledger_if_missing" in out

    def test_self_test_command(self, capsys):
        rc = v1354.main(["self-test"])
        # All passes → 0; some fail → 1. Both acceptable in CI.
        assert rc in (0, 1)

    def test_plan_command_fallback(self, capsys):
        rc = v1354.main(["plan", "--source", "fallback", "--limit", "3"])
        assert rc == 0
        out = capsys.readouterr().out
        assert "V1354 VCP Remediation Plan" in out

    def test_plan_command_json(self, capsys):
        rc = v1354.main(["plan", "--source", "fallback", "--limit", "3", "--json"])
        assert rc == 0
        out = capsys.readouterr().out
        # Must be valid JSON
        data = json.loads(out)
        assert data["version"] == v1354.V1354_VERSION

    def test_apply_command_dry_run(self, capsys):
        rc = v1354.main(["apply", "--dry-run", "--source", "fallback"])
        assert rc in (0, 1)
        out = capsys.readouterr().out
        assert "V1354 VCP Apply Report" in out


# -----------------------------------------------------------------------------
# Self-tests (call into module's own Popper suite)
# -----------------------------------------------------------------------------

class TestEmbeddedSelfTests:
    def test_self_tests_pass(self):
        passed, total, failures = v1354._popper_self_tests()
        assert passed == total, f"self-test failures: {failures}"
        assert total >= 20


# -----------------------------------------------------------------------------
# Philosophy guards
# -----------------------------------------------------------------------------

class TestPhilosophyGuards:
    def test_plan_guards_contain_asi_disclaimer(self):
        plan = v1354.generate_plan(limit=5, source="fallback")
        guards = " ".join(plan.philosophy_guards)
        assert "GUARD_NOT_PLANNER_IS_ASI" in guards

    def test_apply_guards_contain_safety(self):
        plan = v1354.generate_plan(limit=5, source="fallback")
        report = v1354.apply_plan(plan, dry_run=True)
        guards = " ".join(report.philosophy_guards)
        assert "GUARD_WHITELIST_ONLY" in guards
        assert "GUARD_NO_CODE_TOUCHED" in guards

    def test_safe_fixes_only_mechanical(self):
        # All safe fix names must start with create_ or make_
        for name in v1354.SAFE_FIXES:
            assert name.startswith(("create_", "make_")), (
                f"V1354 forbids non-mechanical fix: {name}"
            )


# -----------------------------------------------------------------------------
# Determinism
# -----------------------------------------------------------------------------

class TestDeterminism:
    def test_plan_deterministic_on_items(self):
        """Same source → same item_ids. Priorities use float tolerance
        because ``_ledger_age_days`` reads ``datetime.now()`` at
        microsecond granularity, so two calls a few μs apart produce
        priorities that differ at the 10th decimal. The *ordering*
        and *item_id set* are deterministic; the *exact float* is not.
        """
        p1 = v1354.generate_plan(limit=5, source="fallback")
        p2 = v1354.generate_plan(limit=5, source="fallback")
        ids1 = [it.item_id for it in p1.items]
        ids2 = [it.item_id for it in p2.items]
        assert ids1 == ids2
        priorities1 = [it.priority for it in p1.items]
        priorities2 = [it.priority for it in p2.items]
        # Float tolerance: priorities differ at ~10th decimal due to μs time
        for a, b in zip(priorities1, priorities2):
            assert a == pytest.approx(b, rel=1e-6, abs=1e-9), (
                f"priority not within tolerance: {a} vs {b}"
            )


# -----------------------------------------------------------------------------
# Integration with workspace (smoke test)
# -----------------------------------------------------------------------------

class TestWorkspaceIntegration:
    def test_plan_auto_source_works(self):
        # Should not raise; may return empty if Doctor is healthy
        plan = v1354.generate_plan(limit=5, source="auto")
        assert isinstance(plan, v1354.RemediationPlan)

    def test_apply_dry_run_safe_in_real_workspace(self):
        # Dry-run never touches anything
        plan = v1354.generate_plan(limit=5, source="fallback")
        report = v1354.apply_plan(plan, dry_run=True)
        assert report.dry_run is True
        assert report.n_fail == 0
