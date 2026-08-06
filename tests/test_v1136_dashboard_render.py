"""Apeireth ASI V1136 → Dashboard Render Tests (R11 perf 真链路).

Tests cover:
  1. Render correctness — V1136 score 来源真测, 缓存不伪造分数
  2. Real failure state preservation — 失败原样透传 (主 17:58 不假装)
  3. Cache key 是 stable hash(result) — 同输入同输出
  4. p50/p95/p99 真实 — bench_render + bench_render_loop
  5. Serialization — JSON-safe (Path / dataclass / Mapping)
  6. R11 perf target — cold p95 ≤ 250ms
  7. CLI invocation — (主 00:56 任何人都能接手)

主 17:43 实事求是 + 主 17:58 不假装 + 主 19:33 走在前人经验上 + 主 22:33 北极星.
"""
from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path
from typing import List

import pytest

from apeireth.v1136_asi_v05_3dim_real_measurement import (
    V1136Result,
    measure_v05_3dims,
    measure_continuity_real,
    measure_autonomy_real,
    measure_transferability_real,
)
from apeireth.v1118_perf_optimizer_v01 import SubmoduleResultCache
from apeireth.v1136_dashboard_render import (
    V1136_DASHBOARD_VERSION,
    RenderPerfStats,
    V1136DashboardRender,
    RenderBenchResult,
    BenchLoopResult,
    _percentile,
    _stable_hash,
    _collect_sub_latencies,
    render_v1136_dashboard,
    bench_render,
    bench_render_loop,
)


WORKDIR = Path(__file__).resolve().parent.parent


# ---------------------------------------------------------------------------
# Fixtures — V1136 真测 (主 17:43 实事求是)
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def v1136_result() -> V1136Result:
    """Module-scoped V1136Result — 慢, 但全 suite 共享 (主 17:43)."""
    return measure_v05_3dims(v04_score=0.8538, run_chaos=False)


# ---------------------------------------------------------------------------
# 1. Percentile helper — stdlib statistics.quantiles (主 19:33)
# ---------------------------------------------------------------------------


class TestPercentile:
    def test_constant_array(self):
        values = [0.10] * 20
        assert _percentile(values, 50) == pytest.approx(0.10, rel=1e-6)
        assert _percentile(values, 95) == pytest.approx(0.10, rel=1e-6)

    def test_handles_outlier(self):
        values = [0.01] * 99 + [0.50]
        assert _percentile(values, 99) >= 0.01

    def test_empty_returns_zero(self):
        assert _percentile([], 95) == 0.0

    def test_single_value(self):
        assert _percentile([0.42], 50) == pytest.approx(0.42, rel=1e-6)
        assert _percentile([0.42], 95) == pytest.approx(0.42, rel=1e-6)

    def test_clamps_to_range(self):
        values = list(range(100))
        assert _percentile(values, 0) == pytest.approx(0.0, abs=1e-9)
        assert _percentile(values, 100) == pytest.approx(99.0, abs=1e-9)


# ---------------------------------------------------------------------------
# 2. Render correctness — V1136 score 来源真测, 缓存命中不伪造分数
# ---------------------------------------------------------------------------


class TestRenderCorrectness:
    def test_score_always_comes_from_v1136_result(self, v1136_result):
        """主 17:43 守门: 缓存命中/未命中, score 必须 == v1136_result 真测."""
        cache = SubmoduleResultCache(maxsize=4)
        cold = render_v1136_dashboard(v1136_result, cache=cache)
        warm = render_v1136_dashboard(v1136_result, cache=cache)
        assert cold.cache_hit is False
        assert warm.cache_hit is True
        # Score 不依赖 cache
        assert cold.v1136_score == v1136_result.v05_total_v1136
        assert warm.v1136_score == v1136_result.v05_total_v1136
        assert cold.continuity == v1136_result.continuity
        assert cold.autonomy == v1136_result.autonomy
        assert cold.transferability == v1136_result.transferability
        # Markdown 内容相同 (因为输入 hash 相同 → cache 命中)
        assert cold.markdown == warm.markdown

    def test_different_input_yields_different_markdown(self):
        """不同 V1136Result → cache miss → 不同 markdown."""
        r1 = measure_v05_3dims(v04_score=0.8538, run_chaos=False)
        # 构造一个 scores 不同的结果 (不调真测, 只造一个 dataclass)
        from dataclasses import replace
        r2 = replace(r1, v05_total_v1136=0.9999, continuity=0.99)
        cache = SubmoduleResultCache(maxsize=4)
        m1 = render_v1136_dashboard(r1, cache=cache)
        m2 = render_v1136_dashboard(r2, cache=cache)
        # m2 不可能 cache 命中 (hash 不同)
        assert m2.cache_hit is False
        assert m1.markdown != m2.markdown

    def test_render_path_is_v1136_real(self, v1136_result):
        r = render_v1136_dashboard(v1136_result)
        assert r.render_path == "v1136_real"
        assert r.bytes_written > 0
        assert r.dimensions >= 18

    def test_markdown_contains_3_dim_real_header(self, v1136_result):
        r = render_v1136_dashboard(v1136_result)
        assert "R11 V1136 → Dashboard" in r.markdown
        assert "Continuity" in r.markdown
        assert "Autonomy" in r.markdown
        assert "Transferability" in r.markdown
        assert f"{v1136_result.continuity:.4f}" in r.markdown
        assert f"{v1136_result.v05_total_v1136:.4f}" in r.markdown


# ---------------------------------------------------------------------------
# 3. Real failure state preservation (主 17:58 不假装)
# ---------------------------------------------------------------------------


class TestFailureStatePreserved:
    def test_failures_count_surfaces_in_render(self, v1136_result):
        """V1136 真测里 failures 字段必须出现在 dashboard render 里."""
        r = render_v1136_dashboard(v1136_result)
        # failures 计数与 detail 一致
        assert r.continuity_failures == int((v1136_result.continuity_detail or {}).get("failed", 0))
        assert r.autonomy_failures == int((v1136_result.autonomy_detail or {}).get("failed", 0))
        assert r.transferability_failures == int(
            (v1136_result.transferability_detail or {}).get("failed", 0)
        )

    def test_failures_list_written_to_markdown(self, v1136_result):
        """真实失败列表原样写入 dashboard markdown (主 17:58 不假装)."""
        # 强制注入一个失败 — 用 monkeypatch 改 detail.failure
        from dataclasses import replace
        cont = dict(v1136_result.continuity_detail or {})
        cont["failures"] = ["fake_injected_for_test: simulated v1089 hotcold fail"]
        cont["failed"] = 1
        mutated = replace(v1136_result, continuity_detail=cont)
        r = render_v1136_dashboard(mutated)
        assert "fake_injected_for_test" in r.markdown
        assert "Failures" in r.markdown or "失败" in r.markdown or "failures" in r.markdown.lower()

    def test_zero_score_submeasurement_appears_as_failed(self):
        """子测度 score=0 → 标记为 ❌ failed, 不掩盖."""
        from dataclasses import replace
        cont = dict(measure_continuity_real())
        cont["sub_scores"] = {"v1052_consolidation": 0.0, "v1072_eternal_identity": 1.0}
        cont["failures"] = ["v1052_consolidation: simulated"]
        cont["failed"] = 1
        cont["implemented"] = 1
        # 手工构造 V1136Result (不重跑 measure_v05_3dims, 避免 flaky)
        result = V1136Result(
            continuity=0.5,
            autonomy=0.7,
            transferability=0.6,
            v05_total_v1136=0.85 * 0.8538 + 0.05 * (0.5 + 0.7 + 0.6),
            v05_total_v1125=0.85 * 0.8538 + 0.05 * 3 * 0.85,
            v04_score=0.8538,
            delta_v05_total=-0.1,
            continuity_detail=cont,
            autonomy_detail={"sub_scores": {}, "failures": [], "failed": 0,
                             "implemented": 0, "total": 0, "elapsed_seconds": 0.0},
            transferability_detail={"sub_scores": {}, "failures": [], "failed": 0,
                                    "implemented": 0, "total": 0, "elapsed_seconds": 0.0},
            chaos_report=None,
            v3_guards_pass=False,
            elapsed_seconds=0.1,
            timestamp=time.time(),
        )
        r = render_v1136_dashboard(result)
        assert "v1052_consolidation" in r.markdown
        assert "❌" in r.markdown  # failed marker

    def test_v3_guards_pass_flag_propagates(self, v1136_result):
        """V3 守门失败 → render.v3_guards_pass == False, 不掩盖."""
        from dataclasses import replace
        mutated = replace(v1136_result, v3_guards_pass=False)
        r = render_v1136_dashboard(mutated)
        assert r.v3_guards_pass is False


# ---------------------------------------------------------------------------
# 4. Stable hash — cache key is content-derived, not object identity
# ---------------------------------------------------------------------------


class TestStableHash:
    def test_same_content_same_hash(self, v1136_result):
        from dataclasses import replace
        a = v1136_result
        b = replace(v1136_result)
        assert _stable_hash(a) == _stable_hash(b)

    def test_different_score_different_hash(self, v1136_result):
        from dataclasses import replace
        a = v1136_result
        b = replace(v1136_result, continuity=v1136_result.continuity + 0.1)
        assert _stable_hash(a) != _stable_hash(b)

    def test_different_failure_state_different_hash(self, v1136_result):
        from dataclasses import replace
        cont = dict(v1136_result.continuity_detail)
        cont["failed"] = (cont.get("failed", 0)) + 1
        a = v1136_result
        b = replace(v1136_result, continuity_detail=cont)
        assert _stable_hash(a) != _stable_hash(b)


# ---------------------------------------------------------------------------
# 5. Sub-measurement latency extraction
# ---------------------------------------------------------------------------


class TestSubLatencies:
    def test_collects_three_dim_latencies(self, v1136_result):
        lats = _collect_sub_latencies(v1136_result)
        # 3 个 detail 各有一个 elapsed_seconds (V1136 主流程里 measure_continuity_real
        # / measure_autonomy_real / measure_transferability_real 都返回 elapsed_seconds)
        assert len(lats) == 3
        for lat in lats:
            assert lat >= 0.0

    def test_empty_detail_yields_empty(self):
        r = V1136Result(
            continuity=0.5, autonomy=0.5, transferability=0.5,
            v05_total_v1136=0.5, v05_total_v1125=0.5, v04_score=0.5,
            delta_v05_total=0.0,
            continuity_detail={}, autonomy_detail={}, transferability_detail={},
            chaos_report=None, v3_guards_pass=True,
            elapsed_seconds=0.1, timestamp=time.time(),
        )
        assert _collect_sub_latencies(r) == []


# ---------------------------------------------------------------------------
# 6. Bench — p50/p95/p99 (R11 perf 守门)
# ---------------------------------------------------------------------------


class TestBenchRender:
    def test_returns_cold_warm_combined(self, v1136_result):
        b = bench_render(v1136_result, trials=10)
        assert isinstance(b, RenderBenchResult)
        assert b.iterations == 10
        assert b.cold.trials == 5
        assert b.warm.trials == 5
        assert b.combined.trials == 10
        # cache 计数对得上
        assert b.cold.cache_hits + b.cold.cache_misses == 5
        assert b.warm.cache_hits + b.warm.cache_misses == 5

    def test_cold_path_is_unhit(self, v1136_result):
        """冷路径 cache 必须每次都 miss (主 17:43 实事求是)."""
        b = bench_render(v1136_result, trials=8)
        # cold 5 次, 每次 cache.clear() → 必须全部 miss
        assert b.cold.cache_misses == b.cold.trials

    def test_warm_path_is_all_hit(self, v1136_result):
        """热路径 cache 必须全部 hit."""
        b = bench_render(v1136_result, trials=8)
        assert b.warm.cache_hits == b.warm.trials
        assert b.warm.cache_misses == 0

    def test_p95_within_250ms_target(self, v1136_result):
        """R11 perf 守门: combined p95 ≤ 250ms (V1130 dashboard perf target 复用)."""
        b = bench_render(v1136_result, trials=20)
        assert b.combined.p95_s <= 0.250, (
            f"combined p95 = {b.combined.p95_s:.4f}s > 250ms target"
        )
        assert b.cold.p95_s <= 0.250, (
            f"cold p95 = {b.cold.p95_s:.4f}s > 250ms target"
        )

    def test_trials_must_be_at_least_2(self, v1136_result):
        with pytest.raises(ValueError, match="trials must be >= 2"):
            bench_render(v1136_result, trials=1)

    def test_p50_leq_p95_leq_p99(self, v1136_result):
        """百分位数学守门: p50 ≤ p95 ≤ p99."""
        b = bench_render(v1136_result, trials=20)
        assert b.combined.p50_s <= b.combined.p95_s + 1e-9
        assert b.combined.p95_s <= b.combined.p99_s + 1e-9
        assert b.combined.min_s <= b.combined.p50_s + 1e-9
        assert b.combined.p99_s <= b.combined.max_s + 1e-9


# ---------------------------------------------------------------------------
# 7. Bench loop — N 次稳定性 (主 23:44 干到底)
# ---------------------------------------------------------------------------


class TestBenchLoop:
    def test_returns_bench_loop_result(self, v1136_result):
        r = bench_render_loop(v1136_result, iterations=3, trials_per_iter=10)
        assert isinstance(r, BenchLoopResult)
        assert r.iterations == 3
        assert r.loop_p95_s > 0.0

    def test_iterations_must_be_at_least_1(self, v1136_result):
        with pytest.raises(ValueError, match="iterations must be >= 1"):
            bench_render_loop(v1136_result, iterations=0)


# ---------------------------------------------------------------------------
# 8. JSON serialization — Path / dataclass safe (主 17:43 实事求是)
# ---------------------------------------------------------------------------


class TestSerialization:
    def test_render_to_dict_is_json_safe(self, v1136_result):
        r = render_v1136_dashboard(v1136_result)
        d = r.to_dict()
        # No Path / no set / no tuple unserializable in default JSON encoder
        json.dumps(d, default=str)  # must not raise

    def test_bench_to_dict_is_json_safe(self, v1136_result):
        b = bench_render(v1136_result, trials=10)
        json.dumps(b.to_dict(), default=str)

    def test_loop_to_dict_is_json_safe(self, v1136_result):
        r = bench_render_loop(v1136_result, iterations=3, trials_per_iter=10)
        json.dumps(r.to_dict(), default=str)


# ---------------------------------------------------------------------------
# 9. CLI — (主 00:56 任何人都能接手)
# ---------------------------------------------------------------------------


class TestCLI:
    def _run(self, *args, timeout=120):
        """Run CLI with explicit UTF-8 encoding (Windows GBK default fails on Unicode)."""
        return subprocess.run(
            [sys.executable, "-m", "apeireth.v1136_dashboard_render", *args],
            cwd=str(WORKDIR), capture_output=True,
            encoding="utf-8", errors="replace", timeout=timeout,
        )

    def test_cli_runs_default(self):
        """--help → exit 0; 不带参数 → exit 0 + 一行总结."""
        proc = self._run()
        assert proc.returncode == 0, proc.stderr
        assert proc.stdout is not None
        assert "R11 V1136 → Dashboard" in proc.stdout

    def test_cli_json(self):
        proc = self._run("--json")
        assert proc.returncode == 0, proc.stderr
        payload = json.loads(proc.stdout)
        assert "render" in payload
        assert payload["render"]["v1136_score"] > 0.0
        assert payload["render"]["render_path"] == "v1136_real"

    def test_cli_json_with_bench(self):
        proc = self._run("--json", "--bench", "--trials", "10",
                         "--bench-iterations", "2", timeout=180)
        assert proc.returncode == 0, proc.stderr
        payload = json.loads(proc.stdout)
        assert "bench" in payload
        assert "bench_loop" in payload
        b = payload["bench"]["combined"]
        assert b["p50_s"] >= 0.0
        assert b["p95_s"] >= b["p50_s"] - 1e-9
        assert b["p99_s"] >= b["p95_s"] - 1e-9

    def test_cli_report(self):
        proc = self._run("--report", "--bench", "--trials", "10", timeout=180)
        assert proc.returncode == 0, proc.stderr
        assert proc.stdout is not None
        assert "R11 V1136 → Dashboard Perf Report" in proc.stdout
        assert "cold p50/p95/p99" in proc.stdout
        assert "warm p50/p95/p99" in proc.stdout

    def test_cli_writes_markdown(self, tmp_path):
        out_path = tmp_path / "dash.md"
        proc = self._run("--write", str(out_path))
        assert proc.returncode == 0, proc.stderr
        assert out_path.exists()
        content = out_path.read_text(encoding="utf-8")
        assert "R11 V1136 → Dashboard" in content
        assert "Continuity" in content