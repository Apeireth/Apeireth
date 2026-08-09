"""Tests for V1428 — ASI 真生产 deployment artifacts generator + validator."""

from __future__ import annotations

import json
import sys
import tempfile
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from apeireth import v1428_asi_real_deployment_artifacts as m


# ============================================================================
# Constants / structural
# ============================================================================


def test_module_constants_present():
    assert m.V1428_VERSION == "0.1.0"
    assert m.V1428_SCHEMA == "v1428.asi-real-deployment-artifacts/v1"
    assert m.V1428_MODULE == "v1428_asi_real_deployment_artifacts"


def test_all_artifact_kinds_count():
    assert len(m.ALL_ARTIFACT_KINDS) == 6


def test_all_artifact_kinds_complete():
    names = {k.value for k in m.ALL_ARTIFACT_KINDS}
    expected = {"docker-compose.yml", "k8s-asi.yaml", "Dockerfile", "requirements.txt", "start-asi.sh", ".env.example"}
    assert names == expected


def test_guards_well_formed():
    assert len(m.V1428_GUARDS) >= 13
    expected = {
        "GUARD_NO_DOCKER_REQUIRED",
        "GUARD_NO_V1008_WRITE",
        "GUARD_NO_V1032_WRITE",
        "GUARD_ARTIFACTS_REAL",
        "GUARD_YAML_VALID",
        "GUARD_DOCKERFILE_VALID",
        "GUARD_SHELL_VALID",
        "GUARD_REQUIREMENTS_VALID",
        "GUARD_ENV_VALID",
        "GUARD_READINESS_DEFINED",
        "GUARD_POPPER_RUNS",
        "GUARD_CHAIN_OK",
        "GUARD_HONEST_DISCLOSURE",
    }
    for g in expected:
        assert g in m.V1428_GUARDS


def test_v3_guards_well_formed():
    assert len(m.V1428_V3_GUARDS) >= 5


def test_borrowed_real():
    assert len(m.V1428_BORROWED) >= 8
    keys = [b[0] for b in m.V1428_BORROWED]
    assert "V1008" in keys
    assert "V1032" in keys


# ============================================================================
# Validators
# ============================================================================


def test_validate_yaml_basic_accepts_valid():
    ok, errors = m._validate_yaml_basic("foo: bar\nbaz: qux\n")
    assert ok is True
    assert errors == []


def test_validate_yaml_basic_rejects_empty():
    ok, errors = m._validate_yaml_basic("")
    assert ok is False
    assert len(errors) > 0


def test_validate_yaml_basic_rejects_tabs():
    ok, errors = m._validate_yaml_basic("\tfoo: bar\n")
    assert ok is False


def test_validate_dockerfile_accepts_valid():
    ok, errors = m._validate_dockerfile("FROM python:3.13\nCMD [\"python\"]\n")
    assert ok is True


def test_validate_dockerfile_rejects_no_from():
    ok, errors = m._validate_dockerfile("CMD [\"python\"]\n")
    assert ok is False
    assert any("FROM" in e for e in errors)


def test_validate_dockerfile_rejects_no_cmd():
    ok, errors = m._validate_dockerfile("FROM python:3.13\n")
    assert ok is False
    assert any("CMD" in e for e in errors)


def test_validate_shell_accepts_valid():
    ok, errors = m._validate_shell("#!/usr/bin/env bash\nset -e\necho hi\n")
    assert ok is True


def test_validate_shell_rejects_no_shebang():
    ok, errors = m._validate_shell("set -e\necho hi\n")
    assert ok is False
    assert any("shebang" in e for e in errors)


def test_validate_shell_rejects_no_set_e():
    ok, errors = m._validate_shell("#!/usr/bin/env bash\necho hi\n")
    assert ok is False
    assert any("set -e" in e for e in errors)


def test_validate_requirements_accepts_valid():
    ok, errors = m._validate_requirements("pytest>=8.0\n")
    assert ok is True


def test_validate_requirements_rejects_empty():
    ok, errors = m._validate_requirements("")
    assert ok is False


def test_validate_env_accepts_valid():
    ok, errors = m._validate_env("ASI_MODE=core\n")
    assert ok is True


def test_validate_env_rejects_empty():
    ok, errors = m._validate_env("")
    assert ok is False


# ============================================================================
# Spec builders
# ============================================================================


def test_build_default_specs_returns_six(tmp_path):
    specs = m.build_default_specs(tmp_path)
    assert len(specs) == 6


def test_build_default_specs_kinds_complete(tmp_path):
    specs = m.build_default_specs(tmp_path)
    kinds = {s.kind for s in specs}
    assert set(m.ALL_ARTIFACT_KINDS).issubset(kinds)


def test_build_default_specs_content_non_empty(tmp_path):
    specs = m.build_default_specs(tmp_path)
    for spec in specs:
        assert len(spec.content) > 10
        # path is set but file is not written until generate_artifacts is called


# ============================================================================
# generate_artifacts + validate
# ============================================================================


def test_generate_artifacts_writes_all_files(tmp_path):
    specs = m.build_default_specs(tmp_path)
    paths = m.generate_artifacts(specs)
    assert len(paths) == 6
    for kind, p in paths.items():
        assert p.exists()


def test_validate_artifact_returns_validation_result(tmp_path):
    specs = m.build_default_specs(tmp_path)
    for spec in specs:
        result = m.validate_artifact(spec)
        assert isinstance(result, m.ValidationResult)
        assert result.kind == spec.kind.value


def test_validate_artifact_all_default_specs_pass(tmp_path):
    specs = m.build_default_specs(tmp_path)
    for spec in specs:
        result = m.validate_artifact(spec)
        assert result.ok is True, f"{spec.kind.value}: {result.errors}"


def test_validate_all_returns_readiness(tmp_path):
    specs = m.build_default_specs(tmp_path)
    readiness = m.validate_all(specs)
    assert isinstance(readiness, m.DeploymentReadiness)
    assert readiness.n_total == 6
    assert readiness.n_valid == 6
    assert readiness.readiness == 1.0


def test_generate_and_validate_round_trip(tmp_path):
    readiness = m.generate_and_validate(tmp_path)
    assert readiness.n_total == 6
    assert readiness.readiness == 1.0


# ============================================================================
# Dataclasses
# ============================================================================


def test_artifact_spec_to_dict():
    spec = m.ArtifactSpec(
        kind=m.ArtifactKind.COMPOSE,
        content="x",
        path=Path("/tmp/x.yml"),
        description="test",
    )
    d = spec.to_dict()
    assert d["kind"] == "docker-compose.yml"


def test_validation_result_to_dict():
    r = m.ValidationResult(kind="x", path="/x", ok=True, errors=[], warnings=[])
    d = r.to_dict()
    assert d["kind"] == "x"
    assert d["ok"] is True


def test_deployment_readiness_to_dict():
    r = m.DeploymentReadiness(
        n_total=2,
        n_valid=2,
        readiness=1.0,
        per_kind={},
        output_dir="/tmp",
        started_iso="2026-08-10T00:00:00Z",
        ended_iso="2026-08-10T00:00:01Z",
    )
    d = r.to_dict()
    assert d["n_total"] == 2


# ============================================================================
# Module metadata
# ============================================================================


def test_module_meta_returns_dict():
    meta = m.module_meta()
    assert isinstance(meta, dict)
    assert meta["version"] == "0.1.0"
    assert meta["n_artifact_kinds"] == 6


# ============================================================================
# popper_self_test
# ============================================================================


def test_popper_self_test_passes():
    ok, n, checks = m.popper_self_test()
    assert ok is True
    assert n == 13
    for c in checks:
        assert c["ok"] is True, f"failed: {c}"


def test_popper_self_test_returns_13_checks():
    ok, n, checks = m.popper_self_test()
    assert len(checks) == 13


# ============================================================================
# chain_delegate
# ============================================================================


def test_chain_delegate_v1428_true():
    result = m.chain_delegate()
    assert result["v1428"] is True


def test_chain_delegate_all_ok_true():
    result = m.chain_delegate()
    assert result["all_ok"] is True


def test_chain_delegate_includes_v1008_v1032():
    result = m.chain_delegate()
    for v in ("V1008", "V1032", "V1418", "V1426", "V1427"):
        assert v in result


# ============================================================================
# render_report_md
# ============================================================================


def test_render_report_md_returns_string(tmp_path):
    readiness = m.generate_and_validate(tmp_path)
    md = m.render_report_md(readiness)
    assert isinstance(md, str)
    assert "V1428" in md
    assert "readiness" in md.lower()


def test_render_report_md_contains_all_artifact_kinds(tmp_path):
    readiness = m.generate_and_validate(tmp_path)
    md = m.render_report_md(readiness)
    for kind in m.ALL_ARTIFACT_KINDS:
        assert kind.value in md


# ============================================================================
# run_cli
# ============================================================================


def test_run_cli_version():
    rc = m.run_cli(["version"])
    assert rc == 0


def test_run_cli_meta():
    rc = m.run_cli(["meta"])
    assert rc == 0


def test_run_cli_meta_json():
    rc = m.run_cli(["meta", "--json", "true"])
    assert rc == 0


def test_run_cli_demo():
    rc = m.run_cli(["demo"])
    assert rc == 0


def test_run_cli_help():
    rc = m.run_cli(["help"])
    assert rc == 0


def test_run_cli_popper():
    rc = m.run_cli(["popper"])
    assert rc == 0


def test_run_cli_chain():
    rc = m.run_cli(["chain"])
    assert rc == 0


def test_run_cli_generate(tmp_path):
    rc = m.run_cli(["generate", "--output-dir", str(tmp_path)])
    assert rc == 0
    # verify files exist
    for kind in m.ALL_ARTIFACT_KINDS:
        assert (tmp_path / kind.value).exists()


def test_run_cli_validate(tmp_path):
    rc = m.run_cli(["validate", "--output-dir", str(tmp_path)])
    assert rc == 0


def test_run_cli_summary(tmp_path):
    rc = m.run_cli(["summary", "--output-dir", str(tmp_path)])
    assert rc == 0


def test_run_cli_report(tmp_path):
    rc = m.run_cli(["report", "--output-dir", str(tmp_path)])
    assert rc == 0


def test_run_cli_unknown_command():
    rc = m.run_cli(["bogus"])
    assert rc != 0


def test_run_cli_empty_argv_defaults_to_help():
    rc = m.run_cli([])
    assert rc == 0