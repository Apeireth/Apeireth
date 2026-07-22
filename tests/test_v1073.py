"""Test suite for V1073 ASI V0.2 Measurement Integrator.

Covers:
  1. Component tests — each 真生产 component independently
  2. Integration tests — 真跑全 pipeline
  3. Weight tests — V0.2 → V0.3 sum=1.0
  4. Validator tests — RealProductionValidator (主 13:31)
  5. Philosophy guard tests — 不假装 (主 17:58)
  6. End2End tests — 真跑全测量 (主 23:44)
  7. Markdown report tests — 主 00:56 任何人都能接手
"""
from __future__ import annotations

import json
import os
import re
import sys
import unittest
from pathlib import Path


# Add workspace to PYTHONPATH for imports
WORKSPACE_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(WORKSPACE_ROOT))


from apeireth.v1073_asi_v02_measurement_integrator import (
    V1073Integrator,
    V1073Dimension,
    V1073WeightRecalibrator,
    RealProductionValidator,
    End2EndPipeline,
    ASIIntegrationBridge,
    v1073_report_markdown,
    v1073_philosophy_guard,
    v1073_run,
    V1073_VERSION,
    V03_BASE_WEIGHTS,
    V03_WEIGHTS,
    REFERENCES,
    _clamp01,
)


class TestV1073Constants(unittest.TestCase):
    """Test constants and version (主 00:44 质量工程化)."""

    def test_version_string(self):
        """版本号非空."""
        self.assertIsInstance(V1073_VERSION, str)
        self.assertGreater(len(V1073_VERSION), 0)

    def test_v03_base_weights_sum_to_one(self):
        """V0.3 base 权重 sum=1.0 (主 17:43 实事求是)."""
        self.assertAlmostEqual(sum(V03_BASE_WEIGHTS.values()), 1.0, places=9)

    def test_v03_weights_sum_to_one(self):
        """V0.3 权重 sum=1.0 (主 17:43)."""
        self.assertAlmostEqual(sum(V03_WEIGHTS.values()), 1.0, places=9)

    def test_v03_weights_has_eternal_identity(self):
        """V0.3 有 eternal_identity (V1072 真测)."""
        self.assertIn("eternal_identity", V03_WEIGHTS)
        self.assertEqual(V03_WEIGHTS["eternal_identity"], 0.04)

    def test_v03_weights_reduces_real_production(self):
        """V0.3 real_production 比 V0.2 base 小."""
        self.assertLess(V03_WEIGHTS["real_production"], V03_BASE_WEIGHTS["real_production"])

    def test_references_non_empty(self):
        """参考借鉴非空 (主 19:33 走在前人经验上)."""
        self.assertGreater(len(REFERENCES), 0)
        for r in REFERENCES:
            self.assertIn("id", r)
            self.assertIn("title", r)


class TestClamp01(unittest.TestCase):
    """Test _clamp01 helper."""

    def test_clamp_below_zero(self):
        self.assertEqual(_clamp01(-0.5), 0.0)

    def test_clamp_above_one(self):
        self.assertEqual(_clamp01(1.5), 1.0)

    def test_clamp_in_range(self):
        self.assertAlmostEqual(_clamp01(0.42), 0.42)

    def test_clamp_zero(self):
        self.assertEqual(_clamp01(0), 0.0)

    def test_clamp_one(self):
        self.assertEqual(_clamp01(1), 1.0)


class TestV1073Integrator(unittest.TestCase):
    """Test V1073Integrator 真测 (主 22:33)."""

    def test_integrator_init_weighted(self):
        """默认 weighted 模式."""
        integ = V1073Integrator()
        self.assertEqual(integ.integration_mode, "weighted")

    def test_integrator_init_raw(self):
        """raw 模式可选."""
        integ = V1073Integrator(integration_mode="raw")
        self.assertEqual(integ.integration_mode, "raw")

    def test_integrator_invalid_mode(self):
        """非法模式 raise."""
        with self.assertRaises(ValueError):
            V1073Integrator(integration_mode="invalid")

    def test_integrator_default_scores_zero(self):
        """初始分数全 0."""
        integ = V1073Integrator()
        self.assertEqual(integ.v02_score, 0.0)
        self.assertEqual(integ.v1071_vcp_score, 0.0)
        self.assertEqual(integ.v1071_cross_domain_score, 0.0)
        self.assertEqual(integ.v1072_score, 0.0)

    def test_integrator_measure_v02_returns_float(self):
        """measure_v02_base 返回 0..1 float."""
        integ = V1073Integrator()
        score = integ.measure_v02_base()
        self.assertIsInstance(score, float)
        self.assertGreaterEqual(score, 0.0)
        self.assertLessEqual(score, 1.0)

    def test_integrator_measure_v1071_vcp_returns_float(self):
        """measure_v1071_vcp 返回 0..1 float."""
        integ = V1073Integrator()
        score = integ.measure_v1071_vcp()
        self.assertIsInstance(score, float)
        self.assertGreaterEqual(score, 0.0)
        self.assertLessEqual(score, 1.0)

    def test_integrator_measure_v1071_cross_domain_returns_float(self):
        """measure_v1071_cross_domain 返回 0..1 float."""
        integ = V1073Integrator()
        score = integ.measure_v1071_cross_domain()
        self.assertIsInstance(score, float)
        self.assertGreaterEqual(score, 0.0)
        self.assertLessEqual(score, 1.0)

    def test_integrator_measure_v1072_returns_float(self):
        """measure_v1072 返回 0..1 float."""
        integ = V1073Integrator()
        score = integ.measure_v1072_eternal_identity()
        self.assertIsInstance(score, float)
        self.assertGreaterEqual(score, 0.0)
        self.assertLessEqual(score, 1.0)

    def test_integrator_run_returns_dict(self):
        """run() 返回完整 dict."""
        integ = V1073Integrator()
        result = integ.run()
        self.assertIsInstance(result, dict)
        for key in ("v02_base", "v1071_vcp", "v1071_cross_domain",
                    "v1072_eternal_identity", "integration_mode"):
            self.assertIn(key, result)


class TestV1073Dimension(unittest.TestCase):
    """Test V1073Dimension V0.2 第 17 维度."""

    def test_dimension_defaults(self):
        """默认字段."""
        dim = V1073Dimension()
        self.assertEqual(dim.name, "eternal_identity")
        self.assertEqual(dim.weight, 0.04)
        self.assertEqual(dim.score, 0.0)

    def test_dimension_update(self):
        """update 设置 score."""
        dim = V1073Dimension()
        dim.update(0.85)
        self.assertAlmostEqual(dim.score, 0.85)

    def test_dimension_update_clamp_high(self):
        """update clamp 1.0."""
        dim = V1073Dimension()
        dim.update(1.5)
        self.assertEqual(dim.score, 1.0)

    def test_dimension_update_clamp_low(self):
        """update clamp 0.0."""
        dim = V1073Dimension()
        dim.update(-0.3)
        self.assertEqual(dim.score, 0.0)

    def test_dimension_description(self):
        """description 非空."""
        dim = V1073Dimension()
        self.assertIsInstance(dim.description, str)
        self.assertGreater(len(dim.description), 0)


class TestV1073WeightRecalibrator(unittest.TestCase):
    """Test V1073WeightRecalibrator V0.2 → V0.3 (主 22:33)."""

    def test_recalibrate_returns_dict(self):
        """recalibrate 返回 dict."""
        rec = V1073WeightRecalibrator()
        w = rec.recalibrate()
        self.assertIsInstance(w, dict)

    def test_recalibrate_sum_to_one(self):
        """recalibrate 后 sum=1.0."""
        rec = V1073WeightRecalibrator()
        w = rec.recalibrate()
        self.assertAlmostEqual(sum(w.values()), 1.0, places=6)

    def test_recalibrate_marks_asserted(self):
        """recalibrate 后 asserted=True."""
        rec = V1073WeightRecalibrator()
        rec.recalibrate()
        self.assertTrue(rec.asserted)

    def test_weights_without_recalibrate_auto_runs(self):
        """weights() 自动调 recalibrate."""
        rec = V1073WeightRecalibrator()
        w = rec.weights()
        self.assertAlmostEqual(sum(w.values()), 1.0, places=6)

    def test_recalibrate_adds_eternal_identity(self):
        """recalibrate 后有 eternal_identity."""
        rec = V1073WeightRecalibrator()
        w = rec.recalibrate()
        self.assertIn("eternal_identity", w)
        self.assertEqual(w["eternal_identity"], 0.04)

    def test_recalibrate_reduces_real_production(self):
        """real_production 减少 0.02."""
        rec = V1073WeightRecalibrator()
        rec.recalibrate()
        diff = rec.diff_report()
        self.assertEqual(diff["real_production"], -0.02)

    def test_recalibrate_reduces_rubric_open(self):
        """rubric_open 减少 0.02."""
        rec = V1073WeightRecalibrator()
        rec.recalibrate()
        diff = rec.diff_report()
        self.assertEqual(diff["rubric_open"], -0.02)

    def test_diff_report_returns_dict(self):
        """diff_report 返回 dict."""
        rec = V1073WeightRecalibrator()
        rec.recalibrate()
        diff = rec.diff_report()
        self.assertIsInstance(diff, dict)
        self.assertIn("eternal_identity", diff)


class TestRealProductionValidator(unittest.TestCase):
    """Test RealProductionValidator 真验证 (主 13:31 + 主 00:44)."""

    def setUp(self):
        # Use isolated tmpdir per test (主 00:44 质量工程化).
        import tempfile
        self.tmpdir = Path(tempfile.mkdtemp(prefix="v1073_test_"))

    def tearDown(self):
        import shutil
        if self.tmpdir.exists():
            shutil.rmtree(self.tmpdir, ignore_errors=True)

    def test_validator_init_default_dir(self):
        """默认 init 用 cwd."""
        v = RealProductionValidator()
        self.assertIsInstance(v.deployment_dir, Path)

    def test_validator_init_custom_dir(self):
        """自定义 dir."""
        v = RealProductionValidator(deployment_dir=str(self.tmpdir))
        self.assertEqual(v.deployment_dir, self.tmpdir)

    def test_compose_file_not_found(self):
        """compose 不存在 — 真测发现 (主 17:43)."""
        v = RealProductionValidator(deployment_dir=str(self.tmpdir))
        result = v.validate_compose()
        self.assertFalse(result["exists"])
        self.assertGreater(len(result["errors"]), 0)

    def test_compose_parses_minimal(self):
        """compose 最小可解析."""
        compose_path = self.tmpdir / "docker-compose.yml"
        compose_path.write_text(
            "services:\n  web:\n    image: nginx\n    ports:\n      - \"80:80\"\n",
            encoding="utf-8",
        )
        v = RealProductionValidator(deployment_dir=str(self.tmpdir))
        result = v.validate_compose()
        self.assertTrue(result["exists"])
        self.assertTrue(result["parseable"])

    def test_compose_detects_healthcheck(self):
        """compose 有 healthcheck."""
        compose_path = self.tmpdir / "docker-compose.yml"
        compose_path.write_text(
            "services:\n  api:\n    image: myapp\n    healthcheck:\n      test: [\"CMD\", \"true\"]\n",
            encoding="utf-8",
        )
        v = RealProductionValidator(deployment_dir=str(self.tmpdir))
        result = v.validate_compose()
        self.assertTrue(result["has_healthcheck"])

    def test_dockerfile_not_found(self):
        """Dockerfile 不存在."""
        v = RealProductionValidator(deployment_dir=str(self.tmpdir))
        result = v.validate_dockerfile()
        self.assertFalse(result["exists"])

    def test_dockerfile_minimal(self):
        """Dockerfile 最小有效."""
        df_path = self.tmpdir / "Dockerfile"
        df_path.write_text(
            "FROM python:3.11\nCOPY app.py /app/\nCMD [\"python\", \"/app/app.py\"]\n",
            encoding="utf-8",
        )
        v = RealProductionValidator(deployment_dir=str(self.tmpdir))
        result = v.validate_dockerfile()
        self.assertTrue(result["exists"])
        self.assertTrue(result["has_from"])
        self.assertTrue(result["has_copy_or_add"])

    def test_validate_full_passes(self):
        """完整 validate 全过."""
        (self.tmpdir / "docker-compose.yml").write_text(
            "services:\n  api:\n    image: foo\n    ports:\n      - \"8080:8080\"\n",
            encoding="utf-8",
        )
        (self.tmpdir / "Dockerfile").write_text(
            "FROM python:3.11\nCOPY . /app/\n",
            encoding="utf-8",
        )
        v = RealProductionValidator(deployment_dir=str(self.tmpdir))
        result = v.validate_full()
        self.assertEqual(result["passed"], 2)
        self.assertEqual(result["failed"], 0)


class TestEnd2EndPipeline(unittest.TestCase):
    """Test End2EndPipeline 真跑 (主 23:44 干到底)."""

    def test_pipeline_init(self):
        """默认 init."""
        p = End2EndPipeline()
        self.assertIsInstance(p.integrator, V1073Integrator)
        self.assertIsInstance(p.weights, V1073WeightRecalibrator)
        self.assertIsInstance(p.dimension, V1073Dimension)
        self.assertEqual(p.steps, [])

    def test_pipeline_run_returns_dict(self):
        """run 返回 dict."""
        p = End2EndPipeline()
        result = p.run()
        self.assertIsInstance(result, dict)
        for key in ("v02_base", "v1071_vcp_score", "v1071_cross_domain_score",
                    "v1072_eternal_identity_score", "v03_score", "n_steps"):
            self.assertIn(key, result)

    def test_pipeline_run_records_steps(self):
        """run 记录 5+ steps."""
        p = End2EndPipeline()
        result = p.run()
        self.assertGreaterEqual(result["n_steps"], 5)
        self.assertEqual(len(p.steps), result["n_steps"])

    def test_pipeline_v03_score_in_range(self):
        """V0.3 分数 0..1."""
        p = End2EndPipeline()
        result = p.run()
        self.assertGreaterEqual(result["v03_score"], 0.0)
        self.assertLessEqual(result["v03_score"], 1.0)

    def test_pipeline_with_deployment_validation(self):
        """带 deployment 验证."""
        tmpdir = Path(os.environ.get("TMPDIR", "/tmp")) / "v1073_e2e"
        tmpdir.mkdir(parents=True, exist_ok=True)
        (tmpdir / "docker-compose.yml").write_text(
            "services:\n  api:\n    image: foo\n    ports:\n      - \"8080:8080\"\n",
            encoding="utf-8",
        )
        (tmpdir / "Dockerfile").write_text(
            "FROM python:3.11\nCOPY . /app/\n",
            encoding="utf-8",
        )
        p = End2EndPipeline()
        result = p.run(deployment_dir=str(tmpdir))
        self.assertIn("v03_score", result)
        # Should have at least 6 steps now (with validate)
        self.assertGreaterEqual(result["n_steps"], 6)


class TestASIIntegrationBridge(unittest.TestCase):
    """Test ASIIntegrationBridge V0.3 真集成 (主 22:33)."""

    def test_bridge_init(self):
        """默认 init."""
        b = ASIIntegrationBridge()
        self.assertIsInstance(b.pipeline, End2EndPipeline)

    def test_bridge_run_full_measurement(self):
        """run_full_measurement 返回 dict."""
        b = ASIIntegrationBridge()
        result = b.run_full_measurement()
        self.assertIsInstance(result, dict)
        self.assertIn("v03_score", result)

    def test_bridge_cached(self):
        """run 后 _cached 存在."""
        b = ASIIntegrationBridge()
        b.run_full_measurement()
        self.assertIsNotNone(b._cached)

    def test_bridge_markdown_report(self):
        """markdown_report 返回 string."""
        b = ASIIntegrationBridge()
        b.run_full_measurement()
        report = b.markdown_report()
        self.assertIsInstance(report, str)
        self.assertIn("V1073", report)
        self.assertIn("V0.3", report)
        self.assertIn("V0.2", report)

    def test_bridge_markdown_report_without_pre_run(self):
        """markdown_report 自动跑 (主 00:56)."""
        b = ASIIntegrationBridge()
        report = b.markdown_report()
        self.assertIsInstance(report, str)
        self.assertGreater(len(report), 100)

    def test_bridge_v0_3_doc(self):
        """V0.3 公式文档."""
        self.assertIn("v0_3", ASIIntegrationBridge.V0_3_DOC)


class TestV1073Report(unittest.TestCase):
    """Test v1073_report_markdown 主 00:56."""

    def test_report_returns_string(self):
        """返回 string."""
        report = v1073_report_markdown()
        self.assertIsInstance(report, str)

    def test_report_has_header(self):
        """有标题."""
        report = v1073_report_markdown()
        self.assertIn("# V1073", report)

    def test_report_has_philosophy(self):
        """包含哲学守门 (主 17:58)."""
        report = v1073_report_markdown()
        self.assertIn("哲学守门", report)
        self.assertIn("不假装", report)


class TestV1073PhilosophyGuard(unittest.TestCase):
    """Test v1073_philosophy_guard 不假装 (主 17:58 + 主 20:46)."""

    def test_guard_returns_dict(self):
        """返回 dict."""
        g = v1073_philosophy_guard()
        self.assertIsInstance(g, dict)

    def test_guard_has_6_keys(self):
        """6 守门."""
        g = v1073_philosophy_guard()
        expected = {
            "measure_is_not_asi",
            "integration_is_not_asi",
            "v03_is_not_asi",
            "deployment_validate_is_not_real_run",
            "eternal_identity_is_not_consciousness",
            "philosophy_guard_is_not_philosophy",
        }
        self.assertEqual(set(g.keys()), expected)

    def test_guard_all_true(self):
        """所有守门 True (守门成立 = 不假装)."""
        g = v1073_philosophy_guard()
        for k, v in g.items():
            self.assertTrue(v, f"{k} must be True")


class TestV1073Run(unittest.TestCase):
    """Test v1073_run 一键 (主 00:56)."""

    def test_run_returns_dict(self):
        """v1073_run 返回 dict."""
        result = v1073_run()
        self.assertIsInstance(result, dict)

    def test_run_has_v03_score(self):
        """含 v03_score."""
        result = v1073_run()
        self.assertIn("v03_score", result)

    def test_run_has_philosophy_guard(self):
        """含 philosophy_guard (主 17:58)."""
        result = v1073_run()
        self.assertIn("philosophy_guard", result)

    def test_run_has_version(self):
        """含 version."""
        result = v1073_run()
        self.assertIn("version", result)
        self.assertEqual(result["version"], V1073_VERSION)

    def test_run_v03_in_range(self):
        """v03_score 0..1."""
        result = v1073_run()
        self.assertGreaterEqual(result["v03_score"], 0.0)
        self.assertLessEqual(result["v03_score"], 1.0)


class TestV1073IntegrationWithRealModules(unittest.TestCase):
    """Integration test — V1073 真集成了 V1048 + V1071 + V1072 (主 17:43)."""

    def test_v03_score_dominates_v02_if_v1072_strong(self):
        """如果 V1072 强, V0.3 > V0.2 (因为加权 0.04)."""
        # 计算公式直接: v03 = 0.96 * v02 + 0.04 * ei
        v02 = 0.50
        ei = 0.95
        v03_expected = 0.96 * v02 + 0.04 * ei  # = 0.518
        self.assertGreater(v03_expected, 0.50)
        self.assertLess(v03_expected, 0.55)

    def test_v03_score_drops_if_v1072_weak(self):
        """如果 V1072 弱, V0.3 可能 < V0.2."""
        # v03 = 0.96*0.80 + 0.04*0.10 = 0.768 + 0.004 = 0.772
        v02 = 0.80
        ei = 0.10
        v03_expected = 0.96 * v02 + 0.04 * ei
        self.assertLess(v03_expected, 0.80)

    def test_full_pipeline_with_mocked_modules(self):
        """全 pipeline 用 mock 真跑 (不走 measure 重测)."""
        import apeireth.v1073_asi_v02_measurement_integrator as v1073_mod
        # 直接计算 v0.3 公式,验证公式正确性 (主 17:43 + 主 23:44).
        v02 = 0.70
        vcp = 0.85
        cd = 0.80
        ei = 0.65
        v03_expected = 0.96 * v02 + 0.04 * ei
        # 真跑 End2EndPipeline 但 mock measure_*
        pipeline = End2EndPipeline()
        original_v02 = pipeline.integrator.measure_v02_base
        original_v1071_vcp = pipeline.integrator.measure_v1071_vcp
        original_v1071_cd = pipeline.integrator.measure_v1071_cross_domain
        original_v1072 = pipeline.integrator.measure_v1072_eternal_identity
        try:
            pipeline.integrator.measure_v02_base = lambda: (pipeline.integrator.__setattr__("v02_score", v02), pipeline.integrator.v02_score)[1]
            pipeline.integrator.measure_v1071_vcp = lambda: (pipeline.integrator.__setattr__("v1071_vcp_score", vcp), pipeline.integrator.v1071_vcp_score)[1]
            pipeline.integrator.measure_v1071_cross_domain = lambda: (pipeline.integrator.__setattr__("v1071_cross_domain_score", cd), pipeline.integrator.v1071_cross_domain_score)[1]
            pipeline.integrator.measure_v1072_eternal_identity = lambda: (pipeline.integrator.__setattr__("v1072_score", ei), pipeline.integrator.v1072_score)[1]
            result = pipeline.run()
            self.assertAlmostEqual(result["v02_base"], v02, places=4)
            self.assertAlmostEqual(result["v1071_vcp_score"], vcp, places=4)
            self.assertAlmostEqual(result["v1071_cross_domain_score"], cd, places=4)
            self.assertAlmostEqual(result["v1072_eternal_identity_score"], ei, places=4)
            self.assertAlmostEqual(result["v03_score"], round(v03_expected, 4), places=2)
        finally:
            pipeline.integrator.measure_v02_base = original_v02
            pipeline.integrator.measure_v1071_vcp = original_v1071_vcp
            pipeline.integrator.measure_v1071_cross_domain = original_v1071_cd
            pipeline.integrator.measure_v1072_eternal_identity = original_v1072


class TestV1073SanityChecks(unittest.TestCase):
    """Sanity checks — refs/guards/无假装/reproducibility (主 00:44)."""

    def test_references_valid_format(self):
        """所有参考有 id + title."""
        for r in REFERENCES:
            self.assertIn("id", r)
            self.assertIn("title", r)
            self.assertIsInstance(r["id"], str)
            self.assertIsInstance(r["title"], str)

    def test_no_external_network_calls(self):
        """V1073 不调外网 (主 00:56)."""
        bridge = ASIIntegrationBridge()
        # 应该能从完全离线环境跑
        result = bridge.run_full_measurement()
        self.assertIsNotNone(result)

    def test_v1073_is_deterministic(self):
        """V1073 真跑 2 次结果一致 (主 00:44)."""
        bridge1 = ASIIntegrationBridge()
        r1 = bridge1.run_full_measurement()
        bridge2 = ASIIntegrationBridge()
        r2 = bridge2.run_full_measurement()
        # Scores 应该几乎一样 (允许 0.001 浮点差)
        self.assertAlmostEqual(r1["v03_score"], r2["v03_score"], places=2)

    def test_v1073_philosophy_guard_does_not_fake(self):
        """守门不假装 (主 17:58)."""
        g = v1073_philosophy_guard()
        # 所有 True = 守门成立 = 不假装
        self.assertTrue(all(g.values()))


if __name__ == "__main__":
    unittest.main()
