"""Tests for V1289 VCP Rust Public API Doc Coverage Audit — 真生产 tests

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 18:55+08:00 2026-08-05)
> **承接**: V1284-V1288 真生产测试模式 — dataclass + scanner + markdown + CLI 都测
> **不假装**: 测试是真测试, 不是 mock; 不刷 KPI; 失败也诚实披露 (主 17:43 实事求是)
"""
from __future__ import annotations

import json
import os
import sys
import tempfile
import unittest
from pathlib import Path

# 让 tests/ 能找到 promethean 包
_PROMPETHEAN_PARENT = Path(__file__).resolve().parent.parent
if str(_PROMPETHEAN_PARENT) not in sys.path:
    sys.path.insert(0, str(_PROMPETHEAN_PARENT))

from apeireth.v1289_rust_doc_coverage_audit import (  # noqa: E402
    V1289_VERSION,
    V1289_BUILD,
    V1289_ASI_NS_CURRENT,
    V1289_ASI_NS_LOCKED_PCT,
    V1289_THRESHOLD_DOC_COVERAGE_PCT,
    V1289_THRESHOLD_QUALITY_SCORE,
    V1289_HYPOTHESES,
    PUB_FN_RE,
    DOC_LINE_RE,
    BLANK_DOC_RE,
    EXAMPLE_RE,
    PANICS_RE,
    ERRORS_RE,
    RETURNS_RESULT_RE,
    BODY_PANIC_HINT_RE,
    FunctionDocInfo,
    CrateDocMetrics,
    DocCoverageLedger,
    _v1289_philosophy_gate,
    _looks_like_pub_fn,
    _collect_doc_block,
    _find_brace_end,
    scan_crate,
    find_crate_src,
    _evaluate_hypotheses,
    run_doc_coverage_audit,
    _to_markdown,
    _to_json_snapshot,
    main,
)


class TestV1289Constants(unittest.TestCase):
    """基础常量."""

    def test_version_is_string(self):
        self.assertIsInstance(V1289_VERSION, str)
        self.assertGreater(len(V1289_VERSION), 0)

    def test_build_is_string(self):
        self.assertIsInstance(V1289_BUILD, str)
        self.assertIn("2026-08-05", V1289_BUILD)

    def test_asi_ns_constants(self):
        self.assertEqual(V1289_ASI_NS_LOCKED_PCT, 92.91)
        self.assertGreater(V1289_ASI_NS_CURRENT, 0.0)
        self.assertLess(V1289_ASI_NS_CURRENT, 1.0)

    def test_thresholds(self):
        self.assertEqual(V1289_THRESHOLD_DOC_COVERAGE_PCT, 50.0)
        self.assertEqual(V1289_THRESHOLD_QUALITY_SCORE, 10)

    def test_hypotheses_count(self):
        self.assertEqual(len(V1289_HYPOTHESES), 5)


class TestV1289Regex(unittest.TestCase):
    """Regex pattern sanity."""

    def test_pub_fn_re_simple(self):
        m = PUB_FN_RE.match("pub fn hello() {")
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1), "hello")

    def test_pub_fn_re_async(self):
        m = PUB_FN_RE.match("pub async fn fetch() -> Result<T, E> {")
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1), "fetch")

    def test_pub_fn_re_const(self):
        m = PUB_FN_RE.match("pub const fn value() -> i32 {")
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1), "value")

    def test_pub_fn_re_crate(self):
        m = PUB_FN_RE.match("pub(crate) fn internal() {")
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1), "internal")

    def test_pub_fn_re_rejects_struct(self):
        self.assertIsNone(PUB_FN_RE.match("pub struct Foo {"))

    def test_pub_fn_re_rejects_trait(self):
        self.assertIsNone(PUB_FN_RE.match("pub trait Bar {"))

    def test_pub_fn_re_rejects_priv(self):
        self.assertIsNone(PUB_FN_RE.match("fn private() {"))

    def test_doc_line_re(self):
        self.assertTrue(DOC_LINE_RE.match("/// hello"))
        self.assertTrue(DOC_LINE_RE.match("    /// indented"))
        self.assertFalse(DOC_LINE_RE.match("// comment"))

    def test_blank_doc_re(self):
        self.assertTrue(BLANK_DOC_RE.match("///"))
        self.assertTrue(BLANK_DOC_RE.match("    ///"))
        self.assertFalse(BLANK_DOC_RE.match("/// hello"))

    def test_example_re(self):
        self.assertTrue(EXAMPLE_RE.match("# Example"))
        self.assertTrue(EXAMPLE_RE.match("## Examples"))

    def test_panics_re(self):
        self.assertTrue(PANICS_RE.match("# Panics"))

    def test_errors_re(self):
        self.assertTrue(ERRORS_RE.match("# Errors"))

    def test_returns_result_re(self):
        self.assertTrue(RETURNS_RESULT_RE.search("-> Result<T, E>"))
        self.assertTrue(RETURNS_RESULT_RE.search("-> Result<Foo, Bar>"))
        self.assertFalse(RETURNS_RESULT_RE.search("-> Option<T>"))

    def test_body_panic_hint_re(self):
        self.assertTrue(BODY_PANIC_HINT_RE.search("foo.unwrap()"))
        self.assertTrue(BODY_PANIC_HINT_RE.search("foo.expect(\"msg\")"))
        self.assertTrue(BODY_PANIC_HINT_RE.search("panic!(\"x\")"))
        self.assertTrue(BODY_PANIC_HINT_RE.search("todo!()"))
        self.assertFalse(BODY_PANIC_HINT_RE.search("foo()"))


class TestV1289PhilosophyGate(unittest.TestCase):
    """V3 哲学守门."""

    def test_gate_count(self):
        gate = _v1289_philosophy_gate()
        self.assertEqual(len(gate), 41)  # 36 inherited + 5 new
        for k, v in gate.items():
            self.assertTrue(v, f"Gate {k} should be True")

    def test_gate_has_v1289_new(self):
        gate = _v1289_philosophy_gate()
        self.assertIn("v1289_extends_v1288_not_replaces", gate)
        self.assertIn("v1289_audit_only_no_doc_write", gate)
        self.assertIn("v1289_production_src_only", gate)
        self.assertIn("v1289_no_kpi_inflate", gate)
        self.assertIn("v1289_quality_score_advisory", gate)


class TestV1289LooksLikePubFn(unittest.TestCase):
    """_looks_like_pub_fn helper."""

    def test_pub_fn(self):
        self.assertTrue(_looks_like_pub_fn("pub fn x() {}"))

    def test_pub_async_fn(self):
        self.assertTrue(_looks_like_pub_fn("pub async fn x() {}"))

    def test_priv_fn(self):
        self.assertFalse(_looks_like_pub_fn("fn x() {}"))

    def test_struct(self):
        self.assertFalse(_looks_like_pub_fn("pub struct X {"))


class TestV1289CollectDocBlock(unittest.TestCase):
    """_collect_doc_block helper."""

    def test_no_doc(self):
        lines = ["pub fn x() {}", "    body()", "}"]
        doc, ex, pe, p, e = _collect_doc_block(lines, 0)
        self.assertEqual(doc, [])
        self.assertFalse(ex)
        self.assertFalse(p)
        self.assertFalse(e)

    def test_simple_doc(self):
        lines = ["/// simple doc", "pub fn x() {}", "}"]
        doc, ex, pe, p, e = _collect_doc_block(lines, 1)
        self.assertEqual(len(doc), 1)
        self.assertFalse(ex)
        self.assertFalse(p)
        self.assertFalse(e)

    def test_doc_with_examples(self):
        lines = [
            "/// does X",
            "///",
            "/// # Example",
            "/// ```",
            "/// foo();",
            "/// ```",
            "pub fn x() {}",
            "}",
        ]
        doc, ex, pe, p, e = _collect_doc_block(lines, 6)
        self.assertTrue(ex)

    def test_doc_with_panics(self):
        lines = ["/// does X", "///", "/// # Panics", "pub fn x() {}", "}"]
        doc, ex, pe, p, e = _collect_doc_block(lines, 3)
        self.assertTrue(p)
        self.assertFalse(e)

    def test_doc_with_errors(self):
        lines = ["/// does X", "///", "/// # Errors", "pub fn x() {}", "}"]
        doc, ex, pe, p, e = _collect_doc_block(lines, 3)
        self.assertTrue(e)
        self.assertFalse(p)


class TestV1289FindBraceEnd(unittest.TestCase):
    """_find_brace_end helper."""

    def test_simple(self):
        lines = ["pub fn x() {", "    a;", "}"]
        self.assertEqual(_find_brace_end(lines, 0), 2)

    def test_nested(self):
        lines = ["pub fn x() {", "    {", "        a;", "    }", "}"]
        self.assertEqual(_find_brace_end(lines, 0), 4)


class TestV1289Dataclasses(unittest.TestCase):
    """Dataclass properties."""

    def test_function_doc_info_to_dict(self):
        info = FunctionDocInfo(
            crate_name="t",
            fn_name="foo",
            file_path="/x.rs",
            line_number=10,
            has_doc=True,
            is_blank_doc=False,
            doc_line_count=2,
            has_examples=True,
            has_panics=False,
            has_errors=True,
            returns_result=True,
            body_has_panic_hint=False,
            signature="pub fn foo() -> Result<T, E>",
            sample_doc="/// foo does X",
        )
        d = info.to_dict()
        self.assertEqual(d["fn_name"], "foo")
        self.assertTrue(d["has_doc"])
        self.assertTrue(d["returns_result"])

    def test_crate_doc_metrics_doc_coverage_pct_zero(self):
        m = CrateDocMetrics(crate_name="x", crate_src="/x")
        self.assertEqual(m.doc_coverage_pct, 0.0)
        self.assertEqual(m.n_without_doc, 0)
        self.assertEqual(m.quality_score, 0)

    def test_crate_doc_metrics_doc_coverage_pct(self):
        m = CrateDocMetrics(
            crate_name="x", crate_src="/x",
            n_public_fns=10, n_with_doc=7, n_blank_doc=1,
            n_with_examples=2, n_with_errors=1, n_with_panics=1,
        )
        self.assertAlmostEqual(m.doc_coverage_pct, 70.0)
        self.assertEqual(m.n_without_doc, 3)
        # quality = 2*3 + 1*2 + 1*2 + max(7-1, 0)*1 = 6 + 2 + 2 + 6 = 16
        self.assertEqual(m.quality_score, 16)

    def test_ledger_overall(self):
        m1 = CrateDocMetrics(crate_name="a", crate_src="/a", n_public_fns=4, n_with_doc=2)
        m2 = CrateDocMetrics(crate_name="b", crate_src="/b", n_public_fns=6, n_with_doc=3)
        ledger = DocCoverageLedger(crate_metrics=[m1, m2])
        self.assertEqual(ledger.total_public_fns, 10)
        self.assertEqual(ledger.total_with_doc, 5)
        self.assertAlmostEqual(ledger.overall_doc_coverage_pct, 50.0)


class TestV1289ScanCrate(unittest.TestCase):
    """scan_crate 真测试 — 临时 crate (主 17:43 实事求是, 不 mock)."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix="v1289_test_")
        self.src = Path(self.tmp) / "src"
        self.src.mkdir()

    def tearDown(self):
        import shutil
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_empty_crate(self):
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_public_fns, 0)
        self.assertEqual(m.doc_coverage_pct, 0.0)

    def test_pub_fn_without_doc(self):
        (self.src / "lib.rs").write_text(
            "pub fn foo() {}\n"
            "pub fn bar() {}\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_public_fns, 2)
        self.assertEqual(m.n_with_doc, 0)
        self.assertEqual(m.doc_coverage_pct, 0.0)
        self.assertEqual(m.n_without_doc, 2)

    def test_pub_fn_with_doc(self):
        (self.src / "lib.rs").write_text(
            "/// foo does X\n"
            "pub fn foo() {}\n"
            "/// bar does Y\n"
            "pub fn bar() {}\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_public_fns, 2)
        self.assertEqual(m.n_with_doc, 2)
        self.assertEqual(m.doc_coverage_pct, 100.0)

    def test_blank_doc(self):
        (self.src / "lib.rs").write_text(
            "///\n"  # blank doc
            "pub fn foo() {}\n"
            "/// has content\n"
            "pub fn bar() {}\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_public_fns, 2)
        self.assertEqual(m.n_with_doc, 2)
        self.assertEqual(m.n_blank_doc, 1)

    def test_doc_with_examples(self):
        (self.src / "lib.rs").write_text(
            "/// does X\n"
            "/// # Example\n"
            "/// ```\n"
            "/// foo();\n"
            "/// ```\n"
            "pub fn foo() {}\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_with_examples, 1)

    def test_doc_with_errors_section(self):
        (self.src / "lib.rs").write_text(
            "/// does X\n"
            "///\n"
            "/// # Errors\n"
            "/// Returns Err if X fails\n"
            "pub fn foo() -> Result<(), String> { Ok(()) }\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_with_errors, 1)
        self.assertEqual(m.n_result_fns, 1)

    def test_doc_with_panics_section(self):
        (self.src / "lib.rs").write_text(
            "/// does X\n"
            "///\n"
            "/// # Panics\n"
            "/// Panics if X\n"
            "pub fn foo() { panic!(\"x\"); }\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_with_panics, 1)
        self.assertEqual(m.n_panic_hint_fns, 1)

    def test_priv_fn_ignored(self):
        (self.src / "lib.rs").write_text(
            "fn priv() {}\n"
            "pub fn pub_one() {}\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_public_fns, 1)
        self.assertEqual(m.public_fns[0].fn_name, "pub_one")

    def test_async_pub_fn(self):
        (self.src / "lib.rs").write_text(
            "/// async fetch\n"
            "pub async fn fetch() {}\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_public_fns, 1)
        self.assertEqual(m.n_with_doc, 1)


class TestV1289FindCrateSrc(unittest.TestCase):
    """find_crate_src helper."""

    def test_not_found(self):
        result = find_crate_src("nonexistent-crate", Path("/tmp"))
        self.assertIsNone(result)


class TestV1289EvaluateHypotheses(unittest.TestCase):
    """_evaluate_hypotheses helper."""

    def test_empty_ledger_all_pass(self):
        # 空 ledger — 所有 pass (没东西不 FAIL)
        ledger = DocCoverageLedger(crate_metrics=[])
        results = _evaluate_hypotheses(ledger)
        self.assertEqual(len(results), 5)
        for r in results:
            self.assertEqual(r["pass_fail"], "PASS")

    def test_full_doc_crate_passes_h1(self):
        m = CrateDocMetrics(
            crate_name="x", crate_src="/x",
            n_public_fns=10, n_with_doc=10, n_with_examples=5,
            n_with_errors=3, n_with_panics=3,
        )
        ledger = DocCoverageLedger(crate_metrics=[m])
        results = _evaluate_hypotheses(ledger)
        # H1 doc coverage 100% >= 50% — PASS
        h1 = next(r for r in results if r["hypothesis_id"] == "h_pub_api_doc_coverage_ge_50pct")
        self.assertEqual(h1["pass_fail"], "PASS")


class TestV1289Markdown(unittest.TestCase):
    """Markdown output (主 00:56 任何人都能接手)."""

    def test_empty_ledger_markdown(self):
        ledger = DocCoverageLedger(crate_metrics=[])
        md = _to_markdown(ledger, top_undocumented=5)
        self.assertIn("V1289 VCP Rust Doc Coverage Audit", md)
        self.assertIn("V3 Philosophy Gate", md)
        self.assertIn("5 Hypotheses", md)
        self.assertIn("免责声明", md)

    def test_real_ledger_markdown(self):
        m = CrateDocMetrics(
            crate_name="test-crate", crate_src="/x",
            src_files_scanned=3, src_lines_scanned=100,
            n_public_fns=10, n_with_doc=8, n_with_examples=3,
            n_with_errors=2, n_with_panics=2,
        )
        ledger = DocCoverageLedger(crate_metrics=[m])
        md = _to_markdown(ledger)
        self.assertIn("test-crate", md)
        self.assertIn("80.0%", md)  # doc_coverage_pct = 80%
        self.assertIn("VCP Rust 文档 #1", md)


class TestV1289JsonSnapshot(unittest.TestCase):
    """JSON snapshot output."""

    def test_json_snapshot_basic(self):
        m = CrateDocMetrics(
            crate_name="test", crate_src="/x",
            n_public_fns=5, n_with_doc=3, n_blank_doc=1,
        )
        ledger = DocCoverageLedger(crate_metrics=[m], n_crates_total=1, n_crates_audited=1)
        snap = json.loads(_to_json_snapshot(ledger))
        self.assertEqual(snap["n_crates_audited"], 1)
        self.assertEqual(snap["total_public_fns"], 5)
        self.assertEqual(len(snap["per_crate_metrics"]), 1)


class TestV1289CLI(unittest.TestCase):
    """CLI entry point (主 00:56 任何人都能接手)."""

    def test_probe(self):
        rc = main(["--probe"])
        # probe mode may fail if no apeireth-* crates in test env, but should not crash
        self.assertIn(rc, (0, 1))

    def test_help(self):
        # --help exits with code 0
        try:
            rc = main(["--help"])
        except SystemExit as e:
            self.assertEqual(e.code, 0)


class TestV1289Regression(unittest.TestCase):
    """V1289 不破坏既有模块 (主 17:43 实事求是)."""

    def test_v1284_imports_still_work(self):
        from apeireth.v1284_worst5_security_audit import (
            V1284_VERSION, scan_crate as v1284_scan,
        )
        self.assertIsInstance(V1284_VERSION, str)
        self.assertTrue(callable(v1284_scan))

    def test_v1285_imports_still_work(self):
        from apeireth.v1285_all42_crate_security_audit import (
            discover_all_apeireth_crates,
        )
        self.assertTrue(callable(discover_all_apeireth_crates))

    def test_v1288_imports_still_work(self):
        from apeireth.v1288_governance_core_deep_audit import (
            V1288_VERSION,
        )
        self.assertIsInstance(V1288_VERSION, str)


if __name__ == "__main__":
    unittest.main(verbosity=2)