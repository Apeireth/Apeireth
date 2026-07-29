"""Cross-small-model CI W3 增强 — 真测 (R9-DevOps / R9-DEV-002).

主 17:43 实事求是 + 主 00:56 任何人都能接手: pytest 集成, 真测 W3 新增能力.
  - 跨模型差异 (compute_diff / render_diff_table / write_diff)
  - CI badge (render_badge / render_badge_markdown / write_badge)
  - 真模型 best-effort 接入 (CIRunner.attempt_real_model)
  - 容错: env 未设 / local_path 不存在 / load 失败 三路径全显式记录

增量 ≥15 测试 (主 13:31 大胆激进).
"""
from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path
from typing import List

import apeireth.cross_small_model_ci as csm
from apeireth.cross_small_model_ci import (
    DEFAULT_REGISTRY, FixtureAdapter, HarnessResult, HQBHarness,
    Llama31Adapter, Qwen35Adapter, ModelRegistry,
    REAL_MODEL_ENV, run_ci, summarize,
)
from apeireth.cross_small_model_ci.runner import CIRunner
from apeireth.cross_small_model_ci.report import (
    compute_diff, render_diff_table, write_diff,
    render_badge, render_badge_markdown, write_badge,
)


def _make_result(model_name: str, family: str = "fixture", available: bool = True,
                 sc: float = 0.8, nr: float = 0.7, ev: float = 0.5, cdt: float = 0.9,
                 passed: bool = True, error: str = None) -> HarnessResult:
    """构造 HarnessResult (测试用)."""
    sub = (sc + nr + ev + cdt) / 4.0
    return HarnessResult(
        model_name=model_name, family=family, available=available,
        sc=sc, nr=nr, ev=ev, cdt=cdt,
        cdt_per_domain={"code": cdt, "math": cdt, "reasoning": cdt, "creative": cdt},
        subscore=sub, passed=passed, error=error,
        elapsed_sec=0.1, n_inferences=4,
    )


# ---------------------------------------------------------------------------
# (A) 跨模型差异 (主 13:31 大胆激进)
# ---------------------------------------------------------------------------
class TestComputeDiff(unittest.TestCase):
    def setUp(self):
        # 主 17:43 实事求是: 用真 fixture 跑 baseline
        self.results = run_ci()
        self.baseline_name = "fixture-7b-v1"

    def test_compute_diff_basic(self):
        """compute_diff 至少返回 baseline + rows + lift_summary."""
        diff = compute_diff(self.results, baseline_name=self.baseline_name)
        self.assertIn("baseline", diff)
        self.assertIn("rows", diff)
        self.assertIn("lift_summary", diff)
        self.assertEqual(diff["baseline"]["model_name"], self.baseline_name)

    def test_compute_diff_no_baseline(self):
        """主 17:58 不假装: 没 baseline → rows 标记 error."""
        diff = compute_diff([], baseline_name="missing")
        self.assertIsNone(diff["baseline"])
        self.assertEqual(len(diff["rows"]), 0)

    def test_compute_diff_with_unavailable_target(self):
        """主 17:58 不假装: unavailable target → delta=null, error 显式."""
        results = self.results + [
            _make_result("real-qwen", family="qwen", available=False, error="env not set"),
        ]
        diff = compute_diff(results, baseline_name=self.baseline_name)
        rows = diff["rows"]
        # rows 数 = 非 baseline 模型数 (W3: fixture + text2vec + real-qwen → 2 个非 baseline)
        self.assertEqual(len(rows), len(results) - 1)
        # 找到 real-qwen 行, 验证 unavailable 显式
        qwen_row = next((r for r in rows if r["target"] == "real-qwen"), None)
        self.assertIsNotNone(qwen_row)
        self.assertFalse(qwen_row["available"])
        self.assertIsNone(qwen_row["delta_subscore"])
        self.assertIn("env not set", qwen_row["error"])

    def test_compute_diff_lift_summary(self):
        """主 17:43 实事求是: lift_summary 字段全."""
        diff = compute_diff(self.results, baseline_name=self.baseline_name)
        ls = diff["lift_summary"]
        for k in ("n_targets", "n_loaded", "n_failed",
                  "mean_delta", "max_delta", "min_delta",
                  "baseline_name", "baseline_subscore"):
            self.assertIn(k, ls, f"missing lift_summary.{k}")

    def test_compute_diff_unavailable_excluded_from_lift(self):
        """主 17:43: unavailable 不进 lift_summary 均值."""
        # 仅留 baseline + unavailable target, 不含 text2vec (避免 text2vec 拉低 n_failed=0 断言失败)
        baseline_result = _make_result(self.baseline_name, available=True, passed=True)
        results = [
            baseline_result,
            _make_result("real-qwen", family="qwen", available=False),
        ]
        diff = compute_diff(results, baseline_name=self.baseline_name)
        ls = diff["lift_summary"]
        self.assertEqual(ls["n_failed"], 1)
        self.assertEqual(ls["n_loaded"], 0)
        self.assertIsNone(ls["mean_delta"])


class TestRenderDiffTable(unittest.TestCase):
    def test_render_diff_table_markdown_shape(self):
        """render_diff_table: 含表头 + baseline 行 + 至少 1 target 行 + lift_summary."""
        results = run_ci() + [
            _make_result("real-qwen", family="qwen", available=False, error="env not set"),
        ]
        diff = compute_diff(results, baseline_name="fixture-7b-v1")
        md = render_diff_table(diff)
        self.assertIn("跨模型差异", md)
        self.assertIn("baseline = fixture-7b-v1", md)
        self.assertIn("real-qwen", md)
        self.assertIn("lift_summary", md)

    def test_render_diff_table_no_baseline(self):
        """compute_diff 没 baseline → render_diff_table 走兜底分支."""
        diff = compute_diff([], baseline_name="missing")
        md = render_diff_table(diff)
        self.assertIn("no baseline found", md)


class TestWriteDiff(unittest.TestCase):
    def test_write_diff_writes_json(self):
        """write_diff: 落盘 JSON 可 parse, 含 computed_at + rows + lift_summary."""
        results = run_ci()
        diff = compute_diff(results, baseline_name="fixture-7b-v1")
        with tempfile.TemporaryDirectory() as tmp:
            p = write_diff(diff, path=Path(tmp) / "diff.json")
            self.assertTrue(p.exists())
            data = json.loads(p.read_text(encoding="utf-8"))
            self.assertEqual(data["baseline"]["model_name"], "fixture-7b-v1")
            self.assertIn("lift_summary", data)
            self.assertIn("computed_at", data)


# ---------------------------------------------------------------------------
# (B) CI badge (主 13:31 大胆激进: shields.io 2014 + GHA 2020 借鉴)
# ---------------------------------------------------------------------------
class TestRenderBadge(unittest.TestCase):
    def test_badge_pass_status(self):
        """主 17:43: all_pass=True → status=pass, color=green."""
        results = [_make_result("a"), _make_result("b", passed=True)]
        badge = render_badge(results)
        self.assertEqual(badge["badge"]["status"], "pass")
        self.assertEqual(badge["badge"]["color"], "green")
        self.assertEqual(badge["n_passed"], 2)

    def test_badge_fail_status(self):
        """主 17:43: 全 fail → status=fail, color=red."""
        results = [_make_result("a", passed=False, available=True, sc=0.1)]
        badge = render_badge(results)
        self.assertEqual(badge["badge"]["status"], "fail")
        self.assertEqual(badge["badge"]["color"], "red")

    def test_badge_unknown_when_all_unavailable(self):
        """主 17:58 不假装: 全 unavailable → status=unknown, color=lightgrey."""
        results = [_make_result("a", family="qwen", available=False, passed=False)]
        badge = render_badge(results)
        self.assertEqual(badge["badge"]["status"], "unknown")
        self.assertEqual(badge["badge"]["color"], "lightgrey")

    def test_badge_mixed_status(self):
        """主 17:43: 部分 pass 部分 fail → status=mixed, color=yellow."""
        results = [
            _make_result("a", passed=True),
            _make_result("b", family="qwen", available=False, passed=False),
        ]
        badge = render_badge(results)
        self.assertEqual(badge["badge"]["status"], "mixed")
        self.assertEqual(badge["badge"]["color"], "yellow")

    def test_badge_empty_results(self):
        """主 17:58 不假装: 空 results → status=unknown."""
        badge = render_badge([])
        self.assertEqual(badge["badge"]["status"], "unknown")
        self.assertEqual(badge["n_models"], 0)

    def test_badge_with_lift_summary(self):
        """主 13:31: 有 diff → message 含 lift 段, lift_summary 非空."""
        # 用 baseline + 1 unavailable, 避免 text2vec 影响 n_loaded 断言
        baseline_result = _make_result("fixture-7b-v1", available=True, passed=True)
        results = [
            baseline_result,
            _make_result("real-qwen", available=False, error="env not set"),
        ]
        diff = compute_diff(results, baseline_name="fixture-7b-v1")
        badge = render_badge(results, diff=diff)
        self.assertIn("lift_summary", badge)
        ls = badge["lift_summary"]
        self.assertEqual(ls["n_failed"], 1)
        self.assertEqual(ls["n_loaded"], 0)


class TestRenderBadgeMarkdown(unittest.TestCase):
    def test_badge_markdown_url_shape(self):
        """主 00:56: badge Markdown 是 shields.io URL, 含 label + color."""
        results = [_make_result("a", passed=True)]
        badge = render_badge(results)
        md = render_badge_markdown(badge)
        self.assertIn("https://img.shields.io/badge/", md)
        self.assertIn("cross-small-model-ci", md)
        self.assertIn("green", md)

    def test_badge_markdown_handles_plain_badge_dict(self):
        """主 00:56: render_badge_markdown 接受 {label, message, color} 字典."""
        plain = {"label": "ci", "message": "ok", "color": "green"}
        md = render_badge_markdown(plain)
        self.assertIn("https://img.shields.io/badge/ci-ok-green", md)


class TestWriteBadge(unittest.TestCase):
    def test_write_badge_writes_json(self):
        """write_badge: 落盘 JSON 含 schemaVersion + badge + lift_summary + computed_at."""
        results = run_ci()
        with tempfile.TemporaryDirectory() as tmp:
            p = write_badge(results, path=Path(tmp) / "badge.json")
            self.assertTrue(p.exists())
            data = json.loads(p.read_text(encoding="utf-8"))
            self.assertEqual(data["schemaVersion"], 1)
            self.assertIn("badge", data)
            self.assertIn("lift_summary", data)
            self.assertIn("computed_at", data)


# ---------------------------------------------------------------------------
# (C) 真模型 best-effort 接入 (主 13:31 大胆激进 + 主 17:58 不假装)
# ---------------------------------------------------------------------------
class TestRealModelAttempt(unittest.TestCase):
    def setUp(self):
        # 隔离 env: 用 monkeypatch 替代, 避免污染其他测试
        self._saved_env = {k: os.environ.get(k) for k in REAL_MODEL_ENV.values()}

    def tearDown(self):
        for k, v in self._saved_env.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v

    def test_attempt_real_model_no_env(self):
        """主 17:58: env 未设 → unavailable, 显式 error (不假装)."""
        os.environ.pop(REAL_MODEL_ENV["qwen"], None)
        runner = CIRunner()
        r = runner.attempt_real_model("qwen")
        self.assertFalse(r.available)
        self.assertIn("not set", r.error)
        self.assertFalse(r.passed)

    def test_attempt_real_model_env_set_but_path_missing(self):
        """主 17:58: env 设有但路径不存在 → unavailable + 显式 error."""
        os.environ[REAL_MODEL_ENV["qwen"]] = "C:/nonexistent/qwen-model-dir"
        runner = CIRunner()
        r = runner.attempt_real_model("qwen")
        self.assertFalse(r.available)
        self.assertIn("does not exist", r.error)
        self.assertFalse(r.passed)

    def test_attempt_real_model_unknown_family(self):
        """主 17:58: unknown family → error 显式."""
        runner = CIRunner()
        r = runner.attempt_real_model("nonexistent-family")
        self.assertFalse(r.available)
        self.assertIn("unknown family", r.error)

    def test_attempt_real_model_honor_env_disabled(self):
        """主 13:31: honor_real_model_env=False → 立刻返回 disabled error."""
        os.environ[REAL_MODEL_ENV["qwen"]] = "C:/some/path"
        runner = CIRunner(honor_real_model_env=False)
        r = runner.attempt_real_model("qwen")
        self.assertFalse(r.available)
        self.assertIn("disabled", r.error)

    def test_attempt_real_model_qwen_path_exists_but_load_fails(self):
        """主 17:58: 路径存在但模型加载失败 → available=True (路径在), 但 subscore=0 / passed=False.

        加载失败 (空目录 / transformers 找不到 config.json / 别的 IO 错) → infer 全失败
        → 所有 score=0 → subscore=0 → passed=False; error 字段记录失败细节.
        这是 "不假装" 的核心: 不因为 load 失败就标记 unavailable (那是路径问题),
        而是跑 harness 真测, 让分数自己说话.
        """
        # 用临时目录模拟 "model dir 存在" 但里面没真模型文件
        with tempfile.TemporaryDirectory() as tmp:
            os.environ[REAL_MODEL_ENV["qwen"]] = tmp
            runner = CIRunner()
            r = runner.attempt_real_model("qwen")
            self.assertTrue(r.available, "path exists → available=True")
            self.assertFalse(r.passed, "load 失败 → subscore 应 ≤ 阈值")
            # 主 17:43: 真测过 harness, 应有 inference 尝试
            self.assertGreater(r.n_inferences, 0)


class TestRunCIWithRealModelAttempts(unittest.TestCase):
    def setUp(self):
        self._saved_env = {k: os.environ.get(k) for k in REAL_MODEL_ENV.values()}

    def tearDown(self):
        for k, v in self._saved_env.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v

    def test_run_ci_include_real_model_attempts_default(self):
        """主 13:31: include_real_model_attempts=True → 默认追加 qwen + llama attempts."""
        for k in REAL_MODEL_ENV.values():
            os.environ.pop(k, None)
        results = run_ci(include_real_model_attempts=True)
        names = [r.model_name for r in results]
        self.assertIn("fixture-7b-v1", names)
        self.assertIn("real-qwen", names)
        self.assertIn("real-llama", names)
        # 真模型 attempts 都 unavailable (env 未设)
        for r in results:
            if r.model_name.startswith("real-"):
                self.assertFalse(r.available)
                self.assertIsNotNone(r.error)

    def test_run_ci_real_model_families_custom(self):
        """主 00:56: real_model_families 可定制 (不是 hardcode qwen+llama)."""
        for k in REAL_MODEL_ENV.values():
            os.environ.pop(k, None)
        results = run_ci(include_real_model_attempts=True,
                         real_model_families=["hermes"])
        names = [r.model_name for r in results]
        self.assertIn("real-hermes", names)
        self.assertNotIn("real-qwen", names)


# ---------------------------------------------------------------------------
# (D) render_markdown 增强 (含 diff + badge)
# ---------------------------------------------------------------------------
class TestRenderMarkdownWithDiffBadge(unittest.TestCase):
    def test_markdown_includes_diff_section(self):
        """W3: render_markdown 接 diff → 报告含跨模型差异段."""
        results = run_ci()
        diff = compute_diff(results, baseline_name="fixture-7b-v1")
        md = csm.render_markdown(results, diff=diff)
        self.assertIn("跨模型差异", md)

    def test_markdown_includes_badge_section(self):
        """W3: render_markdown 接 badge → 报告含 shields.io badge URL."""
        results = run_ci()
        badge = render_badge(results)
        md = csm.render_markdown(results, badge=badge)
        self.assertIn("https://img.shields.io/badge/", md)


# ---------------------------------------------------------------------------
# (E) export 完整性 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------
class TestPublicExports(unittest.TestCase):
    def test_real_model_env_exported(self):
        self.assertIn("REAL_MODEL_ENV", dir(csm))
        self.assertEqual(set(csm.REAL_MODEL_ENV.keys()),
                         {"qwen", "llama", "hermes", "gemma"})

    def test_diff_api_exported(self):
        for name in ("compute_diff", "render_diff_table", "write_diff"):
            self.assertTrue(callable(getattr(csm, name, None)),
                            f"missing {name}")

    def test_badge_api_exported(self):
        for name in ("render_badge", "render_badge_markdown", "write_badge"):
            self.assertTrue(callable(getattr(csm, name, None)),
                            f"missing {name}")


if __name__ == "__main__":
    unittest.main()
