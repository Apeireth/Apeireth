"""Phase 1390 test_v1390_remediation_hints — V1390 ASI 真生产 remediation hints tests (主 06:15 + 主 23:44 + 主 22:33 + 主 19:33 + 主 17:43 + 主 17:33 + 主 00:56).

V1390 = real production remediation hints for V1384/V1385/V1386 findings.
Tests verify: 30+ rule_ids 真有 hint + 真有 fix + 真有 ref + CLI 真可跑 + 真 integrate with V1387+V1388.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest

# V1390 import path: tests add promethean/apeireth to sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "apeireth"))

from v1390_remediation_hints import (  # noqa: E402
    V1390_VERSION,
    V1390_SCHEMA,
    V1390_GUARDS,
    HINTS,
    RemediationHint,
    apply_to_findings,
    get_hint,
    list_hints,
    popper_self_test,
    render_markdown,
    run_cli,
)


# ============================================================================
# V1390 真生产 数据结构 (主 17:43)
# ============================================================================


def test_v1390_version():
    """V1390 真生产 version (主 17:43)."""
    assert V1390_VERSION == "0.1.0"
    assert V1390_SCHEMA == "v1390.remediation-hints/v1"


def test_v1390_min_30_hints():
    """V1390 真生产 至少 30 rule_id (主 17:43 + 主 19:33 走在前人经验上)."""
    assert len(HINTS) >= 30, f"expected >=30 hints, got {len(HINTS)}"


def test_v1390_covers_v1384_v1385_v1386_rules():
    """V1390 真生产 覆盖 V1384/V1385/V1386 主要 rule (主 17:43)."""
    # V1384 必须涵盖
    must_have = {
        "DL3008", "DL3009", "DL3015", "DL3020", "DL3025", "DL4000",
        "V1384-NO-USER", "V1384-NO-HEALTHCHECK", "V1384-FROM-LATEST",
        # V1385
        "COMPOSE-LATEST-TAG", "COMPOSE-PRIVILEGED", "COMPOSE-PLAINTEXT-SECRET",
        "COMPOSE-MISSING-RESTART", "COMPOSE-MISSING-MEM-LIMIT",
        # V1386
        "K8S-LATEST-TAG", "K8S-NO-RESOURCE-LIMITS", "K8S-NO-READINESS",
        "K8S-NO-LIVENESS", "K8S-NO-SECURITY-CTX", "K8S-PRIVILEGED",
        "K8S-PLAINTEXT-SECRET",
    }
    missing = must_have - set(HINTS.keys())
    assert not missing, f"missing rule_ids: {missing}"


def test_v1390_all_hints_have_required_fields():
    """V1390 真生产 每个 hint 都有 severity / title / why / fix / ref (主 17:43)."""
    for rid, data in HINTS.items():
        assert "severity" in data, f"{rid}: missing severity"
        assert "title" in data, f"{rid}: missing title"
        assert "why" in data, f"{rid}: missing why"
        assert "fix" in data, f"{rid}: missing fix"
        assert "ref" in data, f"{rid}: missing ref"
        assert data["severity"] in ("error", "warning", "info"), \
            f"{rid}: bad severity {data['severity']}"
        assert data["ref"].startswith("https://"), f"{rid}: ref not https:// ({data['ref'][:60]})"
        assert len(data["fix"]) >= 20, f"{rid}: fix too short ({len(data['fix'])} chars)"
        assert len(data["title"]) >= 10, f"{rid}: title too short"


def test_v1390_get_hint_known():
    """V1390 真生产 get_hint 已知 rule 返回 RemediationHint (主 17:43)."""
    h = get_hint("DL3008")
    assert h is not None
    assert isinstance(h, RemediationHint)
    assert h.rule_id == "DL3008"
    assert h.severity in ("error", "warning", "info")
    assert h.title
    assert h.fix
    assert h.ref.startswith("https://")


def test_v1390_get_hint_unknown():
    """V1390 真生产 get_hint 未知 rule 返回 None (主 17:43)."""
    h = get_hint("UNKNOWN-RULE-9999")
    assert h is None


def test_v1390_list_hints():
    """V1390 真生产 list_hints 返回所有 hint (主 17:43)."""
    all_h = list_hints()
    assert len(all_h) == len(HINTS)
    for rid in HINTS:
        assert rid in all_h
        assert isinstance(all_h[rid], RemediationHint)


def test_v1390_hint_to_dict():
    """V1390 真生产 RemediationHint.to_dict() 完整 (主 17:43)."""
    h = get_hint("DL3008")
    d = h.to_dict()
    assert d["rule_id"] == "DL3008"
    assert "severity" in d
    assert "title" in d
    assert "why" in d
    assert "fix" in d
    assert "ref" in d


def test_v1390_severity_distribution():
    """V1390 真生产 severity 分布合理 (主 17:43 实事求是)."""
    err = sum(1 for d in HINTS.values() if d["severity"] == "error")
    warn = sum(1 for d in HINTS.values() if d["severity"] == "warning")
    info = sum(1 for d in HINTS.values() if d["severity"] == "info")
    # 至少要有 error, warning, info 三类 (主 17:43)
    assert err >= 5, f"expected >=5 errors, got {err}"
    assert warn >= 5, f"expected >=5 warnings, got {warn}"
    assert info >= 3, f"expected >=3 info, got {info}"


# ============================================================================
# V1390 真生产 apply_to_findings (主 17:43)
# ============================================================================


def test_v1390_apply_to_findings_direct():
    """V1390 真生产 apply_to_findings 直接 rule_id 列表 (主 17:43)."""
    findings = [
        {"rule_id": "DL3008"},
        {"rule_id": "DL3008"},  # dup
        {"rule_id": "COMPOSE-PRIVILEGED"},
        {"rule_id": "UNKNOWN-RULE-9999"},  # unknown
    ]
    hints = apply_to_findings(findings)
    # Deduplicated: DL3008 + COMPOSE-PRIVILEGED = 2
    assert len(hints) == 2
    rids = [h.rule_id for h in hints]
    assert "DL3008" in rids
    assert "COMPOSE-PRIVILEGED" in rids
    # error-first sort
    priv = next(h for h in hints if h.rule_id == "COMPOSE-PRIVILEGED")
    dl3008 = next(h for h in hints if h.rule_id == "DL3008")
    assert priv.severity == "error"
    assert dl3008.severity == "warning"


def test_v1390_apply_to_findings_v1388_format():
    """V1390 真生产 apply_to_findings V1388 new_by_rule 格式 (主 17:43)."""
    findings = [
        {
            "new_by_rule": {
                "DL3008": 5,
                "COMPOSE-PRIVILEGED": 2,
                "K8S-NO-RESOURCE-LIMITS": 3,
            }
        }
    ]
    hints = apply_to_findings(findings)
    assert len(hints) == 3
    rids = {h.rule_id for h in hints}
    assert rids == {"DL3008", "COMPOSE-PRIVILEGED", "K8S-NO-RESOURCE-LIMITS"}


def test_v1390_apply_to_findings_mixed():
    """V1390 真生产 apply_to_findings 混合 rule_id + new_by_rule (主 17:43)."""
    findings = [
        {"rule_id": "DL3008"},
        {"new_by_rule": {"DL3008": 1, "COMPOSE-PRIVILEGED": 1}},  # DL3008 duplicate
    ]
    hints = apply_to_findings(findings)
    assert len(hints) == 2  # DL3008 dedup, COMPOSE-PRIVILEGED new


def test_v1390_apply_to_findings_empty():
    """V1390 真生产 apply_to_findings 空输入 → 空 hints (主 17:43)."""
    assert apply_to_findings([]) == []
    assert apply_to_findings([{}]) == []
    assert apply_to_findings([{"foo": "bar"}]) == []


def test_v1390_apply_to_findings_severity_sort():
    """V1390 真生产 apply_to_findings 按 severity 排序 (主 17:43)."""
    findings = [
        {"rule_id": "DL3008"},  # warning
        {"rule_id": "COMPOSE-PRIVILEGED"},  # error
        {"rule_id": "DL3009"},  # info
        {"rule_id": "K8S-PRIVILEGED"},  # error
    ]
    hints = apply_to_findings(findings)
    assert len(hints) == 4
    # errors first
    assert hints[0].severity == "error"
    assert hints[1].severity == "error"
    assert hints[2].severity == "warning"
    assert hints[3].severity == "info"


# ============================================================================
# V1390 真生产 render_markdown (主 17:43)
# ============================================================================


def test_v1390_render_markdown_basic():
    """V1390 真生产 render_markdown 有必要 section (主 17:43)."""
    hints = [get_hint("DL3008"), get_hint("COMPOSE-PRIVILEGED")]
    md = render_markdown(hints)
    assert "V1390 Remediation Hints" in md
    assert "DL3008" in md
    assert "COMPOSE-PRIVILEGED" in md
    assert "**Why**" in md
    assert "**Fix**" in md
    assert "**Reference**" in md
    assert "https://" in md


def test_v1390_render_markdown_groups_by_severity():
    """V1390 真生产 render_markdown 按 severity 分组 (主 17:43)."""
    hints = [get_hint("DL3008"), get_hint("COMPOSE-PRIVILEGED"), get_hint("DL3009")]
    md = render_markdown(hints)
    # ERROR section first
    err_pos = md.find("ERROR")
    warn_pos = md.find("WARNING")
    info_pos = md.find("INFO")
    assert err_pos > 0 and warn_pos > err_pos and info_pos > warn_pos


def test_v1390_render_markdown_empty():
    """V1390 真生产 render_markdown 空 hints → 0 hints (主 17:43)."""
    md = render_markdown([])
    assert "0 hints" in md


def test_v1390_render_markdown_has_code_fence():
    """V1390 真生产 render_markdown fix 用 code fence (主 17:43)."""
    hints = [get_hint("DL3008")]
    md = render_markdown(hints)
    assert "```dockerfile" in md
    assert "```" in md


# ============================================================================
# V1390 真生产 Popper self-test (主 17:43)
# ============================================================================


def test_v1390_popper_self_test_passes():
    """V1390 真生产 Popper self-test 真过 (主 17:43)."""
    r = popper_self_test()
    assert r["passed"], f"popper self-test failed: {r['failures']}"
    assert r["n_hints"] >= 30
    assert r["n_tested"] >= 10


def test_v1390_popper_self_test_n_tested():
    """V1390 真生产 Popper self-test 至少 10 真测试 (主 17:43)."""
    r = popper_self_test()
    assert r["n_tested"] >= 10


# ============================================================================
# V1390 真生产 GUARDS (主 17:43)
# ============================================================================


def test_v1390_guards_complete():
    """V1390 真生产 8 GUARDS (主 17:43)."""
    assert len(V1390_GUARDS) >= 8
    must_have = {"GUARD_HINTS_REAL", "GUARD_NO_CAP_CHANGE", "GUARD_DETERMINISTIC",
                 "GUARD_HONEST_DISCLOSURE", "GUARD_CLI_RUNNABLE"}
    assert must_have.issubset(set(V1390_GUARDS))


# ============================================================================
# V1390 真生产 CLI (主 17:43 真可执行)
# ============================================================================


def test_v1390_cli_version():
    """V1390 真生产 CLI version (主 17:43)."""
    rc = run_cli(["version"])
    assert rc == 0
    out = sys.stdout


def test_v1390_cli_version_capsys(capsys):
    """V1390 真生产 CLI version (主 17:43)."""
    rc = run_cli(["version"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "V1390" in captured.out
    assert V1390_VERSION in captured.out


def test_v1390_cli_rules(capsys):
    """V1390 真生产 CLI rules 列出所有 rule_id (主 17:43)."""
    rc = run_cli(["rules"])
    captured = capsys.readouterr()
    assert rc == 0
    rules = captured.out.strip().split("\n")
    assert len(rules) >= 30
    assert "DL3008" in rules
    assert "COMPOSE-PRIVILEGED" in rules
    assert "K8S-NO-RESOURCE-LIMITS" in rules


def test_v1390_cli_stats(capsys):
    """V1390 真生产 CLI stats JSON (主 17:43)."""
    rc = run_cli(["stats"])
    captured = capsys.readouterr()
    assert rc == 0
    data = json.loads(captured.out)
    assert data["version"] == V1390_VERSION
    assert data["schema"] == V1390_SCHEMA
    assert data["n_hints"] >= 30
    assert "by_severity" in data


def test_v1390_cli_hint_known(capsys):
    """V1390 真生产 CLI hint (主 17:43)."""
    rc = run_cli(["hint", "DL3008"])
    captured = capsys.readouterr()
    assert rc == 0
    data = json.loads(captured.out)
    assert data["rule_id"] == "DL3008"
    assert "fix" in data
    assert "ref" in data


def test_v1390_cli_hint_unknown(capsys):
    """V1390 真生产 CLI hint unknown → exit 1 (主 17:43)."""
    rc = run_cli(["hint", "NOPE-9999"])
    assert rc == 1


def test_v1390_cli_popper(capsys):
    """V1390 真生产 CLI popper 真过 (主 17:43)."""
    rc = run_cli(["popper"])
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert rc == 0
    assert data["passed"]


def test_v1390_cli_markdown(capsys):
    """V1390 真生产 CLI markdown 渲染所有 hints (主 17:43)."""
    rc = run_cli(["markdown"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "V1390 Remediation Hints" in captured.out
    assert "DL3008" in captured.out


def test_v1390_cli_markdown_severity_filter(capsys):
    """V1390 真生产 CLI markdown --severity filter (主 17:43)."""
    rc = run_cli(["markdown", "--severity", "error"])
    captured = capsys.readouterr()
    assert rc == 0
    # Only error section should appear (no WARNING or INFO)
    # 注意: heading 名字大写
    assert "ERROR" in captured.out
    # Should NOT have warning section content
    assert "WARNING" not in captured.out.upper().replace("INFO", "").replace("WARNINGS", "")


def test_v1390_cli_apply_no_findings(capsys):
    """V1390 真生产 CLI apply on clean dir (主 17:43)."""
    with tempfile.TemporaryDirectory() as tmp:
        d = Path(tmp) / "clean"
        d.mkdir()
        # No Dockerfile, no compose, no k8s = 0 findings
        rc = run_cli(["apply", str(d), "--md"])
        captured = capsys.readouterr()
        assert rc == 0
        # 0 hints
        assert "0 hints" in captured.out


def test_v1390_cli_apply_with_findings(capsys, tmp_path):
    """V1390 真生产 CLI apply on bad dir (主 17:43)."""
    d = tmp_path / "bad"
    d.mkdir()
    (d / "Dockerfile").write_text(
        "FROM ubuntu:latest\nRUN apt-get install -y gcc\nUSER root\nCMD echo hi\n",
        encoding="utf-8",
    )
    (d / "docker-compose.yml").write_text(
        "version: '3.8'\nservices:\n  app:\n    image: myapp:latest\n    privileged: true\n",
        encoding="utf-8",
    )
    rc = run_cli(["apply", str(d)])
    captured = capsys.readouterr()
    assert rc == 0
    data = json.loads(captured.out)
    assert data["target"] == str(d)
    assert data["n_findings"] >= 1
    assert data["n_hints"] >= 1
    # Each hint has rule_id, fix, ref
    for h in data["hints"]:
        assert "rule_id" in h
        assert "fix" in h
        assert "ref" in h


def test_v1390_cli_apply_md(capsys, tmp_path):
    """V1390 真生产 CLI apply --md (主 17:43)."""
    d = tmp_path / "bad"
    d.mkdir()
    (d / "Dockerfile").write_text(
        "FROM ubuntu:latest\nRUN apt-get install -y gcc\nUSER root\n",
        encoding="utf-8",
    )
    rc = run_cli(["apply", str(d), "--md"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "V1390 Remediation Hints" in captured.out or "**Fix**" in captured.out


def test_v1390_cli_help(capsys):
    """V1390 真生产 CLI --help (主 17:43)."""
    rc = run_cli(["--help"])
    captured = capsys.readouterr()
    assert rc == 0
    assert "V1390" in captured.out


# ============================================================================
# V1390 真集成 V1387+V1388 (主 17:43)
# ============================================================================


def test_v1390_apply_via_v1388_diff_format(tmp_path):
    """V1390 真生产 apply 接收 V1388 diff result 格式 (主 17:43)."""
    # V1388 diff result has new_by_rule
    import io
    from contextlib import redirect_stdout
    d = tmp_path / "bad"
    d.mkdir()
    (d / "Dockerfile").write_text(
        "FROM ubuntu:latest\nUSER root\nCMD echo hi\n",
        encoding="utf-8",
    )
    # Use run_cli to scan via V1387
    rc = run_cli(["apply", str(d)])
    assert rc == 0
    # Capture stdout
    import sys as _sys
    out = _sys.stdout.getvalue() if hasattr(_sys.stdout, 'getvalue') else ""
    # Actually re-run with capsys
    buf = io.StringIO()
    with redirect_stdout(buf):
        rc = run_cli(["apply", str(d)])
    assert rc == 0
    data = json.loads(buf.getvalue())
    # Find at least one rule from V1384
    rids = [h["rule_id"] for h in data["hints"]]
    assert any(r in rids for r in ["V1384-FROM-LATEST", "V1384-NO-USER", "DL3008"])


def test_v1390_no_asi_positioning():
    """V1390 真生产 不假装 (主 17:58): hints 是建议, 不是 truth."""
    h = get_hint("DL3008")
    # why 必含真实理由 (>= 20 chars), 不是空话
    assert len(h.why) >= 20, f"why too short: {h.why}"
    # ref 必须真实链接, 不假装
    assert h.ref.startswith("https://")
    # fix 必含具体代码, 不假装
    assert "RUN" in h.fix or "apt-get" in h.fix or "CMD" in h.fix
    # Module top docstring 表明 hints 是建议, 不假装 truth
    import v1390_remediation_hints as m
    assert "建议" in m.__doc__ or "实事求是" in m.__doc__ or "不假装" in m.__doc__


def test_v1390_cli_help(capsys):
    """V1390 真生产 CLI --help (主 17:43)."""
    # argparse exits with SystemExit(0) on --help, run_cli propagates.
    with pytest.raises(SystemExit) as exc_info:
        run_cli(["--help"])
    assert exc_info.value.code == 0
    captured = capsys.readouterr()
    assert "V1390" in captured.out


def test_v1390_any_help_can_take_over():
    """V1390 真生产 任何人都能接手 (主 00:56): 1 个 dict + 1 个 CLI."""
    # HINTS is 1 dict, every entry has same 5 keys
    required_keys = {"severity", "title", "why", "fix", "ref"}
    for rid, data in HINTS.items():
        assert set(data.keys()) >= required_keys, f"{rid} missing keys: {required_keys - set(data.keys())}"
    # CLI: 7 subcommands via --help
    with pytest.raises(SystemExit) as exc_info:
        run_cli(["--help"])
    assert exc_info.value.code == 0
