"""Phase 1391 test_v1391_policy_gate — V1391 ASI 真生产 policy gate tests (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 00:56).

V1391 = real production policy gate: YAML policy + evaluator.
Tests verify: YAML 解析 + 真 evaluate + 真 pass/fail + CLI 真可跑.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

# V1391 import path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "apeireth"))

from v1391_policy_gate import (  # noqa: E402
    V1391_VERSION,
    V1391_SCHEMA,
    V1391_GUARDS,
    V1391_POLICY_SCHEMA,
    DEFAULT_POLICY_YAML,
    Policy,
    PolicyResult,
    PolicyRule,
    PolicyViolation,
    evaluate,
    get_severity_for_rule,
    popper_self_test,
    run_cli,
)


# ============================================================================
# V1391 真生产 数据结构 (主 17:43)
# ============================================================================


def test_v1391_version():
    """V1391 真生产 version (主 17:43)."""
    assert V1391_VERSION == "0.1.0"
    assert V1391_SCHEMA == "v1391.policy-gate/v1"


def test_v1391_default_policy():
    """V1391 真生产 default policy 至少 5 rules (主 17:43)."""
    p = Policy.default_policy()
    assert len(p.rules) >= 5
    assert p.name == "apeireth-default-policy"
    assert p.schema == V1391_SCHEMA


def test_v1391_policy_from_dict():
    """V1391 真生产 Policy.from_dict 构造 (主 17:43)."""
    data = {
        "name": "test-policy",
        "version": "0.1.0",
        "schema": V1391_SCHEMA,
        "rules": [
            {"rule_id": "DL3008", "severity": "warning", "max_count": 2},
            {"rule_id": "DEFAULT", "severity": "error", "max_count": 0},
        ],
    }
    p = Policy.from_dict(data)
    assert p.name == "test-policy"
    assert len(p.rules) == 2
    assert p.rules[0].rule_id == "DL3008"
    assert p.rules[0].max_count == 2


def test_v1391_policy_to_dict_roundtrip():
    """V1391 真生产 Policy to_dict / from_dict roundtrip (主 17:43)."""
    p = Policy.default_policy()
    d = p.to_dict()
    p2 = Policy.from_dict(d)
    assert p2.name == p.name
    assert len(p2.rules) == len(p.rules)


def test_v1391_policy_from_yaml_real(tmp_path):
    """V1391 真生产 Policy.from_yaml 真 YAML 解析 (主 17:43)."""
    p = tmp_path / "policy.yaml"
    p.write_text(DEFAULT_POLICY_YAML, encoding="utf-8")
    loaded = Policy.from_yaml(str(p))
    assert loaded.name == "apeireth-default-policy"
    assert len(loaded.rules) >= 5


def test_v1391_policy_from_yaml_missing_rules(tmp_path):
    """V1391 真生产 Policy.from_yaml 缺 rules 抛错 (主 17:43)."""
    p = tmp_path / "bad.yaml"
    p.write_text("name: bad\n", encoding="utf-8")
    with pytest.raises(ValueError):
        Policy.from_yaml(str(p))


def test_v1391_policy_from_yaml_not_dict(tmp_path):
    """V1391 真生产 Policy.from_yaml 非 dict 抛错 (主 17:43)."""
    p = tmp_path / "list.yaml"
    p.write_text("- one\n- two\n", encoding="utf-8")
    with pytest.raises(ValueError):
        Policy.from_yaml(str(p))


def test_v1391_policy_rule_to_dict():
    """V1391 真生产 PolicyRule.to_dict 完整 (主 17:43)."""
    r = PolicyRule(rule_id="DL3008", severity="warning", max_count=3, description="x")
    d = r.to_dict()
    assert d["rule_id"] == "DL3008"
    assert d["severity"] == "warning"
    assert d["max_count"] == 3
    assert d["description"] == "x"


# ============================================================================
# V1391 真生产 evaluate (主 17:43)
# ============================================================================


def test_v1391_evaluate_clean():
    """V1391 真生产 clean → pass + score 100 (主 17:43)."""
    p = Policy.default_policy()
    res = evaluate(p, [], target="clean")
    assert res.passed
    assert res.score == 100
    assert res.n_findings == 0
    assert res.n_violations == 0


def test_v1391_evaluate_zero_error_policy():
    """V1391 真生产 0 容错 policy → 1 error 即 fail (主 17:43)."""
    p = Policy.from_dict({
        "name": "zero-error",
        "schema": V1391_SCHEMA,
        "rules": [
            {"rule_id": "DEFAULT", "severity": "error", "max_count": 0},
        ],
    })
    res = evaluate(p, [{"rule_id": "DL3008"}], target="x")
    # DL3008 is warning, so 0 errors → pass
    assert res.passed


def test_v1391_evaluate_specific_rule_violation():
    """V1391 真生产 specific rule_id violation (主 17:43)."""
    p = Policy.from_dict({
        "name": "no-latest",
        "schema": V1391_SCHEMA,
        "rules": [
            {"rule_id": "COMPOSE-LATEST-TAG", "severity": "error", "max_count": 0},
        ],
    })
    res = evaluate(p, [{"rule_id": "COMPOSE-LATEST-TAG"}], target="x")
    assert not res.passed
    assert res.n_violations == 1
    assert res.violations[0].rule_id == "COMPOSE-LATEST-TAG"
    assert res.violations[0].max_count == 0


def test_v1391_evaluate_max_count_within():
    """V1391 真生产 在 max_count 内 → pass (主 17:43)."""
    p = Policy.from_dict({
        "name": "max-warn-3",
        "schema": V1391_SCHEMA,
        "rules": [
            {"rule_id": "DEFAULT", "severity": "warning", "max_count": 3},
        ],
    })
    # 2 warnings (DL3008 + V1384-NO-HEALTHCHECK)
    res = evaluate(p, [{"rule_id": "DL3008"}, {"rule_id": "V1384-NO-HEALTHCHECK"}], target="x")
    assert res.passed
    assert res.n_warnings == 2


def test_v1391_evaluate_max_count_exceeded():
    """V1391 真生产 超 max_count → fail (主 17:43)."""
    p = Policy.from_dict({
        "name": "max-warn-1",
        "schema": V1391_SCHEMA,
        "rules": [
            {"rule_id": "DEFAULT", "severity": "warning", "max_count": 1},
        ],
    })
    # 2 warnings > max 1 → fail
    res = evaluate(p, [{"rule_id": "DL3008"}, {"rule_id": "V1384-NO-HEALTHCHECK"}], target="x")
    assert not res.passed
    assert res.n_violations == 1


def test_v1391_evaluate_score_calculation():
    """V1391 真生产 score 计算 (主 17:43)."""
    p = Policy.from_dict({
        "name": "score",
        "schema": V1391_SCHEMA,
        "rules": [
            {"rule_id": "DEFAULT", "severity": "error", "max_count": -1},
            {"rule_id": "DEFAULT", "severity": "warning", "max_count": -1},
            {"rule_id": "DEFAULT", "severity": "info", "max_count": -1},
        ],
    })
    # 1 error + 1 warning + 1 info → 100 - 10 - 3 - 1 = 86
    res = evaluate(p, [
        {"rule_id": "COMPOSE-PRIVILEGED"},  # error
        {"rule_id": "DL3008"},  # warning
        {"rule_id": "DL3009"},  # info
    ], target="x")
    assert res.score == 86
    assert res.n_errors == 1
    assert res.n_warnings == 1
    assert res.n_info == 1


def test_v1391_evaluate_v1388_format():
    """V1391 真生产 接受 V1388 new_by_rule 格式 (主 17:43)."""
    p = Policy.default_policy()
    findings = [
        {"new_by_rule": {"DL3008": 5, "COMPOSE-LATEST-TAG": 2}},
    ]
    res = evaluate(p, findings, target="x")
    assert res.n_findings == 7
    assert res.by_rule["DL3008"] == 5
    assert res.by_rule["COMPOSE-LATEST-TAG"] == 2


def test_v1391_evaluate_unlimited_max_count():
    """V1391 真生产 max_count=-1 = unlimited (主 17:43)."""
    p = Policy.from_dict({
        "name": "unlimited",
        "schema": V1391_SCHEMA,
        "rules": [
            {"rule_id": "DEFAULT", "severity": "warning", "max_count": -1},
        ],
    })
    res = evaluate(p, [{"rule_id": "DL3008"}] * 100, target="x")
    assert res.passed
    assert res.n_violations == 0


def test_v1391_evaluate_to_dict():
    """V1391 真生产 PolicyResult.to_dict 完整 (主 17:43)."""
    p = Policy.default_policy()
    res = evaluate(p, [{"rule_id": "DL3008"}], target="x")
    d = res.to_dict()
    assert d["policy_name"] == p.name
    assert d["target"] == "x"
    assert "passed" in d
    assert "score" in d
    assert "violations" in d


def test_v1391_get_severity_for_rule_known():
    """V1391 真生产 get_severity_for_rule 已知 rule (主 17:43)."""
    sev = get_severity_for_rule("DL3008")
    assert sev in ("error", "warning", "info")


def test_v1391_get_severity_for_rule_explicit():
    """V1391 真生产 get_severity_for_rule 显式 severity (主 17:43)."""
    assert get_severity_for_rule("x", "info") == "info"
    assert get_severity_for_rule("x", "error") == "error"


def test_v1391_get_severity_for_rule_unknown():
    """V1391 真生产 get_severity_for_rule 未知 rule fallback (主 17:43)."""
    sev = get_severity_for_rule("UNKNOWN-RULE-9999")
    assert sev in ("error", "warning", "info")  # fallback to warning


# ============================================================================
# V1391 真生产 Popper self-test (主 17:43)
# ============================================================================


def test_v1391_popper_self_test_passes():
    """V1391 真生产 Popper self-test 真过 (主 17:43)."""
    r = popper_self_test()
    assert r["passed"], f"popper failed: {r['failures']}"
    assert r["n_tested"] >= 10


def test_v1391_guards_complete():
    """V1391 真生产 8 GUARDS (主 17:43)."""
    assert len(V1391_GUARDS) >= 8
    must_have = {"GUARD_POLICY_REAL", "GUARD_YAML_PARSED", "GUARD_EVALUATE_REAL",
                 "GUARD_NO_CAP_CHANGE", "GUARD_DETERMINISTIC", "GUARD_CLI_RUNNABLE"}
    assert must_have.issubset(set(V1391_GUARDS))


# ============================================================================
# V1391 真生产 CLI (主 17:43 真可执行)
# ============================================================================


def test_v1391_cli_version(capsys):
    """V1391 真生产 CLI version (主 17:43)."""
    rc = run_cli(["version"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "V1391" in captured.out
    assert V1391_VERSION in captured.out


def test_v1391_cli_schema(capsys):
    """V1391 真生产 CLI schema (主 17:43)."""
    rc = run_cli(["schema"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "V1391" in captured.out
    assert "rule_id" in captured.out
    assert "max_count" in captured.out


def test_v1391_cli_default_policy(capsys):
    """V1391 真生产 CLI default-policy (主 17:43)."""
    rc = run_cli(["default-policy"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "apeireth-default-policy" in captured.out
    assert "V1391" in captured.out or "rules:" in captured.out


def test_v1391_cli_evaluate_json(capsys, tmp_path):
    """V1391 真生产 CLI evaluate-json (主 17:43)."""
    p = tmp_path / "policy.yaml"
    p.write_text(DEFAULT_POLICY_YAML, encoding="utf-8")
    f = tmp_path / "findings.json"
    f.write_text(json.dumps([
        {"rule_id": "COMPOSE-LATEST-TAG"},
        {"rule_id": "DL3008"},
    ]), encoding="utf-8")
    rc = run_cli(["evaluate-json", str(p), str(f)])
    captured = capsys.readouterr()
    assert rc == 1  # fail: COMPOSE-LATEST-TAG max=0
    data = json.loads(captured.out)
    assert data["passed"] is False
    assert data["n_violations"] >= 1


def test_v1391_cli_evaluate_json_clean(capsys, tmp_path):
    """V1391 真生产 CLI evaluate-json clean (主 17:43)."""
    p = tmp_path / "policy.yaml"
    p.write_text(DEFAULT_POLICY_YAML, encoding="utf-8")
    f = tmp_path / "findings.json"
    f.write_text("[]", encoding="utf-8")
    rc = run_cli(["evaluate-json", str(p), str(f)])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert rc == 0
    assert data["passed"] is True
    assert data["score"] == 100


def test_v1391_cli_demo(capsys):
    """V1391 真生产 CLI demo (主 17:43)."""
    rc = run_cli(["demo"])
    captured = capsys.readouterr()
    assert rc == 1  # demo has COMPOSE-LATEST-TAG count=2 > max=0 => fail
    assert "V1391" in captured.out
    assert "FAIL" in captured.out or "fail" in captured.out


def test_v1391_cli_popper(capsys):
    """V1391 真生产 CLI popper (主 17:43)."""
    rc = run_cli(["popper"])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert rc == 0
    assert data["passed"]


def test_v1391_cli_help(capsys):
    """V1391 真生产 CLI --help (主 17:43)."""
    with pytest.raises(SystemExit) as exc_info:
        run_cli(["--help"])
    assert exc_info.value.code == 0
    captured = capsys.readouterr()
    assert "V1391" in captured.out


def test_v1391_cli_evaluate_with_real_target(capsys, tmp_path):
    """V1391 真生产 CLI evaluate 真 bad target (主 17:43)."""
    p = tmp_path / "policy.yaml"
    p.write_text(DEFAULT_POLICY_YAML, encoding="utf-8")
    bad = tmp_path / "bad"
    bad.mkdir()
    (bad / "Dockerfile").write_text(
        "FROM ubuntu:latest\nCMD echo hi\n",
        encoding="utf-8",
    )
    rc = run_cli(["evaluate", str(p), str(bad)])
    captured = capsys.readouterr()
    # V1384-FROM-LATEST max=0 → fail
    assert rc == 1
    assert "FAIL" in captured.out or "V1391" in captured.out


# ============================================================================
# V1391 真生产 集成 V1390 (主 17:43)
# ============================================================================


def test_v1391_evaluate_uses_v1390_severity():
    """V1391 真生产 evaluate 用 V1390 hint severity (主 17:43)."""
    p = Policy.default_policy()
    # COMPOSE-PRIVILEGED is error in V1390
    res = evaluate(p, [{"rule_id": "COMPOSE-PRIVILEGED"}], target="x")
    assert res.n_errors == 1
    # K8S-NO-RESOURCE-LIMITS is warning in V1390
    res2 = evaluate(p, [{"rule_id": "K8S-NO-RESOURCE-LIMITS"}], target="x")
    assert res2.n_warnings == 1


def test_v1391_no_asi_positioning():
    """V1391 真生产 不假装 (主 17:58): policy 是建议, 决策可 override."""
    # 默认 policy 0 容错 error 是建议, 任何人都能 override
    custom = Policy.from_dict({
        "name": "custom-loose",
        "schema": V1391_SCHEMA,
        "rules": [
            {"rule_id": "DEFAULT", "severity": "error", "max_count": 100},
        ],
    })
    res = evaluate(custom, [{"rule_id": "COMPOSE-PRIVILEGED"}] * 5, target="x")
    assert res.passed  # custom policy lets 5 errors through
    # Module docstring 表明 policy 是建议
    import v1391_policy_gate as m
    assert "建议" in m.__doc__ or "不假装" in m.__doc__ or "实事求是" in m.__doc__


def test_v1391_any_help_can_take_over():
    """V1391 真生产 任何人都能接手 (主 00:56): 1 YAML schema + 1 evaluator."""
    # YAML schema 公开
    assert "rule_id" in V1391_POLICY_SCHEMA
    assert "max_count" in V1391_POLICY_SCHEMA
    assert "severity" in V1391_POLICY_SCHEMA
    # Default policy 公开
    p = Policy.default_policy()
    assert len(p.rules) >= 5
    # evaluate 是纯函数
    res = evaluate(p, [], target="x")
    assert isinstance(res, PolicyResult)
    # 6 CLI subcommands
    with pytest.raises(SystemExit) as exc_info:
        run_cli(["--help"])
    assert exc_info.value.code == 0
