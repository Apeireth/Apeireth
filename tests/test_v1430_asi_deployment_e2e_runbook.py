"""Tests for V1430 — ASI deployment E2E runbook orchestrator (主 00:44 质量工程化)."""

from __future__ import annotations

import tempfile
from pathlib import Path

import pytest


def test_v1430_importable():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    assert m.V1430_VERSION == "0.1.0"


def test_v1430_guards_count():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    assert len(m.V1430_GUARDS) == 13
    assert len(m.V1430_V3_GUARDS) == 5


def test_v1430_borrowed_count():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    assert len(m.V1430_BORROWED) == 5


def test_v1430_step_generate_returns_step_result():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        s = m.step_generate(Path(tmp))
        assert isinstance(s, m.StepResult)
        assert s.step == "generate"
        assert s.status in (m.StepStatus.PASS, m.StepStatus.FAIL)


def test_v1430_step_lint_returns_step_result():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        m.step_generate(Path(tmp))
        s = m.step_lint(Path(tmp))
        assert isinstance(s, m.StepResult)
        assert s.step == "lint"
        assert s.status in (m.StepStatus.PASS, m.StepStatus.WARN, m.StepStatus.FAIL)


def test_v1430_step_probe_host_returns_step_result():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    s = m.step_probe_host()
    assert isinstance(s, m.StepResult)
    assert s.step == "probe_host"
    assert s.status == m.StepStatus.PASS


def test_v1430_step_smoke_returns_step_result():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        m.step_generate(Path(tmp))
        s = m.step_smoke(Path(tmp))
        assert isinstance(s, m.StepResult)
        assert s.step == "smoke"
        assert s.status == m.StepStatus.PASS


def test_v1430_runbook_full_returns_runbook_report():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        report = m.runbook_full(Path(tmp))
        assert isinstance(report, m.RunbookReport)
        assert len(report.steps) == 4
        assert report.deployment_pass in (True, False)


def test_v1430_deployment_pass_is_bool():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        report = m.runbook_full(Path(tmp))
        assert isinstance(report.deployment_pass, bool)


def test_v1430_runbook_chain_delegate_runs():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    chain = m.runbook_chain_delegate()
    assert isinstance(chain, dict)
    assert "all_ok" in chain
    assert "V1428" in chain["chain"]
    assert "V1429" in chain["chain"]


def test_v1430_module_meta_contains_required_keys():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    meta = m.module_meta()
    assert meta["version"] == "0.1.0"
    assert meta["module"] == "v1430_asi_deployment_e2e_runbook"
    assert "guards" in meta
    assert "v3_guards" in meta
    assert "borrowed" in meta


def test_v1430_smoke_compose_pass():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "docker-compose.yml"
        p.write_text("services:\n  api:\n    image: apeireth:1.0\n")
        ok, note = m._smoke_compose(p)
        assert ok
        assert "parsed_ok" in note


def test_v1430_smoke_compose_fail():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "docker-compose.yml"
        p.write_text("not_a_yaml")
        ok, note = m._smoke_compose(p)
        assert not ok


def test_v1430_smoke_k8s_pass():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "k8s-asi.yaml"
        p.write_text("apiVersion: apps/v1\nkind: Deployment\n")
        ok, note = m._smoke_k8s(p)
        assert ok


def test_v1430_smoke_dockerfile_pass():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "Dockerfile"
        p.write_text("FROM python:3.13-slim\nCMD [\"python\"]\n")
        ok, note = m._smoke_dockerfile(p)
        assert ok


def test_v1430_smoke_requirements_pass():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "requirements.txt"
        p.write_text("pytest>=8.0\n")
        ok, note = m._smoke_requirements(p)
        assert ok


def test_v1430_smoke_startup_pass():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "start-asi.sh"
        p.write_text("#!/usr/bin/env bash\nset -e\n")
        ok, note = m._smoke_startup(p)
        assert ok


def test_v1430_smoke_env_pass():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / ".env.example"
        p.write_text("ASI_MODE=core\n")
        ok, note = m._smoke_env(p)
        assert ok


def test_v1430_smoke_missing_file_fails():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        p = Path(tmp) / "docker-compose.yml"
        ok, note = m._smoke_compose(p)
        assert not ok
        assert "file_missing" in note


def test_v1430_render_report_md_runs():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    with tempfile.TemporaryDirectory() as tmp:
        report = m.runbook_full(Path(tmp))
        md = m.render_report_md(report)
        assert "# V1430" in md
        assert "deployment_pass" in md
        assert "Honest disclosure" in md


def test_v1430_popper_self_test_passes():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    result = m.popper_self_test()
    assert result["n_total"] == 14
    assert result["n_pass"] == 14, f"failed: {result['results']}"


def test_v1430_no_v1428_write_side_effect():
    """V1428 generates artifacts; V1430 calls it but does not mutate V1428 state."""
    import apeireth.v1428_asi_real_deployment_artifacts as v1428
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    v1428_version_before = v1428.V1428_VERSION
    with tempfile.TemporaryDirectory() as tmp:
        m.runbook_full(Path(tmp))
    v1428_version_after = v1428.V1428_VERSION
    assert v1428_version_before == v1428_version_after


def test_v1430_no_v1429_write_side_effect():
    """V1429 provides linter; V1430 calls it but does not mutate V1429 state."""
    import apeireth.v1429_asi_deployment_semantic_linter as v1429
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    v1429_version_before = v1429.V1429_VERSION
    with tempfile.TemporaryDirectory() as tmp:
        m.runbook_full(Path(tmp))
    v1429_version_after = v1429.V1429_VERSION
    assert v1429_version_before == v1429_version_after


def test_v1430_v3_guards_no_phenomenal():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    assert "GUARD_NO_PHENOMENAL_RUNBOOK" in m.V1430_V3_GUARDS


def test_v1430_v3_guards_no_asi():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    assert "GUARD_NO_ASI_RUNBOOK" in m.V1430_V3_GUARDS


def test_v1430_v3_guards_no_human_level():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    assert "GUARD_NO_HUMAN_LEVEL_RUNBOOK" in m.V1430_V3_GUARDS


def test_v1430_v3_guards_no_absolute():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    assert "GUARD_NO_ABSOLUTE_RUNBOOK" in m.V1430_V3_GUARDS


def test_v1430_v3_guards_no_fake_production():
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    assert "GUARD_NO_FAKE_PRODUCTION" in m.V1430_V3_GUARDS


def test_v1430_step_status_pass_warn_fail_skip():
    """StepStatus enum has 4 values."""
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    assert m.StepStatus.PASS == "PASS"
    assert m.StepStatus.WARN == "WARN"
    assert m.StepStatus.FAIL == "FAIL"
    assert m.StepStatus.SKIP == "SKIP"


def test_v1430_smoke_functions_per_kind():
    """Each artifact kind has a smoke function."""
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    assert "docker-compose.yml" in m._SMOKE_FUNCS
    assert "k8s-asi.yaml" in m._SMOKE_FUNCS
    assert "Dockerfile" in m._SMOKE_FUNCS
    assert "requirements.txt" in m._SMOKE_FUNCS
    assert "start-asi.sh" in m._SMOKE_FUNCS
    assert ".env.example" in m._SMOKE_FUNCS
    assert len(m._SMOKE_FUNCS) == 6


def test_v1430_runbook_chain_contains_v1428_v1429():
    """Chain delegate reflects V1428 + V1429 (主 13:08 真实)."""
    import apeireth.v1430_asi_deployment_e2e_runbook as m
    chain = m.runbook_chain_delegate()
    assert "V1428" in chain["chain"]
    assert "V1429" in chain["chain"]
    assert chain["chain"]["V1428"]["ok"]
    assert chain["chain"]["V1429"]["ok"]
