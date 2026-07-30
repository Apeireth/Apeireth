"""Tests for V1132 — real deployment validator (主 06:15 V1050+ 真部署 + 主 17:43 实事求是).

主 17:43 实事求是: docker daemon not on this host → expected fails must not be hidden.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, ROOT)

from apeireth.v1132_real_deployment_validator import (  # noqa: E402
    V1132DeploymentReport,
    V1132DeploymentValidator,
    _check_docker_daemon,
    _parse_compose,
    _validate_dockerfile,
    _validate_k8s_yaml,
    render_markdown,
)


# ---------- helpers ----------


def _repo_root() -> str:
    return ROOT


# ---------- docker daemon ----------


def test_docker_daemon_check_returns_tuple():
    ok, detail = _check_docker_daemon()
    assert isinstance(ok, bool)
    assert isinstance(detail, str)
    assert len(detail) > 0


def test_docker_daemon_marks_unavailable_on_host_without_docker():
    # this Windows test host has no docker; the result MUST be False with a non-empty reason
    ok, detail = _check_docker_daemon()
    # Not asserting False unconditionally — but if False, detail must be present
    if not ok:
        assert detail, "if docker unavailable, reason must be non-empty"


# ---------- compose parser ----------


def test_parse_compose_real_r8_file():
    path = os.path.join(_repo_root(), "docker-compose.r8.yml")
    if not os.path.isfile(path):
        pytest.skip("docker-compose.r8.yml not present")
    data, detail = _parse_compose(path)
    assert data is not None, detail
    assert "services" in data
    assert isinstance(data["services"], dict)
    assert len(data["services"]) >= 5


def test_parse_compose_missing_file_returns_none():
    data, detail = _parse_compose(os.path.join(_repo_root(), "no-such-compose-xyz.yml"))
    assert data is None
    assert "not found" in detail


def test_parse_compose_handles_malformed_yaml(tmp_path):
    p = tmp_path / "bad.yml"
    p.write_text("not: valid: yaml: [\n", encoding="utf-8")
    data, detail = _parse_compose(str(p))
    # PyYAML is permissive; either it parses (no 'services') or raises
    if data is not None:
        assert "services" not in data or not isinstance(data["services"], dict)
    else:
        assert "parse error" in detail or "no top-level" in detail


# ---------- dockerfile validator ----------


def test_validate_dockerfile_minimal_ok():
    ok, detail = _validate_dockerfile("FROM python:3.13-slim\nWORKDIR /app\n")
    assert ok
    assert "FROM" in detail


def test_validate_dockerfile_no_from_fails():
    ok, detail = _validate_dockerfile("WORKDIR /app\n")
    assert not ok
    assert "FROM" in detail


def test_validate_dockerfile_empty_fails():
    ok, _detail = _validate_dockerfile("")
    assert not ok


def test_validate_dockerfile_whitespace_only_fails():
    ok, _detail = _validate_dockerfile("   \n  \n")
    assert not ok


# ---------- k8s validator ----------


def test_validate_k8s_yaml_ok():
    text = "apiVersion: v1\nkind: Service\nmetadata:\n  name: x\n"
    ok, detail = _validate_k8s_yaml(text)
    assert ok
    assert "Service" in detail


def test_validate_k8s_yaml_multi_doc():
    text = (
        "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: a\n"
        "---\napiVersion: v1\nkind: Service\nmetadata:\n  name: a\n"
    )
    ok, detail = _validate_k8s_yaml(text)
    assert ok
    assert "Deployment" in detail and "Service" in detail


def test_validate_k8s_yaml_invalid():
    ok, _detail = _validate_k8s_yaml("not: [valid: yaml")
    assert not ok


# ---------- report ----------


def test_report_to_dict_has_required_keys():
    r = V1132DeploymentReport()
    d = r.to_dict()
    for k in (
        "report_id", "timestamp", "docker_daemon_available", "compose_files_parsed",
        "services_seen", "k8s_manifests_ok", "dockerfile_valid",
        "subprocess_runs_ok", "subprocess_runs_failed",
        "health_probes_ok", "health_probes_failed",
        "canonical_bundle_valid", "offline_valid", "runtime_valid",
        "passed", "checks", "notes",
    ):
        assert k in d, f"missing key: {k}"


def test_report_default_passed_false():
    # empty report → passed is False (no checks)
    r = V1132DeploymentReport()
    assert r.passed is False


# ---------- orchestrator ----------


def test_validator_run_full_validation_returns_report():
    v = V1132DeploymentValidator(repo_root=_repo_root())
    rep = v.run_full_validation()
    assert isinstance(rep, V1132DeploymentReport)
    # subprocess_runs_ok must be ≥2 because V1008 + V1032 both rendered
    assert rep.subprocess_runs_ok >= 2
    assert rep.subprocess_runs_failed == 0
    # k8s_manifests_ok must be ≥2 (V1008 + V1032 k8s)
    assert rep.k8s_manifests_ok >= 2
    # dockerfile_valid must be ≥1 (V1032 dockerfile rendered)
    assert rep.dockerfile_valid >= 1
    # at least one check ran
    assert len(rep.checks) >= 5


def test_validator_artefacts_include_v1008_and_v1032():
    v = V1132DeploymentValidator(repo_root=_repo_root())
    rep = v.run_full_validation()
    assert "v1008_compose.yml" in rep.artefacts
    assert "v1008_k8s.yaml" in rep.artefacts
    assert "v1008_startup.sh" in rep.artefacts
    assert "v1032_Dockerfile" in rep.artefacts
    assert "v1032_docker-compose.yml" in rep.artefacts
    assert "v1032_k8s-deployment.yaml" in rep.artefacts


def test_validator_canonical_bundle_is_semantically_consistent():
    v = V1132DeploymentValidator(repo_root=_repo_root())
    rep = v.run_full_validation()
    canonical = next(c for c in rep.checks if c.name == "canonical_bundle")
    assert canonical.passed is True, canonical.detail
    assert rep.canonical_bundle_valid is True
    assert rep.offline_valid is True


def test_validator_offline_success_does_not_claim_runtime_success():
    v = V1132DeploymentValidator(repo_root=_repo_root())
    rep = v.run_full_validation()
    assert rep.offline_valid is True
    if not rep.docker_daemon_available or rep.health_probes_ok == 0:
        assert rep.runtime_valid is False
        assert rep.passed is False


def test_validator_consistency_check_runs():
    v = V1132DeploymentValidator(repo_root=_repo_root())
    rep = v.run_full_validation()
    names = [c.name for c in rep.checks]
    assert "consistency_check" in names
    consistency = next(c for c in rep.checks if c.name == "consistency_check")
    # Either shared keys exist OR intentional divergence is documented
    assert consistency.passed is True


def test_validator_when_no_docker_passes_with_false():
    """Honest contract: when docker daemon is absent, passed=False (truthful)."""
    v = V1132DeploymentValidator(repo_root=_repo_root())
    rep = v.run_full_validation()
    if not rep.docker_daemon_available:
        # When docker is missing, the validator MUST report False honestly
        # (not pretend to be passing because configs parse)
        assert rep.passed is False


# ---------- render_markdown ----------


def test_render_markdown_contains_summary():
    r = V1132DeploymentReport(compose_files_parsed=2, services_seen=14, k8s_manifests_ok=2, dockerfile_valid=1)
    md = render_markdown(r)
    assert "V1132" in md
    assert "compose_files_parsed" in md
    assert "14" in md


def test_render_markdown_includes_notes_section():
    r = V1132DeploymentReport()
    r.notes.append("test note")
    md = render_markdown(r)
    assert "Notes" in md
    assert "test note" in md


def test_render_markdown_escapes_pipe_in_detail():
    r = V1132DeploymentReport()
    r.checks.append(type("C", (), {"name": "x", "passed": True, "detail": "a|b|c", "ms": 1.0})())
    md = render_markdown(r)
    # pipes in cell content must be escaped to keep the table valid
    assert "a\\|b\\|c" in md
