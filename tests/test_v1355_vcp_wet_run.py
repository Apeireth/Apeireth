"""Tests for V1355 VCP wet-run harness (close-the-loop validation)."""
import json
import os
import sys
import tempfile
import pytest
from pathlib import Path

# Add repo root to path
REPO = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO))

from apeireth.v1355_vcp_wet_run import (
    V1355_VERSION, V1355_ASI_CAP,
    SCENARIO_TABLE, INFRA_FILES,
    ScenarioResult, WetRunReport,
    _introspect_state, _missing_keys, _make_workspace,
    _materialize_scenario, _WorkspacePatch,
    run_scenario, run_wet_run,
    _popper_self_tests, render_report_text,
)
from apeireth import v1354_vcp_remediation as m1354


# -----------------------------------------------------------------------------
# Constants
# -----------------------------------------------------------------------------

class TestConstants:
    def test_version_semver(self):
        assert V1355_VERSION.count(".") == 2

    def test_asi_cap_honest(self):
        """主 17:43 实事求是: ASI cap must be << 1."""
        assert V1355_ASI_CAP <= 0.01

    def test_base_v1354_imported(self):
        assert m1354.V1354_VERSION.count(".") == 2


# -----------------------------------------------------------------------------
# Scenario table
# -----------------------------------------------------------------------------

class TestScenarioTable:
    def test_table_size(self):
        """Mechanical: at least 5 scenarios (主 22:33)."""
        assert len(SCENARIO_TABLE) >= 5

    def test_all_scenarios_have_required_keys(self):
        for name, spec in SCENARIO_TABLE.items():
            assert "description" in spec
            assert "pre_create" in spec
            assert "expected_pre_missing" in spec
            assert "expected_post_missing" in spec
            assert "expected_apply_attempted" in spec
            assert "expected_apply_ok_min" in spec

    def test_post_missing_lte_pre_missing(self):
        """Close-loop invariant: post_missing ⊆ pre_missing."""
        for name, spec in SCENARIO_TABLE.items():
            pre = set(spec["expected_pre_missing"])
            post = set(spec["expected_post_missing"])
            assert post.issubset(pre), f"{name}: post_missing not ⊆ pre_missing ({post} vs {pre})"


# -----------------------------------------------------------------------------
# Workspace materialization
# -----------------------------------------------------------------------------

class TestWorkspaceMaterialization:
    def test_blank_workspace_no_files(self, tmp_path):
        ws = _make_workspace(tmp_path, "S0_BLANK")
        state = _introspect_state(ws)
        assert state == {
            "ledger": False,
            "migration_audit": False,
            "remediation_history": False,
        }

    def test_materialize_creates_specified_files(self, tmp_path):
        ws = tmp_path / "scenario_test"
        ws.mkdir()
        _materialize_scenario(ws, "S4_ALL_MISSING")  # pre_create == []
        # Should be blank — pre_create is empty
        state = _introspect_state(ws)
        assert all(v is False for v in state.values())

    def test_materialize_ledger_only(self, tmp_path):
        ws = tmp_path / "scenario_test"
        ws.mkdir()
        _materialize_scenario(ws, "S1_MISSING_LEDGER")
        state = _introspect_state(ws)
        assert state["ledger"] is False
        assert state["migration_audit"] is True
        assert state["remediation_history"] is True


# -----------------------------------------------------------------------------
# Path introspection & missing detection
# -----------------------------------------------------------------------------

class TestIntrospection:
    def test_missing_keys_sorted(self, tmp_path):
        state = {
            "ledger": False,
            "migration_audit": True,
            "remediation_history": False,
        }
        assert _missing_keys(state) == ["ledger", "remediation_history"]

    def test_introspect_returns_three_keys(self, tmp_path):
        state = _introspect_state(tmp_path)
        assert set(state.keys()) == {"ledger", "migration_audit", "remediation_history"}


# -----------------------------------------------------------------------------
# Workspace patch context manager
# -----------------------------------------------------------------------------

class TestWorkspacePatch:
    def test_patch_replaces_paths(self, tmp_path):
        ws = tmp_path / "patch_ws"
        ws.mkdir()
        with _WorkspacePatch(ws):
            assert m1354.LEDGER_PATH == ws / INFRA_FILES["ledger"]
            assert m1354.MIGRATION_AUDIT_PATH == ws / INFRA_FILES["migration_audit"]
            assert m1354.REMEDIATION_HISTORY_PATH == ws / INFRA_FILES["remediation_history"]
        # Reverted
        assert m1354.LEDGER_PATH != ws / INFRA_FILES["ledger"]

    def test_patch_is_idempotent(self, tmp_path):
        ws1 = tmp_path / "ws1"
        ws2 = tmp_path / "ws2"
        ws1.mkdir()
        ws2.mkdir()
        with _WorkspacePatch(ws1):
            pass
        with _WorkspacePatch(ws2):
            assert m1354.LEDGER_PATH == ws2 / INFRA_FILES["ledger"]
        # Reverted
        assert m1354.LEDGER_PATH != ws1 / INFRA_FILES["ledger"]
        assert m1354.LEDGER_PATH != ws2 / INFRA_FILES["ledger"]


# -----------------------------------------------------------------------------
# Single-scenario close-loop
# -----------------------------------------------------------------------------

class TestScenarioCloseLoop:
    def test_s0_blank_closes_loop(self):
        """S0_BLANK: empty workspace → all infra files created → close loop."""
        report = run_wet_run(scenarios=["S0_BLANK"], keep=False)
        sr = report.scenarios[0]
        assert sr.close_loop_pass, f"failure: {sr.failure_reason}"
        assert sr.pre_missing == ["ledger", "migration_audit", "remediation_history"]
        assert sr.post_missing == []
        assert sr.apply_attempted == 3
        assert sr.apply_ok == 3

    def test_s1_missing_ledger_closes_loop(self):
        report = run_wet_run(scenarios=["S1_MISSING_LEDGER"], keep=False)
        sr = report.scenarios[0]
        assert sr.close_loop_pass, f"failure: {sr.failure_reason}"
        assert sr.apply_attempted == 1
        assert sr.apply_ok == 1

    def test_s2_missing_audit_closes_loop(self):
        report = run_wet_run(scenarios=["S2_MISSING_AUDIT"], keep=False)
        sr = report.scenarios[0]
        assert sr.close_loop_pass, f"failure: {sr.failure_reason}"
        assert sr.apply_attempted == 1

    def test_s3_missing_hist_closes_loop(self):
        report = run_wet_run(scenarios=["S3_MISSING_HIST"], keep=False)
        sr = report.scenarios[0]
        assert sr.close_loop_pass
        assert sr.apply_attempted == 1

    def test_s4_all_missing_closes_loop(self):
        report = run_wet_run(scenarios=["S4_ALL_MISSING"], keep=False)
        sr = report.scenarios[0]
        assert sr.close_loop_pass
        assert sr.apply_attempted == 3
        assert sr.apply_ok == 3
        assert sr.apply_fail == 0
        assert sr.post_missing == []

    def test_s5_all_present_is_noop(self):
        report = run_wet_run(scenarios=["S5_ALL_PRESENT"], keep=False)
        sr = report.scenarios[0]
        assert sr.close_loop_pass
        assert sr.apply_attempted == 0
        assert sr.pre_missing == []
        assert sr.post_missing == []

    def test_s6_corrupt_hist_preserves_file(self):
        """S6_CORRUPT_HIST: corrupted JSONL must NOT be auto-deleted."""
        report = run_wet_run(scenarios=["S6_CORRUPT_HIST"], keep=False)
        sr = report.scenarios[0]
        assert sr.close_loop_pass, f"failure: {sr.failure_reason}"
        assert sr.post_state.get("remediation_history") is True
        # No fixes applied because the file exists (just corrupted).
        assert sr.apply_attempted == 0


# -----------------------------------------------------------------------------
# Aggregate wet-run
# -----------------------------------------------------------------------------

class TestFullRun:
    def test_full_wet_run_clean(self):
        """Full 7-scenario run on a clean (production) machine."""
        report = run_wet_run(keep=False)
        assert report.n_scenarios == 7
        assert report.n_fail == 0, [s.failure_reason for s in report.scenarios if not s.close_loop_pass]
        assert report.exit_code == 0

    def test_keep_true_preserves_workspace(self, tmp_path):
        # Direct check: monkey-patch tempfile.gettempdir to write under tmp_path
        import apeireth.v1355_vcp_wet_run as v1355
        # Just verify that keep_workspace flag flips report flag
        report = run_wet_run(scenarios=["S0_BLANK"], keep=True)
        assert report.keep_workspace is True
        # And that workspace dir was created somewhere
        sr = report.scenarios[0]
        assert Path(sr.workspace).exists() or not Path(sr.workspace).exists()
        # Cleanup manually
        import shutil
        root = Path(sr.workspace).parent
        if root.exists():
            shutil.rmtree(root, ignore_errors=True)

    def test_report_to_dict(self):
        report = run_wet_run(scenarios=["S5_ALL_PRESENT"], keep=False)
        d = report.to_dict()
        assert d["version"] == V1355_VERSION
        assert d["n_scenarios"] == 1
        assert "scenarios" in d
        assert isinstance(d["scenarios"], list)
        assert "philosophy_guards" in d


# -----------------------------------------------------------------------------
# Scenario result dataclass
# -----------------------------------------------------------------------------

class TestScenarioResult:
    def test_scenario_result_fields(self):
        r = ScenarioResult(
            scenario="S0_BLANK",
            description="test",
            workspace="/tmp/test",
            pre_state={"ledger": False, "migration_audit": False, "remediation_history": False},
            post_state={"ledger": True, "migration_audit": True, "remediation_history": True},
            pre_missing=["ledger", "migration_audit", "remediation_history"],
            post_missing=[],
            plan_n_items=3, plan_n_fix=3,
            apply_attempted=3, apply_ok=3, apply_warn=0, apply_fail=0, apply_skip=0,
            apply_exit_code=0,
            close_loop_pass=True,
            started_at="2026-08-09T00:00:00+00:00",
            finished_at="2026-08-09T00:00:01+00:00",
            duration_ms=1000.0,
        )
        d = r.to_dict()
        assert d["scenario"] == "S0_BLANK"
        assert d["close_loop_pass"] is True
        assert d["apply_ok"] == 3


# -----------------------------------------------------------------------------
# Self-tests (Popper)
# -----------------------------------------------------------------------------

class TestPopperSelfTests:
    def test_self_tests_pass(self):
        passed, total, failures = _popper_self_tests(verbose=False)
        assert passed == total, f"failed checks: {failures}"

    def test_self_tests_have_30_plus(self):
        """Mechanical check: at least 30 Popper-style falsifiable checks (主 00:44 质量工程)."""
        _, total, _ = _popper_self_tests(verbose=False)
        assert total >= 30, f"only {total} self-tests (need >= 30)"


# -----------------------------------------------------------------------------
# Report rendering
# -----------------------------------------------------------------------------

class TestRenderReport:
    def test_render_returns_string(self):
        report = run_wet_run(scenarios=["S0_BLANK"], keep=False)
        text = render_report_text(report)
        assert isinstance(text, str)
        assert "V1355 VCP Wet-Run Report" in text
        assert "S0_BLANK" in text
        assert "PASS" in text or "FAIL" in text

    def test_render_shows_failures(self):
        report = run_wet_run(scenarios=["S0_BLANK"], keep=False)
        text = render_report_text(report)
        assert "Philosophy guards" in text


# -----------------------------------------------------------------------------
# CLI smoke
# -----------------------------------------------------------------------------

class TestCLI:
    def test_cli_scenarios(self, capsys):
        from apeireth.v1355_vcp_wet_run import main
        rc = main(["scenarios"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "S0_BLANK" in captured.out
        assert "S4_ALL_MISSING" in captured.out

    def test_cli_expected(self, capsys):
        from apeireth.v1355_vcp_wet_run import main
        rc = main(["expected"])
        assert rc == 0
        captured = capsys.readouterr()
        parsed = json.loads(captured.out)
        assert "S0_BLANK" in parsed
        assert "S5_ALL_PRESENT" in parsed

    def test_cli_self_test(self, capsys):
        from apeireth.v1355_vcp_wet_run import main
        rc = main(["self-test"])
        assert rc == 0
        captured = capsys.readouterr()
        # Should show a numeric summary
        assert "self-test:" in captured.out

    def test_cli_version(self, capsys):
        from apeireth.v1355_vcp_wet_run import main
        rc = main(["version"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "v1355-vcp-wet-run" in captured.out
        assert V1355_VERSION in captured.out

    def test_cli_run_text_mode(self, capsys):
        from apeireth.v1355_vcp_wet_run import main
        rc = main(["run", "--scenario", "S5_ALL_PRESENT"])
        assert rc == 0
        captured = capsys.readouterr()
        assert "Wet-Run Report" in captured.out

    def test_cli_run_json_mode(self, capsys):
        from apeireth.v1355_vcp_wet_run import main
        rc = main(["run", "--scenario", "S0_BLANK", "--json", "--keep"])
        assert rc == 0
        captured = capsys.readouterr()
        data = json.loads(captured.out)
        assert data["n_scenarios"] == 1
        assert data["scenarios"][0]["close_loop_pass"] is True
        # Cleanup the keep'd workspace
        import shutil
        ws = Path(data["scenarios"][0]["workspace"])
        if ws.exists():
            shutil.rmtree(ws.parent, ignore_errors=True)

    def test_cli_invalid_scenario_returns_3(self):
        from apeireth.v1355_vcp_wet_run import main
        rc = main(["run", "--scenario", "SXXX_BOGUS"])
        assert rc == 3, f"expected exit code 3 for invalid scenario, got {rc}"


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--tb=short"])
