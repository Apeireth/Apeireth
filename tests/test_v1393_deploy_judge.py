"""Phase 1393 test_v1393_deploy_judge — V1393 ASI 真生产 deploy-stack judge tests (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 00:56).

V1393 = real production deploy-stack judge: 1 个 CLI 汇总 V1384-V1392.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

# V1393 import path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "apeireth"))

from v1393_deploy_judge import (  # noqa: E402
    V1393_VERSION,
    V1393_SCHEMA,
    V1393_GUARDS,
    VERDICT_RULES,
    JudgeResult,
    _compute_verdict,
    judge,
    popper_self_test,
    render_markdown,
    run_cli,
)


# ============================================================================
# V1393 真生产 数据结构 (主 17:43)
# ============================================================================


def test_v1393_version():
    """V1393 真生产 version (主 17:43)."""
    assert V1393_VERSION == "0.1.0"
    assert V1393_SCHEMA == "v1393.deploy-judge/v1"


def test_v1393_compute_verdict_good():
    """V1393 真生产 verdict GOOD (主 17:43)."""
    assert _compute_verdict(100, "A+", True) == "GOOD"
    assert _compute_verdict(90, "A", True) == "GOOD"


def test_v1393_compute_verdict_ok():
    """V1393 真生产 verdict OK (主 17:43)."""
    assert _compute_verdict(75, "B", True) == "OK"


def test_v1393_compute_verdict_poor():
    """V1393 真生产 verdict POOR (主 17:43)."""
    assert _compute_verdict(50, "D", True) == "POOR"
    assert _compute_verdict(60, "C", True) == "POOR"


def test_v1393_compute_verdict_fail():
    """V1393 真生产 verdict FAIL when policy fails (主 17:43)."""
    assert _compute_verdict(100, "A+", False) == "FAIL"
    assert _compute_verdict(80, "B", False) == "FAIL"


def test_v1393_compute_verdict_critical():
    """V1393 真生产 verdict CRITICAL when grade F (主 17:43)."""
    assert _compute_verdict(30, "F", True) == "CRITICAL"
    assert _compute_verdict(0, "F", False) == "CRITICAL"


def test_v1393_judge_result_to_dict():
    """V1393 真生产 JudgeResult.to_dict 完整 (主 17:43)."""
    r = JudgeResult(target="x", verdict="GOOD")
    d = r.to_dict()
    assert d["schema"] == V1393_SCHEMA
    assert d["version"] == V1393_VERSION
    assert d["target"] == "x"
    assert d["verdict"] == "GOOD"
    assert "n_findings" in d
    assert "policy_pass" in d
    assert "deploy_score" in d
    assert "deploy_grade" in d
    assert "deploy_breakdown" in d
    assert "source_modules" in d


def test_v1393_judge_result_source_modules():
    """V1393 真生产 source_modules (主 17:43)."""
    r = JudgeResult(target="x")
    assert isinstance(r.source_modules, list)


# ============================================================================
# V1393 真生产 judge (主 17:43)
# ============================================================================


def test_v1393_judge_empty_dir(tmp_path):
    """V1393 真生产 judge empty dir (主 17:43)."""
    d = tmp_path / "empty"
    d.mkdir()
    res = judge(str(d))
    assert res.target == str(d)
    assert res.verdict in ("CRITICAL", "FAIL", "POOR", "OK", "GOOD")
    assert res.deploy_score >= 0


def test_v1393_judge_nonexistent_path():
    """V1393 真生产 judge nonexistent path (主 17:43)."""
    res = judge("___nonexistent_path_xyz___")
    assert res.target == "___nonexistent_path_xyz___"
    # Should still produce a valid verdict
    assert res.verdict in ("CRITICAL", "FAIL", "POOR", "OK", "GOOD")


def test_v1393_judge_with_real_bad_dir():
    """V1393 真生产 judge real bad dir (主 17:43)."""
    d = Path(r"C:\tmp\bad-deploy-real")
    if not d.exists():
        pytest.skip("bad-deploy-real not present")
    res = judge(str(d))
    assert res.n_findings >= 1
    assert res.n_hints >= 1
    assert res.deploy_score < 100
    assert res.verdict in ("CRITICAL", "FAIL", "POOR")


def test_v1393_judge_with_policy_yaml(tmp_path):
    """V1393 真生产 judge with explicit policy YAML (主 17:43)."""
    p = tmp_path / "policy.yaml"
    p.write_text("""
name: minimal-policy
version: "0.1.0"
schema: v1391.policy-gate/v1
rules:
  - rule_id: DEFAULT
    severity: error
    max_count: 0
""", encoding="utf-8")
    d = tmp_path / "bad"
    d.mkdir()
    (d / "Dockerfile").write_text("FROM ubuntu:latest\n", encoding="utf-8")
    res = judge(str(d), policy_path=str(p))
    assert res.target == str(d)
    # V1384-FROM-LATEST is error → 1 error > 0 → policy fail
    assert not res.policy_pass
    assert res.policy_n_violations >= 1


def test_v1393_judge_default_policy_fallback(tmp_path):
    """V1393 真生产 judge 不指定 policy → fallback default (主 17:43)."""
    d = tmp_path / "empty"
    d.mkdir()
    res = judge(str(d))
    # Default policy, empty dir → no findings → pass
    assert res.policy_pass


def test_v1393_judge_breakdown_populated(tmp_path):
    """V1393 真生产 judge 4 维度 breakdown 必填 (主 22:33)."""
    d = tmp_path / "empty"
    d.mkdir()
    res = judge(str(d))
    # When no findings, breakdown may be all 100 (default)
    assert "dockerfile" in res.deploy_breakdown
    assert "compose" in res.deploy_breakdown
    assert "k8s" in res.deploy_breakdown
    assert "ci_gate" in res.deploy_breakdown


def test_v1393_judge_json_serializable(tmp_path):
    """V1393 真生产 judge result JSON 可序列化 (主 17:43)."""
    d = tmp_path / "empty"
    d.mkdir()
    res = judge(str(d))
    s = json.dumps(res.to_dict(), indent=2, ensure_ascii=False)
    parsed = json.loads(s)
    assert parsed["schema"] == V1393_SCHEMA


# ============================================================================
# V1393 真生产 render_markdown (主 17:43)
# ============================================================================


def test_v1393_render_markdown_basic(tmp_path):
    """V1393 真生产 render_markdown 完整 (主 17:43)."""
    d = tmp_path / "empty"
    d.mkdir()
    res = judge(str(d))
    md = render_markdown(res)
    assert "V1393" in md
    assert "Verdict" in md
    assert "Source" in md or "source" in md.lower()


def test_v1393_render_markdown_has_breakdown(tmp_path):
    """V1393 真生产 render_markdown 有 4 维度 breakdown (主 22:33)."""
    d = tmp_path / "empty"
    d.mkdir()
    res = judge(str(d))
    md = render_markdown(res)
    assert "dockerfile" in md
    assert "compose" in md
    assert "k8s" in md
    assert "ci_gate" in md


# ============================================================================
# V1393 真生产 Popper self-test (主 17:43)
# ============================================================================


def test_v1393_popper_self_test_passes():
    """V1393 真生产 Popper self-test 真过 (主 17:43)."""
    r = popper_self_test()
    assert r["passed"], f"popper failed: {r['failures']}"
    assert r["n_tested"] >= 10


def test_v1393_guards_complete():
    """V1393 真生产 8 GUARDS (主 17:43)."""
    assert len(V1393_GUARDS) >= 8
    must_have = {"GUARD_JUDGE_REAL", "GUARD_NO_CAP_CHANGE", "GUARD_DETERMINISTIC",
                 "GUARD_HONEST_DISCLOSURE", "GUARD_VERDICT_VALID", "GUARD_DELEGATE_REAL",
                 "GUARD_NO_FALLBACK", "GUARD_CLI_RUNNABLE"}
    assert must_have.issubset(set(V1393_GUARDS))


# ============================================================================
# V1393 真生产 CLI (主 17:43 真可执行)
# ============================================================================


def test_v1393_cli_version(capsys):
    """V1393 真生产 CLI version (主 17:43)."""
    rc = run_cli(["version"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "V1393" in captured.out
    assert V1393_VERSION in captured.out


def test_v1393_cli_judge_clean_dir(capsys, tmp_path):
    """V1393 真生产 CLI judge clean dir (主 17:43)."""
    d = tmp_path / "clean"
    d.mkdir()
    rc = run_cli(["judge", str(d)])
    captured = capsys.readouterr()
    assert "V1393" in captured.out
    assert "verdict" in captured.out.lower() or "GOOD" in captured.out or "OK" in captured.out


def test_v1393_cli_judge_json(capsys, tmp_path):
    """V1393 真生产 CLI judge --json (主 17:43)."""
    d = tmp_path / "clean"
    d.mkdir()
    rc = run_cli(["judge", str(d), "--json"])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert data["schema"] == V1393_SCHEMA
    assert "verdict" in data
    assert "deploy_score" in data


def test_v1393_cli_judge_md(capsys, tmp_path):
    """V1393 真生产 CLI judge --md (主 17:43)."""
    d = tmp_path / "clean"
    d.mkdir()
    rc = run_cli(["judge", str(d), "--md"])
    captured = capsys.readouterr()
    assert "V1393" in captured.out
    assert "Verdict" in captured.out


def test_v1393_cli_judge_with_policy(capsys, tmp_path):
    """V1393 真生产 CLI judge --policy (主 17:43)."""
    p = tmp_path / "policy.yaml"
    p.write_text("""
name: minimal
version: "0.1.0"
schema: v1391.policy-gate/v1
rules:
  - rule_id: DEFAULT
    severity: error
    max_count: 0
""", encoding="utf-8")
    d = tmp_path / "bad"
    d.mkdir()
    (d / "Dockerfile").write_text("FROM ubuntu:latest\n", encoding="utf-8")
    rc = run_cli(["judge", str(d), "--policy", str(p), "--json"])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert data["policy_pass"] is False


def test_v1393_cli_demo(capsys):
    """V1393 真生产 CLI demo (主 17:43)."""
    rc = run_cli(["demo"])
    captured = capsys.readouterr()
    assert rc in (0, 1)
    assert "V1393" in captured.out


def test_v1393_cli_popper(capsys):
    """V1393 真生产 CLI popper (主 17:43)."""
    rc = run_cli(["popper"])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert rc == 0
    assert data["passed"]


def test_v1393_cli_help(capsys):
    """V1393 真生产 CLI --help (主 17:43)."""
    with pytest.raises(SystemExit) as exc_info:
        run_cli(["--help"])
    assert exc_info.value.code == 0
    captured = capsys.readouterr()
    assert "V1393" in captured.out


# ============================================================================
# V1393 真生产 哲学守门 (主 17:58 + 主 17:43)
# ============================================================================


def test_v1393_no_asi_positioning():
    """V1393 真生产 不假装 (主 17:58): judge 是 heuristic, 标注 methodology."""
    import v1393_deploy_judge as m
    # Module docstring 表明 judge 是 heuristic
    assert "heuristic" in m.__doc__ or "建议" in m.__doc__ or "不假装" in m.__doc__
    # VERDICT_RULES 5 决策 公开
    assert len(VERDICT_RULES) >= 4


def test_v1393_any_help_can_take_over():
    """V1393 真生产 任何人都能接手 (主 00:56): 1 judge + 1 CLI."""
    # JudgeResult simple dataclass
    r = JudgeResult(target="x")
    assert r.verdict == "GOOD"
    # judge 是单一函数
    res = judge("___nonexistent_xyz___")
    assert isinstance(res, JudgeResult)
    # 3 CLI subcommands
    with pytest.raises(SystemExit) as exc_info:
        run_cli(["--help"])
    assert exc_info.value.code == 0


def test_v1393_judge_deterministic(tmp_path):
    """V1393 真生产 same input → same verdict (主 17:43)."""
    d = tmp_path / "empty"
    d.mkdir()
    res1 = judge(str(d))
    res2 = judge(str(d))
    assert res1.verdict == res2.verdict
    assert res1.deploy_score == res2.deploy_score
    assert res1.deploy_grade == res2.deploy_grade
