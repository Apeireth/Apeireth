"""Tests for V1290 — VCP Rust Doc Section Depth Audit.

V1290 = 真生产全 42 crates public API doc section 深度审计 (主 17:43 实事求是).
测试覆盖: regex + section detection + scan_crate + find_crate_src + _evaluate_hypotheses
+ Markdown + JSON + CLI + Regression (V1284/V1285/V1288/V1289 imports 仍 OK).
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

# Path setup (主 00:56 任何人都能接手)
PROMETHEAN_ROOT = Path(__file__).resolve().parent.parent
if str(PROMETHEAN_ROOT) not in sys.path:
    sys.path.insert(0, str(PROMETHEAN_ROOT))

from apeireth.v1290_rust_doc_section_depth_audit import (  # noqa: E402
    APEIRETH_RUST_CRATE_NAMES,
    CrateDocDepthMetrics,
    DocSectionDepthLedger,
    FunctionDocDepth,
    V1290_SECTION_WEIGHTS,
    V1290_THRESHOLD_ARGS_PCT,
    V1290_THRESHOLD_AVG_SECTIONS,
    V1290_THRESHOLD_EXAMPLES_PCT,
    V1290_THRESHOLD_RETURNS_PCT,
    V1290_THRESHOLD_SAFETY_PCT,
    _collect_doc_block,
    _count_args_in_sig,
    _evaluate_hypotheses,
    _find_brace_end,
    _is_unsafe_pub_fn,
    _score_sections,
    find_crate_src,
    main,
    scan_crate,
    to_markdown,
    top_n_by_score,
)


class TestV1290Regex(unittest.TestCase):
    """Regex patterns for pub fn / sections / args / unsafe / Result."""

    def test_pub_fn_basic(self):
        from apeireth.v1290_rust_doc_section_depth_audit import PUB_FN_RE
        m = PUB_FN_RE.match("pub fn foo() {}")
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1), "foo")

    def test_pub_async_fn(self):
        from apeireth.v1290_rust_doc_section_depth_audit import PUB_FN_RE
        m = PUB_FN_RE.match("    pub async fn fetch(&self) {}")
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1), "fetch")

    def test_pub_const_fn(self):
        from apeireth.v1290_rust_doc_section_depth_audit import PUB_FN_RE
        m = PUB_FN_RE.match("pub const fn MAX() -> u32 { 100 }")
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1), "MAX")

    def test_pub_crate_fn(self):
        from apeireth.v1290_rust_doc_section_depth_audit import PUB_FN_RE
        m = PUB_FN_RE.match("pub(crate) fn bar() {}")
        self.assertIsNotNone(m)
        self.assertEqual(m.group(1), "bar")

    def test_priv_fn_ignored(self):
        from apeireth.v1290_rust_doc_section_depth_audit import PUB_FN_RE
        self.assertIsNone(PUB_FN_RE.match("fn priv() {}"))

    def test_examples_heading(self):
        from apeireth.v1290_rust_doc_section_depth_audit import EXAMPLES_HEADING_RE
        self.assertTrue(EXAMPLES_HEADING_RE.search("/// # Examples"))
        self.assertTrue(EXAMPLES_HEADING_RE.search("/// ## Example"))

    def test_panics_heading(self):
        from apeireth.v1290_rust_doc_section_depth_audit import PANICS_HEADING_RE
        self.assertTrue(PANICS_HEADING_RE.search("/// # Panics"))

    def test_errors_heading(self):
        from apeireth.v1290_rust_doc_section_depth_audit import ERRORS_HEADING_RE
        self.assertTrue(ERRORS_HEADING_RE.search("/// # Errors"))

    def test_safety_heading(self):
        from apeireth.v1290_rust_doc_section_depth_audit import SAFETY_HEADING_RE
        self.assertTrue(SAFETY_HEADING_RE.search("/// # Safety"))

    def test_returns_heading(self):
        from apeireth.v1290_rust_doc_section_depth_audit import RETURNS_HEADING_RE
        self.assertTrue(RETURNS_HEADING_RE.search("/// # Returns"))

    def test_args_heading(self):
        from apeireth.v1290_rust_doc_section_depth_audit import ARGS_HEADING_RE
        self.assertTrue(ARGS_HEADING_RE.search("/// # Arguments"))


class TestV1290SectionDetection(unittest.TestCase):
    """_collect_doc_block — section detection per doc block."""

    def test_no_doc(self):
        lines = ["pub fn foo() {}"]
        doc_lines, sections = _collect_doc_block(lines, 0)
        self.assertEqual(doc_lines, [])
        self.assertFalse(sections["has_examples"])

    def test_doc_with_examples(self):
        lines = [
            "/// does X",
            "///",
            "/// # Examples",
            "/// ```",
            "/// foo()",
            "/// ```",
            "pub fn foo() {}",
        ]
        doc_lines, sections = _collect_doc_block(lines, 6)
        self.assertEqual(len(doc_lines), 6)
        self.assertTrue(sections["has_examples"])

    def test_doc_with_all_sections(self):
        lines = [
            "/// does X",
            "///",
            "/// # Arguments",
            "/// # Returns",
            "/// # Examples",
            "/// # Panics",
            "/// # Errors",
            "/// # Safety",
            "pub unsafe fn foo() -> Result<(), E> { panic!(\"x\"); }",
        ]
        doc_lines, sections = _collect_doc_block(lines, 8)
        self.assertEqual(len(doc_lines), 8)
        for k in ("has_examples", "has_errors", "has_panics",
                  "has_safety", "has_returns", "has_args"):
            self.assertTrue(sections[k], f"{k} should be True")


class TestV1290ArgsCount(unittest.TestCase):
    """_count_args_in_sig — args counting (简化 regex)."""

    def test_zero_args(self):
        n = _count_args_in_sig("pub fn foo() {}")
        self.assertEqual(n, 0)

    def test_one_arg(self):
        n = _count_args_in_sig("pub fn foo(x: u32) {}")
        self.assertEqual(n, 1)

    def test_three_args(self):
        n = _count_args_in_sig("pub fn foo(a: u32, b: u32, c: u32) {}")
        self.assertEqual(n, 3)

    def test_self_excluded(self):
        n = _count_args_in_sig("pub fn foo(&self, x: u32) {}")
        self.assertEqual(n, 1)


class TestV1290IsUnsafe(unittest.TestCase):
    """_is_unsafe_pub_fn — unsafe detection."""

    def test_safe_pub_fn(self):
        self.assertFalse(_is_unsafe_pub_fn("pub fn foo() {}"))

    def test_unsafe_pub_fn(self):
        self.assertTrue(_is_unsafe_pub_fn("pub unsafe fn foo() {}"))

    def test_async_unsafe_pub_fn(self):
        self.assertTrue(_is_unsafe_pub_fn("pub async unsafe fn foo() {}"))


class TestV1290ScoreSections(unittest.TestCase):
    """_score_sections — section depth score (主 17:43 实事求是)."""

    def test_no_sections(self):
        score = _score_sections({
            "has_examples": False, "has_errors": False, "has_panics": False,
            "has_safety": False, "has_returns": False, "has_args": False,
        })
        self.assertEqual(score, 0)

    def test_only_examples(self):
        score = _score_sections({
            "has_examples": True, "has_errors": False, "has_panics": False,
            "has_safety": False, "has_returns": False, "has_args": False,
        })
        self.assertEqual(score, V1290_SECTION_WEIGHTS["examples"])

    def test_all_sections(self):
        score = _score_sections({
            "has_examples": True, "has_errors": True, "has_panics": True,
            "has_safety": True, "has_returns": True, "has_args": True,
        })
        expected = sum(V1290_SECTION_WEIGHTS.values())
        self.assertEqual(score, expected)


class TestV1290FindBraceEnd(unittest.TestCase):
    """_find_brace_end — body brace counting (复用 V1289)."""

    def test_single_line(self):
        lines = ["pub fn foo() { }"]
        self.assertEqual(_find_brace_end(lines, 0), 0)

    def test_multi_line(self):
        lines = [
            "pub fn foo() {",
            "    bar();",
            "}",
        ]
        self.assertEqual(_find_brace_end(lines, 0), 2)


class TestV1290ScanCrate(unittest.TestCase):
    """scan_crate — 真扫描单个 crate production src/."""

    def setUp(self):
        import tempfile
        self.tmpdir = tempfile.mkdtemp()
        self.src = Path(self.tmpdir) / "src"
        self.src.mkdir()

    def test_empty_crate(self):
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_public_fns, 0)
        self.assertEqual(m.n_with_doc, 0)

    def test_pub_fn_with_no_doc(self):
        (self.src / "lib.rs").write_text("pub fn foo() {}\n")
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_public_fns, 1)
        self.assertEqual(m.n_with_doc, 0)
        self.assertEqual(m.n_sections_total, 0)

    def test_pub_fn_with_doc_no_sections(self):
        (self.src / "lib.rs").write_text(
            "/// does X\n"
            "pub fn foo() {}\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_with_doc, 1)
        self.assertEqual(m.n_sections_total, 0)
        self.assertEqual(m.avg_sections_per_doc, 0.0)

    def test_pub_fn_with_examples_section(self):
        (self.src / "lib.rs").write_text(
            "/// does X\n"
            "///\n"
            "/// # Examples\n"
            "/// ```\n"
            "/// foo()\n"
            "/// ```\n"
            "pub fn foo() {}\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_with_examples, 1)
        self.assertEqual(m.examples_pct, 100.0)
        self.assertEqual(m.n_sections_total, 1)
        self.assertEqual(m.avg_sections_per_doc, 1.0)

    def test_pub_fn_with_all_sections(self):
        (self.src / "lib.rs").write_text(
            "/// does X\n"
            "///\n"
            "/// # Arguments\n"
            "/// - x: u32\n"
            "/// # Returns\n"
            "/// The result.\n"
            "/// # Examples\n"
            "/// # Panics\n"
            "/// # Errors\n"
            "/// # Safety\n"
            "pub unsafe fn foo(x: u32) -> Result<(), E> { panic!(\"x\"); }\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_public_fns, 1)
        self.assertEqual(m.n_with_doc, 1)
        self.assertEqual(m.n_with_examples, 1)
        self.assertEqual(m.n_with_errors, 1)
        self.assertEqual(m.n_with_panics, 1)
        self.assertEqual(m.n_with_safety, 1)
        self.assertEqual(m.n_with_returns, 1)
        self.assertEqual(m.n_with_args, 1)
        self.assertEqual(m.n_sections_total, 6)
        self.assertEqual(m.n_unsafe_fns, 1)
        self.assertEqual(m.n_unsafe_with_safety, 1)
        self.assertEqual(m.safety_pct_unsafe, 100.0)

    def test_result_fn(self):
        (self.src / "lib.rs").write_text(
            "/// does X\n"
            "pub fn foo() -> Result<(), E> { Ok(()) }\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_returns_value_fns, 1)
        self.assertEqual(m.n_returns_value_with_returns, 0)

    def test_returns_void_excluded(self):
        (self.src / "lib.rs").write_text(
            "/// does X\n"
            "pub fn foo() {}\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_returns_value_fns, 0)

    def test_multiarg_fn(self):
        (self.src / "lib.rs").write_text(
            "/// does X\n"
            "pub fn foo(a: u32, b: u32, c: u32) {}\n"
        )
        m = scan_crate("test_crate", self.src)
        self.assertEqual(m.n_multiarg_fns, 1)
        self.assertEqual(m.n_multiarg_with_args, 0)


class TestV1290FindCrateSrc(unittest.TestCase):
    """find_crate_src helper."""

    def test_not_found(self):
        result = find_crate_src("nonexistent-crate", Path("/tmp"))
        self.assertIsNone(result)


class TestV1290EvaluateHypotheses(unittest.TestCase):
    """_evaluate_hypotheses helper."""

    def test_empty_ledger_all_pass(self):
        ledger = DocSectionDepthLedger(crate_metrics=[])
        results = _evaluate_hypotheses(ledger)
        self.assertEqual(len(results), 5)
        for r in results:
            self.assertEqual(r["pass_fail"], "PASS")

    def test_high_depth_passes(self):
        m = CrateDocDepthMetrics(
            crate_name="x", crate_src="/x",
            n_public_fns=10, n_with_doc=10, n_sections_total=20,
            n_with_examples=8, n_with_errors=5, n_with_panics=3,
            n_with_safety=2, n_with_returns=6, n_with_args=4,
        )
        ledger = DocSectionDepthLedger(crate_metrics=[m])
        results = _evaluate_hypotheses(ledger)
        # H1: avg = 2.0 >= 1.5 → PASS
        h1 = next(r for r in results if r["hypothesis_id"] == "h_avg_sections_per_doc_ge_1p5")
        self.assertEqual(h1["pass_fail"], "PASS")
        # H2: examples 8/10 = 80% >= 10% → PASS
        h2 = next(r for r in results if r["hypothesis_id"] == "h_examples_pct_ge_10pct")
        self.assertEqual(h2["pass_fail"], "PASS")

    def test_zero_sections_fails_h1(self):
        m = CrateDocDepthMetrics(
            crate_name="x", crate_src="/x",
            n_public_fns=10, n_with_doc=10, n_sections_total=0,
        )
        ledger = DocSectionDepthLedger(crate_metrics=[m])
        results = _evaluate_hypotheses(ledger)
        h1 = next(r for r in results if r["hypothesis_id"] == "h_avg_sections_per_doc_ge_1p5")
        self.assertEqual(h1["pass_fail"], "FAIL")


class TestV1290TopN(unittest.TestCase):
    """top_n_by_score helper."""

    def test_top_3(self):
        m1 = CrateDocDepthMetrics(crate_name="a", crate_src="/a", section_depth_score=10)
        m2 = CrateDocDepthMetrics(crate_name="b", crate_src="/b", section_depth_score=30)
        m3 = CrateDocDepthMetrics(crate_name="c", crate_src="/c", section_depth_score=20)
        m4 = CrateDocDepthMetrics(crate_name="d", crate_src="/d", section_depth_score=5)
        ledger = DocSectionDepthLedger(crate_metrics=[m1, m2, m3, m4])
        top = top_n_by_score(ledger, 3)
        names = [m.crate_name for m in top]
        self.assertEqual(names, ["b", "c", "a"])

    def test_bottom_2(self):
        m1 = CrateDocDepthMetrics(crate_name="a", crate_src="/a", section_depth_score=10)
        m2 = CrateDocDepthMetrics(crate_name="b", crate_src="/b", section_depth_score=30)
        m3 = CrateDocDepthMetrics(crate_name="c", crate_src="/c", section_depth_score=20)
        m4 = CrateDocDepthMetrics(crate_name="d", crate_src="/d", section_depth_score=5)
        ledger = DocSectionDepthLedger(crate_metrics=[m1, m2, m3, m4])
        bottom = top_n_by_score(ledger, 2, reverse=True)
        names = [m.crate_name for m in bottom]
        self.assertEqual(names, ["d", "a"])


class TestV1290Markdown(unittest.TestCase):
    """Markdown output (主 00:56 任何人都能接手)."""

    def test_empty_ledger_markdown(self):
        ledger = DocSectionDepthLedger(crate_metrics=[])
        results = _evaluate_hypotheses(ledger)
        md = to_markdown(ledger, results)
        self.assertIn("# V1290", md)
        self.assertIn("Total crates: 0", md)

    def test_real_ledger_markdown(self):
        m = CrateDocDepthMetrics(
            crate_name="apeireth-x", crate_src="/x",
            n_public_fns=10, n_with_doc=10, n_sections_total=20,
            n_with_examples=8, n_with_errors=5, n_with_panics=3,
            section_depth_score=100,
        )
        ledger = DocSectionDepthLedger(crate_metrics=[m])
        results = _evaluate_hypotheses(ledger)
        md = to_markdown(ledger, results)
        self.assertIn("apeireth-x", md)
        self.assertIn("Bottom-5", md)


class TestV1290JsonSnapshot(unittest.TestCase):
    """JSON snapshot."""

    def test_json_snapshot_basic(self):
        m = CrateDocDepthMetrics(
            crate_name="x", crate_src="/x", n_public_fns=5, n_with_doc=3,
        )
        ledger = DocSectionDepthLedger(crate_metrics=[m])
        snap = ledger.to_dict()
        self.assertEqual(snap["total_crates"], 1)
        self.assertEqual(snap["total_public_fns"], 5)
        self.assertEqual(snap["total_with_doc"], 3)


class TestV1290CLI(unittest.TestCase):
    """CLI entry (主 00:56 任何人都能接手)."""

    def test_help(self):
        from io import StringIO
        from unittest.mock import patch
        with patch("sys.stdout", new_callable=StringIO):
            with self.assertRaises(SystemExit) as cm:
                main(["--help"])
            self.assertEqual(cm.exception.code, 0)

    def test_probe(self):
        rc = main(["--probe"])
        self.assertEqual(rc, 0)


class TestV1290CrateList(unittest.TestCase):
    """42 crates list integrity."""

    def test_42_crates_listed(self):
        self.assertEqual(len(APEIRETH_RUST_CRATE_NAMES), 42)

    def test_no_duplicates(self):
        self.assertEqual(len(APEIRETH_RUST_CRATE_NAMES), len(set(APEIRETH_RUST_CRATE_NAMES)))

    def test_all_start_with_apeireth(self):
        for name in APEIRETH_RUST_CRATE_NAMES:
            self.assertTrue(name.startswith("apeireth-"), f"{name} should start with apeireth-")


class TestV1290Thresholds(unittest.TestCase):
    """Threshold constants sanity."""

    def test_thresholds_positive(self):
        self.assertGreater(V1290_THRESHOLD_AVG_SECTIONS, 0)
        self.assertGreater(V1290_THRESHOLD_EXAMPLES_PCT, 0)
        self.assertGreater(V1290_THRESHOLD_RETURNS_PCT, 0)
        self.assertGreater(V1290_THRESHOLD_SAFETY_PCT, 0)
        self.assertGreater(V1290_THRESHOLD_ARGS_PCT, 0)

    def test_section_weights_positive(self):
        for k, v in V1290_SECTION_WEIGHTS.items():
            self.assertGreater(v, 0, f"{k} should have positive weight")


class TestV1290Regression(unittest.TestCase):
    """Regression — V1284/V1285/V1288/V1289 imports 仍 OK (主 19:33 走在前人肩上)."""

    def test_v1284_imports_still_work(self):
        from apeireth import v1284_worst5_security_audit
        self.assertTrue(hasattr(v1284_worst5_security_audit, "scan_crate"))

    def test_v1285_imports_still_work(self):
        from apeireth import v1285_all42_crate_security_audit
        self.assertTrue(hasattr(v1285_all42_crate_security_audit, "scan_crate"))

    def test_v1288_imports_still_work(self):
        from apeireth import v1288_governance_core_deep_audit
        self.assertTrue(hasattr(v1288_governance_core_deep_audit, "run_governance_audit"))

    def test_v1289_imports_still_work(self):
        from apeireth import v1289_rust_doc_coverage_audit
        self.assertTrue(hasattr(v1289_rust_doc_coverage_audit, "scan_crate"))


if __name__ == "__main__":
    unittest.main()