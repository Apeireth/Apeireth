"""R9-PO-002 V1118 true performance optimizer tests.

The suite deliberately checks semantic equivalence as well as timing evidence.  It
contains no sleep-based speedups and uses only stdlib timers/real work.
"""

from __future__ import annotations

import importlib
import json
import math
import os
import statistics
import sys
import time
from pathlib import Path
from types import SimpleNamespace
from typing import Any, Dict, List

import pytest

PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROJECT_ROOT))

from apeireth.v1074_asi_production_runner import (  # noqa: E402
    MarkdownReportGenerator,
    ProductionRunner,
    REFERENCES,
    StatusSnapshot,
    V1074_VERSION,
)
from apeireth.v1118_perf_optimizer_v01 import (  # noqa: E402
    DimensionJob,
    LazyImporter,
    MarkdownTemplateCompiler,
    ParallelDimensionEvaluator,
    ProjectMetrics,
    SnapshotCompressor,
    SubmoduleResultCache,
    V1074_MIN_SAVINGS_PCT,
    V1074_TARGET_S,
    V1118_LRU_MAXSIZE,
    V1118_PARALLEL_WORKERS,
    V1118OptimizedRunner,
    V1118Optimizers,
    _execute_dimension_job,
    _cli,
    project_state_token,
)


def _snapshot(history: bool = False) -> StatusSnapshot:
    score_history: List[Dict[str, Any]] = []
    if history:
        score_history = [
            {
                "snapshot_id": "snap_history_0001",
                "ts_iso": "2026-07-29T00:00:00+00:00",
                "v03_score": 0.88,
            },
            {
                "snapshot_id": "snap_history_0002",
                "ts_iso": "2026-07-29T01:00:00+00:00",
                "v03_score": 0.89,
            },
        ]
    return StatusSnapshot(
        snapshot_id="snap_v1118_test",
        ts=1_785_283_200.0,
        ts_iso="2026-07-29T00:00:00+00:00",
        version=V1074_VERSION,
        level="ASI",
        level_score=0.89,
        v02_base=0.89,
        v03_score=0.89,
        n_modules=1_127,
        n_tests=5_408,
        n_commits=471,
        dim_breakdown={
            "phi_proxy": 1.0,
            "capabilities": 0.7513,
            "cross_domain": 1.0,
            "engineering": 1.0,
            "vcp_4": 0.98,
            "v2_philosophy": 0.87,
            "rubric_open": 1.0,
            "real_production": 0.942,
            "cognitive_core": 0.75,
            "self_organizing_core": 0.82,
            "plugin_core": 0.84,
            "self_improving_core": 0.81,
            "neurosymbolic": 0.80,
            "world_model": 0.73,
            "reinforcement_learning": 0.89,
            "scientific_method": 0.95,
            "eternal_identity": 0.86,
        },
        v1071_vcp_score=0.98,
        v1071_cross_domain=1.0,
        v1072_eternal_identity=0.86,
        philosophy_guard_ok=True,
        score_history=score_history,
        notes={"build_ts": "2026-07-29T00:00:00+00:00"},
        refs=REFERENCES,
    )


def _measurement() -> Dict[str, float]:
    return {
        "v02_base": 0.89,
        "v1071_vcp_score": 0.98,
        "v1071_cross_domain_score": 1.0,
        "v1072_eternal_identity_score": 0.86,
        "v03_score": 0.89,
    }


def _make_project(root: Path, test_count: int = 2) -> Path:
    (root / "apeireth").mkdir(parents=True)
    (root / "tests").mkdir(parents=True)
    (root / "apeireth" / "v1.py").write_text("VALUE = 1\n", encoding="utf-8")
    (root / "apeireth" / "v2.py").write_text("VALUE = 2\n", encoding="utf-8")
    tests = "\n".join(f"def test_{index}():\n    assert True\n" for index in range(test_count))
    (root / "tests" / "test_v1.py").write_text(tests, encoding="utf-8")
    return root


def _runner_with_fast_measurement(root: Path) -> ProductionRunner:
    runner = ProductionRunner(project_dir=str(root))
    runner.builder.measure_v03 = _measurement
    return runner


# ---------------------------------------------------------------------------
# Optimizer 1 — deferred import
# ---------------------------------------------------------------------------

class TestLazyImporter:
    def test_starts_unresolved(self) -> None:
        lazy = LazyImporter("math")
        assert lazy.stats()["resolved"] is False

    def test_resolves_module(self) -> None:
        lazy = LazyImporter("math")
        assert lazy.get() is math

    def test_resolves_attribute(self) -> None:
        lazy = LazyImporter("math", "sqrt")
        assert lazy.get()(81) == 9

    def test_resolves_only_once(self) -> None:
        lazy = LazyImporter("math", "sqrt")
        first = lazy.get()
        second = lazy.get()
        assert first is second
        assert lazy.stats()["resolve_count"] == 1
        assert lazy.stats()["get_count"] == 2

    def test_reset_requires_new_resolution(self) -> None:
        lazy = LazyImporter("math")
        lazy.get()
        lazy.reset()
        assert lazy.stats()["resolved"] is False
        lazy.get()
        assert lazy.stats()["resolve_count"] == 1

    def test_empty_module_rejected(self) -> None:
        with pytest.raises(ValueError):
            LazyImporter("")

    def test_bad_module_error_is_not_hidden(self) -> None:
        lazy = LazyImporter("apeireth.this_module_does_not_exist")
        with pytest.raises(ModuleNotFoundError):
            lazy.get()

    def test_benchmark_is_semantically_equal_and_faster(self) -> None:
        result = LazyImporter("math", "sqrt").benchmark(trials=3_000)
        assert result.semantics_equal is True
        assert result.optimized_s < result.baseline_s
        assert result.speedup > 1.0


# ---------------------------------------------------------------------------
# Optimizer 2 — compact JSON
# ---------------------------------------------------------------------------

class TestSnapshotCompressor:
    def test_round_trip_unicode(self) -> None:
        obj = {"平台": "阿佩瑞斯", "values": [1, 2, 3]}
        compact = SnapshotCompressor().compress(obj)
        assert json.loads(compact) == obj

    def test_removes_redundant_separators(self) -> None:
        compact = SnapshotCompressor().compress({"a": 1, "b": [2, 3]})
        assert ": " not in compact
        assert ", " not in compact

    def test_is_smaller_than_pretty_snapshot(self) -> None:
        obj = _snapshot(history=True).to_dict()
        pretty = json.dumps(obj, ensure_ascii=False, default=str, indent=2)
        compact = SnapshotCompressor().compress(obj)
        assert len(compact.encode("utf-8")) < len(pretty.encode("utf-8"))

    def test_stats_track_encoded_bytes(self) -> None:
        compressor = SnapshotCompressor()
        payload = compressor.compress({"平台": "阿佩瑞斯"}, baseline_bytes=100)
        stats = compressor.stats()
        assert stats["compress_count"] == 1
        assert stats["output_bytes"] == len(payload.encode("utf-8"))
        assert stats["observed_pretty_bytes"] == 100
        assert 0 < stats["ratio"] < 1

    def test_default_str_preserves_non_json_object(self) -> None:
        compact = SnapshotCompressor().compress({"path": Path("a/b")})
        assert json.loads(compact)["path"] == str(Path("a/b"))

    def test_benchmark_reports_equal_and_smaller_payload(self) -> None:
        result = SnapshotCompressor().benchmark(_snapshot(history=True).to_dict(), trials=50)
        assert result.semantics_equal is True
        assert "bytes" in result.notes
        assert result.optimized_s < result.baseline_s


# ---------------------------------------------------------------------------
# Optimizer 3 — two-worker multiprocessing
# ---------------------------------------------------------------------------

class TestParallelDimensionEvaluator:
    def test_required_worker_count_is_two(self) -> None:
        assert V1118_PARALLEL_WORKERS == 2
        evaluator = ParallelDimensionEvaluator()
        assert evaluator.max_workers == 2
        evaluator.close()

    def test_invalid_worker_count_rejected(self) -> None:
        with pytest.raises(ValueError):
            ParallelDimensionEvaluator(0)

    def test_serial_project_metrics_are_real(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path, test_count=3)
        evaluator = ParallelDimensionEvaluator()
        assert evaluator.evaluate_project_serial(str(root)) == ProjectMetrics(2, 3, 0)
        evaluator.close()

    def test_parallel_equals_serial(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path, test_count=7)
        evaluator = ParallelDimensionEvaluator()
        try:
            serial = evaluator.evaluate_project_serial(str(root))
            parallel = evaluator.evaluate_project(str(root))
            assert parallel == serial == ProjectMetrics(2, 7, 0)
            assert evaluator.stats()["parallel_runs"] == 1
            assert evaluator.stats()["fallbacks"] == 0
        finally:
            evaluator.close()

    def test_unknown_dimension_is_rejected(self, tmp_path: Path) -> None:
        job = DimensionJob("bad", "not-an-operation", str(tmp_path))
        with pytest.raises(ValueError):
            _execute_dimension_job(job)

    def test_close_is_idempotent(self) -> None:
        evaluator = ParallelDimensionEvaluator()
        evaluator.close()
        evaluator.close()

    def test_stats_distinguish_serial_and_parallel(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path, test_count=1)
        evaluator = ParallelDimensionEvaluator()
        evaluator.evaluate_project_serial(str(root))
        stats = evaluator.stats()
        assert stats["serial_runs"] == 1
        assert stats["parallel_runs"] == 0
        evaluator.close()

    def test_project_benchmark_has_semantic_guard(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path, test_count=30)
        evaluator = ParallelDimensionEvaluator()
        try:
            result = evaluator.benchmark_project(str(root), trials=2)
            assert result.semantics_equal is True
            assert result.trials == 2
        finally:
            evaluator.close()


# ---------------------------------------------------------------------------
# Optimizer 4 — true LRU cache
# ---------------------------------------------------------------------------

class TestSubmoduleResultCache:
    def test_default_capacity_is_32(self) -> None:
        cache = SubmoduleResultCache()
        assert cache.maxsize == V1118_LRU_MAXSIZE == 32

    def test_invalid_capacity_rejected(self) -> None:
        with pytest.raises(ValueError):
            SubmoduleResultCache(0)

    def test_miss_returns_default(self) -> None:
        cache = SubmoduleResultCache()
        assert cache.get("missing", 42) == 42
        assert cache.stats()["misses"] == 1

    def test_put_then_hit(self) -> None:
        cache = SubmoduleResultCache()
        cache.put("v1073", {"score": 0.89})
        assert cache.get("v1073") == {"score": 0.89}
        assert cache.stats()["hits"] == 1

    def test_true_lru_read_changes_eviction_order(self) -> None:
        cache = SubmoduleResultCache(maxsize=2)
        cache.put("a", 1)
        cache.put("b", 2)
        assert cache.get("a") == 1  # a is now MRU; b must be evicted
        cache.put("c", 3)
        assert cache.get("b") is None
        assert cache.get("a") == 1
        assert cache.get("c") == 3
        assert cache.stats()["evictions"] == 1

    def test_update_does_not_evict(self) -> None:
        cache = SubmoduleResultCache(maxsize=2)
        cache.put("a", 1)
        cache.put("a", 2)
        assert cache.get("a") == 2
        assert cache.stats()["evictions"] == 0

    def test_get_or_compute_calls_factory_once(self) -> None:
        cache = SubmoduleResultCache()
        calls = []

        def factory() -> int:
            calls.append(1)
            return 99

        assert cache.get_or_compute("key", factory) == 99
        assert cache.get_or_compute("key", factory) == 99
        assert len(calls) == 1

    def test_none_is_a_cacheable_value(self) -> None:
        cache = SubmoduleResultCache()
        calls = []

        def factory() -> None:
            calls.append(1)
            return None

        cache.get_or_compute("none", factory)
        cache.get_or_compute("none", factory)
        assert len(calls) == 1

    def test_clear_resets_values_and_counters(self) -> None:
        cache = SubmoduleResultCache()
        cache.put("a", 1)
        cache.get("a")
        cache.get("b")
        cache.clear()
        assert cache.keys() == ()
        assert cache.stats()["hits"] == 0
        assert cache.stats()["misses"] == 0

    def test_cache_benchmark_uses_real_cpu_work(self) -> None:
        cache = SubmoduleResultCache()

        def factory() -> int:
            return sum(index * index for index in range(20_000))

        result = cache.benchmark("cpu", factory, trials=5)
        assert result.semantics_equal is True
        assert result.optimized_s < result.baseline_s
        assert result.speedup > 2.0


# ---------------------------------------------------------------------------
# Optimizer 5 — precompiled Markdown
# ---------------------------------------------------------------------------

class TestMarkdownTemplateCompiler:
    def test_header_and_footer_are_precompiled(self) -> None:
        compiler = MarkdownTemplateCompiler()
        assert compiler.render_header() == "# ASI Status Report\n\n"
        assert compiler.render_footer().endswith("V1074 Production Runner._\n")

    def test_summary_template(self) -> None:
        assert MarkdownTemplateCompiler.render_summary_item("key", "value") == "- **key**: value\n"

    def test_dimension_template(self) -> None:
        assert MarkdownTemplateCompiler.render_dim_row("phi", 0.98765) == "| phi | 0.9877 |\n"

    def test_history_template(self) -> None:
        row = MarkdownTemplateCompiler.render_history_row("snap", "now", 0.89123)
        assert row == "| snap | now | 0.8912 |\n"

    @pytest.mark.parametrize("history", [False, True])
    def test_render_is_byte_equal_to_v1074(self, history: bool) -> None:
        snapshot = _snapshot(history=history)
        baseline = MarkdownReportGenerator().render(snapshot)
        optimized = MarkdownTemplateCompiler().render(snapshot)
        assert optimized == baseline

    def test_reference_block_is_cached(self) -> None:
        compiler = MarkdownTemplateCompiler()
        snapshot = _snapshot()
        compiler.render(snapshot)
        compiler.render(snapshot)
        assert compiler.stats()["reference_cache_hits"] == 1

    def test_template_benchmark_has_byte_equality(self) -> None:
        snapshot = _snapshot(history=True)
        baseline = MarkdownReportGenerator().render
        result = MarkdownTemplateCompiler().benchmark(snapshot, baseline, trials=100)
        assert result.semantics_equal is True
        assert result.trials == 100


# ---------------------------------------------------------------------------
# Orchestrator, integration and V3 truth guards
# ---------------------------------------------------------------------------

class TestV1118Optimizers:
    def test_optimizer_names_are_exactly_five(self) -> None:
        assert V1118Optimizers.OPT_NAMES == ("lazy", "compress", "parallel", "cache", "template")

    def test_optimizers_start_disabled(self) -> None:
        opt = V1118Optimizers()
        assert not any(opt.enabled.values())
        opt.close()

    def test_enable_disable_and_enable_all(self) -> None:
        opt = V1118Optimizers()
        assert opt.enable("cache").is_enabled("cache") is True
        assert opt.disable("cache").is_enabled("cache") is False
        opt.enable_all()
        assert all(opt.enabled.values())
        opt.disable_all()
        assert not any(opt.enabled.values())
        opt.close()

    def test_unknown_optimizer_is_rejected(self) -> None:
        opt = V1118Optimizers()
        with pytest.raises(ValueError):
            opt.enable("pretend-speedup")
        with pytest.raises(ValueError):
            opt.is_enabled("pretend-speedup")
        opt.close()

    def test_wrap_is_idempotent(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path)
        runner = _runner_with_fast_measurement(root)
        opt = V1118Optimizers()
        first_build = opt.wrap(runner).builder.build
        second_build = opt.wrap(runner).builder.build
        assert first_build is second_build
        opt.close()

    def test_unwrap_restores_instance_methods(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path)
        runner = _runner_with_fast_measurement(root)
        original = runner.builder.build
        opt = V1118Optimizers().enable_all()
        opt.wrap(runner)
        assert runner.builder.build is not original
        opt.unwrap(runner)
        restored = runner.builder.build
        assert restored.__self__ is original.__self__
        assert restored.__func__ is original.__func__
        opt.close()

    def test_two_owners_cannot_wrap_same_runner(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path)
        runner = _runner_with_fast_measurement(root)
        first = V1118Optimizers()
        second = V1118Optimizers()
        first.wrap(runner)
        with pytest.raises(RuntimeError):
            second.wrap(runner)
        first.close()
        second.close()

    def test_cache_reuses_metrics_but_remeasures_dynamic_score(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path)
        runner = ProductionRunner(project_dir=str(root))
        calls: List[int] = []

        def measured() -> Dict[str, float]:
            calls.append(1)
            return _measurement()

        runner.builder.measure_v03 = measured
        opt = V1118Optimizers().enable("cache")
        opt.wrap(runner)
        first = runner.builder.build()
        second = runner.builder.build()
        assert first.v03_score == second.v03_score == 0.89
        assert len(calls) == 2  # dynamic V1073/V1072 is never frozen
        assert opt.parallel.stats()["serial_runs"] == 1
        assert opt.cache.stats()["hits"] == 1
        assert opt.cache.keys()[0][0] == "project_metrics"
        opt.close()

    def test_cache_state_token_invalidates_on_test_change(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path)
        before = project_state_token(str(root))
        tests = root / "tests" / "test_v1.py"
        tests.write_text(tests.read_text(encoding="utf-8") + "\ndef test_new():\n    assert True\n", encoding="utf-8")
        os.utime(tests, None)
        after = project_state_token(str(root))
        assert after != before

    def test_cache_state_token_ignores_non_metric_source_body(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path)
        before = project_state_token(str(root))
        source = root / "apeireth" / "v1.py"
        source.write_text("VALUE = 100\n", encoding="utf-8")
        os.utime(source, None)
        assert project_state_token(str(root)) == before

    def test_lazy_fast_path_falls_back_below_saturation(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path, test_count=3)
        runner = _runner_with_fast_measurement(root)
        opt = V1118Optimizers().enable("lazy")
        opt.wrap(runner)
        snapshot = runner.builder.build()
        assert snapshot.v03_score == 0.89
        assert opt.fast_path_runs == 0
        assert opt.fast_path_fallbacks == 1
        opt.close()

    def test_parallel_counts_feed_original_snapshot_builder(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path, test_count=9)
        runner = _runner_with_fast_measurement(root)
        opt = V1118Optimizers().enable("parallel")
        try:
            opt.wrap(runner)
            snapshot = runner.builder.build()
            assert snapshot.n_modules == 2
            assert snapshot.n_tests == 9
            assert snapshot.n_commits == 0
            assert snapshot.v03_score == 0.89
        finally:
            opt.close()

    def test_compressed_writer_keeps_json_semantics(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path)
        runner = _runner_with_fast_measurement(root)
        opt = V1118Optimizers().enable("compress")
        opt.wrap(runner)
        result = runner.run(write_artifacts=True)
        payload = (root / "artifacts" / "asi_snapshot.json").read_text(encoding="utf-8")
        parsed = json.loads(payload)
        assert result.all_ok is True
        assert parsed["v03_score"] == 0.89
        assert "\n" not in payload
        opt.close()

    def test_template_wrapper_preserves_full_report(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path)
        runner = _runner_with_fast_measurement(root)
        snapshot = runner.builder.build()
        expected = runner.reporter.render(snapshot)
        opt = V1118Optimizers().enable("template")
        opt.wrap(runner)
        assert runner.reporter.render(snapshot) == expected
        opt.close()

    def test_dynamic_disable_uses_original_behavior(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path)
        runner = _runner_with_fast_measurement(root)
        snapshot = runner.builder.build()
        original = runner.reporter.render(snapshot)
        opt = V1118Optimizers().enable("template")
        opt.wrap(runner)
        opt.disable("template")
        assert runner.reporter.render(snapshot) == original
        assert opt.template.stats()["render_count"] == 0
        opt.close()

    def test_bench_returns_visible_before_after_arrays(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path, test_count=4)

        def factory() -> ProductionRunner:
            return _runner_with_fast_measurement(root)

        opt = V1118Optimizers()
        try:
            result = opt.bench(factory, n_trials=1, write_artifacts=False)
            assert len(result.runs_baseline) == 1
            assert len(result.runs_optimized) == 1
            assert result.baseline_scores == [0.89]
            assert result.optimized_scores == [0.89]
            assert result.all_runs_ok is True
        finally:
            opt.close()

    def test_real_v1074_optimized_target_and_truth_guard(self) -> None:
        opt = V1118Optimizers().enable_all()
        times: List[float] = []
        results = []
        try:
            for _ in range(3):
                runner = opt.wrap(ProductionRunner(project_dir=str(PROJECT_ROOT)))
                started = time.perf_counter()
                result = runner.run(write_artifacts=False)
                times.append(time.perf_counter() - started)
                results.append(result)
                opt.unwrap(runner)
            assert statistics.median(times) < V1074_TARGET_S
            assert all(result.all_ok for result in results)
            assert all(0.80 <= result.v03_score < 1.0 for result in results)
            assert opt.fast_path_runs >= 1
            assert opt.last_score_overrides["phi_proxy"] == 1.0
            assert opt.cache.stats()["hits"] >= 1
        finally:
            opt.close()

    def test_optimized_runner_defaults_to_all_enabled(self, tmp_path: Path) -> None:
        root = _make_project(tmp_path)
        optimized = V1118OptimizedRunner(project_dir=str(root))
        try:
            assert all(optimized.opt.enabled.values())
        finally:
            optimized.close()

    def test_cli_self_test(self, capsys: pytest.CaptureFixture[str]) -> None:
        assert _cli(["--self-test"]) == 0
        assert "PASS" in capsys.readouterr().out

    def test_required_thresholds_are_not_weakened(self) -> None:
        assert V1074_TARGET_S == 2.5
        assert V1074_MIN_SAVINGS_PCT == 20.0


def test_compatibility_module_exports_orchestrator() -> None:
    compatibility = importlib.import_module("apeireth.v1118_performance_optimization")
    assert compatibility.V1118Optimizers is V1118Optimizers
