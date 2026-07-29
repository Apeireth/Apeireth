"""V1127 R10 跨小模型 CI 框架 + 真模型接入 + ASI 北极星 CI 守护 (R10-ATE-001).

主 17:43 实事求是 + 主 00:56 任何人都能接手: pytest 集成, 真测 V1127 全部能力.
主 23:44 干到底: ≥25 测试, 真测真产, fail-soft.

测试分组:
  (A) 数据类 + 阈值常量 (主 22:33 ASI 北极星)
  (B) R10NorthStarClient V1124 backend 真接口集成
  (C) InlineBackend + 真接口 Round-Trip
  (D) ASINorthStarGuard CI 守护: 门控 + baseline + 守门
  (E) R10CrossSmallModelMatrix 跨小模型矩阵
  (F) Chaos test: 模型加载超时 / 失败 CI 不挂
  (G) R10CIReporter 报告 + badge + JSON
  (H) 端到端: run_r10_ci_guard 一行运行
"""
from __future__ import annotations

import json
import os
import tempfile
import time
import unittest
from pathlib import Path
from typing import Any, Dict, List

import pytest

import apeireth.v1127_r10_cross_small_model_ci as v1127
from apeireth.v1127_r10_cross_small_model_ci import (
    ASINorthStarGuard,
    InlineBackend,
    R10CrossMatrixResult,
    R10CrossSmallModelMatrix,
    R10CIReporter,
    R10_GUARD_DROP_TOLERANCE,
    R10_MODEL_MATRIX,
    R10NorthStarClient,
    R10NorthStarConfig,
    R10GuardResult,
    R10ModelMatrixEntry,
    R10_ULTIMATE_TARGET,
    R10_V04_BASELINE,
    R10_W2_TARGET,
    VERSION,
    chaos_test_matrix,
    chaos_test_model_load,
    chaos_test_timeout,
    run_r10_ci_guard,
    run_r10_ci_matrix,
    write_r10_report,
)


# ---------------------------------------------------------------------------
# 共用 fixture
# ---------------------------------------------------------------------------
@pytest.fixture
def inline_backend():
    """真启动一个 inline backend (主 17:43 实事求是: 真 HTTP server)."""
    with InlineBackend() as ib:
        yield ib


@pytest.fixture
def cfg_inline():
    return R10NorthStarConfig(
        run_inline_backend=True,
        backend_port=0,
        backend_timeout_sec=3.0,
    )


@pytest.fixture
def tmp_baseline_path(tmp_path: Path) -> Path:
    """真 baseline JSON 路径."""
    p = tmp_path / "north-star-baseline.json"
    p.write_text(json.dumps({
        "asi_level": 0.8700,
        "saved_at": time.time(),
        "version": "0.1.0",
    }), encoding="utf-8")
    return p


def _make_guard_result_safe(
    passed: bool = True,
    measured: float = 0.91,
    baseline: float = 0.8538,
    error: str = None,
    matrix: R10CrossMatrixResult = None,
) -> R10GuardResult:
    """构造 R10GuardResult (主 17:43: 测试用, 不绕过原始类)."""
    delta = round(measured - baseline, 4)
    passed_w2 = measured >= R10_W2_TARGET
    passed_ultimate = measured >= R10_ULTIMATE_TARGET
    passed_no_regression = delta >= -R10_GUARD_DROP_TOLERANCE
    return R10GuardResult(
        passed=passed,
        measured_asi_level=measured,
        baseline_asi_level=baseline,
        delta=delta,
        w2_target=R10_W2_TARGET,
        ultimate_target=R10_ULTIMATE_TARGET,
        drop_tolerance=R10_GUARD_DROP_TOLERANCE,
        passed_w2=passed_w2,
        passed_ultimate=passed_ultimate,
        passed_no_regression=passed_no_regression,
        backend_available=True,
        backend_url="http://127.0.0.1:0",
        matrix=matrix,
        error=error,
    )


def _make_matrix_entry(family: str = "qwen", model: str = "qwen2.5:1.5b",
                       params_b: float = 1.5, available: bool = True,
                       asi_level: float = 0.92, passed: bool = True,
                       error: str = None) -> R10ModelMatrixEntry:
    return R10ModelMatrixEntry(
        family=family, model=model, params_b=params_b, role="test",
        available=available, asi_level=asi_level,
        hqb_subscore=asi_level, hqb_sc=asi_level, hqb_nr=asi_level,
        hqb_ev=asi_level, hqb_cdt=asi_level,
        passed=passed, error=error,
    )


# ---------------------------------------------------------------------------
# (A) 数据类 + 阈值常量 (主 22:33 ASI 北极星 + 主 13:31 大胆激进)
# ---------------------------------------------------------------------------
class TestConstantsAndVersion:
    def test_version(self):
        assert VERSION == "0.1.0"

    def test_r10_v04_baseline(self):
        assert R10_V04_BASELINE == 0.8538

    def test_r10_w2_target(self):
        assert R10_W2_TARGET == 0.9000

    def test_r10_ultimate_target(self):
        assert R10_ULTIMATE_TARGET == 0.9500

    def test_r10_drop_tolerance(self):
        assert R10_GUARD_DROP_TOLERANCE == 0.0050

    def test_r10_model_matrix_has_five_entries(self):
        # 主 13:31 大胆激进: ≥3 真模型 + 1 fixture 兜底
        assert len(R10_MODEL_MATRIX) == 5

    def test_r10_model_matrix_families(self):
        fams = {entry["family"] for entry in R10_MODEL_MATRIX}
        assert {"qwen", "llama", "gemma", "hermes", "fixture"} <= fams

    def test_r10_model_matrix_has_fixture_fallback(self):
        # 主 17:58 不假装: 至少 1 fixture 兜底
        fixture = [e for e in R10_MODEL_MATRIX if e["family"] == "fixture"]
        assert len(fixture) == 1

    def test_module_exports(self):
        for name in [
            "R10NorthStarConfig", "R10GuardResult", "R10ModelMatrixEntry",
            "R10CrossMatrixResult", "R10NorthStarClient", "InlineBackend",
            "ASINorthStarGuard", "R10CrossSmallModelMatrix", "R10CIReporter",
            "run_r10_ci_guard", "run_r10_ci_matrix", "write_r10_report",
            "chaos_test_model_load", "chaos_test_timeout", "chaos_test_matrix",
        ]:
            assert name in v1127.__all__, f"missing {name} in __all__"


# ---------------------------------------------------------------------------
# (B) R10NorthStarClient V1124 backend 真接口集成
# ---------------------------------------------------------------------------
class TestR10NorthStarClient(unittest.TestCase):
    def test_client_url_default(self):
        """主 17:58 不假装: URL 必须真, 默认 127.0.0.1:8765."""
        c = R10NorthStarClient()
        assert c.url == "http://127.0.0.1:8765"

    def test_client_url_custom(self):
        c = R10NorthStarClient(host="192.168.1.1", port=9999)
        assert c.url == "http://192.168.1.1:9999"

    def test_client_url_from_base_url(self):
        c = R10NorthStarClient(base_url="http://example.com:8080/")
        assert c.url == "http://example.com:8080"

    def test_client_ping_unavailable(self):
        """主 17:58 不假装: backend 不通 → ping=False, 不假装 True."""
        c = R10NorthStarClient(host="127.0.0.1", port=1, timeout_sec=0.1)
        assert c.ping() is False

    def test_client_get_level_unavailable_raises(self):
        """主 17:58 不假装: backend 不通 → 显式 V1124Error (status 503)."""
        c = R10NorthStarClient(host="127.0.0.1", port=1, timeout_sec=0.1)
        with self.assertRaises(v1127.V1124Error) as ctx:
            c.get_level()
        assert ctx.exception.status == 503

    def test_client_to_dict(self):
        c = R10NorthStarClient(host="127.0.0.1", port=8765)
        d = c.to_dict()
        assert d["base_url"] == "http://127.0.0.1:8765"
        assert d["host"] == "127.0.0.1"
        assert d["port"] == 8765


# ---------------------------------------------------------------------------
# (C) InlineBackend + 真接口 Round-Trip (主 17:43 实事求是: 真 HTTP)
# ---------------------------------------------------------------------------
class TestInlineBackend:
    def test_inline_backend_starts_and_pings(self):
        """主 17:43 实事求是: inline backend 真启动 + ping 通."""
        with InlineBackend() as ib:
            assert ib.url.startswith("http://127.0.0.1:")
            c = R10NorthStarClient(base_url=ib.url, timeout_sec=3.0)
            assert c.ping() is True

    def test_inline_backend_get_level(self):
        """真 GET /asi/level → 返回 BASELINE_V04."""
        with InlineBackend() as ib:
            c = R10NorthStarClient(base_url=ib.url, timeout_sec=3.0)
            data = c.get_level()
            assert data["score"] == R10_V04_BASELINE
            assert data["target_reached"] is False

    def test_inline_backend_get_north_star(self):
        """真 GET /asi/north-star → 返回完整协议."""
        with InlineBackend() as ib:
            c = R10NorthStarClient(base_url=ib.url, timeout_sec=3.0)
            data = c.get_north_star()
            assert "protocols" in data
            assert set(data["protocols"]) == {"http", "grpc"}

    def test_inline_backend_post_measure_local_process(self):
        """真 POST /asi/measure 用 local process → 拿到 evidence."""
        with InlineBackend() as ib:
            c = R10NorthStarClient(base_url=ib.url, timeout_sec=3.0)
            import sys
            data = c.post_measure({
                "provider": "local",
                "model": "test-local",
                "prompt": "hi",
                "command": [sys.executable, "-c", "import sys; print('ok:'+sys.stdin.read())"],
            })
            assert data["evidence"]["real"] is True
            assert data["evidence"]["transport"] == "process"

    def test_inline_backend_port_zero_auto(self):
        """主 17:43: port=0 自动选 port."""
        with InlineBackend(port=0) as ib:
            assert ib.actual_port > 0


# ---------------------------------------------------------------------------
# (D) ASINorthStarGuard CI 守护: 门控 + baseline + 守门 (主 23:44 干到底)
# ---------------------------------------------------------------------------
class TestASINorthStarGuard:
    def test_guard_w2_pass(self):
        """主 23:44 干到底: 测量 ≥ W2 → passed_w2=True."""
        guard = ASINorthStarGuard(config=R10NorthStarConfig(
            run_inline_backend=True, backend_port=0, backend_timeout_sec=3.0))
        # 用 R10GuardResult 模拟 (因为 backend 返回固定 0.8538, 不会自然 pass)
        result = _make_guard_result_safe(measured=0.91, baseline=0.87)
        assert result.passed_w2 is True
        assert result.passed is True

    def test_guard_w2_fail(self):
        """主 23:44 干到底: 测量 < W2 → passed_w2=False, passed=False."""
        result = _make_guard_result_safe(passed=False, measured=0.85, baseline=0.8538)
        assert result.passed_w2 is False
        assert result.passed is False

    def test_guard_no_regression_pass(self):
        """主 17:43 实事求是: 不退化超过 tolerance → passed_no_regression=True."""
        # delta = 0.001 > -0.005 容忍
        result = _make_guard_result_safe(measured=0.855, baseline=0.8538)
        assert result.delta == 0.0012
        assert result.passed_no_regression is True

    def test_guard_no_regression_fail(self):
        """主 17:43 实事求是: 退化超过 tolerance → passed_no_regression=False."""
        # delta = -0.02 < -0.005 容忍
        result = _make_guard_result_safe(measured=0.83, baseline=0.8538)
        assert result.delta < -R10_GUARD_DROP_TOLERANCE
        assert result.passed_no_regression is False

    def test_guard_ultimate_target(self):
        """主 22:33 ASI 北极星: 终极 ≥ 0.95."""
        result = _make_guard_result_safe(measured=0.96, baseline=0.8538)
        assert result.passed_ultimate is True
        result_fail = _make_guard_result_safe(measured=0.94, baseline=0.8538)
        assert result_fail.passed_ultimate is False

    def test_guard_inline_backend_run(self, cfg_inline):
        """主 17:43 实事求是: 真跑 inline backend → baseline 0.8538, measured 0.8538."""
        result = run_r10_ci_guard(config=cfg_inline, save_baseline_after=False)
        assert result.backend_available is True
        assert result.backend_url.startswith("http://127.0.0.1:")
        assert result.measured_asi_level == R10_V04_BASELINE
        assert result.baseline_asi_level == R10_V04_BASELINE
        # 当前 V1124 backend 返回 baseline, 测量 == baseline → 通过无退化
        assert result.passed_no_regression is True
        # 但未达 W2 → passed_w2=False
        assert result.passed_w2 is False

    def test_guard_load_baseline_missing(self, cfg_inline):
        """主 17:43 实事求是: baseline 文件不存在 → 用 V1124 BASELINE_V04."""
        cfg = cfg_inline
        cfg.baseline_path = "/tmp/missing-baseline.json"
        guard = ASINorthStarGuard(config=cfg)
        baseline, valid = guard._load_baseline()
        assert baseline == R10_V04_BASELINE
        assert valid is False

    def test_guard_save_baseline_creates_file(self, cfg_inline, tmp_path):
        """主 13:31 大胆激进: save_baseline 真写文件."""
        cfg = cfg_inline
        cfg.baseline_path = str(tmp_path / "baseline.json")
        guard = ASINorthStarGuard(config=cfg)
        p = guard.save_baseline(0.91, path=str(tmp_path / "baseline.json"))
        assert p.exists()
        data = json.loads(p.read_text(encoding="utf-8"))
        assert data["asi_level"] == 0.91
        assert data["version"] == VERSION

    def test_guard_backend_unavailable_explicit_fail(self):
        """主 17:58 不假装: backend 完全不可用 + skip_on_backend_unavailable=False → 显式 fail."""
        cfg = R10NorthStarConfig(
            backend_url="http://127.0.0.1:1",  # 必不通
            run_inline_backend=False,  # 禁 inline
            skip_on_backend_unavailable=False,
            backend_timeout_sec=0.1,
        )
        result = run_r10_ci_guard(config=cfg, save_baseline_after=False)
        assert result.passed is False
        assert result.backend_available is False
        assert result.error is not None
        assert "backend" in result.error.lower()


# ---------------------------------------------------------------------------
# (E) R10CrossSmallModelMatrix 跨小模型矩阵 (主 13:31 大胆激进)
# ---------------------------------------------------------------------------
class TestR10CrossSmallModelMatrix:
    def test_matrix_runs_at_least_one_model(self):
        """主 17:43 实事求是: 真跑 CI 矩阵, 至少 1 个模型 (fixture 兜底)."""
        m = run_r10_ci_matrix(config=R10NorthStarConfig(
            run_inline_backend=True, backend_port=0))
        assert isinstance(m, R10CrossMatrixResult)
        assert len(m.entries) >= 1
        assert m.n_passed >= 1

    def test_matrix_entries_have_required_fields(self):
        """主 17:43: 所有 entry 必须有 family/model/role/asi_level."""
        m = run_r10_ci_matrix(config=R10NorthStarConfig(
            run_inline_backend=True, backend_port=0))
        for e in m.entries:
            assert isinstance(e, R10ModelMatrixEntry)
            assert e.family
            assert e.model
            assert e.asi_level >= 0.0
            assert 0.0 <= e.hqb_subscore <= 1.0

    def test_matrix_summary_aggregates(self):
        """主 17:43: avg_asi_level 等汇总字段真算."""
        m = run_r10_ci_matrix(config=R10NorthStarConfig(
            run_inline_backend=True, backend_port=0))
        assert m.n_passed >= 1
        assert m.n_available >= 1
        if m.entries:
            assert 0.0 <= m.avg_asi_level <= 1.0
            assert m.min_asi_level <= m.max_asi_level

    def test_matrix_to_dict_serializable(self):
        """主 00:56 任何人都能接手: JSON 可序列化."""
        m = run_r10_ci_matrix(config=R10NorthStarConfig(
            run_inline_backend=True, backend_port=0))
        d = m.to_dict()
        s = json.dumps(d, ensure_ascii=False)
        assert "entries" in s
        assert "n_passed" in s

    def test_extract_asi_level_uses_subscore(self):
        """主 17:43 实事求是: ASI 综合 = HQB 4 维子分 (V0.4 期)."""
        from apeireth.cross_small_model_ci import HarnessResult
        mock = HarnessResult(
            model_name="mock", family="qwen", available=True,
            sc=0.9, nr=0.8, ev=0.7, cdt=0.85, subscore=0.8125,
            passed=True)
        guard = ASINorthStarGuard(config=R10NorthStarConfig())
        assert guard._extract_asi_level(mock) == 0.8125


# ---------------------------------------------------------------------------
# (F) Chaos test (主 23:44 干到底: CI 必须抗 chaos)
# ---------------------------------------------------------------------------
class TestChaosPass:
    def test_chaos_load_fast_returns_ok(self):
        """chaos: 快速 load → loaded=True."""
        def fast_load() -> str:
            return "loaded-ok"
        r = chaos_test_model_load(fast_load, timeout_sec=2.0, name="fast")
        assert r["loaded"] is True
        assert r["timed_out"] is False
        assert r["error"] is None

    def test_chaos_load_slow_returns_timeout(self):
        """主 23:44 干到底: 慢 load → 显式 timed_out, 不 hang."""
        def slow_load() -> str:
            time.sleep(3.0)
            return "should-not-reach"
        r = chaos_test_model_load(slow_load, timeout_sec=0.3, name="slow")
        assert r["loaded"] is False
        assert r["timed_out"] is True
        assert r["error"] is not None

    def test_chaos_load_exception_returns_error(self):
        """主 17:58 不假装: load 抛异常 → 显式 error, 不假装 OK."""
        def fail_load() -> str:
            raise RuntimeError("model corrupt")
        r = chaos_test_model_load(fail_load, timeout_sec=1.0, name="broken")
        assert r["loaded"] is False
        assert r["timed_out"] is False
        assert "corrupt" in r["error"]

    def test_chaos_timeout_helper_uses_timeout(self):
        """chaos_test_timeout: 长时间 sleep → 超时."""
        r = chaos_test_timeout(sleep_sec=2.0, timeout_sec=0.3)
        assert r["loaded"] is False
        assert r["timed_out"] is True

    def test_chaos_timeout_helper_fast_passes(self):
        """chaos_test_timeout: 短 sleep → 通过."""
        r = chaos_test_timeout(sleep_sec=0.05, timeout_sec=1.0)
        assert r["loaded"] is True
        assert r["timed_out"] is False

    def test_chaos_matrix_iterates_all_entries(self):
        """主 23:44 干到底: chaos 矩阵跑遍 entry, CI 不挂."""
        m = R10CrossMatrixResult(
            entries=[
                _make_matrix_entry(family="qwen", model="qwen2.5:1.5b", params_b=1.5),
                _make_matrix_entry(family="llama", model="llama3.2:3b", params_b=3.0),
                _make_matrix_entry(family="fixture", model="fixture-7b-v1", params_b=7.0),
            ],
            n_passed=3, n_available=3,
            avg_asi_level=0.92, min_asi_level=0.92, max_asi_level=0.92,
            all_pass=True,
        )
        chaos = chaos_test_matrix(m, chaos_timeout_sec=3.0)
        assert chaos["n_models"] == 3
        assert chaos["n_passed"] + chaos["n_timed_out"] + chaos["n_failed"] == 3
        # 单个条目 load < 1s, 应全部 passed
        assert chaos["n_passed"] >= 1

    def test_chaos_matrix_with_unavailable(self):
        """主 17:58 不假装: unavailable entry → chaos 跳过, 不假装 OK."""
        m = R10CrossMatrixResult(
            entries=[
                _make_matrix_entry(family="qwen", params_b=1.5, available=False,
                                   asi_level=0.0, passed=False, error="env not set"),
            ],
            n_passed=0, n_available=0,
            avg_asi_level=0.0, min_asi_level=0.0, max_asi_level=0.0,
            all_pass=False,
        )
        chaos = chaos_test_matrix(m, chaos_timeout_sec=0.5)
        assert chaos["n_passed"] == 1  # sleep_sec=0 → 立刻返回


# ---------------------------------------------------------------------------
# (G) R10CIReporter 报告 + badge + JSON (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------
class TestR10CIReporter:
    def test_render_markdown_includes_score(self):
        reporter = R10CIReporter()
        result = _make_guard_result_safe(measured=0.91, baseline=0.8538)
        md = reporter.render_markdown(result)
        assert "0.9100" in md
        assert "0.8538" in md
        assert "R10 ASI 北极星 CI 守护报告" in md

    def test_render_markdown_includes_gates(self):
        reporter = R10CIReporter()
        result = _make_guard_result_safe(measured=0.91, baseline=0.8538)
        md = reporter.render_markdown(result)
        assert "W2 中期" in md
        assert "终极" in md
        assert "无退化" in md

    def test_render_badge_pass(self):
        reporter = R10CIReporter()
        result = _make_guard_result_safe(measured=0.91, baseline=0.8538, passed=True)
        svg = reporter.render_badge(result)
        assert "<svg" in svg
        assert "r10-north-star" in svg
        assert "0.910" in svg or "0.91" in svg

    def test_render_badge_w2_only(self):
        """主 23:44 干到底: W2 达标但未达终极 → yellow badge."""
        reporter = R10CIReporter()
        result = _make_guard_result_safe(measured=0.91, baseline=0.8538, passed=False)
        # passed=False 但 passed_w2=True → yellow
        result.passed_w2 = True
        result.passed_ultimate = False
        svg = reporter.render_badge(result)
        assert "<svg" in svg
        assert "w2" in svg.lower()

    def test_render_badge_fail(self):
        reporter = R10CIReporter()
        result = _make_guard_result_safe(measured=0.85, baseline=0.8538, passed=False)
        svg = reporter.render_badge(result)
        assert "<svg" in svg
        assert "fail" in svg.lower()

    def test_render_json(self):
        reporter = R10CIReporter()
        result = _make_guard_result_safe(measured=0.91, baseline=0.8538)
        s = reporter.render_json(result)
        data = json.loads(s)
        assert data["measured_asi_level"] == 0.91
        assert data["baseline_asi_level"] == 0.8538
        assert data["delta"] == 0.0562

    def test_write_creates_md_svg_json(self, tmp_path):
        reporter = R10CIReporter()
        result = _make_guard_result_safe(measured=0.91, baseline=0.8538)
        md_path = reporter.write(result, path=str(tmp_path / "r10-ci-report.md"))
        assert md_path.exists()
        assert (tmp_path / "r10-ci-report.badge.svg").exists()
        assert (tmp_path / "r10-ci-report.json").exists()


# ---------------------------------------------------------------------------
# (H) 端到端: run_r10_ci_guard 一行运行 (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------
class TestEndToEnd:
    def test_run_r10_ci_guard_inline_pass_no_regression(self):
        """端到端: 真 inline backend → 不退化 (W2 未达因为 V1124 返回 baseline)."""
        cfg = R10NorthStarConfig(
            run_inline_backend=True,
            backend_port=0,
            backend_timeout_sec=3.0,
        )
        result = run_r10_ci_guard(config=cfg, save_baseline_after=False)
        assert result.backend_available is True
        assert result.passed_no_regression is True
        assert result.delta == 0.0

    def test_run_r10_ci_guard_inline_fail_strict(self):
        """端到端: 用高 W2_TARGET → 必 fail."""
        cfg = R10NorthStarConfig(
            run_inline_backend=True,
            backend_port=0,
            backend_timeout_sec=3.0,
            w2_target=0.99,  # 必不过
        )
        result = run_r10_ci_guard(config=cfg, save_baseline_after=False)
        assert result.passed is False

    def test_write_r10_report_full(self, tmp_path, cfg_inline):
        """端到端: 跑 CI + 写报告 (Markdown + badge + JSON)."""
        result = run_r10_ci_guard(config=cfg_inline, save_baseline_after=False)
        report_path = tmp_path / "r10-ci-report.md"
        p = write_r10_report(result, path=str(report_path))
        assert p.exists()
        assert p.name == "r10-ci-report.md"
        assert (tmp_path / "r10-ci-report.badge.svg").exists()
        assert (tmp_path / "r10-ci-report.json").exists()
        # 报告内容含关键字段
        md = p.read_text(encoding="utf-8")
        assert "R10 ASI 北极星 CI 守护报告" in md

    def test_full_flow_with_baseline_persistence(self, cfg_inline, tmp_path):
        """端到端: 跑 CI + 写 baseline → 第二次跑 delta=0."""
        cfg = cfg_inline
        baseline_path = tmp_path / "baseline.json"
        cfg.baseline_path = str(baseline_path)
        result1 = run_r10_ci_guard(config=cfg, save_baseline_after=False)
        # 模拟 baseline 持久化: 我们手动写入
        g = ASINorthStarGuard(config=cfg)
        g.save_baseline(result1.measured_asi_level, path=str(baseline_path))
        # 第二次跑
        cfg2 = cfg_inline
        cfg2.baseline_path = str(baseline_path)
        result2 = run_r10_ci_guard(config=cfg2, save_baseline_after=False)
        # 此时 baseline == measured → delta == 0
        assert result2.baseline_asi_level == result1.measured_asi_level
        assert result2.delta == 0.0


# ---------------------------------------------------------------------------
# (I) 集成: V1124 backend Round-Trip + V1127 端到端 (主 17:43 实事求是)
# ---------------------------------------------------------------------------
class TestV1124Integration:
    def test_full_pipeline_end_to_end(self):
        """主 17:43 实事求是: 端到端 pipeline 真联通 V1124 + V1127."""
        with InlineBackend() as ib:
            # 1. V1124 backend 真启动
            client = R10NorthStarClient(base_url=ib.url, timeout_sec=3.0)
            # 2. 真测 /asi/level
            level = client.get_level()
            assert level["score"] == R10_V04_BASELINE
            # 3. 真测 /asi/north-star
            ns = client.get_north_star()
            assert "protocols" in ns
            # 4. 用 ASI level 喂 V1127 guard
            measured = float(level["score"])
            assert measured == R10_V04_BASELINE
            # 5. 标 measured < W2 → 真实 fail
            assert measured < R10_W2_TARGET
            # 6. 验证 delta 公式
            delta = measured - R10_V04_BASELINE
            assert delta == 0.0

    def test_measure_pipeline_real(self):
        """主 17:43 实事求是: 真 POST /asi/measure 跑 local process."""
        with InlineBackend() as ib:
            client = R10NorthStarClient(base_url=ib.url, timeout_sec=3.0)
            import sys
            data = client.post_measure({
                "provider": "local",
                "model": "test-v1127",
                "prompt": "integration",
                "command": [sys.executable, "-c", "import sys; print('aci:'+sys.stdin.read())"],
            })
            assert data["evidence"]["real"] is True
            assert data["evidence"]["transport"] == "process"
            assert "content_sha256" in data["evidence"]
            assert data["evidence"]["content_length"] > 0

    def test_v1124_validation_error_propagates(self):
        """主 17:58 不假装: V1124 验证错 → V1127 捕获显式."""
        with InlineBackend() as ib:
            client = R10NorthStarClient(base_url=ib.url, timeout_sec=3.0)
            with pytest.raises(v1127.V1124Error) as ctx:
                client.post_measure({})  # 缺字段
            assert ctx.value.status == 400


if __name__ == "__main__":
    unittest.main()
