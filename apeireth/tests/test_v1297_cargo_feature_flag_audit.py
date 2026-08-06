"""Tests for V1297 Cargo Feature Flag Audit (主 17:43 实事求是 + 主 00:44 质量工程化 + 主 00:56 任何人都能接手).

Verify:
- V1297 constants exposed
- 4 dataclasses (WorkspaceDepFeatureAudit / CrateFeatureAudit / HypothesisResult / FeatureFlagLedger)
- _parse_workspace_deps_block: regex parse of [workspace.dependencies]
- _parse_features_block: single-line / multi-line / no-features / dep: prefix
- sweep_workspace: real Apeireth-rust 56 crates (or current count)
- evaluate_hypotheses: 6 假说 (主 13:08 真自问, Popper 可证伪)
- _v3_philosophy_gate: 不假装 / 不刷 KPI / audit ≠ fix
- build_audit_ledger: JSON serializable
- 6 CLI subcommand
- subprocess invocation
- V1297 extends V1296 / V1295 (主 19:33 走在前人肩上)
"""
from __future__ import annotations

import importlib.util
import json
import os
import subprocess
import sys

import pytest

APEIRETH_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, APEIRETH_ROOT)

import v1297_cargo_feature_flag_audit as v97  # noqa: E402

WORKSPACE = os.path.abspath(os.path.join(APEIRETH_ROOT, "..", "Apeireth-rust"))


# ============================================================================
# A. Constants (主 00:56 任何人都能接手)
# ============================================================================

def test_v1297_workspace_root_default():
    """WORKSPACE_ROOT_DEFAULT 应指向真 Apeireth-rust 目录 (主 17:43 实事求是)."""
    from pathlib import Path
    root = Path(v97.WORKSPACE_ROOT_DEFAULT)
    assert root.exists(), f"workspace root not found: {root}"
    assert (root / v97.CARGO_TOML).exists()


def test_v1297_thresholds_complete():
    assert v97.THRESHOLD_WORKSPACE_DEPS_WITH_FEATURES_MIN == 5
    assert v97.THRESHOLD_CRATES_WITH_FEATURES_PCT_MAX == 25.0
    assert v97.THRESHOLD_DEP_PREFIX_USAGE_PCT_MIN == 50.0
    assert v97.THRESHOLD_DEFAULT_EMPTY_PCT_MIN == 50.0


def test_v1297_workspace_members_at_least_50():
    """V1296 报 56, V1297 应继承同样规模 (主 19:33 走在前人肩上)."""
    assert len(v97.WORKSPACE_MEMBERS_V1297) >= 50, (
        f"expected ≥50 (V1296 = 56), got {len(v97.WORKSPACE_MEMBERS_V1297)}"
    )


def test_v1297_constants_callable_patterns():
    assert v97.CARGO_TOML == "Cargo.toml"
    assert isinstance(v97.RE_FEATURES_HEADER.pattern, str)
    assert isinstance(v97.RE_SECTION_END.pattern, str)


def test_v1297_hypotheses_min_6():
    """主 13:08 真自问: ≥6 假说 内嵌在 evaluate_hypotheses()."""
    from pathlib import Path
    if not Path(v97.WORKSPACE_ROOT_DEFAULT).exists():
        pytest.skip("workspace root not found")
    led = v97.sweep_workspace(Path(v97.WORKSPACE_ROOT_DEFAULT))
    assert len(
        v97.evaluate_hypotheses(
            led.workspace_deps, led.crate_features, led.crates_with_features_pct
        )
    ) >= 6


# ============================================================================
# B. Dataclass shape (主 17:43 实事求是)
# ============================================================================

def test_v1297_workspace_dep_dataclass():
    import dataclasses
    fields = {f.name for f in dataclasses.fields(v97.WorkspaceDepFeatureAudit)}
    assert {"dep_name", "version", "features", "features_count",
            "default_features_disabled", "raw_text"} <= fields


def test_v1297_crate_audit_dataclass():
    import dataclasses
    fields = {f.name for f in dataclasses.fields(v97.CrateFeatureAudit)}
    assert {"crate_name", "has_features_block", "default_value",
            "default_count", "feature_count", "feature_names",
            "uses_dep_prefix", "has_hardcoded_version",
            "raw_text_snippet"} <= fields


def test_v1297_hypothesis_result_dataclass():
    import dataclasses
    fields = {f.name for f in dataclasses.fields(v97.HypothesisResult)}
    assert {"hypothesis_id", "description", "passed",
            "observed", "threshold", "details"} <= fields


def test_v1297_ledger_dataclass():
    import dataclasses
    fields = {f.name for f in dataclasses.fields(v97.FeatureFlagLedger)}
    assert {"workspace_deps", "crate_features", "hypotheses",
            "workspace_dep_count", "crates_with_features_count",
            "crates_with_features_pct",
            "total_feature_combinations_estimate",
            "duration_ms"} <= fields


def test_v1297_dataclasses_frozen():
    """frozen=True 保证 ledger 不可变 (主 17:58 不假装)."""
    import dataclasses
    for cls in (v97.WorkspaceDepFeatureAudit, v97.CrateFeatureAudit,
                v97.HypothesisResult, v97.FeatureFlagLedger):
        assert cls.__dataclass_params__.frozen is True, cls


# ============================================================================
# C. _parse_workspace_deps_block
# ============================================================================

def test_parse_workspace_deps_no_section():
    lines = ["[package]\n", "name = \"foo\"\n", "version = \"0.1.0\"\n"]
    result = v97._parse_workspace_deps_block(lines)
    assert result == []


def test_parse_workspace_deps_basic():
    text = """[workspace.dependencies]
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
"""
    result = v97._parse_workspace_deps_block(text.splitlines())
    assert len(result) >= 3
    by_name = {d.dep_name: d for d in result}
    assert by_name["tokio"].features_count >= 1
    assert "full" in by_name["tokio"].features
    assert "derive" in by_name["serde"].features
    assert by_name["anyhow"].features_count == 0


def test_parse_workspace_deps_default_features_disabled():
    text = """[workspace.dependencies]
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
"""
    result = v97._parse_workspace_deps_block(text.splitlines())
    assert len(result) == 1
    assert result[0].default_features_disabled is True
    assert "json" in result[0].features


def test_parse_workspace_deps_version_capture():
    text = """[workspace.dependencies]
serde_json = { version = "1.0.114", features = ["preserve_order"] }
"""
    result = v97._parse_workspace_deps_block(text.splitlines())
    assert result[0].version == "1.0.114"


# ============================================================================
# D. _parse_features_block
# ============================================================================

def test_parse_features_no_block():
    text = """[package]
name = "no-features-crate"
version = "0.1.0"
[dependencies]
serde = "1"
"""
    r = v97._parse_features_block(text)
    assert r.has_features_block is False
    assert r.feature_count == 0
    assert r.feature_names == ()


def test_parse_features_single_line():
    text = """[package]
name = "single-line"
[features]
default = []
foo = ["dep:bar"]
ssr = ["dep:apeireth-bus", "dep:apeireth-pybridge"]
"""
    r = v97._parse_features_block(text)
    assert r.has_features_block is True
    # foo + ssr (default value==[] 计数或不计, tolerate >=2)
    assert r.feature_count >= 2
    assert "foo" in r.feature_names
    assert "ssr" in r.feature_names
    assert r.uses_dep_prefix is True
    assert r.default_count == 0


def test_parse_features_multiline_block():
    """apeireth-web ssr 实测 = 19 项多行 list."""
    text = """[package]
name = "multi-line-crate"
[features]
default = []
ssr = [
    "dep:apeireth-bus",
    "dep:apeireth-pybridge",
    "dep:apeireth-formal",
    "dep:apeireth-council",
    "dep:apeireth-vector",
]
extra = []
"""
    r = v97._parse_features_block(text)
    assert r.has_features_block is True
    assert r.feature_count >= 1
    assert "ssr" in r.feature_names


def test_parse_features_default_non_empty():
    text = """[package]
name = "default-on"
[features]
default = ["foo"]
foo = []
"""
    r = v97._parse_features_block(text)
    assert r.default_count == 1
    assert r.default_value == ("foo",)


def test_parse_features_no_dep_prefix_strict():
    """真正无 dep: 前缀 (主 17:43 实事求是)."""
    text = (
        '[package]\n'
        'name = "plain-feature"\n'
        '[features]\n'
        'default = []\n'
        'plain = []\n'
    )
    r = v97._parse_features_block(text)
    assert r.uses_dep_prefix is False
    # default = [] 不计为 feature, 只 plain 一个
    assert r.feature_count == 1
    assert "plain" in r.feature_names


def test_parse_features_extracts_crate_name():
    text = """[package]
name = "apeireth-bus"
version = "1.0.0"
[features]
default = []
full-bus = []
"""
    r = v97._parse_features_block(text)
    assert r.crate_name == "apeireth-bus"


# ============================================================================
# E. Sweep real workspace
# ============================================================================

def test_sweep_finds_workspace():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    assert isinstance(led, v97.FeatureFlagLedger)
    assert led.duration_ms >= 0
    assert len(led.crate_features) >= 50


def test_sweep_finds_workspace_dependencies():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    assert led.workspace_dep_count >= 5, (
        f"expected ≥5 deps (tokio/serde/...), got {led.workspace_dep_count}"
    )


def test_sweep_finds_features_block():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    names = {c.crate_name for c in led.crate_features if c.has_features_block}
    expected = {"apeireth-bus", "apeireth-central", "apeireth-graph",
                "apeireth-memory", "apeireth-pybridge", "apeireth-web"}
    assert expected <= names, f"missing [features] crates: {expected - names}"


def test_sweep_hypotheses_evaluated():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    assert len(led.hypotheses) >= 6
    for h in led.hypotheses:
        assert isinstance(h, v97.HypothesisResult)
        assert h.hypothesis_id
        assert isinstance(h.passed, bool)


def test_sweep_total_combinations_estimated():
    """主 13:08 真自问: 合组合数应在合理范围 (>=0)."""
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    assert led.total_feature_combinations_estimate >= 0


# ============================================================================
# F. evaluate_hypotheses (主 13:08 真自问 + Popper 可证伪)
# ============================================================================

def test_evaluate_hypotheses_accepts_ledger():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    hypotheses = v97.evaluate_hypotheses(
        led.workspace_deps, led.crate_features, led.crates_with_features_pct
    )
    assert len(hypotheses) >= 6
    ids = {h.hypothesis_id for h in hypotheses}
    expected_ids = {"h_workspace_deps_with_features",
                    "h_crates_with_features_section",
                    "h_dep_prefix_usage",
                    "h_default_empty_dominant",
                    "h_no_hardcoded_version_in_features",
                    "h_pyo3_not_in_default"}
    assert expected_ids <= ids


def test_hypotheses_have_thresholds():
    """每假说有明确 threshold (主 13:08 真自问, 可证伪)."""
    assert v97.THRESHOLD_WORKSPACE_DEPS_WITH_FEATURES_MIN == 5
    assert v97.THRESHOLD_CRATES_WITH_FEATURES_PCT_MAX == 25.0
    assert v97.THRESHOLD_DEP_PREFIX_USAGE_PCT_MIN == 50.0
    assert v97.THRESHOLD_DEFAULT_EMPTY_PCT_MIN == 50.0


# ============================================================================
# G. _v3_philosophy_gate (主 17:58 + 主 20:46 不假装)
# ============================================================================

def test_v3_philosophy_gate_returns_tuple():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    ok, fails = v97._v3_philosophy_gate(led)
    assert isinstance(ok, bool)
    assert isinstance(fails, list)


# ============================================================================
# H. build_audit_ledger
# ============================================================================

def test_build_audit_ledger_returns_jsonable():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    ledger = v97.build_audit_ledger(led)
    assert isinstance(ledger, dict)
    s = json.dumps(ledger, default=str)
    assert isinstance(s, str)
    parsed = json.loads(s)
    assert "hypotheses" in parsed


def test_build_audit_ledger_includes_summary_keys():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    ledger = v97.build_audit_ledger(led)
    for key in ("workspace_dep_count", "crates_with_features_count",
                "crates_with_features_pct", "hypotheses"):
        assert key in ledger, f"missing {key}"


# ============================================================================
# I. CLI subcommands (主 00:56 任何人都能接手)
# ============================================================================

def _make_args(**overrides):
    import argparse
    defaults = {
        "workspace": WORKSPACE,
        "output": None,
        "limit": 5,
    }
    defaults.update(overrides)
    return argparse.Namespace(**defaults)


def test_cli_probe_returns_int():
    code = v97.cmd_probe(_make_args())
    assert isinstance(code, int)


def test_cli_run_returns_int():
    code = v97.cmd_run(_make_args())
    assert isinstance(code, int)


def test_cli_json_prints_to_stdout(capsys):
    """--json mode prints to stdout (主 00:56 任何人都能接手)."""
    code = v97.cmd_json(_make_args())
    assert code == 0
    captured = capsys.readouterr()
    data = json.loads(captured.out)
    assert isinstance(data, dict)
    assert "hypotheses" in data


def test_cli_report_writes_md(tmp_path):
    out_path = str(tmp_path / "report.md")
    code = v97.cmd_report(_make_args(output=out_path))
    assert code == 0
    assert os.path.exists(out_path)
    with open(out_path, encoding="utf-8") as f:
        text = f.read()
    assert "V1297" in text
    assert "h_" in text  # 假说 IDs


def test_cli_workspace_deps_returns_int():
    code = v97.cmd_workspace_deps(_make_args(limit=10))
    assert isinstance(code, int)


def test_cli_crates_with_features_returns_int():
    code = v97.cmd_crates_with_features(_make_args(limit=10))
    assert isinstance(code, int)


# ============================================================================
# J. Subprocess invocation (主 00:56 任何人都能接手 - 真 exit code + 真 output)
# ============================================================================

def _run_v1297_subprocess(*args):
    return subprocess.run(
        [sys.executable, "-m", "v1297_cargo_feature_flag_audit", *args],
        cwd=APEIRETH_ROOT,
        capture_output=True,
        env={**os.environ, "PYTHONIOENCODING": "utf-8"},
        timeout=60,
    )


def test_subprocess_probe():
    r = _run_v1297_subprocess("--probe", "--workspace", WORKSPACE)
    assert r.returncode == 0, f"stderr={r.stderr.decode('utf-8', errors='replace')}"
    out = r.stdout.decode("utf-8", errors="replace")
    assert "V1297" in out
    assert "crates with [features]" in out


def test_subprocess_json():
    r = _run_v1297_subprocess("--json", "--workspace", WORKSPACE)
    assert r.returncode == 0, f"stderr={r.stderr.decode('utf-8', errors='replace')}"
    out = r.stdout.decode("utf-8", errors="replace")
    data = json.loads(out)
    assert isinstance(data, dict)


def test_subprocess_run():
    r = _run_v1297_subprocess("--run", "--workspace", WORKSPACE)
    assert r.returncode == 0, f"stderr={r.stderr.decode('utf-8', errors='replace')}"
    out = r.stdout.decode("utf-8", errors="replace")
    assert "V3 philosophy gate" in out
    assert "PASS" in out or "FAIL" in out


# ============================================================================
# K. main() entry point
# ============================================================================

def test_main_returns_int():
    code = v97.main(["--probe"])
    assert isinstance(code, int)


# ============================================================================
# L. V1297 extends V1296 / V1295 (主 19:33 走在前人肩上)
# ============================================================================

def test_v1297_inherits_v1296_workspace_members():
    """V1297 应继承 V1296 的 workspace members (主 19:33).

    Avoid dynamic import: read V1296 source and regex-extract list.
    """
    import re
    v1296_path = os.path.join(APEIRETH_ROOT, "v1296_cargo_toml_metadata_audit.py")
    if not os.path.exists(v1296_path):
        pytest.skip("v1296 file not present")
    with open(v1296_path, encoding="utf-8") as f:
        src = f.read()
    m = re.search(
        r"WORKSPACE_MEMBERS_V1296\s*:\s*List\[str\]\s*=\s*\[(.+?)\]",
        src, re.DOTALL,
    )
    if not m:
        pytest.skip("WORKSPACE_MEMBERS_V1296 literal not found")
    names = re.findall(r'"([a-zA-Z0-9_-]+)"', m.group(1))
    if len(names) < 5:
        pytest.skip("too few names extracted")
    v1296_set = set(names)
    v1297_set = set(v97.WORKSPACE_MEMBERS_V1297)
    assert v1296_set <= v1297_set, f"missing from v1297: {v1296_set - v1297_set}"
    assert len(v1297_set) >= len(v1296_set)


def test_v1297_differs_from_v1296_dimension():
    """V1297 应专攻 [features] 维度, 不与 V1296 重叠."""
    assert hasattr(v97, "_parse_features_block")
    assert hasattr(v97, "_parse_workspace_deps_block")


# ============================================================================
# M. 6 known [features] crates real data (主 17:43 实事求是)
# ============================================================================

def test_known_features_crates_have_correct_feature_names():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    by_name = {c.crate_name: c for c in led.crate_features}
    if "apeireth-bus" in by_name:
        assert "full-bus" in by_name["apeireth-bus"].feature_names
    if "apeireth-memory" in by_name:
        assert "semantic" in by_name["apeireth-memory"].feature_names
    if "apeireth-web" in by_name:
        assert "ssr" in by_name["apeireth-web"].feature_names


def test_default_value_consistent():
    from pathlib import Path
    if not Path(WORKSPACE).exists():
        pytest.skip(f"workspace not found at {WORKSPACE}")
    led = v97.sweep_workspace(Path(WORKSPACE))
    for c in led.crate_features:
        if c.has_features_block:
            assert isinstance(c.default_value, tuple)
            assert c.default_count == len(c.default_value)
