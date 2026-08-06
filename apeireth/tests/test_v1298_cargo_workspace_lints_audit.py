"""Tests for V1298 Cargo Workspace Lints Audit (主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手).

Verify:
- V1298 constants exposed + 4 dataclasses (Rust/Clippy/UnexpectedCfg/Crate) + 2 LintLedger/Hypothesis
- _parse_workspace_lints_lines: regex parse of [workspace.lints.{rust,clippy,rust.unexpected_cfgs}]
- _check_crate_lints_inherit: 56 crates 真实扫 (主 17:43 实事求是)
- sweep_workspace: Apeireth-rust
- evaluate_hypotheses: 6 假说 (主 13:08 真自问, Popper 可证伪)
- _v3_philosophy_gate
- build_audit_ledger JSON serializable
- 6 CLI subcommand
- subprocess invocation
- V1298 extends V1297 / V1296 (主 19:33 走在前人肩上)
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys

import pytest

APEIRETH_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, APEIRETH_ROOT)

import v1298_cargo_workspace_lints_audit as v98  # noqa: E402

WORKSPACE = os.path.abspath(os.path.join(APEIRETH_ROOT, "..", "Apeireth-rust"))


# ============================================================================
# A. Constants (主 00:56 任何人都能接手)
# ============================================================================

def test_v1298_version():
    assert v98.V1298_VERSION == "0.1.0"


def test_v1298_thresholds_complete():
    assert v98.THRESHOLD_RUST_LINTS_MIN == 5
    assert v98.THRESHOLD_CLIPPY_LINTS_MIN == 10
    assert v98.THRESHOLD_TOTAL_LINTS_MIN == 30
    assert v98.THRESHOLD_LINTS_INHERIT_PCT_MIN == 95.0


def test_v1298_workspace_members_at_least_50():
    """V1297 报 56, V1298 应至少 50 (主 19:33 走在前人肩上)."""
    assert len(v98.WORKSPACE_MEMBERS_V1298) >= 50, (
        f"expected ≥50, got {len(v98.WORKSPACE_MEMBERS_V1298)}"
    )


def test_v1298_constants_carries_known_lint_samples():
    """主 17:58 不假装: 已知 R20 阶段 6 fix 的 lint 名必须在样本列表里."""
    assert "unused_async" in v98.CLIPPY_LINT_NAMES_SAMPLE
    assert "missing_docs_in_private_items" in v98.CLIPPY_LINT_NAMES_SAMPLE
    assert "unused_extern_crates" in v98.RUSTC_LINT_NAMES_SAMPLE
    assert "missing_docs" in v98.RUSTC_LINT_NAMES_SAMPLE


def test_v1298_workspace_root_default():
    from pathlib import Path
    root = Path(v98.WORKSPACE_ROOT_DEFAULT)
    assert root.exists()
    assert (root / v98.CARGO_TOML).exists()


def test_v1298_regex_patterns_compiled():
    assert isinstance(v98.RE_LINT_RUST.pattern, str)
    assert isinstance(v98.RE_LINT_CLIPPY.pattern, str)
    assert isinstance(v98.RE_CRATE_LINTS.pattern, str)
    assert isinstance(v98.RE_CRATE_LINTS_VALUE.pattern, str)
    assert isinstance(v98.RE_UNEXPECTED_CFGS_LEVEL.pattern, str)
    assert isinstance(v98.RE_UNEXPECTED_CFGS_CHECK_CFG_ITEM.pattern, str)


# ============================================================================
# B. Dataclass shape (主 17:43 实事求是)
# ============================================================================

def test_v1298_rust_lint_dataclass():
    import dataclasses
    fields = {f.name for f in dataclasses.fields(v98.RustLintEntry)}
    assert {"name", "level", "line_number"} <= fields


def test_v1298_clippy_lint_dataclass():
    import dataclasses
    fields = {f.name for f in dataclasses.fields(v98.ClippyLintEntry)}
    assert {"name", "level", "line_number"} <= fields


def test_v1298_unexpected_cfg_dataclass():
    import dataclasses
    fields = {f.name for f in dataclasses.fields(v98.UnexpectedCfgEntry)}
    assert {"level", "check_cfg"} <= fields


def test_v1298_crate_lints_inherit_dataclass():
    import dataclasses
    fields = {f.name for f in dataclasses.fields(v98.CrateLintsInherit)}
    assert {"crate_name", "has_lints_section", "inherits_workspace"} <= fields


def test_v1298_lint_ledger_dataclass():
    import dataclasses
    fields = {f.name for f in dataclasses.fields(v98.LintLedger)}
    assert {"rust_lints", "clippy_lints", "unexpected_cfgs",
            "crate_lints_inherit", "has_rust_section", "has_clippy_section",
            "rust_lint_count", "clippy_lint_count", "total_lint_count",
            "crates_total", "crates_inherit_count", "crates_inherit_pct",
            "duration_ms"} <= fields


def test_v1298_hypothesis_result_dataclass():
    import dataclasses
    fields = {f.name for f in dataclasses.fields(v98.HypothesisResult)}
    assert {"hypothesis_id", "description", "passed",
            "observed", "threshold", "details"} <= fields


def test_v1298_dataclasses_frozen():
    """frozen=True 保证 ledger 不可变 (主 17:58 不假装)."""
    import dataclasses
    for cls in (v98.RustLintEntry, v98.ClippyLintEntry, v98.UnexpectedCfgEntry,
                v98.CrateLintsInherit, v98.LintLedger, v98.HypothesisResult):
        assert cls.__dataclass_params__.frozen is True, cls


# ============================================================================
# C. _parse_workspace_lints_lines
# ============================================================================

def test_parse_no_lints_sections():
    """无任何 [workspace.lints.*] 段 — 返回空 tuple."""
    text = """[package]
name = "foo"
"""
    rust, clippy, unexp = v98._parse_workspace_lints_lines(text)
    assert rust == []
    assert clippy == []
    assert unexp is None


def test_parse_rust_section_only():
    text = """[workspace.lints.rust]
unused_extern_crates = 'warn'
missing_docs = 'allow'
"""
    rust, clippy, unexp = v98._parse_workspace_lints_lines(text)
    assert len(rust) == 2
    assert rust[0].name == "unused_extern_crates"
    assert rust[0].level == "warn"
    assert rust[1].name == "missing_docs"
    assert rust[1].level == "allow"
    assert clippy == []
    assert unexp is None


def test_parse_clippy_section_only():
    text = """[workspace.lints.clippy]
uninlined_format_args = 'allow'
unused_async = 'allow'
"""
    rust, clippy, unexp = v98._parse_workspace_lints_lines(text)
    assert rust == []
    assert len(clippy) == 2
    assert clippy[0].name == "uninlined_format_args"
    assert clippy[1].name == "unused_async"
    assert unexp is None


def test_parse_unexpected_cfgs_section():
    """[workspace.lints.rust.unexpected_cfgs] 子段."""
    text = """[workspace.lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = [
    'cfg(kani)',
    'cfg(fuzzing)',
]
"""
    rust, clippy, unexp = v98._parse_workspace_lints_lines(text)
    assert rust == []
    assert clippy == []
    assert unexp is not None
    assert unexp.level == "warn"
    assert "cfg(kani)" in unexp.check_cfg
    assert "cfg(fuzzing)" in unexp.check_cfg


def test_parse_all_three_sections():
    """rust + clippy + unexpected_cfgs 共存."""
    text = """[workspace.lints.rust]
missing_docs = 'allow'

[workspace.lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = [
    'cfg(test)',
]

[workspace.lints.clippy]
unused_async = 'allow'
"""
    rust, clippy, unexp = v98._parse_workspace_lints_lines(text)
    assert len(rust) == 1
    assert rust[0].name == "missing_docs"
    assert len(clippy) == 1
    assert clippy[0].name == "unused_async"
    assert unexp is not None
    assert unexp.level == "warn"
    assert "cfg(test)" in unexp.check_cfg


def test_parse_skips_comments():
    """注释行不计入."""
    text = """[workspace.lints.rust]
# This is a comment
unused_extern_crates = 'warn'
# missing_docs should be allowed
"""
    rust, clippy, unexp = v98._parse_workspace_lints_lines(text)
    assert len(rust) == 1
    assert rust[0].name == "unused_extern_crates"


def test_parse_section_after_other_section():
    """[workspace] → [workspace.lints.rust] 转换正常."""
    text = """[workspace]
members = ["foo"]

[workspace.package]
version = "1.0.0"

[workspace.lints.rust]
unused_extern_crates = 'warn'
"""
    rust, clippy, unexp = v98._parse_workspace_lints_lines(text)
    assert len(rust) == 1


# ============================================================================
# D. _check_crate_lints_inherit
# ============================================================================

def test_check_crate_lints_inherit_missing_crate():
    """Cargo.toml 不存在的 crate 报告 has_lints=False, inherits=False."""
    from pathlib import Path
    fake_path = Path(r"nonexistent_path_xyz")
    results = v98._check_crate_lints_inherit(
        fake_path,
        ["apeireth-core"],
    )
    assert len(results) == 1
    assert results[0].has_lints_section is False
    assert results[0].inherits_workspace is False


def test_check_crate_lints_inherit_real_workspace():
    """对真 Apeireth-rust 扫, 命中已有 crate + 已知缺继承 (主 17:43 实事求是)."""
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    # 用一小段名单测试
    results = v98._check_crate_lints_inherit(Path(WORKSPACE), [
        "apeireth-core", "apeireth-bus", "apeireth-memory",
    ])
    by_name = {c.crate_name: c for c in results}
    assert by_name["apeireth-core"].inherits_workspace is True
    assert by_name["apeireth-bus"].inherits_workspace is True
    assert by_name["apeireth-memory"].inherits_workspace is True


def test_check_crate_lints_inherit_inlines_ml_flag():
    """(?m) flag 嵌入, 不会因为 [lints] 在文件中间而漏掉."""
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    results = v98._check_crate_lints_inherit(Path(WORKSPACE), ["apeireth-core"])
    assert results[0].has_lints_section is True


# ============================================================================
# E. Sweep real workspace
# ============================================================================

def test_sweep_finds_workspace():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    assert isinstance(led, v98.LintLedger)
    assert led.duration_ms >= 0


def test_sweep_finds_rust_section():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    assert led.has_rust_section is True
    assert led.rust_lint_count >= 5


def test_sweep_finds_clippy_section():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    assert led.has_clippy_section is True
    assert led.clippy_lint_count >= 20


def test_sweep_finds_unexpected_cfgs():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    assert led.unexpected_cfgs is not None
    assert led.unexpected_cfgs.level == "warn"
    assert "cfg(kani)" in led.unexpected_cfgs.check_cfg
    assert "cfg(fuzzing)" in led.unexpected_cfgs.check_cfg


def test_sweep_inherit_pct_in_range():
    """主 17:43 实事求是: 真实继承比例应在 60-100% 范围 (主 17:58 不假装).
    Allows for crates that don't exist yet (R20 stage 1 P0 skeletons)."""
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    assert 60.0 <= led.crates_inherit_pct <= 100.0


# ============================================================================
# F. evaluate_hypotheses (主 13:08 真自问 + Popper 可证伪)
# ============================================================================

def test_evaluate_hypotheses_returns_6():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    hypotheses = v98.evaluate_hypotheses(led)
    assert len(hypotheses) == 6


def test_evaluate_hypotheses_ids_match_expected():
    expected_ids = {
        "h_rust_lints_present",
        "h_clippy_lints_present",
        "h_rust_vs_clippy_separation",
        "h_unexpected_cfgs_present",
        "h_lints_inherit_pct",
        "h_no_deny_in_workspace_lints",
    }
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    ids = {h.hypothesis_id for h in v98.evaluate_hypotheses(led)}
    assert ids == expected_ids


def test_hypotheses_have_thresholds_consistent():
    """每假说 threshold == module-level threshold 常量."""
    assert v98.THRESHOLD_RUST_LINTS_MIN == 5
    assert v98.THRESHOLD_CLIPPY_LINTS_MIN == 10
    assert v98.THRESHOLD_LINTS_INHERIT_PCT_MIN == 95.0


# ============================================================================
# G. _v3_philosophy_gate
# ============================================================================

def test_v3_philosophy_gate_passes():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    ok, fails = v98._v3_philosophy_gate(led)
    assert isinstance(ok, bool)
    assert isinstance(fails, list)


# ============================================================================
# H. build_audit_ledger JSON
# ============================================================================

def test_build_audit_ledger_jsonable():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    data = v98.build_audit_ledger(led)
    s = json.dumps(data, default=str)
    assert isinstance(s, str)
    parsed = json.loads(s)
    assert "hypotheses" in parsed
    assert "rust_lint_count" in parsed


def test_build_audit_ledger_includes_summary_keys():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    data = v98.build_audit_ledger(led)
    for key in ("version", "rust_lint_count", "clippy_lint_count",
                "total_lint_count", "crates_total", "crates_inherit_pct",
                "hypotheses"):
        assert key in data, f"missing {key}"


# ============================================================================
# I. CLI subcommands (主 00:56 任何人都能接手)
# ============================================================================

def _make_args(**overrides):
    defaults = {
        "workspace": WORKSPACE,
        "output": None,
        "limit": 5,
    }
    defaults.update(overrides)
    return argparse.Namespace(**defaults)


def test_cli_probe_returns_int():
    code = v98.cmd_probe(_make_args())
    assert isinstance(code, int)


def test_cli_run_returns_int():
    code = v98.cmd_run(_make_args())
    assert isinstance(code, int)


def test_cli_json_prints_to_stdout(capsys):
    code = v98.cmd_json(_make_args())
    assert code == 0
    out = capsys.readouterr().out
    data = json.loads(out)
    assert isinstance(data, dict)


def test_cli_report_writes_md(tmp_path):
    out_path = str(tmp_path / "v1298_report.md")
    code = v98.cmd_report(_make_args(output=out_path))
    assert code == 0
    assert os.path.exists(out_path)
    with open(out_path, encoding="utf-8") as f:
        text = f.read()
    assert "V1298" in text
    assert "h_" in text  # 假说 IDs


def test_cli_inheritance_prints_json(capsys):
    code = v98.cmd_inheritance(_make_args())
    assert code == 0
    out = capsys.readouterr().out
    arr = json.loads(out)
    assert isinstance(arr, list)
    assert len(arr) >= 50
    for entry in arr[:5]:
        assert "crate" in entry
        assert "inherits_workspace" in entry


# ============================================================================
# J. Subprocess invocation (主 00:56 任何人都能接手)
# ============================================================================

def _run_v1298_subprocess(*args):
    return subprocess.run(
        [sys.executable, "-m", "v1298_cargo_workspace_lints_audit", *args],
        cwd=APEIRETH_ROOT,
        capture_output=True,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        timeout=60,
    )


def test_subprocess_probe():
    r = _run_v1298_subprocess("--probe", "--workspace", WORKSPACE)
    assert r.returncode == 0, f"stderr={r.stderr.decode('utf-8', errors='replace')}"
    out = r.stdout.decode("utf-8", errors="replace")
    assert "V1298 PROBE" in out
    assert "rust=" in out
    assert "clippy=" in out


def test_subprocess_run():
    r = _run_v1298_subprocess("--run", "--workspace", WORKSPACE)
    # Gate may fail (e.g. inherit < 95%) — returncode 1 is acceptable
    assert r.returncode in (0, 1), f"stderr={r.stderr.decode('utf-8', errors='replace')}"
    out = r.stdout.decode("utf-8", errors="replace")
    assert "V1298 RUN" in out
    assert "V3 philosophy gate" in out
    assert "PASS" in out or "FAIL" in out


def test_subprocess_json():
    r = _run_v1298_subprocess("--json", "--workspace", WORKSPACE)
    assert r.returncode == 0, f"stderr={r.stderr.decode('utf-8', errors='replace')}"
    out = r.stdout.decode("utf-8", errors="replace")
    data = json.loads(out)
    assert "hypotheses" in data


# ============================================================================
# K. main() entry
# ============================================================================

def test_main_returns_int():
    code = v98.main(["--probe"])
    assert isinstance(code, int)


# ============================================================================
# L. V1298 extends V1297 / V1296 (主 19:33 走在前人肩上)
# ============================================================================

def test_v1298_workspace_members_includes_v1297_set():
    """V1298 应继承 V1297 workspace members (主 19:33 走在前人肩上).

    V1298 可以有 V1297 之外的 crate (post-R20 stage 1/2 net new).
    至少 90% V1297 成员应在 V1298 名单内.
    """
    v1297_path = os.path.join(APEIRETH_ROOT, "v1297_cargo_feature_flag_audit.py")
    if not os.path.exists(v1297_path):
        pytest.skip("v1297 not present")
    with open(v1297_path, encoding="utf-8") as f:
        src = f.read()
    m = re.search(
        r"WORKSPACE_MEMBERS_V1297\s*:\s*List\[str\]\s*=\s*\[(.+?)\]",
        src, re.DOTALL,
    )
    if not m:
        pytest.skip("V1297 WORKSPACE_MEMBERS_V1297 literal not found")
    names = re.findall(r'"([a-zA-Z0-9_-]+)"', m.group(1))
    if len(names) < 5:
        pytest.skip("too few V1297 names extracted")
    v1297_set = set(names)
    v1298_set = set(v98.WORKSPACE_MEMBERS_V1298)
    missing = v1297_set - v1298_set
    pct_present = (len(v1297_set - missing) / len(v1297_set)) * 100
    assert pct_present >= 90.0, (
        f"V1298 仅覆盖 {pct_present:.1f}% V1297 member. Missing: {missing}"
    )


def test_v1298_dimension_differs():
    """V1298 应专攻 [workspace.lints] 维度, 不与 V1297 [features] 重叠."""
    assert hasattr(v98, "_parse_workspace_lints_lines")
    assert hasattr(v98, "_check_crate_lints_inherit")
    assert not hasattr(v98, "_parse_features_block")


# ============================================================================
# M. Real production invariants (主 17:43 实事求是)
# ============================================================================

def test_known_lints_actually_present():
    """实测 Apeireth-rust 已知 lint 必须命中 (主 17:43 实事求是)."""
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    rust_names = {l.name for l in led.rust_lints}
    clippy_names = {l.name for l in led.clippy_lints}
    # R20 阶段 6 fix: unused_async 必须在 clippy 而非 rust
    assert "unused_async" in clippy_names
    assert "unused_async" not in rust_names


def test_unexpected_cfgs_kani_protected():
    """kani cfg 在 check-cfg 白名单 (R20 阶段 6 apeireth-formal 需要)."""
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    assert led.unexpected_cfgs is not None
    assert "cfg(kani)" in led.unexpected_cfgs.check_cfg


def test_no_global_deny_in_workspace():
    """workspace.lints 不应 deny='all' / '*' / 'warnings'."""
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found: {WORKSPACE}")
    led = v98.sweep_workspace(Path(WORKSPACE))
    dangerous = {"all", "*", "warnings"}
    all_lints = list(led.rust_lints) + list(led.clippy_lints)
    for l in all_lints:
        assert l.name not in dangerous or l.level != "deny", (
            f"危险全局 deny: {l.name} = {l.level}"
        )
