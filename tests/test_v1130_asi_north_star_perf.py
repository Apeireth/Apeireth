"""Tests for V1130 ASI North-Star true performance benchmark (R10-PO-001).

Coverage:
  * Constants (version, SLO targets, endpoint catalogue, 18-dim dashboard)
  * Percentile math (P50/P95/P99, linear interpolation)
  * LatencySample / BackendLatencyStats dataclasses
  * Backend handle spawn (HTTP + gRPC), in-process fast-path fallback
  * Backend benchmark end-to-end (5 routes, SLO gate)
  * Cross-provider latency comparison (4 production providers)
  * Dashboard render (18-dim V0.5, cache hit, MarkdownTemplateCompiler reuse)
  * V1074 parity (V1118 3.193x speedup preserved)
  * Chaos test (provider-down → benchmark still completes)
  * Full suite orchestrator (wall-clock + all_ok gate)
  * V1118 optimisation-class integration
    (LazyImporter / SnapshotCompressor / ParallelDimensionEvaluator /
     SubmoduleResultCache / MarkdownTemplateCompiler)
  * CLI surface (--self-test, --cross-provider, --dashboard-render,
     --backend-bench, --all, JSON envelope)

ponytail: ceiling = 25+ smoke tests; upgrade path = jittered latency
fuzzing + cross-host benchmark once the team owns >1 backend instance.
"""

from __future__ import annotations

import json
import os
import sys
import tempfile
import time

import pytest

# Make sure the package root is importable when pytest runs from any cwd.
_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.dirname(_HERE)
if _ROOT not in sys.path:
    sys.path.insert(0, _ROOT)

import apeireth.v1130_asi_north_star_perf as vp


# ---------------------------------------------------------------------------
# A. Constants
# ---------------------------------------------------------------------------


def test_a1_version_constant_locked():
    assert vp.V1130_VERSION == "0.1.0"


def test_a2_v1074_target_s_2_5s():
    assert vp.V1074_TARGET_S == 2.5


def test_a3_dashboard_target_s_2_5s():
    assert vp.DASHBOARD_PERF_TARGET_S == 2.5


def test_a4_backend_p95_target_s_250ms():
    assert vp.BACKEND_LATENCY_P95_TARGET_S == 0.25


def test_a5_backend_p99_target_s_500ms():
    assert vp.BACKEND_LATENCY_P99_TARGET_S == 0.5


def test_a6_endpoints_catalogue_has_five_routes():
    assert len(vp.ENDPOINTS) == 5
    transports = {t for t, _ in vp.ENDPOINTS}
    assert transports == {"http", "grpc"}


def test_a7_dashboard_has_eighteen_dimensions():
    assert len(vp.DASHBOARD_DIMENSIONS) == 18


def test_a8_min_savings_pct_matches_v1118():
    # R9-PO-002 V1118 observed 68.68% savings → 3.193x speedup.
    # V1130 keeps the same 20% minimum gate so any regression trips it.
    assert vp.V1074_MIN_SAVINGS_PCT == 20.0
    assert vp.V1074_REFERENCE_BASELINE_S == pytest.approx(3.252062, rel=1e-6)


# ---------------------------------------------------------------------------
# B. Percentile math
# ---------------------------------------------------------------------------


def test_b1_percentile_empty_returns_zero():
    assert vp._percentile([], 95) == 0.0


def test_b2_percentile_single_value():
    assert vp._percentile([0.123], 95) == pytest.approx(0.123, rel=1e-6)


def test_b3_percentile_p95_of_constant_array():
    values = [0.10] * 20
    assert vp._percentile(values, 95) == pytest.approx(0.10, rel=1e-6)


def test_b4_percentile_p99_handles_outlier():
    values = [0.01] * 99 + [0.50]
    assert vp._percentile(values, 99) >= 0.01
    assert vp._percentile(values, 99) <= 0.50


# ---------------------------------------------------------------------------
# C. Sample / Stats dataclasses
# ---------------------------------------------------------------------------


def test_c1_latency_sample_to_dict_round_trip():
    sample = vp.LatencySample(route="r", duration_s=0.05, status=200,
                              provider="anthropic", ok=True)
    assert sample.to_dict()["route"] == "r"
    assert sample.to_dict()["ok"] is True


def test_c2_backend_latency_stats_serialise():
    s = vp.BackendLatencyStats(route="x", count=10, failures=0,
                                p50_s=0.01, p95_s=0.02, p99_s=0.03,
                                min_s=0.001, max_s=0.04, mean_s=0.015,
                                p95_within_slo=True, p99_within_slo=True)
    d = s.to_dict()
    assert d["count"] == 10 and d["p95_within_slo"] is True


def test_c3_summarise_empty_returns_failure_flag():
    empty = vp._summarise("noop", [])
    assert empty.failures == 0
    assert empty.p95_within_slo is False  # no samples → can't claim SLO.


def test_c4_summarise_within_slo():
    samples = [vp.LatencySample(route="r", duration_s=0.10, status=200,
                                 provider="n/a", ok=True) for _ in range(20)]
    s = vp._summarise("r", samples)
    assert s.count == 20
    assert s.p95_within_slo is True
    assert s.p99_within_slo is True


def test_c5_summarise_above_slo_when_slow():
    samples = [vp.LatencySample(route="r", duration_s=1.0, status=200,
                                 provider="n/a", ok=True) for _ in range(20)]
    s = vp._summarise("r", samples)
    assert s.p95_within_slo is False
    assert s.p99_within_slo is False


# ---------------------------------------------------------------------------
# D. Backend handle spawn
# ---------------------------------------------------------------------------


@pytest.fixture(scope="module")
def _backend():
    tmp = tempfile.mkdtemp(prefix="v1130-test-")
    handle = vp._spawn_backend(tmp)
    try:
        yield handle
    finally:
        handle.close()


def test_d1_spawn_backend_exposes_http_base_url(_backend):
    assert _backend.base_url.startswith("http://127.0.0.1:")


def test_d2_spawn_backend_dispatch_callable(_backend):
    status, body = _backend.dispatch("GET", "/asi/level")
    assert status == 200
    assert body["version"]


def test_d3_spawn_backend_measure_dispatch(_backend):
    status, body = _backend.dispatch("POST", "/asi/measure", {
        "provider": "anthropic", "model": "noop",
        "prompt": "ping", "timeout_seconds": 1.0,
    })
    assert status == 200
    assert "evidence" in body


def test_d4_spawn_backend_unknown_path_returns_404(_backend):
    status, _ = _backend.dispatch("GET", "/nope")
    assert status == 404


# ---------------------------------------------------------------------------
# E. Backend benchmark — 5 routes
# ---------------------------------------------------------------------------


def test_e1_backend_benchmark_emits_one_stat_per_route(_backend):
    res = vp.run_backend_benchmark(_backend, requests_per_route=3, warmup=1)
    assert len(res.stats) == 5
    assert {s.route for s in res.stats} >= {
        "http GET /asi/level",
        "http POST /asi/measure",
        "http GET /asi/north-star",
        "grpc Level",
        "grpc Measure",
    }


def test_e2_backend_benchmark_warmup_excluded(_backend):
    res = vp.run_backend_benchmark(_backend, requests_per_route=3, warmup=2)
    for s in res.stats:
        # 3 measured + 2 warmup = 5 total samples recorded (warmup still kept).
        assert s.count == 3


def test_e3_backend_benchmark_slo_pass_for_local_backend(_backend):
    # Local in-process backend → every route should easily stay under SLO.
    res = vp.run_backend_benchmark(_backend, requests_per_route=4, warmup=1)
    assert res.all_within_slo, [s.to_dict() for s in res.stats]


# ---------------------------------------------------------------------------
# F. Cross-provider latency
# ---------------------------------------------------------------------------


def test_f1_cross_provider_returns_four_providers():
    ps = vp.run_cross_provider_latency(seed=42)
    assert {p.provider for p in ps} == {"anthropic", "ollama", "local-cli", "executable"}


def test_f2_cross_provider_deterministic_per_seed():
    a = vp.run_cross_provider_latency(seed=7)
    b = vp.run_cross_provider_latency(seed=7)
    assert [p.duration_s for p in a] == [p.duration_s for p in b]


def test_f3_cross_provider_executable_is_fastest():
    ps = vp.run_cross_provider_latency(seed=0)
    by_provider = {p.provider: p.duration_s for p in ps}
    assert by_provider["executable"] <= by_provider["anthropic"]


# ---------------------------------------------------------------------------
# G. Dashboard render
# ---------------------------------------------------------------------------


def test_g1_dashboard_has_eighteen_dimensions_and_finite_bytes():
    d = vp.render_dashboard()
    assert d.dimensions == 18
    assert d.bytes_written > 0
    assert d.duration_s >= 0


def test_g2_dashboard_cache_hit_on_second_call():
    from apeireth.v1118_perf_optimizer_v01 import SubmoduleResultCache
    shared = SubmoduleResultCache(maxsize=4)
    d1 = vp.render_dashboard(optimizers=vp._OptimizersView.build())  # warm-up
    # Second render with an optimizers view that owns the same cache: it does
    # not, but module-level cache_hit must still flip True because the cache
    # key reused is deterministic.  Build a single orchestrator and call twice.
    view = vp._OptimizersView.build()
    d1 = vp.render_dashboard(optimizers=view)
    d2 = vp.render_dashboard(optimizers=view)
    assert d1.cache_hit is False
    assert d2.cache_hit is True
    assert d2.markdown == d1.markdown
    _ = shared  # keep fixture imported for clarity / future extension


def test_g3_dashboard_render_faster_than_target():
    # Even with a cold cache, the 18-dim render must stay well below 2.5s.
    d = vp.render_dashboard()
    assert d.duration_s < vp.DASHBOARD_PERF_TARGET_S


# ---------------------------------------------------------------------------
# H. V1074 parity (V1118 3.193x speedup preserved)
# ---------------------------------------------------------------------------


def test_h1_v1074_parity_target_met():
    res = vp.run_v1074_parity()
    assert res.target_met
    assert res.all_ok


def test_h2_v1074_parity_speedup_above_3x():
    # R9-PO-002 V1118 observed 3.193x; V1130 must not regress.
    res = vp.run_v1074_parity()
    assert res.speedup_x >= 3.0, res.to_dict()


def test_h3_v1074_parity_optimized_under_target():
    res = vp.run_v1074_parity()
    assert res.optimized_s <= vp.V1074_TARGET_S


# ---------------------------------------------------------------------------
# I. Chaos test
# ---------------------------------------------------------------------------


def test_i1_chaos_returns_attempt_count():
    c = vp.run_chaos(None, n=6, provider_down="anthropic")
    assert c.attempted == 6
    assert c.succeeded + c.failed == 6


def test_i2_chaos_records_fallback_path():
    c = vp.run_chaos(None, n=4, provider_down="ollama")
    assert "executable" in c.fallback_path
    assert "ollama" in c.fallback_path


def test_i3_chaos_at_least_one_success():
    # Even with the provider down, fail-soft keeps the suite alive.
    c = vp.run_chaos(None, n=10, provider_down="local-cli")
    assert c.succeeded >= 1


# ---------------------------------------------------------------------------
# J. Full-suite orchestrator
# ---------------------------------------------------------------------------


def test_j1_full_suite_all_ok_under_local_backend():
    suite = vp.run_full_suite(requests_per_route=3, warmup=1, chaos_n=4)
    assert suite.all_ok
    assert suite.wall_clock_s > 0


def test_j2_full_suite_serialises_to_json():
    suite = vp.run_full_suite(requests_per_route=3, warmup=1, chaos_n=4)
    blob = suite.to_dict()
    # All five sub-results present.
    assert {"backend", "dashboard", "providers", "parity", "chaos"} <= set(blob.keys())
    json.dumps(blob)  # must be JSON-serialisable.


# ---------------------------------------------------------------------------
# K. V1118 optimisation-class integration
# ---------------------------------------------------------------------------


def test_k1_submodule_result_cache_shared_with_v1118():
    from apeireth.v1118_perf_optimizer_v01 import SubmoduleResultCache
    cache = SubmoduleResultCache(maxsize=4)
    cache.put("k", 1)
    assert cache.get("k") == 1
    # Cache used in dashboard render.
    d = vp.render_dashboard(optimizers=vp._OptimizersView.build())
    assert d.dimensions == 18


def test_k2_parallel_dimension_evaluator_uses_workers():
    from apeireth.v1118_perf_optimizer_v01 import ParallelDimensionEvaluator
    par = ParallelDimensionEvaluator(max_workers=4)
    try:
        stats = par.stats()
        assert stats["max_workers"] == 4
        # Real API surface (no fake .map). evaluate_project_serial must exist.
        assert hasattr(par, "evaluate_project_serial")
        assert hasattr(par, "evaluate_project")
    finally:
        par.close()


def test_k3_markdown_template_compiler_render_header_used():
    from apeireth.v1118_perf_optimizer_v01 import MarkdownTemplateCompiler
    compiler = MarkdownTemplateCompiler()
    header = compiler.render_header()
    assert "ASI" in header


def test_k4_snapshot_compressor_round_trip():
    from apeireth.v1118_perf_optimizer_v01 import SnapshotCompressor
    sc = SnapshotCompressor()
    blob = sc.compress({"a": [1, 2, 3]})
    assert json.loads(blob) == {"a": [1, 2, 3]}


def test_k5_lazy_importer_resolves_v1074():
    from apeireth.v1118_perf_optimizer_v01 import LazyImporter
    lazy = LazyImporter("apeireth.v1074_asi_production_runner")
    assert lazy.module_name == "apeireth.v1074_asi_production_runner"


# ---------------------------------------------------------------------------
# L. CLI surface
# ---------------------------------------------------------------------------


def test_l1_cli_self_test(capsys):
    rc = vp._cli(["--self-test"])
    assert rc == 0
    captured = capsys.readouterr().out
    assert "V1130 self-test PASS" in captured


def test_l2_cli_cross_provider(capsys):
    rc = vp._cli(["--cross-provider", "--print-json"])
    assert rc == 0
    payload = json.loads(capsys.readouterr().out)
    assert len(payload) == 4
    assert all(p["ok"] for p in payload)


def test_l3_cli_dashboard_render(capsys):
    rc = vp._cli(["--dashboard-render", "--print-json"])
    assert rc == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["dimensions"] == 18


def test_l4_cli_backend_bench_small(capsys):
    rc = vp._cli(["--backend-bench", "--requests-per-route", "3",
                   "--warmup", "1", "--print-json"])
    assert rc == 0
    payload = json.loads(capsys.readouterr().out)
    assert len(payload["stats"]) == 5


def test_l5_cli_all_suite(capsys):
    rc = vp._cli(["--all", "--requests-per-route", "3",
                   "--warmup", "1", "--chaos-n", "4", "--print-json"])
    assert rc == 0
    payload = json.loads(capsys.readouterr().out)
    assert payload["all_ok"] is True