"""Cross-small-model CI — 真测 (R9-DevOps / R9-DEV-001).

主 17:43 实事求是: 真跑 HQB 4 维, fixture model 必须 PASS, ≥2 真模型 adapter 契约存在.
主 19:33 走在前人经验上: pytest 2008 parametrize + GitHub Actions matrix 模式.
"""
from __future__ import annotations

import unittest

import apeireth.cross_small_model_ci as csm
from apeireth.cross_small_model_ci import (
    DEFAULT_REGISTRY,
    DEFAULT_TASKS,
    FixtureAdapter,
    HQBHarness,
    HarnessResult,
    Llama31Adapter,
    Qwen35Adapter,
    HermesAdapter,
    Gemma4Adapter,
    ModelRegistry,
    run_ci,
    summarize,
)
from apeireth.cross_small_model_ci.harness import (
    measure_sc, measure_nr, measure_ev, measure_cdt,
)
from apeireth.cross_small_model_ci.models import _score_response
from apeireth.cross_small_model_ci.tasks import (
    TaskDomain, get_tasks_by_domain, nr_variants,
)
from apeireth.cross_small_model_ci.report import render_markdown, render_json


# ---------------------------------------------------------------------------
# Adapter 契约 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------
class TestAdapterContract(unittest.TestCase):
    def test_qwen35_adapter_metadata(self):
        a = Qwen35Adapter()
        self.assertEqual(a.name, "qwen-3.5-7b")
        self.assertEqual(a.family, "qwen")
        self.assertEqual(a.params_b, 7.0)
        # 主 17:58 不假装: 无 local_path → not available
        self.assertFalse(a.is_available())

    def test_llama31_adapter_metadata(self):
        a = Llama31Adapter()
        self.assertEqual(a.name, "llama-3.1-8b")
        self.assertEqual(a.family, "llama")
        self.assertEqual(a.params_b, 8.0)
        self.assertFalse(a.is_available())

    def test_hermes_adapter_metadata(self):
        a = HermesAdapter()
        self.assertEqual(a.family, "hermes")

    def test_gemma4_adapter_metadata(self):
        a = Gemma4Adapter()
        self.assertEqual(a.family, "gemma")
        self.assertEqual(a.params_b, 9.0)

    def test_fixture_adapter_always_available(self):
        a = FixtureAdapter()
        self.assertTrue(a.is_available())
        # 推断: deterministic answer (主 17:43 实事求是: 同一个 prompt 两次返回相同)
        r1 = a.infer("What is 2+2?")
        r2 = a.infer("What is 2+2?")
        self.assertTrue(r1.ok)
        self.assertEqual(r1.output, r2.output)
        self.assertGreater(len(r1.output), 0)


# ---------------------------------------------------------------------------
# Registry
# ---------------------------------------------------------------------------
class TestModelRegistry(unittest.TestCase):
    def test_default_registry_has_at_least_5_adapters(self):
        # W3 增强 (R9-DEV-002): 加了 text2vec-base-chinese 真 embedding adapter, 现 6 个
        names = DEFAULT_REGISTRY.names()
        self.assertGreaterEqual(len(names), 5)
        self.assertIn("qwen-3.5-7b", names)
        self.assertIn("llama-3.1-8b", names)
        self.assertIn("text2vec-base-chinese", names)
        self.assertIn("fixture-7b-v1", names)

    def test_default_registry_available_includes_fixture_and_text2vec(self):
        # W3 增强: text2vec-base-chinese 已缓存到 HF cache → available
        avail = [a.name for a in DEFAULT_REGISTRY.available()]
        self.assertIn("fixture-7b-v1", avail)
        self.assertIn("text2vec-base-chinese", avail)

    def test_by_name_lookup(self):
        a = DEFAULT_REGISTRY.by_name("fixture-7b-v1")
        self.assertIsNotNone(a)
        self.assertEqual(a.family, "fixture")

    def test_by_name_missing(self):
        self.assertIsNone(DEFAULT_REGISTRY.by_name("nonexistent"))


# ---------------------------------------------------------------------------
# Score function (主 17:43 实事求是: 确定性, 不假装)
# ---------------------------------------------------------------------------
class TestScoreResponse(unittest.TestCase):
    def test_empty_response(self):
        self.assertEqual(_score_response("anything", ""), 0.0)

    def test_relevant_response(self):
        s = _score_response("What is 2+2?", "The answer is 4")
        # 包含数字 + 关键词 (answer, 2) 应有合理分数
        self.assertGreater(s, 0.5)
        self.assertLessEqual(s, 1.0)

    def test_garbage_response(self):
        s = _score_response("Compute 17+25", "xyz")
        # 不含数字 / 关键词, 分数应偏低
        self.assertLess(s, 0.6)


# ---------------------------------------------------------------------------
# Tasks
# ---------------------------------------------------------------------------
class TestTasks(unittest.TestCase):
    def test_default_tasks_cover_4_domains(self):
        domains = {t.domain for t in DEFAULT_TASKS}
        self.assertEqual(domains, set(TaskDomain))

    def test_get_tasks_by_domain(self):
        for d in TaskDomain:
            tasks = get_tasks_by_domain(d)
            self.assertGreater(len(tasks), 0, f"no tasks for {d}")
            self.assertTrue(all(t.domain == d for t in tasks))

    def test_nr_variants_count(self):
        v = nr_variants("hello world")
        # 主 19:33: nr_variants 实际生成 6 个 (原 + 大写 + 小写 + 礼貌后缀 + 礼貌前缀 + 字符替换)
        self.assertEqual(len(v), 6)


# ---------------------------------------------------------------------------
# Harness 单测
# ---------------------------------------------------------------------------
class TestMeasureFunctions(unittest.TestCase):
    def test_measure_sc_deterministic(self):
        a = FixtureAdapter()
        tasks = DEFAULT_TASKS[:2]
        r = measure_sc(a, tasks, csm.SCConfig(n_trials=3, sample_n=2))
        # fixture 决定性 → SC 应高
        self.assertGreater(r["sc"], 0.8)

    def test_measure_nr_deterministic(self):
        a = FixtureAdapter()
        r = measure_nr(a, DEFAULT_TASKS[:1], csm.NRConfig(n_variants=5, sample_n=1))
        self.assertGreater(r["nr"], 0.5)

    def test_measure_cdt(self):
        a = FixtureAdapter()
        r = measure_cdt(a, csm.CDTConfig())
        self.assertIn("code", r["per_domain"])
        self.assertIn("math", r["per_domain"])
        self.assertIn("reasoning", r["per_domain"])
        self.assertIn("creative", r["per_domain"])


# ---------------------------------------------------------------------------
# Harness 整体 + CI runner (主 00:44 质量工程化)
# ---------------------------------------------------------------------------
class TestHQBHarness(unittest.TestCase):
    def test_harness_runs_on_fixture(self):
        h = HQBHarness()
        a = FixtureAdapter()
        r = h.run(a)
        self.assertTrue(r.available)
        self.assertTrue(r.passed, f"fixture should pass HQB, got sub={r.subscore}")
        self.assertGreater(r.subscore, 0.5)
        self.assertGreater(r.n_inferences, 0)
        self.assertGreater(r.sc, 0.0)
        self.assertGreater(r.nr, 0.0)
        self.assertGreater(r.ev, 0.0)
        self.assertGreater(r.cdt, 0.0)

    def test_harness_unavailable_adapter(self):
        a = Qwen35Adapter()  # 无 local_path
        h = HQBHarness()
        r = h.run(a)
        self.assertFalse(r.available)
        self.assertFalse(r.passed)
        self.assertIsNotNone(r.error)


class TestCIRunner(unittest.TestCase):
    def test_run_ci_returns_at_least_fixture(self):
        results = run_ci()
        # 默认 CI 至少跑 fixture (主 17:58 不假装: 必跑 1 个)
        self.assertGreaterEqual(len(results), 1)
        names = [r.model_name for r in results]
        self.assertIn("fixture-7b-v1", names)

    def test_run_ci_at_least_one_pass(self):
        """主 17:43 + 主 00:44 质量工程化: CI 至少 1 个 PASS."""
        results = run_ci()
        n_passed = sum(1 for r in results if r.passed)
        self.assertGreaterEqual(n_passed, 1,
                                f"need >=1 PASS, got {[(r.model_name, r.passed, r.error) for r in results]}")

    def test_summarize(self):
        results = run_ci()
        s = summarize(results)
        self.assertEqual(s["n_models"], len(results))
        self.assertEqual(s["n_passed"], sum(1 for r in results if r.passed))
        self.assertGreaterEqual(s["avg_subscore"], 0.0)


# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------
class TestReport(unittest.TestCase):
    def test_render_json(self):
        results = run_ci()
        js = render_json(results)
        self.assertIn("summary", js)
        self.assertIn("results", js)
        self.assertIn("fixture-7b-v1", js)

    def test_render_markdown(self):
        results = run_ci()
        md = render_markdown(results)
        self.assertIn("Summary", md)
        self.assertIn("SC", md)
        self.assertIn("NR", md)
        self.assertIn("EV", md)
        self.assertIn("CDT", md)
        self.assertIn("fixture-7b-v1", md)
        self.assertIn("跨域迁移", md)


if __name__ == "__main__":
    unittest.main()
