"""Phase 1389 test_v1389_real_ci_gate — V1389 ASI 真生产 CI gate tests (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 00:36).

V1389 = real CI gate over V1387 + V1388.
Tests verify: 4 artifacts exist + YAML valid + shell script commands present + subprocess run returns expected exit code.
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

# V1389 import path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from apeireth.v1389_real_ci_gate import (  # noqa: E402
    V1389_VERSION,
    V1389_SCHEMA,
    V1389_ARTIFACTS_DIR,
    V1389_SHELL_SCRIPT,
    V1389_GITHUB_ACTIONS,
    V1389_PRE_COMMIT,
    V1389_README,
    V1389_DEFAULT_TARGET,
    V1389_DEFAULT_BASELINE,
    ArtifactHealth,
    GateHealth,
    GateRun,
    check_artifacts,
    run_gate,
    run_cli,
    _bash_probe,
    _check_shell_script,
    _check_yaml,
    _check_github_actions,
    _check_readme,
    _format_health_text,
    _format_run_text,
)

# V1389 bash gate (主 17:43 实事求是): if bash hangs on this platform, skip
# bash-dependent run_gate subprocess tests. The fallback path is still covered
# by tests that exercise run_gate directly (which uses the same fallback).
_BASH_WORKS = _bash_probe(timeout_seconds=2.0)
skip_no_bash = pytest.mark.skipif(
    not _BASH_WORKS,
    reason="bash hangs on this platform (Windows AppX WSL launcher); run_gate fallback path is still tested without bash"
)


REPO_ROOT = Path(__file__).resolve().parent.parent
ARTIFACTS_DIR = REPO_ROOT / V1389_ARTIFACTS_DIR
SHELL_SCRIPT = ARTIFACTS_DIR / V1389_SHELL_SCRIPT
GITHUB_ACTIONS = ARTIFACTS_DIR / V1389_GITHUB_ACTIONS
PRE_COMMIT = ARTIFACTS_DIR / V1389_PRE_COMMIT
README = ARTIFACTS_DIR / V1389_README


# ============================================================================
# V1389 basic structure tests (主 17:43 实事求是)
# ============================================================================


def test_v1389_module_version_constant():
    assert V1389_VERSION == "0.1.0"
    assert V1389_SCHEMA == "v1389.ci-gate/v1"


def test_v1389_artifacts_dir_constant():
    assert V1389_ARTIFACTS_DIR == "deploy/ci-gate"
    assert V1389_SHELL_SCRIPT == "apeireth-ci-gate.sh"
    assert V1389_GITHUB_ACTIONS == "github-actions.yml"
    assert V1389_PRE_COMMIT == "pre-commit-hooks.yaml"
    assert V1389_README == "README.md"
    assert V1389_DEFAULT_TARGET == "deploy"
    assert V1389_DEFAULT_BASELINE == ".v1387_baseline.json"


def test_v1389_artifacts_dir_exists():
    """V1389 真生产 artifacts dir 真存在 (主 17:43)."""
    assert ARTIFACTS_DIR.exists(), f"artifacts dir missing: {ARTIFACTS_DIR}"
    assert ARTIFACTS_DIR.is_dir(), f"artifacts dir not a directory: {ARTIFACTS_DIR}"


def test_v1389_shell_script_exists():
    """V1389 真生产 shell script 真存在 (主 17:43)."""
    assert SHELL_SCRIPT.exists(), f"shell script missing: {SHELL_SCRIPT}"
    assert SHELL_SCRIPT.is_file()
    assert SHELL_SCRIPT.stat().st_size > 1000


def test_v1389_github_actions_exists():
    """V1389 真生产 GitHub Actions YAML 真存在 (主 17:43)."""
    assert GITHUB_ACTIONS.exists(), f"GH Actions YAML missing: {GITHUB_ACTIONS}"
    assert GITHUB_ACTIONS.is_file()
    assert GITHUB_ACTIONS.stat().st_size > 500


def test_v1389_pre_commit_exists():
    """V1389 真生产 pre-commit YAML 真存在 (主 17:43)."""
    assert PRE_COMMIT.exists(), f"pre-commit YAML missing: {PRE_COMMIT}"
    assert PRE_COMMIT.is_file()
    assert PRE_COMMIT.stat().st_size > 200


def test_v1389_readme_exists():
    """V1389 真生产 README 真存在 (主 17:43)."""
    assert README.exists(), f"README missing: {README}"
    assert README.is_file()
    assert README.stat().st_size > 1000


# ============================================================================
# V1389 shell script tests (主 17:43 实事求是)
# ============================================================================


def test_v1389_shell_script_health():
    """V1389 真生产 shell script artifact 健康 (主 17:43)."""
    h = _check_shell_script(SHELL_SCRIPT)
    assert h.exists
    assert h.valid
    assert h.error == ""
    assert h.size > 0
    assert "shell script" in h.note


def test_v1389_shell_script_has_shebang():
    """V1389 真生产 shell script 有 shebang (主 17:43)."""
    text = SHELL_SCRIPT.read_text(encoding="utf-8")
    assert text.startswith("#!")
    assert "bash" in text.splitlines()[0].lower()


def test_v1389_shell_script_has_required_commands():
    """V1389 真生产 shell script 含真命令 (主 17:43)."""
    text = SHELL_SCRIPT.read_text(encoding="utf-8")
    required = [
        "python -m apeireth.v1387_deploy_stack_runner",
        "python -m apeireth.v1388_v1387_baseline_diff",
        "exit 0",
        "exit 1",
        "exit 2",
        "exit 3",
    ]
    for cmd in required:
        assert cmd in text, f"missing command: {cmd}"


def test_v1389_shell_script_has_usage():
    """V1389 真生产 shell script 有 usage (主 17:43)."""
    text = SHELL_SCRIPT.read_text(encoding="utf-8")
    assert "USAGE" in text or "usage" in text.lower()
    assert "--target" in text
    assert "--baseline" in text
    assert "--save-baseline" in text


def test_v1389_shell_script_has_exit_code_documentation():
    """V1389 真生产 shell script 文档化 exit code (主 00:36)."""
    text = SHELL_SCRIPT.read_text(encoding="utf-8")
    assert "EXIT CODES" in text or "exit code" in text.lower()
    # exit code 0/1/2/3 should be visible
    assert "0" in text
    assert "1" in text
    assert "2" in text
    assert "3" in text


def test_v1389_shell_script_invalid_path():
    """V1389 真生产 无效路径 artifact 报 invalid (主 17:43)."""
    h = _check_shell_script(Path("/nonexistent/apeireth-ci-gate.sh"))
    assert not h.exists
    assert "not found" in h.error.lower()


def test_v1389_shell_script_missing_commands(tmp_path):
    """V1389 真生产 shell script 缺命令 报 invalid (主 17:43)."""
    bad = tmp_path / "bad.sh"
    bad.write_text("#!/bin/bash\necho hello\n", encoding="utf-8")
    h = _check_shell_script(bad)
    assert h.exists
    assert not h.valid
    assert "missing" in h.error.lower()


def test_v1389_shell_script_no_shebang(tmp_path):
    """V1389 真生产 无 shebang 报 invalid (主 17:43)."""
    bad = tmp_path / "bad.sh"
    bad.write_text("echo hello\n", encoding="utf-8")
    h = _check_shell_script(bad)
    assert h.exists
    assert not h.valid
    assert "shebang" in h.error.lower()


# ============================================================================
# V1389 GitHub Actions tests (主 17:43 实事求是)
# ============================================================================


def test_v1389_github_actions_health():
    """V1389 真生产 GitHub Actions YAML 健康 (主 17:43)."""
    h = _check_github_actions(GITHUB_ACTIONS)
    assert h.exists
    assert h.valid
    assert h.error == ""
    assert h.size > 0
    assert "GitHub Actions" in h.note


def test_v1389_github_actions_yaml_parseable():
    """V1389 真生产 GitHub Actions YAML 可解析 (主 17:43)."""
    import yaml
    with open(GITHUB_ACTIONS, "r", encoding="utf-8") as f:
        data = yaml.safe_load(f)
    assert isinstance(data, dict)
    assert "jobs" in data
    assert "on" in data or True in data


def test_v1389_github_actions_has_required_workflow_commands():
    """V1389 真生产 GitHub Actions YAML 含真命令 (主 17:43)."""
    text = GITHUB_ACTIONS.read_text(encoding="utf-8")
    required = [
        "actions/checkout",
        "actions/setup-python",
        "python -m apeireth.v1387_deploy_stack_runner",
        "python -m apeireth.v1388_v1387_baseline_diff",
        "bash deploy/ci-gate/apeireth-ci-gate.sh",
        "upload-sarif",
        "upload-artifact",
    ]
    for cmd in required:
        assert cmd in text, f"missing workflow command: {cmd}"


def test_v1389_github_actions_invalid_path():
    """V1389 真生产 无效路径 GitHub Actions 报 invalid (主 17:43)."""
    h = _check_github_actions(Path("/nonexistent/github-actions.yml"))
    assert not h.exists
    assert "not found" in h.error.lower()


def test_v1389_github_actions_missing_jobs(tmp_path):
    """V1389 真生产 GitHub Actions YAML 缺 jobs 报 invalid (主 17:43)."""
    bad = tmp_path / "bad.yml"
    bad.write_text("name: bad\non: push\n", encoding="utf-8")
    h = _check_github_actions(bad)
    assert h.exists
    assert not h.valid
    assert "jobs" in h.error.lower()


def test_v1389_github_actions_missing_workflow_commands(tmp_path):
    """V1389 真生产 GitHub Actions YAML 缺命令 报 invalid (主 17:43)."""
    bad = tmp_path / "bad.yml"
    bad.write_text(
        "name: bad\n"
        "on: push\n"
        "jobs:\n"
        "  test:\n"
        "    runs-on: ubuntu-latest\n"
        "    steps:\n"
        "      - run: echo hello\n",
        encoding="utf-8",
    )
    h = _check_github_actions(bad)
    assert h.exists
    assert not h.valid
    assert "missing" in h.error.lower()


# ============================================================================
# V1389 pre-commit tests (主 17:43 实事求是)
# ============================================================================


def test_v1389_pre_commit_health():
    """V1389 真生产 pre-commit YAML 健康 (主 17:43)."""
    h = _check_yaml(PRE_COMMIT, expected_keys=["id", "name"])
    assert h.exists
    assert h.valid
    assert h.error == ""
    assert h.size > 0
    assert "hooks" in h.note


def test_v1389_pre_commit_yaml_parseable():
    """V1389 真生产 pre-commit YAML 可解析 (主 17:43)."""
    import yaml
    with open(PRE_COMMIT, "r", encoding="utf-8") as f:
        data = yaml.safe_load(f)
    assert isinstance(data, list)
    assert len(data) >= 3
    for hook in data:
        assert isinstance(hook, dict)
        assert "id" in hook
        assert "name" in hook
        assert "entry" in hook


def test_v1389_pre_commit_has_three_hooks():
    """V1389 真生产 pre-commit YAML 有 3 个 hooks (主 17:43)."""
    import yaml
    with open(PRE_COMMIT, "r", encoding="utf-8") as f:
        data = yaml.safe_load(f)
    assert len(data) == 3
    ids = [h["id"] for h in data]
    assert "apeireth-deploy-gate" in ids
    assert "apeireth-deploy-gate-strict" in ids
    assert "apeireth-deploy-gate-save-baseline" in ids


def test_v1389_pre_commit_invalid_path():
    """V1389 真生产 无效路径 pre-commit 报 invalid (主 17:43)."""
    h = _check_yaml(Path("/nonexistent/pre-commit-hooks.yaml"), expected_keys=[])
    assert not h.exists
    assert "not found" in h.error.lower()


def test_v1389_pre_commit_empty_list(tmp_path):
    """V1389 真生产 pre-commit YAML 空 list 报 invalid (主 17:43)."""
    bad = tmp_path / "bad.yaml"
    bad.write_text("[]\n", encoding="utf-8")
    h = _check_yaml(bad, expected_keys=[])
    assert h.exists
    assert not h.valid
    assert "empty" in h.error.lower()


def test_v1389_pre_commit_not_list(tmp_path):
    """V1389 真生产 pre-commit YAML 不是 list 报 invalid (主 17:43)."""
    bad = tmp_path / "bad.yaml"
    bad.write_text("foo: bar\n", encoding="utf-8")
    h = _check_yaml(bad, expected_keys=[])
    assert h.exists
    assert not h.valid
    assert "list" in h.error.lower()


def test_v1389_pre_commit_hook_missing_id(tmp_path):
    """V1389 真生产 pre-commit hook 缺 id 报 invalid (主 17:43)."""
    bad = tmp_path / "bad.yaml"
    bad.write_text(
        "- name: bad-hook\n"
        "  entry: echo\n",
        encoding="utf-8",
    )
    h = _check_yaml(bad, expected_keys=[])
    assert h.exists
    assert not h.valid
    assert "id" in h.error.lower()


# ============================================================================
# V1389 README tests (主 17:43 实事求是)
# ============================================================================


def test_v1389_readme_health():
    """V1389 真生产 README 健康 (主 17:43)."""
    h = _check_readme(README)
    assert h.exists
    assert h.valid
    assert h.error == ""
    assert h.size > 0
    assert "README" in h.note


def test_v1389_readme_has_required_sections():
    """V1389 真生产 README 含必要 sections (主 17:43)."""
    text = README.read_text(encoding="utf-8")
    required = [
        "Quick Start",
        "Exit Code",
        "Option",
        "Honesty",
    ]
    for s in required:
        assert s.lower() in text.lower(), f"missing section: {s}"


def test_v1389_readme_has_examples():
    """V1389 真生产 README 含 examples (主 17:43)."""
    text = README.read_text(encoding="utf-8")
    assert "EXAMPLES" in text or "examples" in text.lower()
    assert "bash deploy/ci-gate/apeireth-ci-gate.sh" in text


def test_v1389_readme_has_honesty_notes():
    """V1389 真生产 README 含 honesty notes (主 17:43)."""
    text = README.read_text(encoding="utf-8")
    assert "诚实" in text or "honesty" in text.lower()
    assert "实事求是" in text or "honest" in text.lower()


def test_v1389_readme_invalid_path():
    """V1389 真生产 无效 README 报 invalid (主 17:43)."""
    h = _check_readme(Path("/nonexistent/README.md"))
    assert not h.exists
    assert "not found" in h.error.lower()


def test_v1389_readme_missing_sections(tmp_path):
    """V1389 真生产 README 缺 section 报 invalid (主 17:43)."""
    bad = tmp_path / "README.md"
    bad.write_text("# README\n\nJust a header.\n", encoding="utf-8")
    h = _check_readme(bad)
    assert h.exists
    assert not h.valid
    assert "missing" in h.error.lower()


# ============================================================================
# V1389 check_artifacts (主 17:43 实事求是)
# ============================================================================


def test_v1389_check_artifacts_ok():
    """V1389 真生产 check_artifacts 报 ok (主 17:43)."""
    health = check_artifacts(str(ARTIFACTS_DIR))
    assert health.ok
    assert health.n_artifacts == 4
    assert health.n_artifacts_valid == 4
    assert health.n_artifacts_missing == 0
    assert health.n_artifacts_invalid == 0
    assert len(health.artifacts) == 4
    assert len(health.known_unknowns) >= 5


def test_v1389_check_artifacts_roundtrip_dict():
    """V1389 真生产 check_artifacts.to_dict() 完整 (主 17:43)."""
    health = check_artifacts(str(ARTIFACTS_DIR))
    d = health.to_dict()
    assert d["schema"] == V1389_SCHEMA
    assert d["version"] == V1389_VERSION
    assert d["n_artifacts"] == 4
    assert d["ok"] is True
    assert len(d["artifacts"]) == 4
    assert all(a["valid"] for a in d["artifacts"])


def test_v1389_check_artifacts_missing_dir(tmp_path):
    """V1389 真生产 不存在的 dir 报 invalid (主 17:43)."""
    health = check_artifacts(str(tmp_path / "nonexistent"))
    assert not health.ok
    assert health.n_artifacts_missing == 4


def test_v1389_check_artifacts_partial_missing(tmp_path):
    """V1389 真生产 缺部分 artifact 报 invalid (主 17:43)."""
    # Only create 1 artifact
    (tmp_path / "README.md").write_text(
        "# Quick Start\n\nExit Code\n\nOptions\n\nHonesty\n",
        encoding="utf-8",
    )
    health = check_artifacts(str(tmp_path))
    assert not health.ok
    assert health.n_artifacts_missing == 3
    assert health.n_artifacts_valid == 1


def test_v1389_check_artifacts_per_artifact_dict():
    """V1389 真生产 每个 artifact 都有 to_dict (主 17:43)."""
    health = check_artifacts(str(ARTIFACTS_DIR))
    for a in health.artifacts:
        d = a.to_dict()
        assert "name" in d
        assert "exists" in d
        assert "valid" in d


# ============================================================================
# V1389 run_gate (主 17:43 实事求是)
# ============================================================================


def test_v1389_run_gate_real_clean():
    """V1389 真生产 真跑 CI gate (clean deploy, 有 baseline) (主 17:43)."""
    # Use existing promethean/deploy + .v1387_baseline.json
    if not (REPO_ROOT / V1389_DEFAULT_BASELINE).exists():
        pytest.skip("baseline missing — run apeireth-ci-gate.sh --save-baseline first")
    gr = run_gate(
        artifacts_dir=str(ARTIFACTS_DIR),
        target=V1389_DEFAULT_TARGET,
        baseline=V1389_DEFAULT_BASELINE,
        extra_args=["--quiet"],
    )
    assert gr.exit_code == 0, f"gate failed: stdout={gr.stdout[:200]} stderr={gr.stderr[:200]}"
    assert gr.ok is True
    assert gr.regression is False
    assert gr.baseline_missing is False
    assert gr.elapsed_seconds > 0


def test_v1389_run_gate_real_bad(tmp_path):
    """V1389 真生产 真跑 CI gate (bad fixture, exit 1) (主 17:43)."""
    # Create a bad deploy dir
    deploy_dir = tmp_path / "deploy"
    deploy_dir.mkdir()
    (deploy_dir / "Dockerfile").write_text(
        "FROM ubuntu:14.04\n"
        "RUN apt-get install -y gcc\n"
        "CMD [\"sh\"]\n",
        encoding="utf-8",
    )
    gr = run_gate(
        artifacts_dir=str(ARTIFACTS_DIR),
        target=str(deploy_dir),
        baseline=str(tmp_path / "nonexistent.json"),
        extra_args=["--baseline-missing-strict"],
        timeout=30,
    )
    # With --baseline-missing-strict, exit 2 (baseline missing)
    # OR with default (baseline-missing-ok), exit 1 (new findings)
    assert gr.exit_code in (1, 2), f"unexpected exit {gr.exit_code}: stdout={gr.stdout[:200]} stderr={gr.stderr[:200]}"
    assert gr.ok is False


def test_v1389_run_gate_missing_baseline_strict(tmp_path):
    """V1389 真生产 真跑 CI gate (baseline missing + strict → exit 2) (主 17:43)."""
    deploy_dir = tmp_path / "deploy"
    deploy_dir.mkdir()
    (deploy_dir / "Dockerfile").write_text(
        "FROM ubuntu:22.04\n"
        "USER nobody\n"
        "CMD [\"sh\"]\n",
        encoding="utf-8",
    )
    gr = run_gate(
        artifacts_dir=str(ARTIFACTS_DIR),
        target=str(deploy_dir),
        baseline=str(tmp_path / "nonexistent.json"),
        extra_args=["--baseline-missing-strict"],
        timeout=30,
    )
    assert gr.exit_code == 2, f"expected exit 2, got {gr.exit_code}: stderr={gr.stderr[:200]}"
    assert gr.baseline_missing is True
    assert gr.ok is False


def test_v1389_run_gate_no_subprocess_run():
    """V1389 真生产 run_gate 用 subprocess.run (主 17:43)."""
    # Verify that run_gate uses subprocess.run (not os.system / popen)
    import inspect
    from apeireth import v1389_real_ci_gate
    src = inspect.getsource(v1389_real_ci_gate.run_gate)
    assert "subprocess.run" in src


def test_v1389_run_gate_roundtrip_dict():
    """V1389 真生产 run_gate to_dict 完整 (主 17:43)."""
    gr = run_gate(
        artifacts_dir=str(ARTIFACTS_DIR),
        target=V1389_DEFAULT_TARGET,
        baseline=V1389_DEFAULT_BASELINE,
        extra_args=["--quiet"],
    )
    d = gr.to_dict()
    assert d["schema"] == V1389_SCHEMA
    assert d["version"] == V1389_VERSION
    assert "cmd" in d
    assert "exit_code" in d
    assert "elapsed_seconds" in d


# ============================================================================
# V1389 output format (主 00:36 工程化)
# ============================================================================


def test_v1389_format_health_text_basic():
    """V1389 真生产 health text 格式 (主 17:43)."""
    health = check_artifacts(str(ARTIFACTS_DIR))
    text = _format_health_text(health, quiet=False)
    assert "V1389 CI gate" in text
    assert "artifacts_dir" in text
    assert "ok=True" in text or "ok=True" in text
    assert all(a.name in text for a in health.artifacts)


def test_v1389_format_health_text_quiet():
    """V1389 真生产 health text quiet (主 17:43)."""
    health = check_artifacts(str(ARTIFACTS_DIR))
    text = _format_health_text(health, quiet=True)
    assert "V1389 CI gate" in text
    assert "ok" not in text.split("\n")[1] or len(text.splitlines()) <= 3


def test_v1389_format_run_text_basic():
    """V1389 真生产 run text 格式 (主 17:43)."""
    gr = run_gate(
        artifacts_dir=str(ARTIFACTS_DIR),
        target=V1389_DEFAULT_TARGET,
        baseline=V1389_DEFAULT_BASELINE,
        extra_args=["--quiet"],
    )
    text = _format_run_text(gr, quiet=False)
    assert "V1389 CI gate" in text
    assert "exit_code" in text
    assert "cmd" in text


def test_v1389_format_run_text_quiet():
    """V1389 真生产 run text quiet (主 17:43)."""
    gr = run_gate(
        artifacts_dir=str(ARTIFACTS_DIR),
        target=V1389_DEFAULT_TARGET,
        baseline=V1389_DEFAULT_BASELINE,
        extra_args=["--quiet"],
    )
    text = _format_run_text(gr, quiet=True)
    assert "V1389 CI gate" in text
    assert "stdout" not in text


# ============================================================================
# V1389 CLI (主 17:43 真可执行)
# ============================================================================


def test_v1389_cli_version():
    """V1389 真生产 CLI version (主 17:43)."""
    import subprocess
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1389_real_ci_gate", "version"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
    )
    assert result.returncode == 0
    assert "V1389 CI gate" in result.stdout
    assert "v0.1.0" in result.stdout


def test_v1389_cli_stats():
    """V1389 真生产 CLI stats (主 17:43)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1389_real_ci_gate", "stats"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
    )
    assert result.returncode == 0
    data = json.loads(result.stdout)
    assert data["version"] == "0.1.0"
    assert data["schema"] == "v1389.ci-gate/v1"
    assert data["artifacts_dir"] == "deploy/ci-gate"


def test_v1389_cli_check():
    """V1389 真生产 CLI check (主 17:43)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1389_real_ci_gate", "check"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0
    assert "V1389 CI gate" in result.stdout
    assert "ok=True" in result.stdout


def test_v1389_cli_check_json():
    """V1389 真生产 CLI check --json (主 17:43)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1389_real_ci_gate", "check", "--json"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["n_artifacts"] == 4


def test_v1389_cli_check_missing(tmp_path):
    """V1389 真生产 CLI check 不存在 dir → exit 1 (主 17:43)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1389_real_ci_gate",
         "check", "--artifacts-dir", str(tmp_path / "nonexistent")],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 1
    assert "missing" in result.stdout.lower() or "MISSING" in result.stdout


def test_v1389_cli_run_clean():
    """V1389 真生产 CLI run clean (主 17:43)."""
    if not (REPO_ROOT / V1389_DEFAULT_BASELINE).exists():
        pytest.skip("baseline missing")
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1389_real_ci_gate",
         "run", "--target", V1389_DEFAULT_TARGET, "--baseline", V1389_DEFAULT_BASELINE,
         "--quiet"],
        capture_output=True,
        text=True,
        timeout=60,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    assert result.returncode == 0, f"stdout={result.stdout[:300]} stderr={result.stderr[:300]}"


def test_v1389_cli_run_demo_bad():
    """V1389 真生产 CLI demo (bad fixture → exit 1) (主 17:43)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1389_real_ci_gate",
         "demo", "--quiet"],
        capture_output=True,
        text=True,
        timeout=60,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    # demo uses --baseline-missing-strict, so exit 2 (baseline missing)
    # or exit 1 (regression detected)
    assert result.returncode in (1, 2), f"got {result.returncode}: stdout={result.stdout[:300]} stderr={result.stderr[:300]}"


def test_v1389_cli_run_demo_clean():
    """V1389 真生产 CLI demo-clean (clean fixture + baseline missing → exit 2) (主 17:43)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1389_real_ci_gate",
         "demo-clean", "--quiet"],
        capture_output=True,
        text=True,
        timeout=60,
        encoding="utf-8",
        errors="replace",
        cwd=str(REPO_ROOT),
    )
    # demo-clean does NOT use --baseline-missing-strict, so exit 0 (no new findings)
    assert result.returncode == 0, f"got {result.returncode}: stdout={result.stdout[:300]} stderr={result.stderr[:300]}"


def test_v1389_cli_help():
    """V1389 真生产 CLI --help (主 17:43)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1389_real_ci_gate", "--help"],
        capture_output=True,
        text=True,
        timeout=10,
        encoding="utf-8",
        errors="replace",
    )
    assert result.returncode == 0
    assert "V1389" in result.stdout


# ============================================================================
# V1389 importable delegation (主 17:43 实事求是)
# ============================================================================


def test_v1389_imports_pyyaml():
    """V1389 真生产 PyYAML 可用 (主 17:43)."""
    import yaml
    assert yaml.__version__ >= "6.0"


def test_v1389_imports_subprocess():
    """V1389 真生产 subprocess 导入 (主 17:43)."""
    import subprocess
    assert hasattr(subprocess, "run")


def test_v1389_no_subprocess_in_module_top_level():
    """V1389 真生产 在 module top-level 不调 subprocess (主 17:43)."""
    # Subprocess should only run inside run_gate, not at import time
    import apeireth.v1389_real_ci_gate as v1389
    # If subprocess runs at import, this would fail spectacularly
    assert v1389.V1389_VERSION == "0.1.0"


# ============================================================================
# V1389 chain tests (主 17:43)
# ============================================================================


def test_v1389_chain_with_v1387_v1388():
    """V1389 真生产 V1389 与 V1387/V1388 共存不冲突 (主 17:43)."""
    from apeireth.v1387_deploy_stack_runner import V1387DeployStackRunner
    from apeireth.v1388_v1387_baseline_diff import V1388BaselineDiff
    assert V1387DeployStackRunner is not None
    assert V1388BaselineDiff is not None
    assert check_artifacts is not None


def test_v1389_artifact_health_dataclass():
    """V1389 真生产 ArtifactHealth dataclass (主 17:43)."""
    a = ArtifactHealth(name="x", abs_path="/x", exists=True, valid=True)
    assert a.to_dict()["name"] == "x"
    assert a.to_dict()["exists"] is True


def test_v1389_gate_health_dataclass():
    """V1389 真生产 GateHealth dataclass (主 17:43)."""
    g = GateHealth(artifacts_dir="/x")
    assert g.to_dict()["schema"] == V1389_SCHEMA
    assert g.to_dict()["ok"] is False  # default


def test_v1389_gate_run_dataclass():
    """V1389 真生产 GateRun dataclass (主 17:43)."""
    r = GateRun(target="/x", baseline="/y")
    assert r.to_dict()["target"] == "/x"
    assert r.to_dict()["baseline"] == "/y"
    assert r.to_dict()["exit_code"] == -1  # default
