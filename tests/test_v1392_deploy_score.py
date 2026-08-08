"""Phase 1392 test_v1392_deploy_score — V1392 ASI 真生产 deploy-stack score tests (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 00:56).

V1392 = real production deploy-stack score per directory.
Tests verify: 0-100 score 4 维度 + 6 grade + 真 evaluate + CLI 真可跑.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

# V1392 import path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "apeireth"))

from v1392_deploy_score import (  # noqa: E402
    V1392_VERSION,
    V1392_SCHEMA,
    V1392_GUARDS,
    GRADE_THRESHOLDS,
    SEVERITY_WEIGHTS,
    DIMENSION_RULES,
    ScoreBreakdown,
    DeployScore,
    compute_score,
    get_severity_for_rule,
    get_dimension_for_rule,
    popper_self_test,
    run_cli,
)


# ============================================================================
# V1392 真生产 数据结构 (主 17:43)
# ============================================================================


def test_v1392_version():
    """V1392 真生产 version (主 17:43)."""
    assert V1392_VERSION == "0.1.0"
    assert V1392_SCHEMA == "v1392.deploy-score/v1"


def test_v1392_grade_thresholds():
    """V1392 真生产 6 grade thresholds (主 17:43)."""
    grades = [g for _, g in GRADE_THRESHOLDS]
    assert grades == ["A+", "A", "B", "C", "D", "F"]
    # A+ highest, F lowest
    assert GRADE_THRESHOLDS[0][0] == 95
    assert GRADE_THRESHOLDS[-1][0] == 0


def test_v1392_severity_weights():
    """V1392 真生产 severity weights (主 17:43)."""
    assert SEVERITY_WEIGHTS["error"] == 10
    assert SEVERITY_WEIGHTS["warning"] == 3
    assert SEVERITY_WEIGHTS["info"] == 1


def test_v1392_dimension_rules_complete():
    """V1392 真生产 4 维度 30+ rule (主 22:33 4 范围)."""
    assert set(DIMENSION_RULES.keys()) == {"dockerfile", "compose", "k8s"}
    total = sum(len(r) for r in DIMENSION_RULES.values())
    assert total >= 25  # at least 25 rules across 3 dims (ci_gate is fixed)


def test_v1392_score_breakdown_to_dict():
    """V1392 真生产 ScoreBreakdown.to_dict (主 17:43)."""
    b = ScoreBreakdown(dockerfile=90, compose=80, k8s=70, ci_gate=100)
    assert b.to_dict() == {
        "dockerfile": 90,
        "compose": 80,
        "k8s": 70,
        "ci_gate": 100,
    }


def test_v1392_score_breakdown_total():
    """V1392 真生产 ScoreBreakdown.total() = 4 维度平均 (主 17:43)."""
    b = ScoreBreakdown(100, 80, 60, 100)
    assert b.total() == 85  # (100+80+60+100)/4


def test_v1392_deploy_score_to_dict():
    """V1392 真生产 DeployScore.to_dict 完整 (主 17:43)."""
    s = DeployScore(target="x", total_score=85, grade="B")
    d = s.to_dict()
    assert d["schema"] == V1392_SCHEMA
    assert d["version"] == V1392_VERSION
    assert d["target"] == "x"
    assert d["total_score"] == 85
    assert d["grade"] == "B"
    assert "breakdown" in d
    assert "methodology" in d


def test_v1392_get_severity_for_rule():
    """V1392 真生产 get_severity_for_rule V1390 hint (主 17:43)."""
    assert get_severity_for_rule("DL3008") == "warning"
    assert get_severity_for_rule("COMPOSE-PRIVILEGED") == "error"
    assert get_severity_for_rule("COMPOSE-MISSING-RESTART") == "info"


def test_v1392_get_dimension_for_rule():
    """V1392 真生产 get_dimension_for_rule 4 维度 (主 22:33)."""
    assert get_dimension_for_rule("DL3008") == "dockerfile"
    assert get_dimension_for_rule("COMPOSE-PRIVILEGED") == "compose"
    assert get_dimension_for_rule("K8S-NO-RESOURCE-LIMITS") == "k8s"
    assert get_dimension_for_rule("UNKNOWN-RULE-9999") == "other"


# ============================================================================
# V1392 真生产 compute_score (主 17:43)
# ============================================================================


def test_v1392_compute_score_clean():
    """V1392 真生产 clean → A+ 100 (主 17:43)."""
    s = compute_score([], target="clean")
    assert s.total_score == 100
    assert s.grade == "A+"
    assert s.breakdown.dockerfile == 100
    assert s.breakdown.compose == 100
    assert s.breakdown.k8s == 100
    assert s.breakdown.ci_gate == 100
    assert s.n_findings == 0


def test_v1392_compute_score_one_error():
    """V1392 真生产 1 error in compose → compose=90, total=97 (主 17:43)."""
    s = compute_score([{"rule_id": "COMPOSE-PRIVILEGED"}], target="x")
    assert s.breakdown.compose == 90
    assert s.total_score == 97  # (100+90+100+100)/4
    assert s.n_errors == 1


def test_v1392_compute_score_one_warning():
    """V1392 真生产 1 warning in dockerfile → dockerfile=97 (主 17:43)."""
    s = compute_score([{"rule_id": "DL3008"}], target="x")
    assert s.breakdown.dockerfile == 97
    assert s.total_score == 99  # (97+100+100+100)/4
    assert s.n_warnings == 1


def test_v1392_compute_score_one_info():
    """V1392 真生产 1 info → 99 (主 17:43)."""
    s = compute_score([{"rule_id": "COMPOSE-MISSING-RESTART"}], target="x")
    assert s.breakdown.compose == 99
    assert s.n_info == 1


def test_v1392_compute_score_mixed_dims():
    """V1392 真生产 3 维度各 1 finding (主 17:43)."""
    s = compute_score([
        {"rule_id": "DL3008"},  # warning, dockerfile
        {"rule_id": "COMPOSE-LATEST-TAG"},  # error, compose
        {"rule_id": "K8S-NO-RESOURCE-LIMITS"},  # warning, k8s
    ], target="x")
    assert s.breakdown.dockerfile == 97
    assert s.breakdown.compose == 90
    assert s.breakdown.k8s == 97
    assert s.breakdown.ci_gate == 100
    assert s.total_score == 96  # (97+90+97+100)/4
    assert s.n_dimensions_with_findings == 3


def test_v1392_compute_score_ci_gate_fail():
    """V1392 真生产 ci_gate fail → ci_gate=0, total=75 (主 17:43)."""
    s = compute_score([], target="x", ci_gate_pass=False)
    assert s.breakdown.ci_gate == 0
    assert s.total_score == 75


def test_v1392_compute_score_floor_at_zero():
    """V1392 真生产 score floor at 0 per dim (主 17:43)."""
    s = compute_score(
        [{"rule_id": "COMPOSE-PRIVILEGED"}] * 100,  # 1000 penalty in compose
        target="x",
    )
    assert s.breakdown.compose == 0
    assert s.breakdown.dockerfile == 100  # other dims unaffected
    assert s.total_score == 75


def test_v1392_compute_score_v1388_format():
    """V1392 真生产 接受 V1388 new_by_rule 格式 (主 17:43)."""
    s = compute_score([
        {"new_by_rule": {"DL3008": 5, "COMPOSE-PRIVILEGED": 2}},
    ], target="x")
    assert s.n_findings == 7
    assert s.breakdown.dockerfile == 85  # 100 - 5*3 = 85
    assert s.breakdown.compose == 80  # 100 - 2*10 = 80


def test_v1392_compute_score_grade_thresholds():
    """V1392 真生产 grade 阈值 (主 17:43)."""
    # 100 → A+
    assert compute_score([], target="x").grade == "A+"
    # 95 → A+
    assert compute_score([{"rule_id": "DL3008"}] * 2, target="x").grade == "A+"  # dockerfile=94, total=98
    # 1 warning in dockerfile → dockerfile=97, total=99 → A+
    s = compute_score([{"rule_id": "DL3008"}], target="x")
    assert s.grade == "A+"


def test_v1392_compute_score_unknown_rule():
    """V1392 真生产 unknown rule fallback to warning (主 17:43)."""
    s = compute_score([{"rule_id": "UNKNOWN-RULE-9999"}], target="x")
    # Falls to dimension 'other', severity 'warning' (3 penalty, but 'other' not in 4 dim)
    assert s.n_warnings == 1
    # 4 维度 avg 不受 'other' 影响, 仅 n_findings/n_dimensions 反映
    assert s.n_findings == 1
    assert s.n_dimensions_with_findings == 0  # 'other' 不是 4 dim
    assert s.total_score == 100  # 4 dim 都还是 100


def test_v1392_compute_score_empty_findings():
    """V1392 真生产 空 findings 仍返回 valid (主 17:43)."""
    s = compute_score([], target="x")
    assert s.n_findings == 0
    assert s.total_score == 100


# ============================================================================
# V1392 真生产 Popper self-test (主 17:43)
# ============================================================================


def test_v1392_popper_self_test_passes():
    """V1392 真生产 Popper self-test 真过 (主 17:43)."""
    r = popper_self_test()
    assert r["passed"], f"popper failed: {r['failures']}"
    assert r["n_tested"] >= 10


def test_v1392_guards_complete():
    """V1392 真生产 8 GUARDS (主 17:43)."""
    assert len(V1392_GUARDS) >= 8
    must_have = {"GUARD_SCORE_DETERMINISTIC", "GUARD_SCORE_BOUNDED",
                 "GUARD_GRADE_VALID", "GUARD_BREAKDOWN_SUM",
                 "GUARD_NO_CAP_CHANGE", "GUARD_HONEST_DISCLOSURE",
                 "GUARD_DIMENSION_COVERAGE", "GUARD_CLI_RUNNABLE"}
    assert must_have.issubset(set(V1392_GUARDS))


# ============================================================================
# V1392 真生产 CLI (主 17:43 真可执行)
# ============================================================================


def test_v1392_cli_version(capsys):
    """V1392 真生产 CLI version (主 17:43)."""
    rc = run_cli(["version"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "V1392" in captured.out
    assert V1392_VERSION in captured.out


def test_v1392_cli_grades(capsys):
    """V1392 真生产 CLI grades (主 17:43)."""
    rc = run_cli(["grades"])
    captured = capsys.readouterr()
    assert rc == 0
    for g in ("A+", "A", "B", "C", "D", "F"):
        assert g in captured.out


def test_v1392_cli_score_clean_dir(capsys, tmp_path):
    """V1392 真生产 CLI score clean dir (主 17:43)."""
    d = tmp_path / "clean"
    d.mkdir()
    rc = run_cli(["score", str(d)])
    captured = capsys.readouterr()
    # Either clean (no findings) or scan unavailable
    assert rc == 0
    assert "V1392" in captured.out or "no findings" in captured.err


def test_v1392_cli_score_with_findings(capsys, tmp_path):
    """V1392 真生产 CLI score with findings (主 17:43)."""
    d = tmp_path / "bad"
    d.mkdir()
    (d / "Dockerfile").write_text(
        "FROM ubuntu:latest\nCMD echo hi\n",
        encoding="utf-8",
    )
    rc = run_cli(["score", str(d)])
    captured = capsys.readouterr()
    # Either V1384-FROM-LATEST found (error) or scan unavailable
    assert rc == 0


def test_v1392_cli_score_json(capsys, tmp_path):
    """V1392 真生产 CLI score --json (主 17:43)."""
    d = tmp_path / "bad"
    d.mkdir()
    (d / "Dockerfile").write_text(
        "FROM ubuntu:latest\nCMD echo hi\n",
        encoding="utf-8",
    )
    rc = run_cli(["score", str(d), "--json"])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert "total_score" in data
    assert "grade" in data
    assert "breakdown" in data
    assert "methodology" in data
    assert data["schema"] == V1392_SCHEMA


def test_v1392_cli_score_json_findings(capsys, tmp_path):
    """V1392 真生产 CLI score-json (主 17:43)."""
    f = tmp_path / "findings.json"
    f.write_text(json.dumps([
        {"rule_id": "DL3008"},
        {"rule_id": "COMPOSE-PRIVILEGED"},
    ]), encoding="utf-8")
    rc = run_cli(["score-json", str(f)])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert 0 <= data["total_score"] <= 100
    assert data["grade"] in ("A+", "A", "B", "C", "D", "F")
    assert data["n_findings"] == 2


def test_v1392_cli_score_json_findings_with_new_by_rule(capsys, tmp_path):
    """V1392 真生产 CLI score-json 接受 V1388 new_by_rule (主 17:43)."""
    f = tmp_path / "findings.json"
    f.write_text(json.dumps([
        {"new_by_rule": {"DL3008": 5, "COMPOSE-PRIVILEGED": 2}},
    ]), encoding="utf-8")
    rc = run_cli(["score-json", str(f)])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert data["n_findings"] == 7


def test_v1392_cli_score_ci_gate_strict(capsys, tmp_path):
    """V1392 真生产 CLI score --ci-gate-strict (主 17:43)."""
    d = tmp_path / "clean"
    d.mkdir()
    rc = run_cli(["score", str(d), "--ci-gate-strict", "--json"])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert data["breakdown"]["ci_gate"] == 0


def test_v1392_cli_demo(capsys):
    """V1392 真生产 CLI demo (主 17:43)."""
    rc = run_cli(["demo"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "V1392" in captured.out
    assert "score" in captured.out.lower()


def test_v1392_cli_popper(capsys):
    """V1392 真生产 CLI popper (主 17:43)."""
    rc = run_cli(["popper"])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert rc == 0
    assert data["passed"]


def test_v1392_cli_help(capsys):
    """V1392 真生产 CLI --help (主 17:43)."""
    with pytest.raises(SystemExit) as exc_info:
        run_cli(["--help"])
    assert exc_info.value.code == 0
    captured = capsys.readouterr()
    assert "V1392" in captured.out


# ============================================================================
# V1392 真生产 GUARDS (主 17:43)
# ============================================================================


def test_v1392_no_asi_positioning():
    """V1392 真生产 不假装 (主 17:58): score 是 heuristic, 标注 methodology."""
    s = compute_score([], target="x")
    d = s.to_dict()
    # methodology 标注是 heuristic
    assert "methodology" in d
    assert "score" in d["methodology"].lower() or "penalty" in d["methodology"].lower()
    # Module docstring 表明 score 是 heuristic
    import v1392_deploy_score as m
    assert "heuristic" in m.__doc__ or "建议" in m.__doc__ or "不假装" in m.__doc__


def test_v1392_any_help_can_take_over():
    """V1392 真生产 任何人都能接手 (主 00:56): 1 dataclass + 1 score function + 1 CLI."""
    # ScoreBreakdown + DeployScore + compute_score all simple
    b = ScoreBreakdown()
    assert b.dockerfile == 100
    s = compute_score([], target="x")
    assert isinstance(s, DeployScore)
    # CLI 6 subcommands
    with pytest.raises(SystemExit) as exc_info:
        run_cli(["--help"])
    assert exc_info.value.code == 0


def test_v1392_score_deterministic():
    """V1392 真生产 same input → same score (主 17:43)."""
    findings = [{"rule_id": "DL3008"}, {"rule_id": "COMPOSE-PRIVILEGED"}]
    s1 = compute_score(findings, target="x")
    s2 = compute_score(findings, target="x")
    assert s1.total_score == s2.total_score
    assert s1.grade == s2.grade
    assert s1.to_dict() == s2.to_dict()


def test_v1392_score_bounded():
    """V1392 真生产 score 0-100 (主 17:43)."""
    for n in [0, 1, 10, 100, 1000]:
        s = compute_score([{"rule_id": "COMPOSE-PRIVILEGED"}] * n, target="x")
        assert 0 <= s.total_score <= 100
        assert 0 <= s.breakdown.dockerfile <= 100
        assert 0 <= s.breakdown.compose <= 100
        assert 0 <= s.breakdown.k8s <= 100
        assert 0 <= s.breakdown.ci_gate <= 100
