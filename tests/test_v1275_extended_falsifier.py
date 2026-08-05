"""V1275 — ASI Extended Falsifier tests (主 17:43 实事求是 + 主 22:33 终极授权 + 主 13:31 大胆好奇心).

> Tests cover: import, --probe, run_all_hypotheses, falsify_hypothesis (8 evidence types),
> philosophy gate, expected PASS/FAIL distribution, JSON serialization, Markdown report.

> **V1275 expected behavior** (2026-08-05 real evidence, 主 17:43 实事求是):
> - h_substrate_count: PASS (43 > 30)
> - h_kitchen_modules: FAIL (v1263 test missing - REAL fact, not fake)
> - h_truth_gates_count: PASS (9 >= 9, after regex bug fix)
> - h_recent_substrate_lift: PASS (82 >= 3)
> - h_stream_modules: PASS (5/5 mods + 5/5 tests)
> - h_vcp_modules: PASS (V1272 exists)
> - h_truth_falsifier_self: PASS (V1274 --probe exit=0)
> - h_pipeline_22_samples: PASS (V1268 has '22' reference)
> Total: 7 PASS / 1 FAIL / 0 INCONCLUSIVE
"""
from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

# Add promethean/ to sys.path so `import apeireth.v1275_...` works from tests/
PROJECT_ROOT = Path(__file__).resolve().parent.parent
PROMETHEAN_DIR = PROJECT_ROOT
APEIRETH_DIR = PROMETHEAN_DIR / "apeireth"
TESTS_DIR = PROMETHEAN_DIR / "tests"

if str(PROMETHEAN_DIR) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_DIR))


# ============================================================
# 1. Import + Module Sanity (主 17:43 实事求是)
# ============================================================

def test_v1275_module_imports():
    """V1275 module 必須能 import (主 17:43 实事求是: 真生产不是 fake)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    assert v1275 is not None
    assert hasattr(v1275, "V1275_VERSION")
    assert hasattr(v1275, "V1275_BUILD")
    assert hasattr(v1275, "V1275_ASI_NS_LOCKED_PCT")
    assert v1275.V1275_ASI_NS_LOCKED_PCT == 92.91  # 不刷 KPI


def test_v1275_philosophy_gate_keys():
    """V1275 philosophy_gate 必须 = V1274 (9 keys) + V1275 扩展 1 key = 10 keys (主 17:58 不假装)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    gate = v1275._v1275_philosophy_gate()
    assert isinstance(gate, dict)
    assert len(gate) == 10, f"expected 10 gates, got {len(gate)}"
    # V1274 inherited
    assert gate["v1274_not_new_asi_dim"] is True
    assert gate["v1274_no_asi_v1_claim"] is True
    assert gate["v1274_no_phenomenal_claim"] is True
    assert gate["v1274_truth_is_falsifiability"] is True
    assert gate["v1274_no_kpi_inflate"] is True
    assert gate["v1274_stdlib_only"] is True
    assert gate["v1274_read_only"] is True
    assert gate["v1274_evidence_required"] is True
    assert gate["v1274_failures_disclosed"] is True
    # V1275 extension
    assert gate["v1275_extends_v1274_not_replaces"] is True


def test_v1275_8_hypotheses():
    """V1275 必须 = 8 假说 (主 17:43 实事求是: 真生产 8 个 substrate/recognition 假说)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    specs = v1275._builtin_hypotheses()
    assert len(specs) == 8, f"expected 8 hypotheses, got {len(specs)}"
    expected_ids = {
        "h_substrate_count",
        "h_kitchen_modules",
        "h_truth_gates_count",
        "h_recent_substrate_lift",
        "h_stream_modules",
        "h_vcp_modules",
        "h_truth_falsifier_self",
        "h_pipeline_22_samples",
    }
    actual_ids = {s.hypothesis_id for s in specs}
    assert actual_ids == expected_ids


def test_v1275_hypothesis_spec_dataclass():
    """HypothesisSpec 字段必须齐全 (主 17:43 实事求是)."""
    from apeireth.v1274_asi_truth_falsifier import HypothesisSpec
    spec = HypothesisSpec(
        hypothesis_id="test_h",
        claim="test claim",
        falsification_rule="if X → FAIL",
        severity="info",
        evidence_type="file_count",
        threshold=10,
    )
    d = spec.to_dict()
    assert d["hypothesis_id"] == "test_h"
    assert d["claim"] == "test claim"
    assert d["severity"] == "info"
    assert d["threshold"] == 10


# ============================================================
# 2. CLI --probe (主 00:56 任何人都能接手)
# ============================================================

def test_v1275_cli_probe_runs():
    """--probe 必须能跑 (主 00:56 任何人都能接手)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1275_asi_extended_falsifier", "--probe"],
        cwd=str(PROMETHEAN_DIR),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=15,
        check=False,
    )
    assert result.returncode == 0, f"--probe failed: {result.stderr}"
    assert "V1275" in result.stdout
    assert "philosophy_gate" in result.stdout
    # 8 假说 should be listed
    for hid in [
        "h_substrate_count", "h_kitchen_modules", "h_truth_gates_count",
        "h_recent_substrate_lift", "h_stream_modules", "h_vcp_modules",
        "h_truth_falsifier_self", "h_pipeline_22_samples",
    ]:
        assert hid in result.stdout, f"{hid} not in --probe output"


def test_v1275_cli_run_runs():
    """--run 必须能跑 + 输出 Markdown (主 17:43 实事求是 + 主 00:56)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1275_asi_extended_falsifier", "--run"],
        cwd=str(PROMETHEAN_DIR),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=60,
        check=False,
    )
    assert "V1275 ASI Extended Falsifier" in result.stdout
    assert "PASS:" in result.stdout
    assert "FAIL:" in result.stdout
    assert "INCONCLUSIVE:" in result.stdout
    # Verify 8 假说 in output
    for hid in [
        "h_substrate_count", "h_kitchen_modules", "h_truth_gates_count",
        "h_recent_substrate_lift", "h_stream_modules", "h_vcp_modules",
        "h_truth_falsifier_self", "h_pipeline_22_samples",
    ]:
        assert hid in result.stdout


def test_v1275_cli_json_runs():
    """--json 必须能跑 + 输出 JSON (主 17:43 实事求是)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1275_asi_extended_falsifier", "--json"],
        cwd=str(PROMETHEAN_DIR),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=60,
        check=False,
    )
    assert result.returncode in (0, 1), f"--json unexpected exit: {result.returncode}"
    # Parse JSON (stdout is just JSON, no prefix)
    data = json.loads(result.stdout)
    assert "results" in data
    assert "n_pass" in data
    assert "n_fail" in data
    assert "n_inconclusive" in data
    assert "falsification_rate" in data
    assert "philosophy_gate" in data
    assert len(data["results"]) == 8


# ============================================================
# 3. run_all_hypotheses (主 17:43 实事求是)
# ============================================================

def test_v1275_run_all_returns_truth_ledger():
    """run_all_hypotheses 必须返回 TruthLedger (主 17:43 实事求是)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    from apeireth.v1274_asi_truth_falsifier import TruthLedger
    ledger = v1275.run_all_hypotheses(PROMETHEAN_DIR)
    assert isinstance(ledger, TruthLedger)
    assert len(ledger.results) == 8
    assert ledger.n_pass + ledger.n_fail + ledger.n_inconclusive == 8


def test_v1275_expected_pass_fail_distribution():
    """8 假说 真生产期望分布 (主 17:43 实事求是, 2026-08-05 真实测量).

    期望: 7 PASS / 1 FAIL / 0 INCONCLUSIVE
    - h_substrate_count: PASS (43 > 30)
    - h_kitchen_modules: FAIL (v1263 test missing - REAL fact)
    - h_truth_gates_count: PASS (9 >= 9)
    - h_recent_substrate_lift: PASS (82 >= 3)
    - h_stream_modules: PASS (5/5 mods + 5/5 tests)
    - h_vcp_modules: PASS (V1272 exists)
    - h_truth_falsifier_self: PASS (V1274 --probe exit=0)
    - h_pipeline_22_samples: PASS (V1268 has '22' reference)
    """
    import apeireth.v1275_asi_extended_falsifier as v1275
    ledger = v1275.run_all_hypotheses(PROMETHEAN_DIR)
    # 期望分布
    assert ledger.n_pass == 7, f"expected 7 PASS, got {ledger.n_pass}"
    assert ledger.n_fail == 1, f"expected 1 FAIL, got {ledger.n_fail}"
    assert ledger.n_inconclusive == 0, f"expected 0 INCONCLUSIVE, got {ledger.n_inconclusive}"
    # Falsification rate = 1/8 = 0.125
    assert ledger.falsification_rate == 0.125, f"expected 0.125, got {ledger.falsification_rate}"


def test_v1275_specific_hypothesis_results():
    """验证 8 假说 各自的 PASS/FAIL (主 17:43 实事求是)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    ledger = v1275.run_all_hypotheses(PROMETHEAN_DIR)
    by_id = {r.hypothesis_id: r for r in ledger.results}

    # h_substrate_count: PASS (43 > 30)
    r = by_id["h_substrate_count"]
    assert r.pass_fail == "PASS", f"h_substrate_count: {r.pass_fail}"
    assert isinstance(r.observed_value, int)
    assert r.observed_value > 30

    # h_kitchen_modules: FAIL (v1263 test missing)
    r = by_id["h_kitchen_modules"]
    assert r.pass_fail == "FAIL", f"h_kitchen_modules: {r.pass_fail}"
    assert isinstance(r.observed_value, dict)
    assert "missing_tests" in r.observed_value
    assert "v1263" in r.observed_value["missing_tests"]

    # h_truth_gates_count: PASS (9 >= 9)
    r = by_id["h_truth_gates_count"]
    assert r.pass_fail == "PASS", f"h_truth_gates_count: {r.pass_fail}"
    assert r.observed_value >= 9

    # h_recent_substrate_lift: PASS (82 >= 3)
    r = by_id["h_recent_substrate_lift"]
    assert r.pass_fail == "PASS"
    assert r.observed_value >= 3

    # h_stream_modules: PASS
    r = by_id["h_stream_modules"]
    assert r.pass_fail == "PASS"
    assert r.observed_value["n_modules_ok"] == 5
    assert r.observed_value["n_tests_ok"] == 5
    assert r.observed_value["missing_modules"] == []
    assert r.observed_value["missing_tests"] == []

    # h_vcp_modules: PASS
    r = by_id["h_vcp_modules"]
    assert r.pass_fail == "PASS"
    assert r.observed_value is not None
    assert "v1272" in r.observed_value.lower()

    # h_truth_falsifier_self: PASS
    r = by_id["h_truth_falsifier_self"]
    assert r.pass_fail == "PASS"
    assert r.observed_value["exit_0"] is True

    # h_pipeline_22_samples: PASS
    r = by_id["h_pipeline_22_samples"]
    assert r.pass_fail == "PASS"
    assert r.observed_value["exists"] is True
    assert r.observed_value["twenty_two_mentions"] >= 1


# ============================================================
# 4. Individual falsify_hypothesis (主 17:43 实事求是)
# ============================================================

def test_v1275_falsify_substrate_count():
    """单跑 h_substrate_count (主 17:43 实事求是)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    from apeireth.v1274_asi_truth_falsifier import HypothesisSpec
    spec = HypothesisSpec(
        hypothesis_id="h_substrate_count",
        claim=f"substrate modules > {v1275.V1275_THRESHOLD_SUBSTRATE_COUNT}",
        falsification_rule=f"if count <= {v1275.V1275_THRESHOLD_SUBSTRATE_COUNT} → FAIL",
        severity="critical",
        evidence_type="substrate_count",
        threshold=v1275.V1275_THRESHOLD_SUBSTRATE_COUNT,
    )
    result = v1275.falsify_hypothesis(spec, PROMETHEAN_DIR)
    assert result.pass_fail in ("PASS", "FAIL")
    assert isinstance(result.observed_value, int)


def test_v1275_falsify_stack_exists():
    """单跑 stack_exists 类型 (主 17:43 实事求是)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    from apeireth.v1274_asi_truth_falsifier import HypothesisSpec
    spec = HypothesisSpec(
        hypothesis_id="test_stack",
        claim="test stack",
        falsification_rule="if any missing → FAIL",
        severity="important",
        evidence_type="stack_exists",
        threshold=["v1260"],  # known to exist
    )
    result = v1275.falsify_hypothesis(spec, PROMETHEAN_DIR)
    assert result.pass_fail == "PASS"


def test_v1275_falsify_source_count():
    """单跑 source_count 类型 (主 17:43 实事求是)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    from apeireth.v1274_asi_truth_falsifier import HypothesisSpec
    spec = HypothesisSpec(
        hypothesis_id="test_source",
        claim="test source",
        falsification_rule="if count < 9 → FAIL",
        severity="info",
        evidence_type="source_count",
        threshold=9,
    )
    result = v1275.falsify_hypothesis(spec, PROMETHEAN_DIR)
    assert result.pass_fail == "PASS"
    assert result.observed_value == 9


def test_v1275_falsify_unknown_evidence_returns_inconclusive():
    """未知 evidence_type 必须返回 INCONCLUSIVE (主 17:43 实事求是)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    from apeireth.v1274_asi_truth_falsifier import HypothesisSpec
    spec = HypothesisSpec(
        hypothesis_id="test_unknown",
        claim="test unknown",
        falsification_rule="unknown → INCONCLUSIVE",
        severity="info",
        evidence_type="nonexistent_evidence_type",
        threshold=1,
    )
    result = v1275.falsify_hypothesis(spec, PROMETHEAN_DIR)
    assert result.pass_fail == "INCONCLUSIVE"


def test_v1275_falsify_file_exists():
    """file_exists 类型 - 存在 file (主 17:43 实事求是)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    from apeireth.v1274_asi_truth_falsifier import HypothesisSpec
    spec = HypothesisSpec(
        hypothesis_id="test_file_exists",
        claim="V1272 exists",
        falsification_rule="if missing → FAIL",
        severity="info",
        evidence_type="file_exists",
        threshold="v1272",
    )
    result = v1275.falsify_hypothesis(spec, PROMETHEAN_DIR)
    assert result.pass_fail == "PASS"
    assert result.observed_value is not None


def test_v1275_falsify_file_exists_missing():
    """file_exists 类型 - 不存在 file 必须 FAIL (主 17:43 实事求是)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    from apeireth.v1274_asi_truth_falsifier import HypothesisSpec
    spec = HypothesisSpec(
        hypothesis_id="test_file_missing",
        claim="V9999 exists",
        falsification_rule="if missing → FAIL",
        severity="info",
        evidence_type="file_exists",
        threshold="v9999_nonexistent",
    )
    result = v1275.falsify_hypothesis(spec, PROMETHEAN_DIR)
    assert result.pass_fail == "FAIL"


# ============================================================
# 5. JSON Serialization + Markdown Report (主 17:43 实事求是)
# ============================================================

def test_v1275_truth_ledger_to_dict():
    """TruthLedger 必须可 JSON 序列化 (主 17:43 实事求是)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    ledger = v1275.run_all_hypotheses(PROMETHEAN_DIR)
    d = ledger.to_dict()
    # JSON serializable check
    json_str = json.dumps(d, ensure_ascii=False)
    assert isinstance(json_str, str)
    assert len(json_str) > 100
    # Re-parse
    parsed = json.loads(json_str)
    assert parsed["n_pass"] == 7
    assert parsed["n_fail"] == 1
    assert len(parsed["results"]) == 8


def test_v1275_markdown_report():
    """Markdown report 必须包含所有 8 假说 + 守门 (主 17:43 实事求是)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    ledger = v1275.run_all_hypotheses(PROMETHEAN_DIR)
    md = v1275._to_markdown(ledger)
    assert "V1275 ASI Extended Falsifier" in md
    assert "PASS" in md
    assert "FAIL" in md
    assert "philosophy_gate" in md
    for hid in [
        "h_substrate_count", "h_kitchen_modules", "h_truth_gates_count",
        "h_recent_substrate_lift", "h_stream_modules", "h_vcp_modules",
        "h_truth_falsifier_self", "h_pipeline_22_samples",
    ]:
        assert hid in md


def test_v1275_report_writes_to_file(tmp_path):
    """--report 必须能写 Markdown file (主 17:43 实事求是)."""
    report_path = tmp_path / "v1275_report.md"
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1275_asi_extended_falsifier", "--report", str(report_path)],
        cwd=str(PROMETHEAN_DIR),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=60,
        check=False,
    )
    assert report_path.exists()
    content = report_path.read_text(encoding="utf-8")
    assert "V1275 ASI Extended Falsifier" in content
    assert len(content) > 1000


# ============================================================
# 6. CLI --hypothesis --explain (主 00:56 任何人都能接手)
# ============================================================

def test_v1275_cli_hypothesis_explain():
    """--hypothesis <id> --explain 必须能跑 (主 00:56)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1275_asi_extended_falsifier",
         "--hypothesis", "h_substrate_count", "--explain"],
        cwd=str(PROMETHEAN_DIR),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=30,
        check=False,
    )
    assert "h_substrate_count" in result.stdout
    assert "claim:" in result.stdout
    assert "result:" in result.stdout


def test_v1275_cli_hypothesis_not_found():
    """未知 hypothesis 必须报错 (主 17:43 实事求是)."""
    result = subprocess.run(
        [sys.executable, "-m", "apeireth.v1275_asi_extended_falsifier",
         "--hypothesis", "h_nonexistent", "--explain"],
        cwd=str(PROMETHEAN_DIR),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=15,
        check=False,
    )
    assert result.returncode == 1
    assert "not found" in result.stdout


# ============================================================
# 7. ASI NS 守门 + V3 Philosophy 不假装 (主 17:58 + 主 20:46)
# ============================================================

def test_v1275_no_phenomenal_claim_in_strings():
    """V1275 philosophy_gate 必须包含 no_phenomenal_claim (主 17:58 不假装)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    # Runtime check: philosophy_gate 里有 v1274_no_phenomenal_claim
    gate = v1275._v1275_philosophy_gate()
    assert gate["v1274_no_phenomenal_claim"] is True
    # Source 必須含不假装 Phenomenal 字样
    src = Path(v1275.__file__).read_text(encoding="utf-8", errors="replace")
    assert "不假装 Phenomenal" in src


def test_v1275_no_kpi_inflation():
    """V1275 守门 必须包含 no_kpi_inflate (主 17:43 实事求是: NS 92.91% LOCKED 不刷)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    gate = v1275._v1275_philosophy_gate()
    assert gate["v1274_no_kpi_inflate"] is True
    # V1275 must NOT claim ASI > 92.91%
    assert v1275.V1275_ASI_NS_LOCKED_PCT == 92.91


def test_v1275_v3_philosophy_gate_extends_v1274():
    """V1275 必须声明 extends_v1274_not_replaces (主 17:43 + 主 22:33 方向微调)."""
    import apeireth.v1275_asi_extended_falsifier as v1275
    gate = v1275._v1275_philosophy_gate()
    assert gate["v1275_extends_v1274_not_replaces"] is True


# ============================================================
# 8. Stdlib only (主 19:33 走在前人肩上 + 主 00:56 任何人都能接手)
# ============================================================

def test_v1275_stdlib_only_imports():
    """V1275 只能 import stdlib + 自己的 V1274 (主 19:33 + 主 00:56).

    不应依赖 numpy / scipy / torch / transformers 等.
    """
    import apeireth.v1275_asi_extended_falsifier as v1275
    src = Path(v1275.__file__).read_text(encoding="utf-8", errors="replace")
    # Should import v1274
    assert "from apeireth.v1274" in src
    # Should NOT import external libs
    forbidden = ["import numpy", "import scipy", "import torch",
                 "import transformers", "import pandas"]
    for f in forbidden:
        assert f not in src, f"V1275 must not import {f}"


if __name__ == "__main__":
    sys.exit(pytest.main([__file__, "-v"]))