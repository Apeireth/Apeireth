"""Tests for V1282 Rust apeireth-sovereignty Semantic Audit (VCP 真实源代码深读 #3) — 真生产 tests

> 60+ tests covering: constants, philosophy gate, evidence gatherers, hypotheses,
> falsifier dispatch, runner, output, CLI, regression vs V1281.
> 主 17:43 实事求是: 真 tests, 0 skip, 0 fake.
> 主 19:33 走在前人肩上: 继承 V1274 dataclasses + V1281 audit pattern.
> 主 13:31 大胆激进 + 主 23:44 干到底 + 主 00:56 任何人都能接手.
"""
from __future__ import annotations

import io
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import List
from unittest import mock

import pytest

PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1274_asi_truth_falsifier import (
    HypothesisSpec,
    FalsifierResult,
    TruthLedger,
)
from apeireth.v1282_rust_apeireth_sovereignty_semantic_audit import (
    V1282_VERSION,
    V1282_BUILD,
    V1282_ASI_NS_CURRENT,
    V1282_ASI_NS_LOCKED_PCT,
    V1282_TARGET_CRATE,
    V1282_THRESHOLD_PUB_API_DENSITY,
    V1282_THRESHOLD_IMPL_RATIO,
    V1282_THRESHOLD_DERIVE_MACRO,
    SEMANTIC_PATTERNS,
    _v1282_philosophy_gate,
    _builtin_hypotheses,
    _scan_crate,
    _pub_api_surface,
    _impl_to_struct_ratio,
    _falsify_pub_api_surface,
    _falsify_impl_ratio,
    _falsify_derive_macro,
    FALSIFIER_DISPATCH,
    falsify_hypothesis,
    resolve_promethean_dir,
    resolve_target_crate,
    run_all_hypotheses,
    _to_markdown,
    _to_json_snapshot,
    main,
)


APEIRETH_SOVEREIGNTY_SRC = PROMETHEAN_ROOT / "Apeireth-rust" / "crates" / V1282_TARGET_CRATE / "src"


# ============================================================
# 0. Constants sanity
# ============================================================

class TestConstants:
    def test_version_format(self):
        assert re.match(r"^\d+\.\d+\.\d+$", V1282_VERSION)

    def test_build_format(self):
        assert re.match(r"^\d{4}-\d{2}-\d{2}-\d{4}\+\d{2}$", V1282_BUILD)

    def test_asi_ns_locked(self):
        # 主 22:33: ASI ceiling V0.1 = 0.7905, display 92.91%
        assert V1282_ASI_NS_LOCKED_PCT == 92.91
        assert V1282_ASI_NS_CURRENT == 0.7905

    def test_target_crate(self):
        assert V1282_TARGET_CRATE == "apeireth-sovereignty"

    def test_thresholds_positive(self):
        # 主 17:43 实事求是: 阈值基于真实观察
        assert V1282_THRESHOLD_PUB_API_DENSITY >= 10
        assert V1282_THRESHOLD_IMPL_RATIO > 0
        assert V1282_THRESHOLD_DERIVE_MACRO >= 1

    def test_thresholds_realistic(self):
        # 当前真实数据已观测: 315 / 3.31 / 84 — 阈值合理偏低
        assert V1282_THRESHOLD_PUB_API_DENSITY <= 1000
        assert V1282_THRESHOLD_IMPL_RATIO <= 20
        assert V1282_THRESHOLD_DERIVE_MACRO <= 200


# ============================================================
# 1. V3 Philosophy Gate
# ============================================================

class TestPhilosophyGate:
    def test_all_values_true(self):
        gate = _v1282_philosophy_gate()
        assert all(v is True for v in gate.values()), f"some gates False: {gate}"

    def test_inherits_v1274_v1281_layers(self):
        gate = _v1282_philosophy_gate()
        # V1274 9 + V1275 1 + V1276 1 + V1277 2 + V1278 2 + V1279 2 + V1280 1 + V1281 1
        for k in [
            "v1274_not_new_asi_dim",
            "v1274_no_asi_v1_claim",
            "v1274_no_phenomenal_claim",
            "v1274_truth_is_falsifiability",
            "v1274_no_kpi_inflate",
            "v1274_stdlib_only",
            "v1274_read_only",
            "v1274_evidence_required",
            "v1274_failures_disclosed",
            "v1275_extends_v1274_not_replaces",
            "v1276_extends_v1275_not_replaces",
            "v1277_extends_v1276_not_replaces",
            "v1277_no_free_will_claim",
            "v1278_extends_v1277_not_replaces",
            "v1278_no_strong_emergence_claim",
            "v1279_extends_v1278_not_replaces",
            "v1280_extends_v1279_not_replaces",
            "v1281_extends_v1280_not_replaces",
        ]:
            assert k in gate, f"missing inherited gate: {k}"

    def test_v1282_new_gate(self):
        gate = _v1282_philosophy_gate()
        # V1282 = 扩展, 不替代 V1281; 转去 governance crate 语义, 不再做 technical crate
        assert gate["v1282_extends_v1281_not_replaces"] is True

    def test_gate_count(self):
        gate = _v1282_philosophy_gate()
        # V1274 9 + V1275-V1281 8 + V1282 1 = 20
        assert len(gate) == 20


# ============================================================
# 2. Real Evidence Gatherers (主 17:43 实事求是)
# ============================================================

class TestScanCrate:
    def test_real_scan(self):
        metrics, total_lines, errors = _scan_crate(APEIRETH_SOVEREIGNTY_SRC)
        assert metrics, f"empty metrics, errors={errors}"
        # 主 17:43 实事求是: 真测真实数
        assert metrics["pub_fn_def"] >= 100  # 当前 199
        assert metrics["pub_struct"] >= 30  # 当前 49
        assert total_lines >= 5000

    def test_real_metrics_complete(self):
        metrics, _, errors = _scan_crate(APEIRETH_SOVEREIGNTY_SRC)
        for k in SEMANTIC_PATTERNS:
            assert k in metrics, f"missing metric: {k}"
            assert metrics[k] >= 0

    def test_nonexistent_dir(self):
        metrics, total_lines, errors = _scan_crate(Path("/nonexistent/xyz_1282"))
        assert errors
        assert total_lines == 0

    def test_pattern_keys_complete(self):
        expected = {
            "pub_fn_def", "pub_async_fn_def",
            "trait_def", "impl_block", "derive_macro",
            "pub_struct", "pub_enum", "doc_comment",
        }
        assert set(SEMANTIC_PATTERNS.keys()) == expected


class TestPubApiSurface:
    def test_real_surface(self):
        metrics, _, _ = _scan_crate(APEIRETH_SOVEREIGNTY_SRC)
        surface = _pub_api_surface(metrics)
        # 199 + 4 + 49 + 43 + 20 = 315
        assert surface >= 200, f"got {surface}"
        assert surface == (
            metrics["pub_fn_def"]
            + metrics["pub_async_fn_def"]
            + metrics["pub_struct"]
            + metrics["pub_enum"]
            + metrics["trait_def"]
        )


class TestImplToStructRatio:
    def test_real_ratio(self):
        metrics, _, _ = _scan_crate(APEIRETH_SOVEREIGNTY_SRC)
        ratio = _impl_to_struct_ratio(metrics)
        # 162/49 = 3.31
        assert ratio >= 2.0, f"got {ratio}"

    def test_zero_struct_returns_zero(self):
        metrics = {"pub_struct": 0, "impl_block": 5}
        assert _impl_to_struct_ratio(metrics) == 0.0

    def test_ratio_precision(self):
        # 30 / 20 = 1.5
        metrics = {"pub_struct": 20, "impl_block": 30}
        assert _impl_to_struct_ratio(metrics) == 1.5


class TestResolveTargetCrate:
    def test_finds_crate(self):
        c = resolve_target_crate(PROMETHEAN_ROOT)
        assert c.is_dir()
        assert c.name == "src"
        assert c.parent.name == V1282_TARGET_CRATE


# ============================================================
# 3. Builtin hypotheses
# ============================================================

class TestBuiltinHypotheses:
    def test_returns_three(self):
        specs = _builtin_hypotheses()
        assert len(specs) == 3

    def test_hypotheses_have_required_fields(self):
        specs = _builtin_hypotheses()
        for s in specs:
            assert isinstance(s, HypothesisSpec)
            assert s.hypothesis_id
            assert s.claim
            assert s.falsification_rule
            assert s.severity in ("critical", "important", "info")
            assert s.evidence_type in ("pub_api_surface", "impl_ratio", "derive_macro")
            assert s.threshold is not None

    def test_hypothesis_ids_unique(self):
        specs = _builtin_hypotheses()
        ids = [s.hypothesis_id for s in specs]
        assert len(ids) == len(set(ids))

    def test_expected_hypotheses_present(self):
        specs = _builtin_hypotheses()
        ids = {s.hypothesis_id for s in specs}
        assert "h_pub_api_density" in ids
        assert "h_impl_real_coverage" in ids
        assert "h_derive_macro_usage" in ids

    def test_severity_assignment(self):
        specs = _builtin_hypotheses()
        by_id = {s.hypothesis_id: s for s in specs}
        assert by_id["h_pub_api_density"].severity == "critical"
        assert by_id["h_impl_real_coverage"].severity == "important"
        assert by_id["h_derive_macro_usage"].severity == "info"

    def test_evidence_type_assignment(self):
        specs = _builtin_hypotheses()
        by_id = {s.hypothesis_id: s for s in specs}
        assert by_id["h_pub_api_density"].evidence_type == "pub_api_surface"
        assert by_id["h_impl_real_coverage"].evidence_type == "impl_ratio"
        assert by_id["h_derive_macro_usage"].evidence_type == "derive_macro"

    def test_thresholds_match_constants(self):
        specs = _builtin_hypotheses()
        by_id = {s.hypothesis_id: s for s in specs}
        assert by_id["h_pub_api_density"].threshold == float(V1282_THRESHOLD_PUB_API_DENSITY)
        assert by_id["h_impl_real_coverage"].threshold == float(V1282_THRESHOLD_IMPL_RATIO)
        assert by_id["h_derive_macro_usage"].threshold == float(V1282_THRESHOLD_DERIVE_MACRO)

    def test_vcp_governance_disclaimer_in_claims(self):
        specs = _builtin_hypotheses()
        for s in specs:
            claim_l = s.claim.lower()
            # 主 17:58: 真 production claim 应明确指 VCP / governance / apeireth-sovereignty / 语义
            assert "vcp" in claim_l or V1282_TARGET_CRATE in s.claim or "governance" in claim_l or "语义" in s.claim

    def test_target_crate_in_all_claims(self):
        specs = _builtin_hypotheses()
        for s in specs:
            assert V1282_TARGET_CRATE in s.claim, f"missing target crate in: {s.claim}"


# ============================================================
# 4. Falsifier functions (主 17:43 实事求是 + 主 19:33 Popper)
# ============================================================

class TestFalsifyPubApiSurface:
    def test_real_run(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_pub_api_density")
        r = _falsify_pub_api_surface(s, APEIRETH_SOVEREIGNTY_SRC)
        assert isinstance(r, FalsifierResult)
        assert r.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
        assert r.observed_value is not None

    def test_real_passes(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_pub_api_density")
        r = _falsify_pub_api_surface(s, APEIRETH_SOVEREIGNTY_SRC)
        assert r.pass_fail == "PASS", f"got {r.pass_fail}, notes={r.notes}"

    def test_observed_value_format(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_pub_api_density")
        r = _falsify_pub_api_surface(s, APEIRETH_SOVEREIGNTY_SRC)
        # 主 17:43: notes 应包含 pub_api_surface breakdown
        assert "pub_api_surface=" in r.notes
        assert "pub_fn=" in r.notes


class TestFalsifyImplRatio:
    def test_real_run(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_impl_real_coverage")
        r = _falsify_impl_ratio(s, APEIRETH_SOVEREIGNTY_SRC)
        assert isinstance(r, FalsifierResult)
        assert r.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
        assert r.observed_value is not None

    def test_real_passes(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_impl_real_coverage")
        r = _falsify_impl_ratio(s, APEIRETH_SOVEREIGNTY_SRC)
        assert r.pass_fail == "PASS", f"got {r.pass_fail}, notes={r.notes}"

    def test_observed_value_is_ratio(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_impl_real_coverage")
        r = _falsify_impl_ratio(s, APEIRETH_SOVEREIGNTY_SRC)
        assert r.observed_value >= 2.0  # 3.31
        assert "impl=" in r.notes
        assert "pub_struct=" in r.notes


class TestFalsifyDeriveMacro:
    def test_real_run(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_derive_macro_usage")
        r = _falsify_derive_macro(s, APEIRETH_SOVEREIGNTY_SRC)
        assert isinstance(r, FalsifierResult)
        assert r.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
        assert r.observed_value is not None

    def test_real_passes(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_derive_macro_usage")
        r = _falsify_derive_macro(s, APEIRETH_SOVEREIGNTY_SRC)
        assert r.pass_fail == "PASS", f"got {r.pass_fail}, notes={r.notes}"

    def test_observed_value_format(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_derive_macro_usage")
        r = _falsify_derive_macro(s, APEIRETH_SOVEREIGNTY_SRC)
        assert "derive_macro_applications=" in r.notes


# ============================================================
# 5. Falsifier dispatch
# ============================================================

class TestFalsifierDispatch:
    def test_dispatch_table_complete(self):
        expected_keys = {"pub_api_surface", "impl_ratio", "derive_macro"}
        assert set(FALSIFIER_DISPATCH.keys()) == expected_keys

    def test_dispatch_via_falsify_hypothesis(self):
        specs = _builtin_hypotheses()
        for s in specs:
            r = falsify_hypothesis(s, APEIRETH_SOVEREIGNTY_SRC)
            assert isinstance(r, FalsifierResult)
            assert r.hypothesis_id == s.hypothesis_id

    def test_unknown_evidence_type_returns_inconclusive(self):
        s = HypothesisSpec(
            hypothesis_id="h_unknown_v1282_test",
            claim="fake",
            falsification_rule="fake",
            severity="info",
            evidence_type="nonexistent_v1282_evidence_xyz",
            threshold=1.0,
        )
        r = falsify_hypothesis(s, APEIRETH_SOVEREIGNTY_SRC)
        assert r.pass_fail == "INCONCLUSIVE"
        assert "unknown" in r.notes.lower()


# ============================================================
# 6. Run all hypotheses
# ============================================================

class TestRunAllHypotheses:
    def test_returns_truth_ledger(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert isinstance(ledger, TruthLedger)
        assert ledger.n_pass + ledger.n_fail + ledger.n_inconclusive == 3

    def test_ledger_counts_match_results(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        n_pass = sum(1 for r in ledger.results if r.pass_fail == "PASS")
        n_fail = sum(1 for r in ledger.results if r.pass_fail == "FAIL")
        n_inc = sum(1 for r in ledger.results if r.pass_fail == "INCONCLUSIVE")
        assert n_pass == ledger.n_pass
        assert n_fail == ledger.n_fail
        assert n_inc == ledger.n_inconclusive

    def test_philosophy_gate_in_ledger(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert len(ledger.philosophy_gate) == 20

    def test_run_id_format(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert ledger.run_id.startswith("v1282-")
        assert ledger.run_id.split("-")[1].isdigit()

    def test_default_dir_resolution(self):
        ledger = run_all_hypotheses()
        assert isinstance(ledger, TruthLedger)
        assert "promethean" in ledger.promethean_dir.lower()

    def test_real_promethean_all_pass(self):
        # Real apeireth-sovereignty: pub_api_surface=315, impl_ratio=3.31, derive=84 → 3 PASS
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert ledger.n_pass == 3, (
            f"Expected 3 pass, got pass={ledger.n_pass} "
            f"fail={ledger.n_fail} inc={ledger.n_inconclusive}"
        )

    def test_real_observed_values(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        by_id = {r.hypothesis_id: r for r in ledger.results}
        # 主 17:43 实事求是: 真测真实数
        assert by_id["h_pub_api_density"].observed_value >= 200
        assert by_id["h_impl_real_coverage"].observed_value >= 2.0
        assert by_id["h_derive_macro_usage"].observed_value >= 50


# ============================================================
# 7. Output rendering
# ============================================================

class TestOutput:
    def test_markdown_contains_run_id(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        assert ledger.run_id in md

    def test_markdown_contains_all_hypotheses(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        for r in ledger.results:
            assert r.hypothesis_id in md

    def test_markdown_contains_philosophy_gate(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        for k in ledger.philosophy_gate:
            assert k in md, f"missing gate key in markdown: {k}"

    def test_markdown_disclaimer_present(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        # 主 17:58 + 主 20:46: 不假装 ASI V1
        assert "≠" in md or "不等于" in md or "不假装" in md

    def test_markdown_mentions_target_crate(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        # 主 13:31 + 主 23:44: VCP governance crate 语义审计
        assert V1282_TARGET_CRATE in md

    def test_markdown_mentions_vcp_rust_3(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        # VCP Rust #3 应明确出现
        assert "VCP" in md and ("#3" in md or "governance" in md.lower())

    def test_json_parses(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        snap = _to_json_snapshot(ledger)
        s = json.dumps(snap, ensure_ascii=False)
        parsed = json.loads(s)
        assert parsed["run_id"] == ledger.run_id

    def test_json_includes_all_results(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        snap = _to_json_snapshot(ledger)
        assert len(snap["results"]) == 3

    def test_json_includes_target_crate(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        snap = _to_json_snapshot(ledger)
        assert snap["target_crate"] == V1282_TARGET_CRATE

    def test_json_includes_asi_ns_locked(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        snap = _to_json_snapshot(ledger)
        assert snap["asi_ns_locked_pct"] == 92.91
        assert snap["asi_ns_current"] == 0.7905


# ============================================================
# 8. CLI smoke tests
# ============================================================

class TestCLI:
    def test_probe_runs(self, capsys):
        rc = main(["--probe"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1282" in captured.out
        assert "h_pub_api_density" in captured.out
        assert V1282_TARGET_CRATE in captured.out
        assert "philosophy_gate_keys" in captured.out

    def test_run_outputs_markdown(self, capsys):
        rc = main(["--run"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1282" in captured.out
        assert "h_pub_api_density" in captured.out

    def test_json_outputs_json(self, capsys):
        rc = main(["--json"])
        captured = capsys.readouterr()
        assert rc == 0
        parsed = json.loads(captured.out)
        assert "run_id" in parsed
        assert parsed["target_crate"] == V1282_TARGET_CRATE
        assert len(parsed["results"]) == 3

    def test_report_writes_file(self, tmp_path):
        report_path = tmp_path / "v1282_report.md"
        rc = main(["--report", str(report_path)])
        assert rc == 0
        assert report_path.exists()
        content = report_path.read_text(encoding="utf-8")
        assert "V1282" in content
        assert V1282_TARGET_CRATE in content

    def test_hypothesis_explain(self, capsys):
        rc = main(["--hypothesis", "h_pub_api_density"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "h_pub_api_density" in captured.out
        assert "claim:" in captured.out

    def test_unknown_hypothesis_returns_error(self, capsys):
        rc = main(["--hypothesis", "h_nonexistent_xyz"])
        captured = capsys.readouterr()
        assert rc == 2
        assert "unknown" in captured.err.lower()

    def test_promethean_dir_arg(self):
        rc = main(["--probe", "--promethean-dir", str(PROMETHEAN_ROOT)])
        assert rc == 0


# ============================================================
# 9. Resolve promethean dir
# ============================================================

class TestResolvePrometheanDir:
    def test_explicit_arg(self):
        pd = resolve_promethean_dir(str(PROMETHEAN_ROOT))
        assert pd == PROMETHEAN_ROOT

    def test_invalid_arg_falls_back(self, tmp_path):
        pd = resolve_promethean_dir(str(tmp_path / "nonexistent"))
        assert pd.exists()

    def test_none_arg_falls_back(self):
        pd = resolve_promethean_dir(None)
        assert pd.exists()


# ============================================================
# 10. Regression vs V1281 (主 19:33 走在前人肩上)
# ============================================================

class TestRegressionVsV1281:
    def test_gate_inherits_v1281_layers(self):
        gate = _v1282_philosophy_gate()
        # V1282 = V1281 19 gates + V1282 1 = 20
        v1281_inherited_keys = [
            "v1274_not_new_asi_dim",
            "v1274_no_asi_v1_claim",
            "v1274_no_phenomenal_claim",
            "v1274_truth_is_falsifiability",
            "v1274_no_kpi_inflate",
            "v1274_stdlib_only",
            "v1274_read_only",
            "v1274_evidence_required",
            "v1274_failures_disclosed",
            "v1275_extends_v1274_not_replaces",
            "v1276_extends_v1275_not_replaces",
            "v1277_extends_v1276_not_replaces",
            "v1277_no_free_will_claim",
            "v1278_extends_v1277_not_replaces",
            "v1278_no_strong_emergence_claim",
            "v1279_extends_v1278_not_replaces",
            "v1280_extends_v1279_not_replaces",
            "v1281_extends_v1280_not_replaces",
        ]
        for k in v1281_inherited_keys:
            assert k in gate, f"V1281 inherited gate missing: {k}"

    def test_v1282_does_not_replace_v1281(self):
        gate = _v1282_philosophy_gate()
        # 主 19:33: V1282 扩展 V1281, 不替代
        assert gate["v1282_extends_v1281_not_replaces"]
        # V1281 的关键 gate 必须保留
        assert gate["v1281_extends_v1280_not_replaces"]

    def test_run_includes_v1282_asi_ns_disclaimer(self):
        # 主 17:58: ASI NS LOCKED 不变, 不假装 ASI V1
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        assert "92.91" in md
        assert "0.7905" in md or "LOCKED" in md


# ============================================================
# 11. Threshold-driven FAIL behavior (主 17:43 实事求是)
# ============================================================

class TestThresholdFalsification:
    """确认 FAIL 也诚实展示 (主 17:43 + 主 17:58) — 不刷阈值隐藏."""

    def test_high_pub_api_threshold_returns_fail(self):
        s = HypothesisSpec(
            hypothesis_id="h_test_high_pub_api",
            claim="test",
            falsification_rule="fail",
            severity="critical",
            evidence_type="pub_api_surface",
            threshold=10_000_000,  # 10M pub API surface impossible
        )
        r = _falsify_pub_api_surface(s, APEIRETH_SOVEREIGNTY_SRC)
        assert r.pass_fail == "FAIL"
        assert r.observed_value < 10_000_000

    def test_high_impl_ratio_threshold_returns_fail(self):
        s = HypothesisSpec(
            hypothesis_id="h_test_high_impl_ratio",
            claim="test",
            falsification_rule="fail",
            severity="info",
            evidence_type="impl_ratio",
            threshold=1000.0,
        )
        r = _falsify_impl_ratio(s, APEIRETH_SOVEREIGNTY_SRC)
        assert r.pass_fail == "FAIL"
        assert r.observed_value < 1000.0

    def test_high_derive_threshold_returns_fail(self):
        s = HypothesisSpec(
            hypothesis_id="h_test_high_derive",
            claim="test",
            falsification_rule="fail",
            severity="info",
            evidence_type="derive_macro",
            threshold=10_000,
        )
        r = _falsify_derive_macro(s, APEIRETH_SOVEREIGNTY_SRC)
        assert r.pass_fail == "FAIL"
        assert r.observed_value < 10_000

    def test_low_pub_api_threshold_returns_pass(self):
        s = HypothesisSpec(
            hypothesis_id="h_test_low_pub_api",
            claim="test",
            falsification_rule="pass",
            severity="info",
            evidence_type="pub_api_surface",
            threshold=1,
        )
        r = _falsify_pub_api_surface(s, APEIRETH_SOVEREIGNTY_SRC)
        assert r.pass_fail == "PASS"


# ============================================================
# 12. Contrast with V1281 (主 13:31: governance vs technical)
# ============================================================

class TestContrastWithV1281:
    """主 13:31: V1281 = technical (asi), V1282 = governance (sovereignty)."""

    def test_v1282_target_crate_different_from_v1281(self):
        from apeireth.v1281_rust_apeireth_asi_semantic_audit import V1281_TARGET_CRATE
        assert V1282_TARGET_CRATE != V1281_TARGET_CRATE
        assert V1282_TARGET_CRATE == "apeireth-sovereignty"
        assert V1281_TARGET_CRATE == "apeireth-asi"

    def test_v1282_governance_larger_than_v1281_technical(self):
        # 主 17:43 实事求是: apeireth-sovereignty (governance) 应比 apeireth-asi (technical) 更大
        v1282_metrics, _, _ = _scan_crate(APEIRETH_SOVEREIGNTY_SRC)
        apeireth_asi_src = PROMETHEAN_ROOT / "Apeireth-rust" / "crates" / "apeireth-asi" / "src"
        v1281_metrics, _, _ = _scan_crate(apeireth_asi_src)
        # pub_api_surface
        v1282_surface = _pub_api_surface(v1282_metrics)
        v1281_surface = _pub_api_surface(v1281_metrics)
        assert v1282_surface > v1281_surface, (
            f"sovereignty ({v1282_surface}) should be larger than asi ({v1281_surface})"
        )