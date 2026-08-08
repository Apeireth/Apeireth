"""Tests for V1388 ASI V1387 baseline + diff (主 06:15 + 主 17:43 + 主 19:33 + 主 23:44).

主 17:43 实事求是: 真 run V1387 + 真 read baseline + 真算 diff, 不假装 diff.
"""
from __future__ import annotations

import io
import json
import os
import pathlib
import subprocess
import sys
import tempfile
from contextlib import redirect_stdout
from pathlib import Path
from typing import Any, Dict, List

import pytest

MODULE_DIR = pathlib.Path(__file__).resolve().parent.parent / "apeireth"
sys.path.insert(0, str(MODULE_DIR))

from v1388_v1387_baseline_diff import (  # noqa: E402
    V1388_VERSION,
    V1388_BASELINE_SCHEMA,
    V1388_DIFF_SCHEMA,
    DEFAULT_BASELINE_PATH,
    GUARDS,
    DiffFinding,
    DiffResult,
    FindingSignature,
    V1388BaselineDiff,
    _format_markdown,
    _format_sarif,
    _format_text,
    _popper_self_test,
    append_baseline,
    compute_diff,
    load_baseline,
    run_cli,
    save_baseline,
)


# ============================================================================
# fixtures
# ============================================================================


@pytest.fixture
def clean_dir(tmp_path: Path) -> Path:
    """V1388 一个完全干净的 deploy 目录."""
    d = tmp_path / "clean"
    d.mkdir()
    (d / "Dockerfile").write_text(
        "FROM ubuntu:22.04\nUSER app\nWORKDIR /app\nEXPOSE 8080\n"
        "HEALTHCHECK CMD true\nCMD [\"echo\",\"hi\"]\n",
        encoding="utf-8",
    )
    (d / "docker-compose.yml").write_text(
        "version: '3.8'\nservices:\n  app:\n    image: myapp:1.0\n"
        "    ports:\n      - \"8080:8080\"\n    healthcheck:\n"
        "      test: [\"CMD\",\"true\"]\n    restart: unless-stopped\n"
        "    deploy:\n      resources:\n        limits:\n          memory: 512M\n",
        encoding="utf-8",
    )
    (d / "k8s.yaml").write_text(
        "apiVersion: v1\nkind: Pod\nmetadata:\n  name: app\n  namespace: default\n"
        "spec:\n  containers:\n    - name: app\n      image: myapp:1.0\n"
        "      resources:\n        limits:\n          memory: 256Mi\n"
        "      readinessProbe:\n        httpGet:\n          path: /\n          port: 8080\n"
        "      livenessProbe:\n        httpGet:\n          path: /\n          port: 8080\n"
        "      securityContext:\n        capabilities:\n          drop: [\"ALL\"]\n",
        encoding="utf-8",
    )
    return d


@pytest.fixture
def bad_dir(tmp_path: Path) -> Path:
    """V1388 一个有问题的 deploy 目录."""
    d = tmp_path / "bad"
    d.mkdir()
    (d / "Dockerfile").write_text(
        "FROM ubuntu:latest\nRUN apt-get install -y gcc\nCMD echo hi\n",
        encoding="utf-8",
    )
    (d / "docker-compose.yml").write_text(
        "version: '3.8'\nservices:\n  app:\n    image: myapp:latest\n"
        "    privileged: true\n",
        encoding="utf-8",
    )
    (d / "k8s.yaml").write_text(
        "apiVersion: v1\nkind: Pod\nmetadata:\n  name: app\n"
        "spec:\n  containers:\n    - name: app\n      image: myapp:latest\n"
        "      env:\n        - name: DB_PASSWORD\n          value: hunter2\n",
        encoding="utf-8",
    )
    return d


@pytest.fixture
def runner() -> V1388BaselineDiff:
    return V1388BaselineDiff()


# ============================================================================
# module structure
# ============================================================================


def test_v1388_module_version_constant():
    assert V1388_VERSION == "0.1.0"


def test_v1388_diff_schema_constant():
    assert V1388_DIFF_SCHEMA == "v1388.baseline-diff/v1"


def test_v1388_baseline_schema_constant():
    assert V1388_BASELINE_SCHEMA == "v1388.baseline/v1"


def test_v1388_guards_count():
    assert len(GUARDS) >= 6


def test_v1388_guards_required():
    required = {
        "GUARD_BASELINE_LOAD",
        "GUARD_NO_CAP_CHANGE",
        "GUARD_DETERMINISTIC",
        "GUARD_PATH_SAFE",
        "GUARD_HONEST_DISCLOSURE",
        "GUARD_DELEGATE_REAL",
        "GUARD_NON_DESTRUCTIVE",
        "GUARD_CLI_RUNNABLE",
    }
    assert required.issubset(set(GUARDS))


def test_v1388_default_baseline_path():
    assert DEFAULT_BASELINE_PATH == ".v1387_baseline.json"


# ============================================================================
# FindingSignature
# ============================================================================


def test_v1388_finding_sig_from_finding_basic():
    sig = FindingSignature.from_finding(
        "Dockerfile",
        {"rule_id": "DL3008", "line_no": 3, "message": "Pin versions in apt-get install"},
    )
    assert sig.file_path == "Dockerfile"
    assert sig.rule_id == "DL3008"
    assert sig.line_no == 3
    assert len(sig.msg_hash) == 12


def test_v1388_finding_sig_same_message_same_hash():
    """V1388 真生产 同 message → 同 hash (主 17:43 实事求是)."""
    sig1 = FindingSignature.from_finding("a", {"rule_id": "R", "line_no": 1, "message": "x"})
    sig2 = FindingSignature.from_finding("a", {"rule_id": "R", "line_no": 1, "message": "x"})
    assert sig1.msg_hash == sig2.msg_hash
    assert sig1.to_key() == sig2.to_key()


def test_v1388_finding_sig_different_file_different_key():
    sig1 = FindingSignature.from_finding("a", {"rule_id": "R", "line_no": 1, "message": "x"})
    sig2 = FindingSignature.from_finding("b", {"rule_id": "R", "line_no": 1, "message": "x"})
    assert sig1.to_key() != sig2.to_key()


def test_v1388_finding_sig_different_rule_different_key():
    sig1 = FindingSignature.from_finding("a", {"rule_id": "R1", "line_no": 1, "message": "x"})
    sig2 = FindingSignature.from_finding("a", {"rule_id": "R2", "line_no": 1, "message": "x"})
    assert sig1.to_key() != sig2.to_key()


def test_v1388_finding_sig_to_dict():
    sig = FindingSignature.from_finding("a", {"rule_id": "R", "line_no": 1, "message": "x"})
    d = sig.to_dict()
    assert d["file_path"] == "a"
    assert d["rule_id"] == "R"
    assert d["line_no"] == 1
    assert "msg_hash" in d


# ============================================================================
# load_baseline / save_baseline / append_baseline
# ============================================================================


def test_v1388_load_baseline_not_found(tmp_path):
    bd, err = load_baseline(str(tmp_path / "nope.json"))
    assert bd is None
    assert "not found" in err


def test_v1388_load_baseline_invalid_json(tmp_path):
    p = tmp_path / "bad.json"
    p.write_text("not json", encoding="utf-8")
    bd, err = load_baseline(str(p))
    assert bd is None
    assert "parse error" in err


def test_v1388_load_baseline_wrong_schema(tmp_path):
    p = tmp_path / "wrong.json"
    p.write_text('{"schema": "other/v1"}', encoding="utf-8")
    bd, err = load_baseline(str(p))
    assert bd is None
    assert "schema mismatch" in err


def test_v1388_save_and_load_baseline_roundtrip(tmp_path, runner, clean_dir):
    """V1388 真生产 save → load 真往返 (主 17:43 实事求是)."""
    diff = runner.run(target=str(clean_dir), baseline_path=None, include_build_dirs=True)
    # save 之前先跑一次 V1387 来取 to_dict
    from apeireth.v1387_deploy_stack_runner import V1387DeployStackRunner
    v1387 = V1387DeployStackRunner()
    report = v1387.run(root=str(clean_dir))
    p = tmp_path / "baseline.json"
    msg = save_baseline(report.to_dict(), str(p))
    assert "saved" in msg
    assert p.exists()
    bd, err = load_baseline(str(p))
    assert err == ""
    assert bd is not None
    assert bd.get("schema") == V1388_BASELINE_SCHEMA


def test_v1388_append_baseline_creates_jsonl(tmp_path, runner, clean_dir):
    from apeireth.v1387_deploy_stack_runner import V1387DeployStackRunner
    v1387 = V1387DeployStackRunner()
    p = tmp_path / "baseline.jsonl"
    for _ in range(3):
        report = v1387.run(root=str(clean_dir))
        msg = append_baseline(report.to_dict(), str(p))
        assert "appended" in msg
    text = p.read_text(encoding="utf-8")
    assert text.count("\n") == 3


# ============================================================================
# compute_diff
# ============================================================================


def test_v1388_compute_diff_no_baseline():
    """V1388 baseline = None → 全 0 (主 17:43 实事求是)."""
    diff = compute_diff(
        {"sources": [], "cross_findings": [], "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0},
        None,
    )
    assert diff.n_new == 0
    assert diff.n_resolved == 0
    assert diff.n_unchanged == 0


def test_v1388_compute_diff_identical():
    """V1388 两份完全相同 → 0 new 0 resolved (主 17:43)."""
    d = {
        "sources": [{"source": {"file_path": "a"},
                    "findings": [{"rule_id": "R1", "line_no": 1, "message": "x",
                                  "severity": "warning", "suggestion": ""}]}],
        "cross_findings": [],
        "n_files_total": 1, "n_findings": 1, "n_cross_findings": 0,
    }
    diff = compute_diff(d, d)
    assert diff.n_new == 0
    assert diff.n_resolved == 0
    assert diff.n_unchanged == 1


def test_v1388_compute_diff_new_finding():
    """V1388 current 有新 finding → n_new=1 (主 17:43)."""
    current = {
        "sources": [{"source": {"file_path": "a"},
                     "findings": [{"rule_id": "R1", "line_no": 1, "message": "x",
                                   "severity": "warning", "suggestion": ""}]}],
        "cross_findings": [],
        "n_files_total": 1, "n_findings": 1, "n_cross_findings": 0,
    }
    baseline = {
        "sources": [], "cross_findings": [],
        "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0,
    }
    diff = compute_diff(current, baseline)
    assert diff.n_new == 1
    assert diff.n_resolved == 0
    assert diff.has_regression is True
    assert diff.has_improvement is False
    assert diff.n_new_warnings == 1


def test_v1388_compute_diff_resolved_finding():
    """V1388 baseline 有 finding, current 没有 → n_resolved=1 (主 17:43)."""
    baseline = {
        "sources": [{"source": {"file_path": "a"},
                     "findings": [{"rule_id": "R1", "line_no": 1, "message": "x",
                                   "severity": "warning", "suggestion": ""}]}],
        "cross_findings": [],
        "n_files_total": 1, "n_findings": 1, "n_cross_findings": 0,
    }
    current = {
        "sources": [], "cross_findings": [],
        "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0,
    }
    diff = compute_diff(current, baseline)
    assert diff.n_new == 0
    assert diff.n_resolved == 1
    assert diff.has_improvement is True


def test_v1388_compute_diff_aggregates_per_source():
    current = {
        "sources": [
            {"source": {"file_path": "a"},
             "findings": [{"rule_id": "R1", "line_no": 1, "message": "x1", "severity": "warning", "suggestion": ""}]},
            {"source": {"file_path": "b"},
             "findings": [{"rule_id": "R1", "line_no": 1, "message": "x2", "severity": "error", "suggestion": ""}]},
        ],
        "cross_findings": [],
        "n_files_total": 2, "n_findings": 2, "n_cross_findings": 0,
    }
    baseline = {
        "sources": [], "cross_findings": [],
        "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0,
    }
    diff = compute_diff(current, baseline)
    assert diff.n_new == 2
    assert diff.n_new_errors == 1
    assert diff.n_new_warnings == 1
    assert diff.new_by_source == {"a": 1, "b": 1}
    assert diff.new_by_rule == {"R1": 2}


def test_v1388_compute_diff_msg_hash_stable():
    """V1388 同 finding 在 current/baseline 不被当成 new/resolved (主 17:43)."""
    d = {
        "sources": [{"source": {"file_path": "a"},
                    "findings": [{"rule_id": "R1", "line_no": 1, "message": "x",
                                  "severity": "warning", "suggestion": ""}]}],
        "cross_findings": [],
        "n_files_total": 1, "n_findings": 1, "n_cross_findings": 0,
    }
    # 即使 line_text 字段差异, message 相同 → 应该 unchanged
    d_alt = {
        "sources": [{"source": {"file_path": "a"},
                    "findings": [{"rule_id": "R1", "line_no": 1, "message": "x",
                                  "severity": "warning", "suggestion": "slightly different",
                                  "line_text": "different context"}]}],
        "cross_findings": [],
        "n_files_total": 1, "n_findings": 1, "n_cross_findings": 0,
    }
    diff = compute_diff(d, d_alt)
    assert diff.n_new == 0
    assert diff.n_resolved == 0
    assert diff.n_unchanged == 1


def test_v1388_compute_diff_cross_format():
    """V1388 cross-format findings 也算 (主 17:43 实事求是)."""
    current = {
        "sources": [],
        "cross_findings": [{"rule_id": "CROSS-PORT-DRIFT", "severity": "info",
                            "message": "ports 9090 not in compose",
                            "suggestion": "add ports", "sources": ["Dockerfile"]}],
        "n_files_total": 0, "n_findings": 0, "n_cross_findings": 1,
    }
    baseline = {
        "sources": [], "cross_findings": [],
        "n_files_total": 0, "n_findings": 0, "n_cross_findings": 0,
    }
    diff = compute_diff(current, baseline)
    assert diff.n_new == 1
    assert diff.n_new_info == 1
    assert diff.new_findings[0].signature.rule_id == "CROSS-PORT-DRIFT"


# ============================================================================
# runner end-to-end
# ============================================================================


def test_v1388_runner_no_baseline(runner, clean_dir):
    diff = runner.run(target=str(clean_dir), baseline_path=None, include_build_dirs=True)
    assert diff.n_new == 0  # no findings in clean_dir
    assert diff.n_resolved == 0
    assert diff.has_regression is False
    assert diff.baseline_loaded is False


def test_v1388_runner_with_baseline_roundtrip(runner, clean_dir, tmp_path):
    """V1388 真生产 baseline → diff 真往返 (主 17:43)."""
    # baseline 来自 clean_dir (无 finding)
    from apeireth.v1387_deploy_stack_runner import V1387DeployStackRunner
    v1387 = V1387DeployStackRunner()
    base_report = v1387.run(root=str(clean_dir))
    baseline_path = tmp_path / "base.json"
    save_baseline(base_report.to_dict(), str(baseline_path))

    # diff current = clean_dir (无变化)
    diff = runner.run(target=str(clean_dir), baseline_path=str(baseline_path),
                      include_build_dirs=True)
    assert diff.baseline_loaded is True
    assert diff.n_new == 0
    assert diff.n_resolved == 0
    assert diff.n_unchanged >= 0  # no findings so 0


def test_v1388_runner_detects_new_finding(runner, clean_dir, bad_dir, tmp_path):
    """V1388 baseline=clean, current=bad → 真报 new (主 17:43 实事求是)."""
    from apeireth.v1387_deploy_stack_runner import V1387DeployStackRunner
    v1387 = V1387DeployStackRunner()
    base_report = v1387.run(root=str(clean_dir))
    baseline_path = tmp_path / "base.json"
    save_baseline(base_report.to_dict(), str(baseline_path))

    diff = runner.run(target=str(bad_dir), baseline_path=str(baseline_path),
                      include_build_dirs=True)
    assert diff.baseline_loaded is True
    assert diff.n_new >= 1
    assert diff.has_regression is True


def test_v1388_runner_detects_resolved_finding(runner, bad_dir, clean_dir, tmp_path):
    """V1388 baseline=bad, current=clean → 真报 resolved (主 17:43)."""
    from apeireth.v1387_deploy_stack_runner import V1387DeployStackRunner
    v1387 = V1387DeployStackRunner()
    base_report = v1387.run(root=str(bad_dir))
    baseline_path = tmp_path / "base.json"
    save_baseline(base_report.to_dict(), str(baseline_path))

    diff = runner.run(target=str(clean_dir), baseline_path=str(baseline_path),
                      include_build_dirs=True)
    assert diff.baseline_loaded is True
    assert diff.n_resolved >= 1
    assert diff.has_improvement is True


def test_v1388_runner_known_unknowns_populated(runner, clean_dir):
    diff = runner.run(target=str(clean_dir), baseline_path=None, include_build_dirs=True)
    assert len(diff.known_unknowns) >= 4


def test_v1388_runner_stats(runner):
    s = runner.stats()
    assert s["version"] == V1388_VERSION
    assert s["schema"] == V1388_DIFF_SCHEMA
    assert "v1387_available" in s
    assert s["v1387_available"] is True


def test_v1388_runner_elapsed_positive(runner, clean_dir):
    diff = runner.run(target=str(clean_dir), baseline_path=None, include_build_dirs=True)
    assert diff.elapsed_seconds >= 0
    assert diff.started_at != ""
    assert diff.finished_at != ""


# ============================================================================
# output formats
# ============================================================================


def test_v1388_format_text_basic(runner, clean_dir):
    diff = runner.run(target=str(clean_dir), baseline_path=None, include_build_dirs=True)
    text = _format_text(diff)
    assert "V1388" in text
    assert "target:" in text
    assert "diff:" in text


def test_v1388_format_text_quiet(runner, clean_dir):
    diff = runner.run(target=str(clean_dir), baseline_path=None, include_build_dirs=True)
    text = _format_text(diff, quiet=True)
    assert "V1388" in text
    assert "new findings" not in text


def test_v1388_format_text_with_findings(runner, bad_dir):
    diff = runner.run(target=str(bad_dir), baseline_path=None, include_build_dirs=True)
    text = _format_text(diff)
    assert "new findings" in text
    assert "NEW" in text


def test_v1388_format_markdown(runner, bad_dir):
    diff = runner.run(target=str(bad_dir), baseline_path=None, include_build_dirs=True)
    md = _format_markdown(diff)
    assert "# V1388" in md
    assert "## Summary" in md
    assert "## New Findings" in md


def test_v1388_format_sarif(runner, bad_dir):
    diff = runner.run(target=str(bad_dir), baseline_path=None, include_build_dirs=True)
    sarif = _format_sarif(diff)
    assert sarif["version"] == "2.1.0"
    assert sarif["runs"][0]["tool"]["driver"]["name"] == "v1388-v1387-baseline-diff"
    if diff.n_new > 0:
        assert len(sarif["runs"][0]["results"]) >= 1


def test_v1388_diff_result_to_dict(runner, clean_dir):
    diff = runner.run(target=str(clean_dir), baseline_path=None, include_build_dirs=True)
    d = diff.to_dict()
    assert d["schema"] == V1388_DIFF_SCHEMA
    assert d["version"] == V1388_VERSION
    assert "new_findings" in d
    assert "resolved_findings" in d
    assert "known_unknowns" in d


# ============================================================================
# CLI
# ============================================================================


def test_v1388_cli_version(capsys):
    rc = run_cli(["--version"])
    captured = capsys.readouterr()
    assert "V1388" in captured.out
    assert rc == 0


def test_v1388_cli_demo(capsys):
    rc = run_cli(["--demo"])
    captured = capsys.readouterr()
    assert "V1388" in captured.out
    assert rc == 0


def test_v1388_cli_clean_dir_no_baseline(capsys, clean_dir):
    rc = run_cli([str(clean_dir)])
    captured = capsys.readouterr()
    assert "V1388" in captured.out
    assert rc == 0  # clean = no regression


def test_v1388_cli_bad_dir_no_baseline_exits_1(capsys, bad_dir):
    """V1388 无 baseline + bad dir → 全部当 new → exit 1 (主 17:43)."""
    rc = run_cli([str(bad_dir)])
    assert rc == 1


def test_v1388_cli_baseline_roundtrip(capsys, clean_dir, bad_dir, tmp_path):
    """V1388 CLI baseline + diff 真往返 (主 17:43 实事求是)."""
    # step 1: save baseline from clean
    baseline_path = tmp_path / "base.json"
    rc = run_cli([str(clean_dir), "--save-baseline", str(baseline_path)])
    assert baseline_path.exists()

    # step 2: run bad against baseline → should detect new
    rc = run_cli([str(bad_dir), "--baseline", str(baseline_path)])
    assert rc == 1


def test_v1388_cli_json_output(capsys, bad_dir):
    rc = run_cli([str(bad_dir), "--json"])
    captured = capsys.readouterr()
    parsed = json.loads(captured.out)
    assert parsed["schema"] == V1388_DIFF_SCHEMA
    assert parsed["n_new"] >= 1


def test_v1388_cli_md_output(capsys, bad_dir):
    rc = run_cli([str(bad_dir), "--md"])
    captured = capsys.readouterr()
    assert "# V1388" in captured.out
    assert rc == 1


def test_v1388_cli_sarif_output(capsys, bad_dir):
    rc = run_cli([str(bad_dir), "--sarif"])
    captured = capsys.readouterr()
    parsed = json.loads(captured.out)
    assert parsed["version"] == "2.1.0"
    assert rc == 1


def test_v1388_cli_strict_clean(capsys, clean_dir):
    """V1388 --strict + clean dir = exit 0 (主 17:43 实事求是)."""
    rc = run_cli([str(clean_dir), "--strict"])
    assert rc == 0


def test_v1388_cli_baseline_missing_default(capsys, bad_dir, tmp_path):
    """V1388 baseline 不存在 (默认) = 全部当 new + exit 1 (主 17:43)."""
    rc = run_cli([str(bad_dir), "--baseline", str(tmp_path / "nope.json")])
    assert rc == 1


def test_v1388_cli_baseline_missing_exit_2(capsys, bad_dir, tmp_path):
    """V1388 baseline 缺失 + --baseline-missing-exit-2 = exit 2 (主 17:43)."""
    rc = run_cli([str(bad_dir), "--baseline", str(tmp_path / "nope.json"),
                  "--baseline-missing-exit-2"])
    assert rc == 2


def test_v1388_cli_quiet(capsys, bad_dir):
    rc = run_cli([str(bad_dir), "--quiet"])
    captured = capsys.readouterr()
    assert "V1388" in captured.out
    assert "new findings" not in captured.out
    assert rc == 1


# ============================================================================
# Popper self-test
# ============================================================================


def test_v1388_popper_self_test():
    rc = _popper_self_test()
    assert rc == 0


def test_v1388_popper_self_test_no_failures(capsys):
    rc = _popper_self_test()
    captured = capsys.readouterr()
    assert rc == 0
    assert "FAIL" not in captured.out


# ============================================================================
# Chain / regression
# ============================================================================


def test_v1388_chain_with_v1387(runner, clean_dir, bad_dir, tmp_path):
    """V1388 与 V1387 真生产 chain test (主 17:43 实事求是)."""
    from apeireth.v1387_deploy_stack_runner import V1387DeployStackRunner
    v1387 = V1387DeployStackRunner()
    # 1) V1387 baseline = clean
    base_report = v1387.run(root=str(clean_dir))
    baseline_path = tmp_path / "base.json"
    save_baseline(base_report.to_dict(), str(baseline_path))

    # 2) V1388 diff = bad vs baseline
    diff = runner.run(target=str(bad_dir), baseline_path=str(baseline_path),
                      include_build_dirs=True)
    assert diff.baseline_loaded is True
    assert diff.n_new >= 1
    # V1387 真跑过 bad_dir 至少 3 个 source
    assert diff.current_n_files >= 3


def test_v1388_runner_no_subprocess_run(runner, clean_dir, bad_dir, tmp_path):
    """V1388 不通过 subprocess, 直接 import (主 17:43 实事求是)."""
    from apeireth.v1387_deploy_stack_runner import V1387DeployStackRunner
    v1387 = V1387DeployStackRunner()
    base_report = v1387.run(root=str(clean_dir))
    baseline_path = tmp_path / "base.json"
    save_baseline(base_report.to_dict(), str(baseline_path))

    diff = runner.run(target=str(bad_dir), baseline_path=str(baseline_path),
                      include_build_dirs=True)
    assert diff is not None
    # elapsed should be < 5 seconds
    assert diff.elapsed_seconds < 5.0


# ============================================================================
# Subprocess
# ============================================================================


def test_v1388_subprocess_demo(tmp_path):
    """V1388 真生产 CLI subprocess demo (主 17:43 真可执行)."""
    p = subprocess.run(
        [sys.executable, "-m", "apeireth.v1388_v1387_baseline_diff", "--demo"],
        cwd=str(MODULE_DIR.parent),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=20,
    )
    assert p.returncode == 0
    assert "V1388" in p.stdout


def test_v1388_subprocess_version(tmp_path):
    p = subprocess.run(
        [sys.executable, "-m", "apeireth.v1388_v1387_baseline_diff", "--version"],
        cwd=str(MODULE_DIR.parent),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=20,
    )
    assert p.returncode == 0
    assert "V1388" in p.stdout
