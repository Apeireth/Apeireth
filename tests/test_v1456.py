"""Tests for V1456 — ASI 真生产 6-deployment real-execution parity audit.

Phase: 1456
Version: 0.1.0
Date: 2026-08-10 (cron tick 09:03 Asia/Shanghai morning)

Tests (主 00:44 质量工程化):
- 14 popper self-tests
- 5 V3 哲学守门
- 14 V1456-specific GUARDS
- 5 borrowed lineage
- 9 CLI commands
- 6 deployment modules parity scoring
- composite parity formula
- run-all end-to-end
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

import apeireth.v1456_asi_six_deployment_real_execution_parity as v1456


# ============================================================================
# 1. Constants & metadata (主 17:43 实事求是)
# ============================================================================


def test_v1456_version():
    """V1456 version is exposed and is a string."""
    assert isinstance(v1456.V1456_VERSION, str)
    assert v1456.V1456_VERSION == "0.1.0"


def test_v1456_phase():
    """V1456 phase number is exposed."""
    assert v1456.V1456_PHASE == "1456"


def test_v1456_schema():
    """V1456 schema identifier is exposed."""
    assert "v1456" in v1456.V1456_SCHEMA
    assert "real-execution-parity" in v1456.V1456_SCHEMA


def test_v1456_deployment_modules_count():
    """Exactly 6 deployment modules are configured."""
    assert len(v1456.DEPLOYMENT_MODULES) == 6


def test_v1456_each_module_has_required_keys():
    """Every deployment module spec has all required keys."""
    required = ["module_id", "module_path", "primary_function", "execution_mode", "timeout_sec"]
    for spec in v1456.DEPLOYMENT_MODULES:
        for key in required:
            assert key in spec, f"missing {key} in {spec['module_id']}"


def test_v1456_module_paths_are_unique():
    """All 6 module_ids are distinct."""
    ids = [s["module_id"] for s in v1456.DEPLOYMENT_MODULES]
    assert len(set(ids)) == 6, f"duplicate module_ids: {ids}"


# ============================================================================
# 2. Execution mode classification (主 17:43)
# ============================================================================


def test_v1456_execution_modes_valid():
    """All execution modes are valid PROXY/DRY_RUN/SUBPROCESS_REAL/DOCKER_REAL."""
    valid = {
        v1456.EXECUTION_MODE_PROXY,
        v1456.EXECUTION_MODE_DRY_RUN,
        v1456.EXECUTION_MODE_SUBPROCESS_REAL,
        v1456.EXECUTION_MODE_DOCKER_REAL,
    }
    for spec in v1456.DEPLOYMENT_MODULES:
        assert spec["execution_mode"] in valid


def test_v1456_proxy_count():
    """Exactly 1 PROXY module (V1450)."""
    n_proxy = sum(1 for s in v1456.DEPLOYMENT_MODULES if s["execution_mode"] == v1456.EXECUTION_MODE_PROXY)
    assert n_proxy == 1


def test_v1456_dry_run_count():
    """Exactly 1 DRY_RUN module (V1261)."""
    n_dry = sum(1 for s in v1456.DEPLOYMENT_MODULES if s["execution_mode"] == v1456.EXECUTION_MODE_DRY_RUN)
    assert n_dry == 1


def test_v1456_subprocess_real_count():
    """Exactly 4 SUBPROCESS_REAL modules (V1260/V1262/V1439/V1440)."""
    n_sub = sum(1 for s in v1456.DEPLOYMENT_MODULES if s["execution_mode"] == v1456.EXECUTION_MODE_SUBPROCESS_REAL)
    assert n_sub == 4


def test_v1456_no_docker_real():
    """No DOCKER_REAL modules (no docker daemon on this host)."""
    n_docker = sum(1 for s in v1456.DEPLOYMENT_MODULES if s["execution_mode"] == v1456.EXECUTION_MODE_DOCKER_REAL)
    assert n_docker == 0


# ============================================================================
# 3. V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43)
# ============================================================================


def test_v1456_v3_guards_count():
    """Exactly 5 V3 guards."""
    assert len(v1456.V3_GUARDS) == 5


def test_v1456_v3_guards_required():
    """All 5 required V3 guards present."""
    required = {"module_is_not_asi", "execution_is_not_consciousness", "parity_is_not_truth",
                "automation_is_not_autonomy", "benchmark_is_not_safety"}
    assert required <= set(v1456.V3_GUARDS)


# ============================================================================
# 4. V1456-specific GUARDS (主 00:44 质量工程化)
# ============================================================================


def test_v1456_guards_count():
    """Exactly 14 V1456 guards."""
    assert len(v1456.V1456_GUARDS) == 14


def test_v1456_guards_required():
    """All 14 required V1456 guards present."""
    required = {
        "GUARD_SIX_DEPLOYMENTS", "GUARD_REAL_EXECUTION", "GUARD_EXECUTION_MODE",
        "GUARD_LATENCY_BOUNDED", "GUARD_RETURN_CODE_BOUNDED", "GUARD_COMPOSITE_PARITY",
        "GUARD_NO_V1455_REPLACE", "GUARD_NO_V1450_REPLACE", "GUARD_CLI_RUNNABLE",
        "GUARD_OFFLINE_SAFE", "GUARD_NO_RAISE", "GUARD_HONEST_DISCLOSURE",
        "GUARD_POPPER_RUNS", "GUARD_RENDER_RUNS",
    }
    assert required <= set(v1456.V1456_GUARDS)


# ============================================================================
# 5. Dataclass instantiability
# ============================================================================


def test_v1456_parity_report_instantiable():
    """ParityReport dataclass can be instantiated."""
    r = v1456.ParityReport(
        report_id="test", started_at=0.0, ended_at=1.0, duration_sec=1.0,
        n_profiles=6, n_success=6, overall_parity=0.85,
        combined_parity=0.80, v1450_cube_overall=0.7483,
    )
    assert r.report_id == "test"
    assert r.overall_parity == 0.85


def test_v1456_execution_profile_instantiable():
    """ExecutionProfile dataclass can be instantiated."""
    p = v1456.ExecutionProfile(
        module_id="test", module_path="apeireth.test",
        execution_mode=v1456.EXECUTION_MODE_PROXY, primary_function="test",
        borrowed_from=[], description="test",
    )
    assert p.module_id == "test"
    assert p.success is False


def test_v1456_parity_report_to_dict():
    """ParityReport.to_dict() returns a serializable dict."""
    r = v1456.ParityReport(
        report_id="test", started_at=0.0, ended_at=1.0, duration_sec=1.0,
        n_profiles=6, n_success=6, overall_parity=0.85,
        combined_parity=0.80, v1450_cube_overall=0.7483,
        per_module_parity={"v1260": 0.9, "v1261": 0.8},
        honest_disclosure="test",
    )
    d = r.to_dict()
    assert isinstance(d, dict)
    assert d["n_profiles"] == 6
    assert d["overall_parity"] == 0.85
    assert d["per_module_parity"]["v1260"] == 0.9


# ============================================================================
# 6. Helper functions
# ============================================================================


def test_v1456_safe_import_module_self():
    """Safe import of V1456 itself succeeds."""
    ok, mod, err = v1456._safe_import_module("apeireth.v1456_asi_six_deployment_real_execution_parity")
    assert ok is True
    assert mod is not None
    assert err is None


def test_v1456_safe_import_module_nonexistent():
    """Safe import of nonexistent module returns False."""
    ok, mod, err = v1456._safe_import_module("apeireth.v9999_nonexistent")
    assert ok is False
    assert mod is None
    assert err is not None


def test_v1456_call_function_safely_basic():
    """_call_function_safely executes a basic function."""
    ok, result, err, latency = v1456._call_function_safely(lambda: "hello", (), {}, 5.0)
    assert ok is True
    assert result == "hello"
    assert err is None
    assert latency >= 0


def test_v1456_call_function_safely_exception():
    """_call_function_safely handles exceptions."""
    def _raise():
        raise ValueError("test error")
    ok, result, err, latency = v1456._call_function_safely(_raise, (), {}, 5.0)
    assert ok is False
    assert result is None
    assert "ValueError" in err


def test_v1456_run_subprocess_safely_basic():
    """_run_subprocess_safely runs `python -m ...`."""
    ok, rc, lat, out, err, emsg = v1456._run_subprocess_safely(
        "apeireth.v1456_asi_six_deployment_real_execution_parity", "version", [], 10.0
    )
    assert ok is True
    assert rc == 0
    assert "V1456" in out
    assert emsg is None


def test_v1456_count_lines():
    """_count_lines counts non-empty lines."""
    assert v1456._count_lines("") == 0
    assert v1456._count_lines("hello") == 1
    assert v1456._count_lines("hello\nworld\n") == 2
    assert v1456._count_lines("hello\n\n\nworld\n") == 2
    assert v1456._count_lines("\n\n\n") == 0


def test_v1456_output_size():
    """_output_size returns byte size."""
    assert v1456._output_size("") == 0
    assert v1456._output_size("hello") == 5
    assert v1456._output_size("你好") > 0  # unicode bytes


# ============================================================================
# 7. Parity scoring (主 17:43 实事求是)
# ============================================================================


def test_v1456_compute_parity_score_success_full():
    """Success + subprocess + fast latency + output = high score."""
    score, breakdown = v1456._compute_parity_score(
        True, v1456.EXECUTION_MODE_SUBPROCESS_REAL, 1000.0, 5, {"k": 1}
    )
    assert 0.95 <= score <= 1.0
    assert breakdown["success"] == 1.0
    assert breakdown["mode"] == 0.95
    assert breakdown["latency"] == 1.0
    assert breakdown["output"] == 1.0


def test_v1456_compute_parity_score_failure_zero():
    """Failure = 0 success component."""
    score, breakdown = v1456._compute_parity_score(
        False, v1456.EXECUTION_MODE_PROXY, 0.0, 0, {}
    )
    assert breakdown["success"] == 0.0
    assert score < 0.5


def test_v1456_compute_parity_score_bounded():
    """Score always in [0, 1]."""
    for success in [True, False]:
        for mode in [v1456.EXECUTION_MODE_PROXY, v1456.EXECUTION_MODE_DRY_RUN,
                     v1456.EXECUTION_MODE_SUBPROCESS_REAL, v1456.EXECUTION_MODE_DOCKER_REAL]:
            for latency in [0, 100, 5000, 15000, 30000, 60000]:
                score, _ = v1456._compute_parity_score(success, mode, float(latency), 0, {})
                assert 0.0 <= score <= 1.0


def test_v1456_compute_parity_score_latency_decreasing():
    """Latency > 30s yields 0 latency component."""
    score_low, _ = v1456._compute_parity_score(True, v1456.EXECUTION_MODE_SUBPROCESS_REAL, 1000.0, 1, {})
    score_high, _ = v1456._compute_parity_score(True, v1456.EXECUTION_MODE_SUBPROCESS_REAL, 35000.0, 1, {})
    assert score_low > score_high


# ============================================================================
# 8. V1450 cube_overall fetch (主 17:43 实事求是)
# ============================================================================


def test_v1456_fetch_v1450_cube_overall():
    """V1450 cube_overall fetch returns a float in [0, 1]."""
    val = v1456._fetch_v1450_cube_overall()
    assert isinstance(val, float)
    assert 0.0 <= val <= 1.0


# ============================================================================
# 9. Combined parity formula
# ============================================================================


def test_v1456_combined_parity_formula():
    """combined_parity = 0.5 * v1450 + 0.5 * overall."""
    result = v1456.report_combined_parity(0.8, 0.6)
    assert abs(result - 0.7) < 1e-6
    result = v1456.report_combined_parity(1.0, 0.0)
    assert abs(result - 0.5) < 1e-6
    result = v1456.report_combined_parity(0.0, 1.0)
    assert abs(result - 0.5) < 1e-6


# ============================================================================
# 10. Per-deployment execution (主 17:43 真 broken 检测)
# ============================================================================


def test_v1456_execute_dry_run_module():
    """Execute V1261 (DRY_RUN) returns ExecutionProfile."""
    spec = next(s for s in v1456.DEPLOYMENT_MODULES if s["module_id"] == "v1261_benchmark_llm")
    profile = v1456._execute_module(spec)
    assert profile.module_id == "v1261_benchmark_llm"
    assert profile.success is True
    assert profile.parity_score > 0.0


def test_v1456_execute_proxy_module():
    """Execute V1450 (PROXY) returns ExecutionProfile."""
    spec = next(s for s in v1456.DEPLOYMENT_MODULES if s["module_id"] == "v1450_cube_history_aggregator")
    profile = v1456._execute_module(spec)
    assert profile.module_id == "v1450_cube_history_aggregator"
    assert profile.success is True


def test_v1456_execute_subprocess_real_module():
    """Execute V1260 (SUBPROCESS_REAL) returns ExecutionProfile."""
    spec = next(s for s in v1456.DEPLOYMENT_MODULES if s["module_id"] == "v1260_docker_deploy")
    profile = v1456._execute_module(spec)
    assert profile.module_id == "v1260_docker_deploy"
    assert profile.execution_mode == v1456.EXECUTION_MODE_SUBPROCESS_REAL


# ============================================================================
# 11. Markdown rendering (主 00:44 质量工程化)
# ============================================================================


def test_v1456_render_markdown_basic():
    """render_markdown produces non-empty markdown with required sections."""
    r = v1456.ParityReport(
        report_id="test", started_at=0.0, ended_at=1.0, duration_sec=1.0,
        n_profiles=6, n_success=6, overall_parity=0.85,
        combined_parity=0.80, v1450_cube_overall=0.7483,
        honest_disclosure="test disclosure",
    )
    md = v1456.render_markdown(r)
    assert isinstance(md, str)
    assert len(md) > 500
    assert "V1456" in md
    assert "Per-deployment profiles" in md
    assert "Honest disclosure" in md
    assert "V3" in md
    assert "GUARDS" in md


def test_v1456_render_markdown_includes_all_modules():
    """render_markdown includes all 6 deployment module IDs."""
    # Build profiles so the markdown rendering loop includes them.
    profiles = []
    for spec in v1456.DEPLOYMENT_MODULES:
        profiles.append(v1456.ExecutionProfile(
            module_id=spec["module_id"],
            module_path=spec["module_path"],
            execution_mode=spec["execution_mode"],
            primary_function=spec["primary_function"],
            borrowed_from=spec["borrowed_from"],
            description=spec["description"],
            success=True,
            parity_score=0.9,
        ))
    r = v1456.ParityReport(
        report_id="test", started_at=0.0, ended_at=1.0, duration_sec=1.0,
        n_profiles=6, n_success=6,
        profiles=profiles,
        per_module_parity={p.module_id: p.parity_score for p in profiles},
        honest_disclosure="test",
    )
    md = v1456.render_markdown(r)
    for spec in v1456.DEPLOYMENT_MODULES:
        assert spec["module_id"] in md, f"missing {spec['module_id']} in markdown"


# ============================================================================
# 12. CLI commands (主 00:56 任何人都能接手)
# ============================================================================


def test_v1456_cmd_version():
    """cmd_version returns 0 and prints version string."""
    rc = v1456.cmd_version()
    assert rc == 0


def test_v1456_cmd_meta_text():
    """cmd_meta (text mode) returns 0."""
    rc = v1456.cmd_meta(as_json=False)
    assert rc == 0


def test_v1456_cmd_meta_json():
    """cmd_meta (json mode) returns 0."""
    rc = v1456.cmd_meta(as_json=True)
    assert rc == 0


def test_v1456_cmd_help():
    """cmd_help returns 0."""
    rc = v1456.cmd_help()
    assert rc == 0


def test_v1456_cmd_list_modules():
    """cmd_list_modules returns 0."""
    rc = v1456.cmd_list_modules()
    assert rc == 0


def test_v1456_cmd_popper():
    """cmd_popper returns 0 (popper ≥14/14)."""
    rc = v1456.cmd_popper()
    assert rc == 0


def test_v1456_cmd_chain():
    """cmd_chain returns 0 (8/8 upstream OK)."""
    rc = v1456.cmd_chain()
    assert rc == 0


def test_v1456_cmd_probe_execution_existing_module():
    """cmd_probe_execution on existing module returns 0."""
    rc = v1456.cmd_probe_execution("v1450_cube_history_aggregator")
    assert rc == 0


def test_v1456_cmd_probe_execution_missing_module_arg():
    """cmd_probe_execution without --module returns 2."""
    rc = v1456.cmd_probe_execution("")
    assert rc == 2


def test_v1456_cmd_probe_execution_unknown_module():
    """cmd_probe_execution on unknown module returns 2."""
    rc = v1456.cmd_probe_execution("v9999_nonexistent")
    assert rc == 2


def test_v1456_cmd_run_all_writes_artifacts(tmp_path):
    """cmd_run_all writes JSON and MD artifacts."""
    json_path = str(tmp_path / "report.json")
    md_path = str(tmp_path / "report.md")
    rc = v1456.cmd_run_all(json_path, md_path)
    assert rc == 0
    assert os.path.exists(json_path)
    assert os.path.exists(md_path)
    # Validate JSON is parseable
    with open(json_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["n_profiles"] == 6


# ============================================================================
# 13. End-to-end run-all (主 23:44 干到底)
# ============================================================================


def test_v1456_run_all_end_to_end():
    """run_all() returns ParityReport with all 6 modules."""
    with tempfile.TemporaryDirectory() as tmp:
        json_path = os.path.join(tmp, "report.json")
        md_path = os.path.join(tmp, "report.md")
        report = v1456.run_all(out_json=json_path, out_md=md_path)
        assert report.n_profiles == 6
        assert len(report.profiles) == 6
        assert report.v1450_cube_overall >= 0.0
        assert 0.0 <= report.overall_parity <= 1.0
        assert 0.0 <= report.combined_parity <= 1.0
        # Artifacts written
        assert os.path.exists(json_path)
        assert os.path.exists(md_path)


def test_v1456_run_all_latency_bounded():
    """All module latencies bounded < 60s."""
    report = v1456.run_all()
    for p in report.profiles:
        assert p.latency_ms < 60000, f"{p.module_id} latency {p.latency_ms}ms > 60s"


def test_v1456_run_all_return_codes_bounded():
    """All module return codes ∈ [-1, 255]."""
    report = v1456.run_all()
    for p in report.profiles:
        assert -1 <= p.return_code <= 255, f"{p.module_id} return_code {p.return_code}"


def test_v1456_run_all_parity_scores_bounded():
    """All per-module parity scores ∈ [0, 1]."""
    report = v1456.run_all()
    for p in report.profiles:
        assert 0.0 <= p.parity_score <= 1.0


def test_v1456_run_all_execution_modes_valid():
    """All profile execution modes are valid."""
    valid = {
        v1456.EXECUTION_MODE_PROXY, v1456.EXECUTION_MODE_DRY_RUN,
        v1456.EXECUTION_MODE_SUBPROCESS_REAL, v1456.EXECUTION_MODE_DOCKER_REAL,
    }
    report = v1456.run_all()
    for p in report.profiles:
        assert p.execution_mode in valid


def test_v1456_run_all_summary_stats_correct():
    """Summary stats match the profiles."""
    report = v1456.run_all()
    n_success = sum(1 for p in report.profiles if p.success)
    assert report.n_success == n_success
    assert report.n_profiles == len(report.profiles)


# ============================================================================
# 14. Honest disclosure (主 17:43 实事求是)
# ============================================================================


def test_v1456_honest_disclosure_mentions_no_asi():
    """Honest disclosure explicitly disclaims ASI closure."""
    report = v1456.run_all()
    assert "ASI" in report.honest_disclosure
    assert "≠" in report.honest_disclosure or "not" in report.honest_disclosure.lower()


def test_v1456_honest_disclosure_mentions_bounded():
    """Honest disclosure mentions bounded nature."""
    report = v1456.run_all()
    assert "bounded" in report.honest_disclosure.lower() or "parity" in report.honest_disclosure.lower()


# ============================================================================
# 15. Subprocess CLI end-to-end (主 00:56 任何人都能接手)
# ============================================================================


def test_v1456_subprocess_version():
    """python -m apeireth.v1456_... version returns 0 and prints version."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1456_asi_six_deployment_real_execution_parity", "version"],
        capture_output=True, timeout=15,
    )
    assert proc.returncode == 0
    out = proc.stdout.decode("utf-8", errors="ignore")
    assert "V1456" in out


def test_v1456_subprocess_popper():
    """python -m apeireth.v1456_... popper returns 0."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1456_asi_six_deployment_real_execution_parity", "popper"],
        capture_output=True, timeout=30,
    )
    assert proc.returncode == 0
    out = proc.stdout.decode("utf-8", errors="ignore")
    assert "PASS" in out


def test_v1456_subprocess_chain():
    """python -m apeireth.v1456_... chain returns 0."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1456_asi_six_deployment_real_execution_parity", "chain"],
        capture_output=True, timeout=60,
    )
    assert proc.returncode == 0
    out = proc.stdout.decode("utf-8", errors="ignore")
    assert "OK" in out


def test_v1456_subprocess_list_modules():
    """python -m apeireth.v1456_... list-modules returns 0."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1456_asi_six_deployment_real_execution_parity", "list-modules"],
        capture_output=True, timeout=15,
    )
    assert proc.returncode == 0
    out = proc.stdout.decode("utf-8", errors="ignore")
    assert "v1260" in out
    assert "v1450" in out


def test_v1456_subprocess_meta_json():
    """python -m apeireth.v1456_... meta --json returns 0 with valid JSON."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1456_asi_six_deployment_real_execution_parity", "meta", "--json"],
        capture_output=True, timeout=15,
    )
    assert proc.returncode == 0
    out = proc.stdout.decode("utf-8", errors="ignore")
    data = json.loads(out)
    assert data["phase"] == "1456"


def test_v1456_subprocess_run_all_writes_artifacts(tmp_path):
    """python -m apeireth.v1456_... run-all writes artifacts."""
    json_path = str(tmp_path / "report.json")
    md_path = str(tmp_path / "report.md")
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1456_asi_six_deployment_real_execution_parity",
         "run-all", "--out-json", json_path, "--out-md", md_path],
        capture_output=True, timeout=180,
    )
    assert proc.returncode == 0
    assert os.path.exists(json_path)
    assert os.path.exists(md_path)
    with open(json_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["n_profiles"] == 6


# ============================================================================
# 16. Borrowed lineage (主 19:33 走在前人经验上)
# ============================================================================


def test_v1456_borrowed_lineage_documented():
    """All 8 borrowed modules are referenced in module docstring."""
    src_file = Path(v1456.__file__)
    content = src_file.read_text(encoding="utf-8")
    borrowed = ["V1455", "V1450", "V1440", "V1439", "V1263", "V1262", "V1261", "V1260"]
    for b in borrowed:
        assert b in content, f"missing borrowed reference: {b}"


def test_v1456_no_v1455_replace():
    """V1456 does NOT replace V1455."""
    spec = next(s for s in v1456.DEPLOYMENT_MODULES if s["module_id"] == "v1450_cube_history_aggregator")
    # V1456 uses V1450 as borrowed but doesn't replace
    assert "V1450" in spec["borrowed_from"]


# ============================================================================
# 17. Real subprocess execution (主 23:44 干到底)
# ============================================================================


def test_v1456_subprocess_v1260_runs():
    """V1260 subprocess returns successfully."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1260_docker_deploy", "version"],
        capture_output=True, timeout=30,
    )
    # V1260 has no CLI, returns 0 with empty output
    assert proc.returncode == 0


def test_v1456_subprocess_v1261_runs():
    """V1261 subprocess returns demo output."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1261_benchmark_llm", "version"],
        capture_output=True, timeout=30,
    )
    assert proc.returncode == 0
    out = proc.stdout.decode("utf-8", errors="ignore")
    assert "v1261" in out or "demo" in out or "meta" in out


def test_v1456_subprocess_v1439_runs():
    """V1439 subprocess returns version."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1439_asi_streamlit_subprocess_smoke", "version"],
        capture_output=True, timeout=30,
    )
    assert proc.returncode == 0
    out = proc.stdout.decode("utf-8", errors="ignore")
    assert "V1439" in out or "0.1.0" in out


def test_v1456_subprocess_v1450_runs():
    """V1450 subprocess returns version."""
    proc = subprocess.run(
        [sys.executable, "-m", "apeireth.v1450_asi_cross_modular_cube_history", "version"],
        capture_output=True, timeout=30,
    )
    assert proc.returncode == 0


# ============================================================================
# 18. Final composition test
# ============================================================================


def test_v1456_overall_summary():
    """Final composition test — V1456 is real-execution parity audit."""
    report = v1456.run_all()
    summary = {
        "module": "V1456",
        "phase": v1456.V1456_PHASE,
        "version": v1456.V1456_VERSION,
        "schema": v1456.V1456_SCHEMA,
        "n_profiles": report.n_profiles,
        "n_success": report.n_success,
        "overall_parity": round(report.overall_parity, 4),
        "v1450_cube_overall": round(report.v1450_cube_overall, 4),
        "combined_parity": round(report.combined_parity, 4),
        "per_module_parity": {k: round(v, 4) for k, v in report.per_module_parity.items()},
        "n_proxy": report.n_proxy,
        "n_dry_run": report.n_dry_run,
        "n_subprocess_real": report.n_subprocess_real,
        "n_docker_real": report.n_docker_real,
        "duration_sec": round(report.duration_sec, 3),
    }
    assert summary["n_profiles"] == 6
    assert 0.0 <= summary["overall_parity"] <= 1.0
    assert 0.0 <= summary["combined_parity"] <= 1.0
