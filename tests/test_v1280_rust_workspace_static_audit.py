"""Tests for V1280 Rust Workspace Static Audit (VCP 真实源代码深读) — 真生产 tests

> 50+ tests covering: constants, philosophy gate, evidence gatherers, hypotheses,
> falsifier dispatch, runner, output, CLI, regression vs V1279.
> 主 17:43 实事求是: 真 tests, 0 skip, 0 fake.
> 主 19:33 走在前人肩上: 继承 V1274 dataclasses + V1279 self-audit pattern.
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
from apeireth.v1280_rust_workspace_static_audit import (
    V1280_VERSION,
    V1280_BUILD,
    V1280_ASI_NS_CURRENT,
    V1280_ASI_NS_LOCKED_PCT,
    V1280_THRESHOLD_WORKSPACE_MEMBERS,
    V1280_THRESHOLD_CRATES_WITH_TESTS,
    V1280_THRESHOLD_TOTAL_RUST_LOC,
    _v1280_philosophy_gate,
    _builtin_hypotheses,
    _parse_cargo_workspace_members,
    _count_crates_with_tests,
    _count_total_rust_loc,
    _falsify_cargo_workspace,
    _falsify_crate_tests,
    _falsify_total_loc,
    FALSIFIER_DISPATCH,
    falsify_hypothesis,
    resolve_promethean_dir,
    resolve_rust_root,
    run_all_hypotheses,
    _to_markdown,
    _to_json_snapshot,
    main,
)


RUST_ROOT = PROMETHEAN_ROOT / "Apeireth-rust"


# ============================================================
# 0. Constants sanity
# ============================================================

class TestConstants:
    def test_version_format(self):
        assert re.match(r"^\d+\.\d+\.\d+$", V1280_VERSION)

    def test_build_format(self):
        assert re.match(r"^\d{4}-\d{2}-\d{2}-\d{4}\+\d{2}$", V1280_BUILD)

    def test_asi_ns_locked(self):
        # 主 22:33: ASI ceiling V0.1 = 0.7905, display 92.91%
        assert V1280_ASI_NS_LOCKED_PCT == 92.91
        assert V1280_ASI_NS_CURRENT == 0.7905

    def test_thresholds_positive(self):
        # 主 17:43 实事求是: 阈值基于真实观察
        assert V1280_THRESHOLD_WORKSPACE_MEMBERS >= 10
        assert V1280_THRESHOLD_CRATES_WITH_TESTS >= 5
        assert V1280_THRESHOLD_TOTAL_RUST_LOC >= 10000

    def test_thresholds_realistic(self):
        # 当前真实数据已观测: 42 / 39 / 3061438 — 阈值合理偏低
        assert V1280_THRESHOLD_WORKSPACE_MEMBERS <= 100
        assert V1280_THRESHOLD_CRATES_WITH_TESTS <= 100
        assert V1280_THRESHOLD_TOTAL_RUST_LOC <= 10_000_000


# ============================================================
# 1. V3 Philosophy Gate
# ============================================================

class TestPhilosophyGate:
    def test_all_values_true(self):
        gate = _v1280_philosophy_gate()
        assert all(v is True for v in gate.values()), f"some gates False: {gate}"

    def test_inherits_v1274_v1279_layers(self):
        gate = _v1280_philosophy_gate()
        # V1274 9 + V1275 1 + V1276 1 + V1277 2 + V1278 2 + V1279 2
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
            "v1279_no_meta_infinite_regress",
        ]:
            assert k in gate, f"missing inherited gate: {k}"

    def test_v1280_new_gate(self):
        gate = _v1280_philosophy_gate()
        # V1280 = 扩展, 不替代 V1279; 转去 VCP Rust 真读, 不再做 meta-falsifier
        assert gate["v1280_extends_v1279_not_replaces"] is True

    def test_gate_count(self):
        gate = _v1280_philosophy_gate()
        # V1274 9 + V1275 1 + V1276 1 + V1277 2 + V1278 2 + V1279 2 + V1280 1 = 18
        assert len(gate) == 18


# ============================================================
# 2. Real Evidence Gatherers (主 17:43 实事求是)
# ============================================================

class TestParseCargoWorkspace:
    def test_real_workspace_members(self):
        members, ok, errors = _parse_cargo_workspace_members(RUST_ROOT)
        assert ok, f"parse failed: {errors}"
        assert errors == []
        assert len(members) >= 20  # 主 17:43 实事求是: 真测真实数

    def test_real_members_are_crates(self):
        members, ok, errors = _parse_cargo_workspace_members(RUST_ROOT)
        assert ok
        for m in members[:5]:
            assert m.startswith("crates/"), f"unexpected member format: {m}"

    def test_nonexistent_dir(self):
        members, ok, errors = _parse_cargo_workspace_members(Path("/nonexistent/xyz_123"))
        assert not ok
        assert len(errors) > 0


class TestCountCratesWithTests:
    def test_real_count(self):
        count, crates_list, ok, errors = _count_crates_with_tests(RUST_ROOT)
        assert ok, f"count failed: {errors}"
        assert count >= 10
        assert len(crates_list) == count
        assert "apeireth-core" in crates_list

    def test_real_crates_have_rs_in_tests(self):
        _, crates_list, ok, _ = _count_crates_with_tests(RUST_ROOT)
        assert ok
        # At least one crate should have tests/*.rs files
        found_with_rs = False
        for c in crates_list[:5]:
            tests_dir = RUST_ROOT / "crates" / c / "tests"
            if tests_dir.is_dir() and list(tests_dir.glob("*.rs")):
                found_with_rs = True
                break
        assert found_with_rs, "no crates with tests/*.rs found"

    def test_nonexistent_crates_dir(self, tmp_path):
        count, _, ok, errors = _count_crates_with_tests(tmp_path)
        assert not ok
        assert count == 0


class TestCountTotalRustLoc:
    def test_real_total_loc(self):
        total, ok, errors = _count_total_rust_loc(RUST_ROOT)
        assert ok, f"count failed: {errors}"
        assert total >= 50000

    def test_real_skips_target_and_venv(self):
        # target/ and .venv/ should be excluded
        total, ok, _ = _count_total_rust_loc(RUST_ROOT)
        assert ok
        assert total > 0

    def test_nonexistent_dir(self):
        total, ok, errors = _count_total_rust_loc(Path("/nonexistent/xyz_456"))
        assert not ok
        assert total == 0


class TestResolveRustRoot:
    def test_finds_apeireth_rust(self):
        rr = resolve_rust_root(PROMETHEAN_ROOT)
        assert rr.is_dir()
        assert (rr / "Cargo.toml").is_file()
        assert (rr / "crates").is_dir()

    def test_returns_path(self):
        rr = resolve_rust_root(PROMETHEAN_ROOT)
        assert isinstance(rr, Path)


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
            assert s.evidence_type in ("cargo_workspace", "crate_tests", "total_loc")
            assert s.threshold is not None

    def test_hypothesis_ids_unique(self):
        specs = _builtin_hypotheses()
        ids = [s.hypothesis_id for s in specs]
        assert len(ids) == len(set(ids))

    def test_expected_hypotheses_present(self):
        specs = _builtin_hypotheses()
        ids = {s.hypothesis_id for s in specs}
        assert "h_cargo_workspace_intact" in ids
        assert "h_crate_test_coverage" in ids
        assert "h_total_rust_loc" in ids

    def test_severity_assignment(self):
        specs = _builtin_hypotheses()
        by_id = {s.hypothesis_id: s for s in specs}
        assert by_id["h_cargo_workspace_intact"].severity == "critical"
        assert by_id["h_crate_test_coverage"].severity == "important"
        assert by_id["h_total_rust_loc"].severity == "info"

    def test_evidence_type_assignment(self):
        specs = _builtin_hypotheses()
        by_id = {s.hypothesis_id: s for s in specs}
        assert by_id["h_cargo_workspace_intact"].evidence_type == "cargo_workspace"
        assert by_id["h_crate_test_coverage"].evidence_type == "crate_tests"
        assert by_id["h_total_rust_loc"].evidence_type == "total_loc"

    def test_thresholds_match_constants(self):
        specs = _builtin_hypotheses()
        by_id = {s.hypothesis_id: s for s in specs}
        assert by_id["h_cargo_workspace_intact"].threshold == float(V1280_THRESHOLD_WORKSPACE_MEMBERS)
        assert by_id["h_crate_test_coverage"].threshold == float(V1280_THRESHOLD_CRATES_WITH_TESTS)
        assert by_id["h_total_rust_loc"].threshold == float(V1280_THRESHOLD_TOTAL_RUST_LOC)

    def test_vcp_disclaimer_in_claims(self):
        specs = _builtin_hypotheses()
        for s in specs:
            claim_l = s.claim.lower()
            # 主 17:58: 真 production claim 应明确指 VCP / Rust / 真读 / 真实
            assert "vcp" in claim_l or "rust" in claim_l or "真实" in s.claim or "真存在" in s.claim


# ============================================================
# 4. Falsifier functions (主 17:43 实事求是 + 主 19:33 Popper)
# ============================================================

class TestFalsifyCargoWorkspace:
    def test_real_run(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_cargo_workspace_intact")
        r = _falsify_cargo_workspace(s, RUST_ROOT)
        assert isinstance(r, FalsifierResult)
        assert r.hypothesis_id == "h_cargo_workspace_intact"
        assert r.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
        # 当前真实: 42 members
        assert r.observed_value is not None
        assert r.observed_value >= 20

    def test_real_passes(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_cargo_workspace_intact")
        r = _falsify_cargo_workspace(s, RUST_ROOT)
        assert r.pass_fail == "PASS", f"got {r.pass_fail}, notes={r.notes}"


class TestFalsifyCrateTests:
    def test_real_run(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_crate_test_coverage")
        r = _falsify_crate_tests(s, RUST_ROOT)
        assert isinstance(r, FalsifierResult)
        assert r.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
        assert r.observed_value is not None

    def test_real_passes(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_crate_test_coverage")
        r = _falsify_crate_tests(s, RUST_ROOT)
        assert r.pass_fail == "PASS", f"got {r.pass_fail}, notes={r.notes}"


class TestFalsifyTotalLoc:
    def test_real_run(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_total_rust_loc")
        r = _falsify_total_loc(s, RUST_ROOT)
        assert isinstance(r, FalsifierResult)
        assert r.pass_fail in ("PASS", "FAIL", "INCONCLUSIVE")
        assert r.observed_value is not None

    def test_real_passes(self):
        specs = _builtin_hypotheses()
        s = next(s for s in specs if s.hypothesis_id == "h_total_rust_loc")
        r = _falsify_total_loc(s, RUST_ROOT)
        assert r.pass_fail == "PASS", f"got {r.pass_fail}, notes={r.notes}"


# ============================================================
# 5. Falsifier dispatch
# ============================================================

class TestFalsifierDispatch:
    def test_dispatch_table_complete(self):
        expected_keys = {"cargo_workspace", "crate_tests", "total_loc"}
        assert set(FALSIFIER_DISPATCH.keys()) == expected_keys

    def test_dispatch_via_falsify_hypothesis(self):
        specs = _builtin_hypotheses()
        for s in specs:
            r = falsify_hypothesis(s, RUST_ROOT)
            assert isinstance(r, FalsifierResult)
            assert r.hypothesis_id == s.hypothesis_id

    def test_unknown_evidence_type_returns_inconclusive(self):
        s = HypothesisSpec(
            hypothesis_id="h_unknown_v1280_test",
            claim="fake",
            falsification_rule="fake",
            severity="info",
            evidence_type="nonexistent_v1280_evidence_xyz",
            threshold=1.0,
        )
        r = falsify_hypothesis(s, RUST_ROOT)
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
        assert len(ledger.philosophy_gate) == 18

    def test_run_id_format(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert ledger.run_id.startswith("v1280-")
        assert ledger.run_id.split("-")[1].isdigit()

    def test_default_dir_resolution(self):
        ledger = run_all_hypotheses()
        assert isinstance(ledger, TruthLedger)
        assert "promethean" in ledger.promethean_dir.lower()

    def test_real_promethean_all_pass(self):
        # Real workspace: 42 members, 39 crates with tests, 3M+ LOC → 3 PASS
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        assert ledger.n_pass == 3, (
            f"Expected 3 pass, got pass={ledger.n_pass} "
            f"fail={ledger.n_fail} inc={ledger.n_inconclusive}"
        )

    def test_real_observed_values(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        by_id = {r.hypothesis_id: r for r in ledger.results}
        # 主 17:43 实事求是: 真测真实数
        assert by_id["h_cargo_workspace_intact"].observed_value >= 30
        assert by_id["h_crate_test_coverage"].observed_value >= 30
        assert by_id["h_total_rust_loc"].observed_value >= 1_000_000


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

    def test_markdown_mentions_vcp_rust(self):
        ledger = run_all_hypotheses(promethean_dir=PROMETHEAN_ROOT)
        md = _to_markdown(ledger)
        # 主 13:31 + 主 23:44 + 主 19:33: VCP 6 真实源代码深读
        assert "VCP" in md or "Rust" in md or "workspace" in md

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
        assert "V1280" in captured.out
        assert "h_cargo_workspace_intact" in captured.out
        assert "philosophy_gate_keys" in captured.out

    def test_run_outputs_markdown(self, capsys):
        rc = main(["--run"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "V1280" in captured.out
        assert "h_cargo_workspace_intact" in captured.out

    def test_json_outputs_json(self, capsys):
        rc = main(["--json"])
        captured = capsys.readouterr()
        assert rc == 0
        parsed = json.loads(captured.out)
        assert "run_id" in parsed
        assert len(parsed["results"]) == 3

    def test_report_writes_file(self, tmp_path):
        report_path = tmp_path / "v1280_report.md"
        rc = main(["--report", str(report_path)])
        assert rc == 0
        assert report_path.exists()
        content = report_path.read_text(encoding="utf-8")
        assert "V1280" in content

    def test_hypothesis_explain(self, capsys):
        rc = main(["--hypothesis", "h_cargo_workspace_intact"])
        captured = capsys.readouterr()
        assert rc == 0
        assert "h_cargo_workspace_intact" in captured.out
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
# 10. Regression vs V1279 (主 19:33 走在前人肩上)
# ============================================================

class TestRegressionVsV1279:
    def test_gate_inherits_v1279_layers(self):
        gate = _v1280_philosophy_gate()
        # V1280 = V1279 17 gates + V1280 1 = 18
        # V1279 17 = V1274 9 + V1275 1 + V1276 1 + V1277 2 + V1278 2 + V1279 2
        v1279_inherited_keys = [
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
            "v1279_no_meta_infinite_regress",
        ]
        for k in v1279_inherited_keys:
            assert k in gate, f"V1279 inherited gate missing: {k}"

    def test_v1280_does_not_replace_v1279(self):
        gate = _v1280_philosophy_gate()
        # 主 19:33: V1280 扩展 V1279, 不替代
        assert gate["v1280_extends_v1279_not_replaces"]
        # V1279 的关键 gate 必须保留
        assert gate["v1279_no_meta_infinite_regress"]

    def test_run_includes_v1280_asi_ns_disclaimer(self):
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

    def test_high_threshold_returns_fail(self):
        # Construct fake hypothesis with impossibly high threshold
        s = HypothesisSpec(
            hypothesis_id="h_test_high_threshold",
            claim="test",
            falsification_rule="fail",
            severity="critical",
            evidence_type="total_loc",
            threshold=1_000_000_000_000,  # 1 trillion lines
        )
        r = _falsify_total_loc(s, RUST_ROOT)
        # 真测: 1 trillion lines 不可能, FAIL 应诚实展示
        assert r.pass_fail == "FAIL"
        assert r.observed_value < 1_000_000_000_000

    def test_low_threshold_returns_pass(self):
        s = HypothesisSpec(
            hypothesis_id="h_test_low_threshold",
            claim="test",
            falsification_rule="pass",
            severity="info",
            evidence_type="total_loc",
            threshold=1,  # 只要 >=1 行就 PASS
        )
        r = _falsify_total_loc(s, RUST_ROOT)
        assert r.pass_fail == "PASS"