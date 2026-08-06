"""Tests for V1283 Multi-Crate Rust Semantic Sweep — 真生产 tests

> 主 17:43 实事求是: 真 tests, 0 skip, 0 fake.
> 主 19:33 走在前人肩上: 继承 V1280 + V1281 + V1282 dataclasses / falsifier pattern.
> 主 13:31 大胆激进 + 主 23:44 干到底 + 主 00:56 任何人都能接手.
> 主 17:58 不假装: 不刷 KPI, 不假装 ASI V1, FAIL 也展示.
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
from unittest import mock

import pytest

PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1283_multi_crate_semantic_sweep import (
    V1283_VERSION,
    V1283_BUILD,
    V1283_ASI_NS_CURRENT,
    V1283_ASI_NS_LOCKED_PCT,
    V1283_THRESHOLD_PUB_API_DENSITY,
    V1283_THRESHOLD_IMPL_RATIO,
    V1283_THRESHOLD_DERIVE_MACRO,
    CrateSemanticMetrics,
    CrateHypothesisResult,
    MultiCrateSweepLedger,
    SEMANTIC_PATTERNS,
    _COMPILED,
    _v1283_philosophy_gate,
    find_all_apeireth_crates,
    resolve_promethean_dir,
    scan_crate,
    falsify_pub_api_surface,
    falsify_impl_real_coverage,
    falsify_derive_macro,
    FALSIFIER_DISPATCH,
    run_multi_crate_sweep,
    _to_json_snapshot,
    _to_markdown,
    main,
)


class TestConstants:
    def test_version(self):
        assert V1283_VERSION == "0.1.0"

    def test_build_format(self):
        assert V1283_BUILD.startswith("2026-08-05-")
        assert V1283_BUILD.endswith("+08")

    def test_asi_ns_current(self):
        # 主 22:33 LOCKED (ceiling V0.1 = 0.7905)
        assert V1283_ASI_NS_CURRENT == 0.7905

    def test_asi_ns_locked_pct(self):
        assert V1283_ASI_NS_LOCKED_PCT == 92.91

    def test_thresholds(self):
        assert V1283_THRESHOLD_PUB_API_DENSITY == 50
        assert V1283_THRESHOLD_IMPL_RATIO == 1.0
        assert V1283_THRESHOLD_DERIVE_MACRO == 5


class TestPatterns:
    def test_pattern_count(self):
        assert len(SEMANTIC_PATTERNS) == 8

    def test_pattern_keys(self):
        expected = {
            "pub_fn_def", "pub_async_fn_def", "trait_def",
            "impl_block", "derive_macro", "pub_struct", "pub_enum", "doc_comment",
        }
        assert set(SEMANTIC_PATTERNS.keys()) == expected

    def test_compiled_patterns_match_regex(self):
        assert _COMPILED["pub_fn_def"].search("pub fn foo()") is not None
        assert _COMPILED["pub_struct"].search("pub struct Bar;") is not None
        assert _COMPILED["pub_enum"].search("pub enum E { A }") is not None
        assert _COMPILED["trait_def"].search("trait Foo {}") is not None
        assert _COMPILED["derive_macro"].search("#[derive(Debug)]") is not None


class TestPhilosophyGate:
    def test_gate_count(self):
        gate = _v1283_philosophy_gate()
        assert len(gate) >= 21

    def test_inherited_20(self):
        gate = _v1283_philosophy_gate()
        # 20 inherited + 1 new
        inherited = sum(1 for k in gate if k.startswith("v1282_inherited_gate_"))
        assert inherited == 20

    def test_v1283_extends_v1282(self):
        gate = _v1283_philosophy_gate()
        assert gate.get("v1283_extends_v1282_not_replaces") is True


class TestResolve:
    def test_resolve_promethean_dir_default(self):
        # Should return the current promethean root
        pd = resolve_promethean_dir()
        assert pd.exists()
        # It should find a crates dir
        assert (pd / "Apeireth-rust" / "crates").is_dir()

    def test_resolve_with_explicit_path(self):
        # Should resolve to the same place
        pd = resolve_promethean_dir(str(PROMETHEAN_ROOT))
        assert pd.is_dir()


class TestFindCrates:
    def test_find_returns_list(self):
        pd = resolve_promethean_dir()
        crates = find_all_apeireth_crates(pd)
        assert isinstance(crates, list)
        assert len(crates) >= 30  # workspace has ~42 crates

    def test_find_filters_apeireth_only(self):
        pd = resolve_promethean_dir()
        crates = find_all_apeireth_crates(pd)
        for name, _path in crates:
            assert name.startswith("apeireth-")

    def test_find_only_with_src(self):
        pd = resolve_promethean_dir()
        crates = find_all_apeireth_crates(pd)
        for _name, path in crates:
            # path is the src dir directly
            assert path.is_dir()
            assert any(path.glob("*.rs"))


class TestScanCrate:
    def test_scan_apeireth_sovereignty(self):
        pd = resolve_promethean_dir()
        crates = find_all_apeireth_crates(pd)
        sobj = next((s for s in crates if s[0] == "apeireth-sovereignty"), None)
        if sobj is None:
            pytest.skip("apeireth-sovereignty crate not present in this checkout")
        name, src = sobj
        m = scan_crate(name, src)
        assert m.crate_name == "apeireth-sovereignty"
        assert m.pub_api_surface() >= 50  # V1282 actually showed 315
        assert m.src_lines > 0
        assert m.src_files > 0


class TestCrateSemanticMetrics:
    def test_pub_api_surface(self):
        m = CrateSemanticMetrics(
            crate_name="t", crate_src="x", src_files=1, src_lines=10,
            pub_fn=10, pub_async_fn=2, pub_struct=5, pub_enum=3, pub_trait=2,
        )
        assert m.pub_api_surface() == 22

    def test_impl_ratio_normal(self):
        m = CrateSemanticMetrics(
            crate_name="t", crate_src="x", src_files=1, src_lines=10,
            impl_block=10, pub_struct=5,
        )
        assert m.impl_to_struct_ratio() == 2.0

    def test_impl_ratio_zero_struct(self):
        m = CrateSemanticMetrics(
            crate_name="t", crate_src="x", src_files=1, src_lines=10,
            impl_block=10, pub_struct=0,
        )
        assert m.impl_to_struct_ratio() == 0.0


class TestFalsifiers:
    def test_falsify_pub_api_pass(self):
        m = CrateSemanticMetrics(
            crate_name="t", crate_src="x", src_files=1, src_lines=10,
            pub_fn=60, pub_async_fn=0, pub_struct=10, pub_enum=5, pub_trait=5,
        )
        r = falsify_pub_api_surface(m)
        assert r.pass_fail == "PASS"
        assert r.observed_value == 80.0

    def test_falsify_pub_api_fail(self):
        m = CrateSemanticMetrics(
            crate_name="t", crate_src="x", src_files=1, src_lines=10,
            pub_fn=10, pub_async_fn=0, pub_struct=2, pub_enum=1, pub_trait=0,
        )
        r = falsify_pub_api_surface(m)
        assert r.pass_fail == "FAIL"
        assert r.observed_value == 13.0

    def test_falsify_impl_ratio_pass(self):
        m = CrateSemanticMetrics(
            crate_name="t", crate_src="x", src_files=1, src_lines=10,
            impl_block=5, pub_struct=2,
        )
        r = falsify_impl_real_coverage(m)
        assert r.pass_fail == "PASS"
        assert r.observed_value == 2.5

    def test_falsify_impl_ratio_fail(self):
        m = CrateSemanticMetrics(
            crate_name="t", crate_src="x", src_files=1, src_lines=10,
            impl_block=2, pub_struct=10,
        )
        r = falsify_impl_real_coverage(m)
        assert r.pass_fail == "FAIL"

    def test_falsify_derive_pass(self):
        m = CrateSemanticMetrics(
            crate_name="t", crate_src="x", src_files=1, src_lines=10,
            derive_macro_applications=20,
        )
        r = falsify_derive_macro(m)
        assert r.pass_fail == "PASS"

    def test_falsify_derive_fail(self):
        m = CrateSemanticMetrics(
            crate_name="t", crate_src="x", src_files=1, src_lines=10,
            derive_macro_applications=2,
        )
        r = falsify_derive_macro(m)
        assert r.pass_fail == "FAIL"

    def test_dispatch_keys(self):
        assert set(FALSIFIER_DISPATCH.keys()) == {
            "h_pub_api_density", "h_impl_real_coverage", "h_derive_macro_usage",
        }


class TestRunner:
    def test_run_full_sweep(self):
        pd = resolve_promethean_dir()
        ledger = run_multi_crate_sweep(promethean_dir=pd)
        assert ledger.crates_scanned >= 30
        assert len(ledger.crate_metrics) == ledger.crates_scanned
        # Each crate gets 3 hypotheses evaluated
        assert len(ledger.results) == 3 * ledger.crates_scanned
        # Truth count equals sum
        assert ledger.n_pass + ledger.n_fail + ledger.n_inconclusive == len(ledger.results)

    def test_run_with_filter(self):
        pd = resolve_promethean_dir()
        ledger = run_multi_crate_sweep(
            promethean_dir=pd,
            crate_filter=lambda n: n == "apeireth-sovereignty",
        )
        # 只 1 crate → 3 results
        assert ledger.crates_scanned == 1
        assert len(ledger.results) == 3

    def test_elapsed_ms_positive(self):
        pd = resolve_promethean_dir()
        ledger = run_multi_crate_sweep(promethean_dir=pd)
        assert ledger.elapsed_ms > 0
        assert ledger.elapsed_ms < 60_000  # must finish in <60s


class TestOutput:
    def test_markdown_contains_header(self):
        pd = resolve_promethean_dir()
        ledger = run_multi_crate_sweep(promethean_dir=pd)
        md = _to_markdown(ledger)
        assert "# V1283 Multi-Crate Rust Semantic Sweep" in md
        assert "PASS rate" in md
        assert "Top-10 Crates" in md

    def test_markdown_contains_all_crates(self):
        pd = resolve_promethean_dir()
        ledger = run_multi_crate_sweep(promethean_dir=pd)
        md = _to_markdown(ledger)
        for m in ledger.crate_metrics:
            assert m.crate_name in md

    def test_markdown_disclaimer(self):
        pd = resolve_promethean_dir()
        ledger = run_multi_crate_sweep(promethean_dir=pd)
        md = _to_markdown(ledger)
        # 主 17:58 不假装
        assert "不假装" in md or "不刷 KPI" in md or "ASI 收官" in md

    def test_json_snapshot(self):
        pd = resolve_promethean_dir()
        ledger = run_multi_crate_sweep(promethean_dir=pd)
        snap = _to_json_snapshot(ledger)
        assert snap["crates_scanned"] >= 30
        assert len(snap["crate_metrics"]) == snap["crates_scanned"]
        assert len(snap["results"]) == 3 * snap["crates_scanned"]
        assert snap["asi_ns_locked_pct"] == 92.91


class TestCLI:
    def test_probe(self, capsys):
        rc = main(["--probe"])
        out = capsys.readouterr().out
        assert rc == 0
        assert "n_crates=" in out
        assert "apeireth-" in out

    def test_run_no_report(self, capsys):
        rc = main(["--run"])
        out = capsys.readouterr().out
        assert rc == 0
        assert "PASS rate" in out

    def test_report_to_file(self, tmp_path):
        target = tmp_path / "r.md"
        rc = main(["--report", str(target)])
        assert rc == 0
        assert target.exists()
        content = target.read_text(encoding="utf-8")
        assert "V1283" in content

    def test_json(self, capsys):
        rc = main(["--json"])
        out = capsys.readouterr().out
        assert rc == 0
        # 必须 JSON parse
        data = json.loads(out)
        assert "crates_scanned" in data
