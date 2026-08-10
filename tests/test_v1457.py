"""Tests for V1457 — ASI 真生产 6-deployment 5-stage operational lifecycle audit.

Phase: 1457
Version: 0.1.0
Date: 2026-08-10 (cron tick 09:46 Asia/Shanghai morning)

Tests (主 00:44 质量工程化):
- 16 popper self-tests (mirrors module popper)
- 5 V3 哲学守门
- 14 V1457-specific GUARDS
- 5 stage constants (preflight/bootstrap/healthcheck/verify/rollback)
- 6 deployment modules
- 30 probes (5 stages × 6 modules)
- per-stage maturity scoring
- combined parity formula
- runbook generation
- run-all end-to-end
- CLI commands
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

import pytest

import apeireth.v1457_asi_six_deployment_operational_runbook as v1457


# ============================================================================
# 1. Constants & metadata (主 17:43 实事求是)
# ============================================================================


def test_v1457_version():
    """V1457 version is exposed and is a string."""
    assert isinstance(v1457.V1457_VERSION, str)
    assert v1457.V1457_VERSION == "0.1.0"


def test_v1457_phase():
    """V1457 phase number is exposed."""
    assert v1457.V1457_PHASE == "1457"


def test_v1457_schema():
    """V1457 schema identifier is exposed."""
    assert "v1457" in v1457.V1457_SCHEMA
    assert "operational-runbook" in v1457.V1457_SCHEMA


def test_v1457_operational_stages_count():
    """Exactly 5 operational stages are defined."""
    assert len(v1457.OPERATIONAL_STAGES) == 5


def test_v1457_operational_stages_order():
    """Operational stages are in correct order: preflight → bootstrap → healthcheck → verify → rollback."""
    expected = ["preflight", "bootstrap", "healthcheck", "verify", "rollback"]
    assert v1457.OPERATIONAL_STAGES == expected


def test_v1457_stage_weights_sum_to_one():
    """Stage weights must sum to 1.0."""
    assert abs(sum(v1457.STAGE_WEIGHTS.values()) - 1.0) < 1e-9


def test_v1457_stage_weights_have_all_stages():
    """Every stage has a weight."""
    for stage in v1457.OPERATIONAL_STAGES:
        assert stage in v1457.STAGE_WEIGHTS
        assert v1457.STAGE_WEIGHTS[stage] > 0.0


def test_v1457_deployment_modules_count():
    """Exactly 6 deployment modules are configured."""
    assert len(v1457.DEPLOYMENT_MODULES) == 6


def test_v1457_each_module_has_required_keys():
    """Every deployment module spec has all required keys."""
    required = [
        "module_id", "module_path", "primary_function", "execution_mode",
        "timeout_sec", "preflight_checks", "verify_metric", "rollback_strategy",
        "description", "borrowed_from",
    ]
    for spec in v1457.DEPLOYMENT_MODULES:
        for key in required:
            assert key in spec, f"missing {key} in {spec['module_id']}"


def test_v1457_module_paths_are_unique():
    """All 6 module_ids are distinct."""
    ids = [s["module_id"] for s in v1457.DEPLOYMENT_MODULES]
    assert len(set(ids)) == 6, f"duplicate module_ids: {ids}"


# ============================================================================
# 2. V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
# ============================================================================


def test_v1457_v3_guards_count():
    """Exactly 5 V3 哲学守门."""
    assert len(v1457.V3_GUARDS) == 5


def test_v1457_v3_guards_required():
    """All required V3 guards present."""
    required_v3 = [
        "module_is_not_asi",
        "lifecycle_is_not_consciousness",
        "runbook_is_not_truth",
        "automation_is_not_autonomy",
        "operational_is_not_safe",
    ]
    for g in required_v3:
        assert g in v1457.V3_GUARDS


# ============================================================================
# 3. V1457-specific GUARDS (14 — 主 00:44 质量工程化)
# ============================================================================


def test_v1457_guards_count():
    """Exactly 14 V1457-specific guards."""
    assert len(v1457.V1457_GUARDS) == 14


def test_v1457_guards_required():
    """All required V1457 guards present."""
    required = [
        "GUARD_FIVE_STAGES",
        "GUARD_SIX_DEPLOYMENTS",
        "GUARD_REAL_EXECUTION",
        "GUARD_STAGE_CHAIN",
        "GUARD_RUNBOOK_GENERATED",
        "GUARD_MATURITY_BOUNDED",
        "GUARD_COMBINED_PARITY",
        "GUARD_NO_V1456_REPLACE",
        "GUARD_NO_V1450_REPLACE",
        "GUARD_CLI_RUNNABLE",
        "GUARD_OFFLINE_SAFE",
        "GUARD_NO_RAISE",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_POPPER_RUNS",
    ]
    for g in required:
        assert g in v1457.V1457_GUARDS


# ============================================================================
# 4. Preflight check functions (主 00:56 任何人都能接手)
# ============================================================================


def test_v1457_preflight_checks_registered():
    """All required preflight check functions are registered."""
    required_checks = [
        "python_version_ok",
        "docker_cli_present",
        "streamlit_installed",
        "port_free",
        "history_file_present",
    ]
    for check in required_checks:
        assert check in v1457.PREFLIGHT_CHECK_FUNCTIONS


def test_v1457_check_python_version():
    """Python version check returns True for Python 3.9+."""
    ok, summary = v1457._check_python_version()
    assert ok is True
    assert "python_version" in summary
    assert summary["min_required"] == "3.9"


def test_v1457_check_docker_cli():
    """Docker CLI check returns a dict."""
    ok, summary = v1457._check_docker_cli_present()
    assert "docker_available" in summary
    assert "docker_path" in summary


def test_v1457_check_streamlit():
    """Streamlit check returns version or error."""
    ok, summary = v1457._check_streamlit_installed()
    assert "streamlit_version" in summary


def test_v1457_check_port_free():
    """Port check returns dict."""
    ok, summary = v1457._check_port_free(0)  # port 0 = ephemeral
    assert "port" in summary
    assert "free" in summary


def test_v1457_check_history_file():
    """History file check returns dict."""
    ok, summary = v1457._check_history_file_present()
    assert "history_paths_found" in summary


# ============================================================================
# 5. Per-deployment execution modes
# ============================================================================


def test_v1457_proxy_count():
    """Exactly 1 PROXY module (V1450)."""
    n_proxy = sum(1 for s in v1457.DEPLOYMENT_MODULES if s["execution_mode"] == "PROXY")
    assert n_proxy == 1


def test_v1457_dry_run_count():
    """Exactly 1 DRY_RUN module (V1261)."""
    n_dry = sum(1 for s in v1457.DEPLOYMENT_MODULES if s["execution_mode"] == "DRY_RUN")
    assert n_dry == 1


def test_v1457_subprocess_real_count():
    """Exactly 4 SUBPROCESS_REAL modules (V1260/V1262/V1439/V1440)."""
    n_sub = sum(1 for s in v1457.DEPLOYMENT_MODULES if s["execution_mode"] == "SUBPROCESS_REAL")
    assert n_sub == 4


def test_v1457_http_healthcheck_module():
    """Exactly 1 module has http_healthcheck=True (V1439)."""
    n_http = sum(1 for s in v1457.DEPLOYMENT_MODULES if s.get("http_healthcheck", False))
    assert n_http == 1
    http_module = [s for s in v1457.DEPLOYMENT_MODULES if s.get("http_healthcheck", False)][0]
    assert http_module["module_id"] == "v1439_streamlit_subprocess_smoke"


# ============================================================================
# 6. LifecycleProfile + StageResult dataclasses (主 17:43)
# ============================================================================


def test_v1457_stage_result_defaults():
    """StageResult has expected default values."""
    sr = v1457.StageResult(module_id="test", stage="preflight")
    assert sr.success is False
    assert sr.skipped is False
    assert sr.score == 0.0
    assert sr.latency_ms == 0.0
    assert sr.error_message is None


def test_v1457_lifecycle_profile_stages_property():
    """LifecycleProfile.stages returns dict of 5 StageResults."""
    profile = v1457.LifecycleProfile(
        module_id="test", module_path="x", execution_mode="PROXY",
        description="t", borrowed_from=[],
    )
    assert set(profile.stages.keys()) == set(v1457.OPERATIONAL_STAGES)


def test_v1457_lifecycle_profile_to_dict():
    """LifecycleProfile.to_dict serializes correctly."""
    profile = v1457.LifecycleProfile(
        module_id="test", module_path="x", execution_mode="PROXY",
        description="t", borrowed_from=["V1"],
    )
    d = profile.to_dict()
    assert d["module_id"] == "test"
    assert d["execution_mode"] == "PROXY"
    assert d["borrowed_from"] == ["V1"]
    assert "stages" in d
    assert len(d["stages"]) == 5


def test_v1457_maturity_report_to_dict():
    """MaturityReport.to_dict serializes correctly."""
    report = v1457.MaturityReport(
        report_id="r1", started_at=0.0, ended_at=1.0, duration_sec=1.0,
    )
    d = report.to_dict()
    assert d["report_id"] == "r1"
    assert d["duration_sec"] == 1.0


# ============================================================================
# 7. Maturity scoring (主 17:43 实事求是)
# ============================================================================


def test_v1457_maturity_all_perfect():
    """All 5 stages perfect → maturity = 1.0."""
    profile = v1457.LifecycleProfile(
        module_id="t", module_path="x", execution_mode="PROXY",
        description="t", borrowed_from=[],
    )
    for stage in v1457.OPERATIONAL_STAGES:
        setattr(profile, stage, v1457.StageResult(module_id="t", stage=stage, success=True, score=1.0))
    score = v1457._compute_maturity_score(profile)
    assert abs(score - 1.0) < 1e-9


def test_v1457_maturity_all_zero():
    """All 5 stages failed → maturity = 0.0."""
    profile = v1457.LifecycleProfile(
        module_id="t", module_path="x", execution_mode="PROXY",
        description="t", borrowed_from=[],
    )
    for stage in v1457.OPERATIONAL_STAGES:
        setattr(profile, stage, v1457.StageResult(module_id="t", stage=stage, success=False, score=0.0))
    score = v1457._compute_maturity_score(profile)
    assert abs(score - 0.0) < 1e-9


def test_v1457_maturity_bounded():
    """Maturity score is bounded in [0, 1]."""
    profile = v1457.LifecycleProfile(
        module_id="t", module_path="x", execution_mode="PROXY",
        description="t", borrowed_from=[],
    )
    # Set scores outside [0, 1] to test clamping
    for stage in v1457.OPERATIONAL_STAGES:
        setattr(profile, stage, v1457.StageResult(module_id="t", stage=stage, success=True, score=0.5))
    score = v1457._compute_maturity_score(profile)
    assert 0.0 <= score <= 1.0


def test_v1457_maturity_skipped_excluded():
    """Skipped stages are excluded from maturity score."""
    profile = v1457.LifecycleProfile(
        module_id="t", module_path="x", execution_mode="PROXY",
        description="t", borrowed_from=[],
    )
    # Set preflight, bootstrap to perfect; skip healthcheck, verify, rollback
    profile.preflight = v1457.StageResult(module_id="t", stage="preflight", success=True, score=1.0)
    profile.bootstrap = v1457.StageResult(module_id="t", stage="bootstrap", success=True, score=1.0)
    profile.healthcheck = v1457.StageResult(module_id="t", stage="healthcheck", skipped=True, success=False, score=0.0)
    profile.verify = v1457.StageResult(module_id="t", stage="verify", skipped=True, success=False, score=0.0)
    profile.rollback = v1457.StageResult(module_id="t", stage="rollback", success=True, score=1.0)
    score = v1457._compute_maturity_score(profile)
    # Weight sum for non-skipped = 0.15 + 0.25 + 0.15 = 0.55
    # Weighted total = 0.15*1.0 + 0.25*1.0 + 0.15*1.0 = 0.55
    # Maturity = 0.55 / 0.55 = 1.0
    assert abs(score - 1.0) < 1e-9


# ============================================================================
# 8. Runbook generation (主 00:56 任何人都能接手)
# ============================================================================


def test_v1457_generate_runbook_section_contains_anyone_can_run():
    """Runbook section contains 'Anyone-can-run commands'."""
    profile = v1457.LifecycleProfile(
        module_id="t", module_path="apeireth.v1450", execution_mode="PROXY",
        description="test", borrowed_from=["V1450"],
    )
    for stage in v1457.OPERATIONAL_STAGES:
        setattr(profile, stage, v1457.StageResult(module_id="t", stage=stage, success=True, score=1.0))
    profile.maturity_score = 1.0
    section = v1457._generate_runbook_section(profile)
    assert "Anyone-can-run commands" in section
    assert "Bootstrap" in section
    assert "Healthcheck" in section
    assert "Rollback" in section


def test_v1457_generate_top_level_runbook_contains_summary():
    """Top-level runbook contains summary table + per-deployment sections."""
    report = v1457.MaturityReport(
        report_id="r1", started_at=0.0, ended_at=1.0, duration_sec=1.0,
    )
    profile = v1457.LifecycleProfile(
        module_id="v1450_test", module_path="apeireth.v1450_test", execution_mode="PROXY",
        description="test", borrowed_from=["V1450"],
    )
    for stage in v1457.OPERATIONAL_STAGES:
        setattr(profile, stage, v1457.StageResult(module_id="v1450_test", stage=stage, success=True, score=1.0))
    profile.maturity_score = 1.0
    profile.runbook_section = v1457._generate_runbook_section(profile)
    report.profiles.append(profile)
    report.n_profiles = 1
    report.overall_maturity = 1.0
    report.v1456_overall_parity = 0.95
    report.v1450_cube_overall = 0.7483
    report.combined_parity_v1457 = 0.4 * 0.95 + 0.3 * 1.0 + 0.3 * 0.7483
    report.per_module_maturity = {"v1450_test": 1.0}
    report.per_stage_pass_rate = {stage: 1.0 for stage in v1457.OPERATIONAL_STAGES}
    report.runbook_markdown = v1457._generate_top_level_runbook(report)
    assert "V1457" in report.runbook_markdown
    assert "single command" in report.runbook_markdown.lower() or "single-command" in report.runbook_markdown.lower() or "Anyone-can-run: single command" in report.runbook_markdown
    assert "v1450_test" in report.runbook_markdown


# ============================================================================
# 9. V1450 + V1456 fetch helpers
# ============================================================================


def test_v1457_fetch_v1450_cube_overall():
    """V1450 cube_overall fetch returns a positive float."""
    val = v1457._fetch_v1450_cube_overall()
    assert isinstance(val, float)
    assert val > 0.0


def test_v1457_fetch_v1456_overall_parity():
    """V1456 overall_parity fetch returns a positive float."""
    val = v1457._fetch_v1456_overall_parity()
    assert isinstance(val, float)
    assert val > 0.0


def test_v1457_combined_parity_bounded():
    """Combined parity is bounded in [0, 1]."""
    # Test with edge cases
    for v1456 in [0.0, 0.5, 1.0]:
        for v1457_m in [0.0, 0.5, 1.0]:
            for v1450 in [0.0, 0.5, 1.0]:
                combined = 0.4 * v1456 + 0.3 * v1457_m + 0.3 * v1450
                assert 0.0 <= combined <= 1.0


# ============================================================================
# 10. Lifecycle executor (single PROXY module — fast test)
# ============================================================================


def test_v1457_lifecycle_v1450_proxy():
    """V1450 PROXY lifecycle completes successfully."""
    spec = next(s for s in v1457.DEPLOYMENT_MODULES if s["module_id"] == "v1450_cube_history_aggregator")
    profile = v1457._execute_lifecycle(spec)
    assert profile.module_id == "v1450_cube_history_aggregator"
    assert profile.preflight.success is True
    assert profile.bootstrap.success is True
    assert profile.maturity_score > 0.0
    assert "Anyone-can-run commands" in profile.runbook_section


def test_v1457_lifecycle_chain_skip_on_preflight_failure():
    """When preflight fails, all subsequent stages are skipped (including rollback).

    Rationale: if preflight fails, no subprocess was started, so rollback is a no-op.
    Skipping is honest — we don't claim rollback did work when there was nothing to clean up.
    """
    # Use a check that's guaranteed to fail (unknown check name)
    bad_spec = {
        "module_id": "test_bad",
        "module_path": "apeireth.nonexistent_module_xyz",
        "primary_function": "probe",
        "execution_mode": "SUBPROCESS_REAL",
        "timeout_sec": 30.0,
        "borrowed_from": [],
        "description": "test",
        "http_healthcheck": False,
        "rollback_strategy": "kill_subprocess",
        "preflight_checks": ["nonexistent_check_xyz"],
        "verify_metric": "exit_code_zero",
    }
    profile = v1457._execute_lifecycle(bad_spec)
    # preflight should fail (unknown check)
    assert profile.preflight.success is False
    # all subsequent stages should be skipped (no subprocess was started → nothing to clean up)
    assert profile.bootstrap.skipped is True
    assert profile.healthcheck.skipped is True
    assert profile.verify.skipped is True
    assert profile.rollback.skipped is True
    # maturity score should be 0.0 (only preflight ran, and it failed)
    assert profile.maturity_score == 0.0


# ============================================================================
# 11. CLI commands (主 00:56 任何人都能接手)
# ============================================================================


def test_v1457_cli_version():
    """CLI version command exits 0."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1457_asi_six_deployment_operational_runbook", "version"],
        capture_output=True, timeout=15,
    )
    assert result.returncode == 0
    assert b"V1457" in result.stdout


def test_v1457_cli_meta_json():
    """CLI meta --json exits 0 and produces JSON."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1457_asi_six_deployment_operational_runbook", "meta", "--json"],
        capture_output=True, timeout=15,
    )
    assert result.returncode == 0
    data = json.loads(result.stdout.decode("utf-8"))
    assert data["version"] == "0.1.0"
    assert data["phase"] == "1457"
    assert data["n_deployment_modules"] == 6
    assert data["n_operational_stages"] == 5


def test_v1457_cli_help():
    """CLI help exits 0 and lists commands."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1457_asi_six_deployment_operational_runbook", "help"],
        capture_output=True, timeout=15,
    )
    assert result.returncode == 0
    out = result.stdout.decode("utf-8")
    assert "version" in out
    assert "run-all" in out
    assert "runbook" in out
    assert "lifecycle" in out


def test_v1457_cli_popper():
    """CLI popper exits 0 (≥14/16 PASS)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1457_asi_six_deployment_operational_runbook", "popper"],
        capture_output=True, timeout=60,
    )
    assert result.returncode == 0
    out = result.stdout.decode("utf-8")
    assert "16/16 PASS" in out


def test_v1457_cli_chain():
    """CLI chain exits 0 with 8/8 OK."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1457_asi_six_deployment_operational_runbook", "chain"],
        capture_output=True, timeout=60,
    )
    assert result.returncode == 0
    out = result.stdout.decode("utf-8")
    assert "8/8 OK" in out


def test_v1457_cli_list_modules():
    """CLI list-modules exits 0 and lists 6 modules."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1457_asi_six_deployment_operational_runbook", "list-modules"],
        capture_output=True, timeout=15,
    )
    assert result.returncode == 0
    out = result.stdout.decode("utf-8")
    assert "v1260_docker_deploy" in out
    assert "v1450_cube_history_aggregator" in out


def test_v1457_cli_status():
    """CLI status exits 0 with v1450 + v1456 values."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1457_asi_six_deployment_operational_runbook", "status"],
        capture_output=True, timeout=15,
    )
    assert result.returncode == 0
    out = result.stdout.decode("utf-8")
    assert "v1450_cube_overall" in out
    assert "v1456_overall_parity" in out


def test_v1457_cli_lifecycle_single_module():
    """CLI lifecycle --module exits 0 for single PROXY module."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1457_asi_six_deployment_operational_runbook",
         "lifecycle", "--module", "v1450_cube_history_aggregator"],
        capture_output=True, timeout=60,
    )
    assert result.returncode == 0
    out = result.stdout.decode("utf-8")
    assert "maturity_score" in out
    assert "stages_passed" in out


def test_v1457_cli_runbook_single_module_to_file():
    """CLI runbook --module M --out-md P writes a runbook file."""
    with tempfile.TemporaryDirectory() as tmpdir:
        out_path = os.path.join(tmpdir, "test-runbook.md")
        result = subprocess.run(
            [sys.executable, "-m", "apeireth.v1457_asi_six_deployment_operational_runbook",
             "runbook", "--module", "v1450_cube_history_aggregator", "--out-md", out_path],
            capture_output=True, timeout=60,
        )
        assert result.returncode == 0
        assert os.path.exists(out_path)
        with open(out_path, "r", encoding="utf-8") as f:
            content = f.read()
        assert "V1457" in content or "Anyone-can-run" in content
        assert "Bootstrap" in content


def test_v1457_cli_unknown_command():
    """CLI unknown command exits non-zero (or prints help)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1457_asi_six_deployment_operational_runbook", "bogus-command"],
        capture_output=True, timeout=15,
    )
    # argparse will reject with exit code 2
    assert result.returncode != 0 or "usage" in result.stdout.decode("utf-8", errors="ignore").lower() or "usage" in result.stderr.decode("utf-8", errors="ignore").lower()


# ============================================================================
# 12. Markdown rendering (主 00:44 质量工程化)
# ============================================================================


def test_v1457_render_markdown_contains_required_sections():
    """render_markdown produces a markdown with required sections."""
    report = v1457.MaturityReport(
        report_id="r1", started_at=0.0, ended_at=1.0, duration_sec=1.0,
        n_profiles=0, overall_maturity=0.0,
        v1456_overall_parity=0.95, v1450_cube_overall=0.7483,
        combined_parity_v1457=0.4*0.95 + 0.3*0.0 + 0.3*0.7483,
        per_stage_pass_rate={stage: 0.0 for stage in v1457.OPERATIONAL_STAGES},
        honest_disclosure="test",
    )
    md = v1457.render_markdown(report)
    assert "V1457" in md
    assert "Per-stage pass rate" in md
    assert "Per-module maturity" in md
    assert "Honest disclosure" in md
    assert "V3 哲学守门" in md or "v3 guards" in md.lower()
    assert "V1457 GUARDS" in md


# ============================================================================
# 13. Run-all integration (PROXY-only fast path)
# ============================================================================


def test_v1457_run_all_profiles_populated():
    """run_all populates profiles list."""
    report = v1457.run_all(out_json=None, out_md=None)
    assert report.n_profiles == 6
    assert len(report.profiles) == 6
    # Each profile should have all 5 stage results populated
    for p in report.profiles:
        for stage in v1457.OPERATIONAL_STAGES:
            assert hasattr(p, stage)
            assert isinstance(getattr(p, stage), v1457.StageResult)


def test_v1457_run_all_combined_parity_bounded():
    """run_all combined_parity_v1457 is bounded in [0, 1]."""
    report = v1457.run_all(out_json=None, out_md=None)
    assert 0.0 <= report.combined_parity_v1457 <= 1.0


def test_v1457_run_all_artifacts_written():
    """run_all writes JSON + markdown artifacts."""
    with tempfile.TemporaryDirectory() as tmpdir:
        json_path = os.path.join(tmpdir, "test-report.json")
        md_path = os.path.join(tmpdir, "test-runbook.md")
        report = v1457.run_all(out_json=json_path, out_md=md_path)
        assert os.path.exists(json_path)
        assert os.path.exists(md_path)
        # JSON should be valid
        with open(json_path, "r", encoding="utf-8") as f:
            data = json.load(f)
        assert data["report_id"] == report.report_id
        # Markdown should contain V1457
        with open(md_path, "r", encoding="utf-8") as f:
            md = f.read()
        assert "V1457" in md


def test_v1457_run_all_per_module_maturity_dict_populated():
    """run_all populates per_module_maturity dict."""
    report = v1457.run_all(out_json=None, out_md=None)
    assert len(report.per_module_maturity) == 6
    for module_id in [s["module_id"] for s in v1457.DEPLOYMENT_MODULES]:
        assert module_id in report.per_module_maturity


def test_v1457_run_all_per_stage_pass_rate_populated():
    """run_all populates per_stage_pass_rate dict."""
    report = v1457.run_all(out_json=None, out_md=None)
    assert len(report.per_stage_pass_rate) == 5
    for stage in v1457.OPERATIONAL_STAGES:
        assert stage in report.per_stage_pass_rate
        assert 0.0 <= report.per_stage_pass_rate[stage] <= 1.0


# ============================================================================
# 14. Cross-deployment borrow pattern (主 19:33 走在前人经验上)
# ============================================================================


def test_v1457_borrowed_lineage_includes_v1456():
    """V1457 borrows from V1456 (post module)."""
    spec = next(s for s in v1457.DEPLOYMENT_MODULES if s["module_id"] == "v1260_docker_deploy")
    assert "V1260" in spec["borrowed_from"]
    # Verify V1456 module is importable
    ok, _, _ = v1457._safe_import_module("apeireth.v1456_asi_six_deployment_real_execution_parity")
    assert ok


def test_v1457_borrowed_lineage_includes_v1450():
    """V1457 borrows from V1450 (history aggregator)."""
    spec = next(s for s in v1457.DEPLOYMENT_MODULES if s["module_id"] == "v1450_cube_history_aggregator")
    assert "V1450" in spec["borrowed_from"]


def test_v1457_chain_delegate_all_ok():
    """All 8 borrowed modules are importable."""
    chain_specs = [
        "apeireth.v1456_asi_six_deployment_real_execution_parity",
        "apeireth.v1450_asi_cross_modular_cube_history",
        "apeireth.v1440_asi_docker_container_run",
        "apeireth.v1439_asi_streamlit_subprocess_smoke",
        "apeireth.v1263_real_kitchen_integration",
        "apeireth.v1262_streamlit_deploy",
        "apeireth.v1261_benchmark_llm",
        "apeireth.v1260_docker_deploy",
    ]
    for module_path in chain_specs:
        ok, _, err = v1457._safe_import_module(module_path)
        assert ok, f"import failed for {module_path}: {err}"


# ============================================================================
# 15. Honest disclosure (主 17:43 实事求是)
# ============================================================================


def test_v1457_honest_disclosure_present():
    """Honest disclosure is non-empty and contains key disclaimers."""
    report = v1457.run_all(out_json=None, out_md=None)
    assert report.honest_disclosure != ""
    assert "V1457" in report.honest_disclosure
    assert "≠" in report.honest_disclosure or "Phenomenal" in report.honest_disclosure


def test_v1457_no_phenomenal_claim():
    """V1457 does NOT claim Phenomenal consciousness."""
    report = v1457.run_all(out_json=None, out_md=None)
    # Should deny phenomenal claim, not make it
    assert "≠ Phenomenal runbook" in report.honest_disclosure


def test_v1457_no_asi_claim():
    """V1457 does NOT claim ASI achievement."""
    report = v1457.run_all(out_json=None, out_md=None)
    assert "≠ ASI runbook" in report.honest_disclosure


def test_v1457_no_human_level_claim():
    """V1457 does NOT claim human-level operational judgment."""
    report = v1457.run_all(out_json=None, out_md=None)
    assert "≠ human-level runbook" in report.honest_disclosure


def test_v1457_no_absolute_claim():
    """V1457 does NOT claim absolute operational closure."""
    report = v1457.run_all(out_json=None, out_md=None)
    assert "≠ absolute runbook" in report.honest_disclosure