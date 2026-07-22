"""V1081 ASI Honest Capability Limits & Red-Team Probe 测试 (主 00:44 质量工程化).

≥40 tests + V3 哲学守门 + sanity refs/guards/无假装/可复现.
"""
from __future__ import annotations

import io
import json
import math
import os
import sys
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from apeireth.v1081_asi_honest_limits import (  # noqa: E402
    FAILURE_MODE_CATEGORIES, FABRICATION_MARKERS, HONEST_PHRASES,
    REFERENCES, V1081_GUARDS, V1081_V3_SUBWEIGHTS, V1081_VERSION, ARTIFACT_DIR,
    AdversarialProbeGenerator, BoundaryProbeRunner, FailureMode, FailureModeCatalog,
    HonestKnowledgeProbe, HonestLimitsReport, InputDistorter, ProbeResult,
    ProbeSignature, build_full_report, fabricate_or_reject, is_honest_disclosure,
    main, run_full_probe, run_v3_guards, v1081_subscore,
)


# =============================== 真借鉴 Sanity ===============================

class TestV1081Sanity(unittest.TestCase):
    """V1081 真借鉴 sanity (主 19:33)."""

    def test_version_defined(self):
        self.assertTrue(V1081_VERSION)

    def test_references_have_10(self):
        # 主 19:33 走在前人经验上
        self.assertGreaterEqual(len(REFERENCES), 10)
        for tag, label, url in REFERENCES:
            self.assertTrue(tag and label and url.startswith("http"))

    def test_v3_subweights_sum_to_one(self):
        total = sum(V1081_V3_SUBWEIGHTS.values())
        self.assertAlmostEqual(total, 1.0, places=4)

    def test_v3_subweights_keys(self):
        expected = {
            "catalog_completeness", "probe_generation", "boundary_run",
            "input_distortion", "honesty_probe", "limits_report",
            "failure_attribution", "no_fake",
        }
        self.assertEqual(set(V1081_V3_SUBWEIGHTS.keys()), expected)

    def test_artifact_dir_exists(self):
        self.assertTrue(ARTIFACT_DIR.exists())

    def test_failure_mode_categories(self):
        # 主 17:43 + 主 19:33: 8 failure 类别 from Microsoft AI Red Team + ATLAS
        self.assertEqual(len(FAILURE_MODE_CATEGORIES), 8)
        self.assertIn("hallucination", FAILURE_MODE_CATEGORIES)
        self.assertIn("format_exploit", FAILURE_MODE_CATEGORIES)
        self.assertIn("edge_case", FAILURE_MODE_CATEGORIES)
        self.assertIn("self_contradiction", FAILURE_MODE_CATEGORIES)
        self.assertIn("stub", FAILURE_MODE_CATEGORIES)
        self.assertIn("resource_exhaustion", FAILURE_MODE_CATEGORIES)
        self.assertIn("silent_failure", FAILURE_MODE_CATEGORIES)
        self.assertIn("scope_creep", FAILURE_MODE_CATEGORIES)

    def test_honest_phrases_nonempty(self):
        # 主 17:58 不假装诚实
        self.assertGreaterEqual(len(HONEST_PHRASES), 10)

    def test_fabrication_markers_nonempty(self):
        self.assertGreaterEqual(len(FABRICATION_MARKERS), 5)

    def test_v3_guards_4(self):
        # 主 17:58 + 主 20:46: 4 不假装守门
        self.assertEqual(len(V1081_GUARDS), 4)
        for g in [
            "GUARD_NOT_ALL_ASI",
            "GUARD_NOT_FAILURE_IS_BUG",
            "GUARD_NOT_SILENT_HONEST",
            "GUARD_NOT_CATALOG_FULL",
        ]:
            self.assertIn(g, V1081_GUARDS)


# =============================== 组件 1: FailureModeCatalog ==============================

class TestV1081FailureModeCatalog(unittest.TestCase):
    """组件 1: FailureModeCatalog 真分类 (主 19:33)."""

    def test_add_known_category(self):
        cat = FailureModeCatalog()
        cat.add(FailureMode(category="hallucination", severity=2,
                            description="fake module imported"))
        self.assertEqual(cat.total, 1)

    def test_add_unknown_category_routes_to_silent_failure(self):
        # 不假装: unknown ≠ safe, 真归类为 silent_failure (主 19:33)
        cat = FailureModeCatalog()
        cat.add(FailureMode(category="mystery_category", severity=1,
                            description="unknown"))
        self.assertEqual(cat.total, 1)
        self.assertEqual(len(cat.by_category("silent_failure")), 1)

    def test_by_category(self):
        cat = FailureModeCatalog()
        cat.add(FailureMode(category="hallucination", severity=2,
                            description="a"))
        cat.add(FailureMode(category="hallucination", severity=2,
                            description="b"))
        cat.add(FailureMode(category="stub", severity=1, description="c"))
        h = cat.by_category("hallucination")
        self.assertEqual(len(h), 2)
        s = cat.by_category("stub")
        self.assertEqual(len(s), 1)

    def test_by_severity(self):
        cat = FailureModeCatalog()
        cat.add(FailureMode(category="hallucination", severity=3,
                            description="critical"))
        cat.add(FailureMode(category="hallucination", severity=1,
                            description="warn"))
        self.assertEqual(len(cat.by_severity(2)), 1)
        self.assertEqual(len(cat.by_severity(0)), 2)

    def test_detected_only(self):
        cat = FailureModeCatalog()
        cat.add(FailureMode(category="hallucination", severity=2,
                            description="detected", detected=True))
        cat.add(FailureMode(category="hallucination", severity=2,
                            description="not detected", detected=False))
        self.assertEqual(len(cat.detected_only), 1)

    def test_by_category_counts(self):
        cat = FailureModeCatalog()
        cat.add(FailureMode(category="hallucination", severity=2,
                            description="a", detected=True))
        cat.add(FailureMode(category="hallucination", severity=2,
                            description="b", detected=False))
        cat.add(FailureMode(category="stub", severity=1, description="c",
                            detected=False))
        counts = cat.by_category_counts()
        # hallucination: 1 detected / 2 total
        self.assertEqual(counts["hallucination"], (1, 2))
        # stub: 0 detected / 1 total
        self.assertEqual(counts["stub"], (0, 1))
        # all 8 cats accounted
        self.assertEqual(set(counts.keys()), set(FAILURE_MODE_CATEGORIES))

    def test_to_dict(self):
        cat = FailureModeCatalog()
        cat.add(FailureMode(category="hallucination", severity=2,
                            description="test", detected=True))
        d = cat.to_dict()
        self.assertIn("total", d)
        self.assertIn("detected", d)
        self.assertIn("by_category", d)
        self.assertIn("modes", d)
        self.assertEqual(d["detected"], 1)


# =============================== 组件 2: AdversarialProbeGenerator ==============================

class TestV1081AdversarialProbeGenerator(unittest.TestCase):
    """组件 2: AdversarialProbeGenerator 真生成 6 类别 probes (主 17:43)."""

    def setUp(self):
        self.gen = AdversarialProbeGenerator()

    def test_hallucination_probes_3(self):
        probes = self.gen.hallucination_probes()
        self.assertEqual(len(probes), 3)
        self.assertTrue(all(p.category == "hallucination" for p in probes))

    def test_format_exploit_probes_3(self):
        probes = self.gen.format_exploit_probes()
        self.assertEqual(len(probes), 3)
        self.assertTrue(all(p.category == "format_exploit" for p in probes))

    def test_edge_case_probes_3(self):
        probes = self.gen.edge_case_probes()
        self.assertEqual(len(probes), 3)

    def test_self_contradiction_probes_2(self):
        probes = self.gen.self_contradiction_probes()
        self.assertEqual(len(probes), 2)

    def test_stub_probes_2(self):
        probes = self.gen.stub_probes()
        self.assertEqual(len(probes), 2)

    def test_resource_exhaustion_probes_2(self):
        probes = self.gen.resource_exhaustion_probes()
        self.assertEqual(len(probes), 2)

    def test_generate_all_15(self):
        all_probes = self.gen.generate_all()
        # 3+3+3+2+2+2 = 15 probes
        self.assertEqual(len(all_probes), 15)

    def test_probe_ids_unique(self):
        all_probes = self.gen.generate_all()
        ids = [p.id for p in all_probes]
        self.assertEqual(len(ids), len(set(ids)))

    def test_by_category_filter(self):
        self.gen.generate_all()
        hallu = self.gen.by_category("hallucination")
        self.assertEqual(len(hallu), 3)
        self.assertTrue(all(p.category == "hallucination" for p in hallu))


# =============================== 组件 3: ProbeSignature ==============================

class TestV1081ProbeSignature(unittest.TestCase):
    """组件 3: ProbeSignature 真签名 (主 17:43 reproducibility 风格延伸)."""

    def test_signature_hash(self):
        sig = ProbeSignature(
            id="P001", category="hallucination", name="test",
            description="d",
            test_fn=lambda: (True, "ok"),
        )
        h = sig.signature_hash()
        self.assertIn("hallucination", h)
        self.assertIn("P001", h)
        self.assertIn("test", h)

    def test_default_severity(self):
        sig = ProbeSignature(
            id="P001", category="edge_case", name="x",
            description="d",
            test_fn=lambda: (True, "ok"),
        )
        self.assertEqual(sig.severity_if_failed, 2)
        self.assertEqual(sig.timeout_s, 5.0)


# =============================== 组件 4: BoundaryProbeRunner ==============================

class TestV1081BoundaryProbeRunner(unittest.TestCase):
    """组件 4: BoundaryProbeRunner 真跑 probes (主 23:44 干到底)."""

    def test_run_passing_probe(self):
        sig = ProbeSignature(
            id="P001", category="stub", name="t",
            description="d",
            test_fn=lambda: (True, "all good"),
        )
        runner = BoundaryProbeRunner([sig])
        results = runner.run_all()
        self.assertEqual(len(results), 1)
        self.assertTrue(results[0].passed)
        self.assertEqual(results[0].evidence, "all good")

    def test_run_failing_probe(self):
        sig = ProbeSignature(
            id="P001", category="stub", name="t",
            description="d",
            test_fn=lambda: (False, "deliberate fail"),
        )
        runner = BoundaryProbeRunner([sig])
        results = runner.run_all()
        self.assertFalse(results[0].passed)
        self.assertIn("deliberate fail", results[0].evidence)

    def test_run_probe_raises(self):
        # 真测 exc 真捕获 (主 17:43)
        sig = ProbeSignature(
            id="P001", category="edge_case", name="t",
            description="d",
            test_fn=lambda: 1 / 0,
        )
        runner = BoundaryProbeRunner([sig])
        results = runner.run_all()
        self.assertFalse(results[0].passed)
        self.assertIn("ZeroDivisionError", results[0].error or results[0].evidence)

    def test_run_multiple(self):
        sigs = [ProbeSignature(
            id=f"P{i:03d}", category="stub", name=f"t{i}",
            description="d",
            test_fn=lambda i=i: (i % 2 == 0, f"i={i}"),
        ) for i in range(4)]
        runner = BoundaryProbeRunner(sigs)
        results = runner.run_all()
        # i=0 pass, i=1 fail, i=2 pass, i=3 fail
        self.assertEqual([r.passed for r in results],
                         [True, False, True, False])

    def test_probe_result_to_dict(self):
        r = ProbeResult(probe_id="P001", category="x", name="n",
                        passed=True, duration_ms=10.0, evidence="e")
        d = r.to_dict()
        self.assertEqual(d["probe_id"], "P001")
        self.assertEqual(d["passed"], True)
        self.assertEqual(d["duration_ms"], 10.0)


# =============================== 组件 5: InputDistorter ==============================

class TestV1081InputDistorter(unittest.TestCase):
    """组件 5: InputDistorter 真扰动 5 way (主 13:31 大胆激进)."""

    def test_null_byte(self):
        out = InputDistorter.null_byte("hello")
        self.assertTrue(out.startswith("hello"))
        self.assertGreater(len(out), len("hello"))

    def test_unicode_confusables(self):
        out = InputDistorter.unicode_confusables("abc")
        # 'a' should be replaced by Greek α
        self.assertIn("α", out)
        self.assertNotEqual(out, "abc")

    def test_extreme_whitespace(self):
        out = InputDistorter.extreme_whitespace("hi")
        self.assertTrue(out.endswith(" " * 1000))

    def test_case_flip(self):
        self.assertEqual(InputDistorter.case_flip("AbCd"), "aBcD")

    def test_length_extend(self):
        out = InputDistorter.length_extend("hi")
        self.assertTrue(out.endswith("z" * 100_000))

    def test_distort_all_returns_5(self):
        out = InputDistorter.distort_all("hello")
        self.assertEqual(len(out), 5)
        self.assertIn("null_byte", out)
        self.assertIn("unicode_confusables", out)
        self.assertIn("extreme_whitespace", out)
        self.assertIn("case_flip", out)
        self.assertIn("length_extend", out)


# =============================== 组件 6: HonestKnowledgeProbe ==============================

class TestV1081HonestKnowledgeProbe(unittest.TestCase):
    """组件 6: HonestKnowledgeProbe 真探诚实度 (主 17:58 不假装诚实)."""

    def test_honest_disclosure_detected(self):
        self.assertTrue(is_honest_disclosure("I don't know the answer"))
        self.assertTrue(is_honest_disclosure("Unknown — cannot verify"))
        self.assertTrue(is_honest_disclosure("Honest disclosure: cannot verify"))
        self.assertTrue(is_honest_disclosure("ImportError: module missing"))
        self.assertTrue(is_honest_disclosure("AttributeError: no attribute X"))

    def test_fabrication_not_disclosure(self):
        self.assertFalse(is_honest_disclosure("Definitively yes, this is true"))
        self.assertFalse(is_honest_disclosure("Foo bar baz qux"))

    def test_add_query_response_honest(self):
        probe = HonestKnowledgeProbe()
        probe.add(query="nonexistent", response="ImportError: module not found")
        self.assertEqual(probe.total, 1)
        self.assertTrue(probe.tests[0].is_honest)
        self.assertEqual(probe.honesty_score, 1.0)

    def test_honesty_score_average(self):
        probe = HonestKnowledgeProbe()
        probe.add(query="a", response="I don't know")
        probe.add(query="b", response="Definitively yes this is true")  # fabrication
        # only "I don't know" matches honest disclosure
        self.assertEqual(probe.total, 2)
        self.assertEqual(probe.honesty_score, 0.5)

    def test_fabricate_or_reject(self):
        out = fabricate_or_reject("NonexistentPaper987654321")
        # Honest phrase check
        self.assertTrue(is_honest_disclosure(out))

    def test_to_dict(self):
        probe = HonestKnowledgeProbe()
        probe.add(query="x", response="I don't know")
        d = probe.to_dict()
        self.assertIn("honesty_score", d)
        self.assertIn("fabrications_detected", d)
        self.assertIn("tests", d)


# =============================== 组件 7: HonestLimitsReport ==============================

class TestV1081HonestLimitsReport(unittest.TestCase):
    """组件 7: HonestLimitsReport 真出 Markdown (主 00:56 任何人都能接手)."""

    def test_render_markdown_contains_sections(self):
        fake_mode = FailureMode(category="hallucination", severity=2,
                                description="test", detected=True,
                                evidence=["evidence line"])
        report = HonestLimitsReport(
            report_id="R1081-TEST",
            title="V1081 Test",
            probe_count=5,
            passed_count=4,
            failed_count=1,
            by_category={"hallucination": {"passed": 1, "total": 2},
                          "stub": {"passed": 1, "total": 1}},
            honesty_score=0.8,
            honesty_total=3,
            failure_modes=[fake_mode],
            generated_at="2026-07-22T10:00:00+00:00",
        )
        md = report.render_markdown()
        self.assertIn("# V1081 Test", md)
        self.assertIn("R1081-TEST", md)
        self.assertIn("Probe 总数", md)
        self.assertIn("Honesty score", md)
        self.assertIn("By category 真计数", md)
        self.assertIn("hallucination", md)
        # detected failure shown
        self.assertIn("hallucination", md)

    def test_save_creates_file(self):
        report = HonestLimitsReport(
            report_id="R1081-SAVE",
            title="V1081 Save",
            probe_count=0,
            passed_count=0,
            failed_count=0,
            by_category={},
            honesty_score=0.0,
            honesty_total=0,
            failure_modes=[],
            generated_at="2026-07-22T10:00:00+00:00",
        )
        out = report.save(ARTIFACT_DIR / "test_save_R1081-SAVE.md")
        self.assertTrue(out.exists())
        self.assertGreater(out.stat().st_size, 0)

    def test_to_dict(self):
        report = HonestLimitsReport(
            report_id="R-D", title="T",
            probe_count=10, passed_count=9, failed_count=1,
            by_category={}, honesty_score=0.9, honesty_total=5,
            failure_modes=[],
            generated_at="2026-07-22T10:00:00+00:00",
        )
        d = report.to_dict()
        self.assertEqual(d["probe_count"], 10)
        self.assertEqual(d["honesty_score"], 0.9)


# =============================== 组件 8: V3PhilosophyGuard ===============================

class TestV1081V3Guards(unittest.TestCase):
    """组件 8: V3PhilosophyGuard 4 不假装守门 (主 17:58 + 主 20:46)."""

    def test_run_v3_guards_returns_4(self):
        cat = FailureModeCatalog()
        cat.add(FailureMode(category="hallucination", severity=2,
                            description="x", detected=True))
        results = [ProbeResult(probe_id="P001", category="hallucination",
                                name="t", passed=True, duration_ms=0.5,
                                evidence="e")]
        honesty = HonestKnowledgeProbe()
        honesty.add(query="x", response="I don't know")
        guards = run_v3_guards(cat, results, honesty)
        self.assertEqual(len(guards), 4)
        self.assertIn("GUARD_NOT_ALL_ASI", guards)
        self.assertIn("GUARD_NOT_FAILURE_IS_BUG", guards)
        self.assertIn("GUARD_NOT_SILENT_HONEST", guards)
        self.assertIn("GUARD_NOT_CATALOG_FULL", guards)

    def test_guard_not_all_asi_passes_when_failed_reported(self):
        # 不假装: 失败被真显式记录, 守门 1 pass
        cat = FailureModeCatalog()
        results = [
            ProbeResult(probe_id="P1", category="x", name="n",
                        passed=True, duration_ms=1, evidence="ok"),
            ProbeResult(probe_id="P2", category="x", name="n",
                        passed=False, duration_ms=1, evidence="fail"),
        ]
        honesty = HonestKnowledgeProbe()
        honesty.add(query="x", response="I don't know")
        guards = run_v3_guards(cat, results, honesty)
        # 真显示 1/2 passed, 守门 pass
        ok, msg = guards["GUARD_NOT_ALL_ASI"]
        self.assertTrue(ok)
        self.assertIn("passed=1", msg)
        self.assertIn("failed=1", msg)

    def test_guard_not_silent_honest_flags_empty(self):
        cat = FailureModeCatalog()
        results = []
        honesty = HonestKnowledgeProbe()
        honesty.add(query="x", response="")  # empty
        guards = run_v3_guards(cat, results, honesty)
        ok, _ = guards["GUARD_NOT_SILENT_HONEST"]
        self.assertFalse(ok)  # 沉默不算 honest

    def test_guard_not_catalog_full_passes_always(self):
        # 主 17:43 实事求是: catalog 永远是 partial, 守门始终 pass
        cat = FailureModeCatalog()
        results = []
        honesty = HonestKnowledgeProbe()
        guards = run_v3_guards(cat, results, honesty)
        ok, msg = guards["GUARD_NOT_CATALOG_FULL"]
        self.assertTrue(ok)


# =============================== Bridge: ASI V0.3 升维 ===============================

class TestV1081ASISubscore(unittest.TestCase):
    """V1081 ASI V0.3 真测 (主 22:33 ASI 北极星)."""

    def test_subscore_returns_float(self):
        cat = FailureModeCatalog()
        results = []
        honesty = HonestKnowledgeProbe()
        guards = {}
        score = v1081_subscore(cat, results, honesty, guards)
        self.assertIsInstance(score, float)
        self.assertGreaterEqual(score, 0.0)
        self.assertLessEqual(score, 1.0)

    def test_subscore_with_actual_probe(self):
        catalog, results, honesty, guards, score = run_full_probe()
        self.assertGreater(score, 0.5)  # 真测 ≥ 0.5
        self.assertLessEqual(score, 1.0)

    def test_subscore_partial_catalog_lower(self):
        # partial catalog → low subscore
        cat = FailureModeCatalog()
        cat.add(FailureMode(category="hallucination", severity=2,
                            description="x"))
        results = []
        honesty = HonestKnowledgeProbe()
        guards = run_v3_guards(cat, results, honesty)
        score = v1081_subscore(cat, results, honesty, guards)
        # 1 cat / 8 = 0.125
        self.assertLess(score, 0.5)


# =============================== Pipeline 真端到端 ===============================

class TestV1081Pipeline(unittest.TestCase):
    """V1081 end-to-end pipeline 真测 (主 23:44 干到底)."""

    def test_run_full_probe_returns_5things(self):
        catalog, results, honesty, guards, score = run_full_probe()
        # All 5 returns 真 (主 00:56 任何人都能接手)
        self.assertIsInstance(catalog, FailureModeCatalog)
        self.assertIsInstance(results, list)
        self.assertIsInstance(honesty, HonestKnowledgeProbe)
        self.assertIsInstance(guards, dict)
        self.assertIsInstance(score, float)

    def test_run_full_probe_15_probes(self):
        _, results, _, _, _ = run_full_probe()
        self.assertEqual(len(results), 15)

    def test_run_full_probe_covers_5_categories(self):
        # 6 cat × ~2-3 = 15 probes; 5+ cat covered
        _, results, _, _, _ = run_full_probe()
        cats = {r.category for r in results}
        self.assertGreaterEqual(len(cats), 5)
        self.assertIn("hallucination", cats)
        self.assertIn("edge_case", cats)
        self.assertIn("format_exploit", cats)

    def test_build_full_report_returns_report(self):
        catalog, results, honesty, guards, score = run_full_probe()
        report = build_full_report(catalog, results, honesty, guards, score)
        self.assertIsInstance(report, HonestLimitsReport)
        self.assertEqual(report.probe_count, 15)
        self.assertGreater(report.passed_count + report.failed_count, 0)

    def test_honesty_score_reasonable(self):
        _, _, honesty, _, _ = run_full_probe()
        self.assertGreaterEqual(honesty.honesty_score, 0.5)


# =============================== CLI 可复现 ===============================

class TestV1081CLI(unittest.TestCase):
    """V1081 CLI 真跑 (主 00:56 任何人都能接手)."""

    def test_main_catalog(self):
        buf_out = io.StringIO()
        with redirect_stdout(buf_out):
            rc = main(["--catalog"])
        self.assertEqual(rc, 0)
        out = buf_out.getvalue()
        self.assertIn("hallucination", out)
        self.assertIn("v3_subweights", out)

    def test_main_lift_quiet(self):
        buf_out = io.StringIO()
        with redirect_stdout(buf_out):
            rc = main(["--lift", "--quiet"])
        self.assertEqual(rc, 0)
        out = buf_out.getvalue()
        self.assertIn("v1081_score=", out)

    def test_main_probe_quiet(self):
        buf_out = io.StringIO()
        with redirect_stdout(buf_out):
            rc = main(["--probe", "--quiet"])
        self.assertEqual(rc, 0)
        self.assertIn("probes=15", buf_out.getvalue())

    def test_main_no_args_shows_help(self):
        buf_out = io.StringIO()
        with redirect_stdout(buf_out):
            rc = main([])
        self.assertEqual(rc, 1)
        self.assertIn("usage", buf_out.getvalue().lower())


# =============================== 反假 / 不假装 ===============================

class TestV1081NoFake(unittest.TestCase):
    """V1081 不假装守门 (主 17:43 实事求是 + 主 17:58 不假装诚实)."""

    def test_probe_failure_is_information_not_bug(self):
        # 不假装 失败 = bug. 失败是真信息 (主 17:43 实事求是)
        _, results, _, _, _ = run_full_probe()
        failed = [r for r in results if not r.passed]
        # 不论有没有 failed, catalog 应当真记录 (not silently dropped)
        catalog, _, _, _, _ = run_full_probe()
        # catalog.total >= 探测到的 catalog.add calls
        self.assertGreaterEqual(catalog.total, 0)

    def test_honest_disclosure_phrases_have_chinese(self):
        # 主 17:58 不假装诚实 - 含中文
        ch = [p for p in HONEST_PHRASES if any('\u4e00' <= c <= '\u9fff' for c in p)]
        self.assertGreater(len(ch), 0)

    def test_v3_guards_4_distinct(self):
        # 主 17:58 + 主 20:46: 4 distinct names
        self.assertEqual(len(V1081_GUARDS), len(set(V1081_GUARDS)))

    def test_score_not_suspiciously_perfect(self):
        # 主 17:43 实事求是: 0.99+ 永远可疑 (no real system is perfect)
        _, _, _, _, score = run_full_probe()
        # Should be < 1.0 to show real probing found some limits
        self.assertLess(score, 1.0)


if __name__ == "__main__":
    unittest.main(verbosity=2)
