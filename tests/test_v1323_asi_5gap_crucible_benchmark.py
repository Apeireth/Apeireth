"""V1323 ASI 5-Gap Crucible Real Benchmark — Popper self-tests.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 17:55 +08:00 2026-08-08)
> **Trigger**: post-V1322 chain — V1323 = 22-sample real benchmark of V1322 Crucible
> **目标**: 50+ Popper self-tests covering benchmark runner + stats + coverage + edges + bridge

50 tests organized in 7 sections:
1. BENCHMARK_QUERIES invariants (8 tests)
2. BenchmarkRunner (8 tests)
3. DimensionStats / _percentile (8 tests)
4. CoverageReport (6 tests)
5. EdgeCaseReport (6 tests)
6. BenchmarkAggregate (8 tests)
7. V1323Bridge (6 tests)
"""
from __future__ import annotations

import json
import math
import statistics

import pytest

from apeireth.v1322_asi_5gap_crucible import (
    ASI_5_GAPS,
    ASI_ANCHORS,
    ASII5GapCrucible,
    CROSS_GAP_CELLS,
    CrucibleResult,
    V3_GUARD_MARKERS,
)
from apeireth.v1323_asi_5gap_crucible_benchmark import (
    ASI_ANCHORS_V1323,
    BENCHMARK_QUERIES,
    BenchmarkAggregate,
    BenchmarkRunner,
    CoverageReport,
    DimensionStats,
    EdgeCaseReport,
    QueryResult,
    SUBSTRATE_CHAIN_V1323,
    V1323Bridge,
    V1323_VERSION,
    V3_GUARD_MARKERS_V1323,
    _assert_benchmark_queries_locked,
    _percentile,
    _self_test,
    build_aggregate,
    build_bridge,
    compute_coverage,
    compute_dimension_stats,
    compute_edge_cases,
    main,
)


# ============================================================================
# Section 1: BENCHMARK_QUERIES invariants — 8 tests
# ============================================================================


class TestBenchmarkQueries:
    def test_n_queries_locked_22(self):
        assert len(BENCHMARK_QUERIES) == 22

    def test_invariant_check_passes(self):
        # Should not raise
        _assert_benchmark_queries_locked()

    def test_all_queries_have_4_fields(self):
        for q in BENCHMARK_QUERIES:
            assert len(q) == 4
            qid, category, text, focus = q
            assert isinstance(qid, str)
            assert isinstance(category, str)
            assert isinstance(text, str)
            assert isinstance(focus, str)

    def test_query_ids_unique(self):
        ids = [q[0] for q in BENCHMARK_QUERIES]
        assert len(ids) == len(set(ids))

    def test_5_gap_direct_queries_present(self):
        gap_direct = [q for q in BENCHMARK_QUERIES if q[1].startswith("gap_direct_")]
        assert len(gap_direct) == 5
        gap_names = [q[2].split(":")[0] for q in gap_direct]
        # Should have time, freedom, recognition, emergence, truth all represented
        text_blob = " ".join(q[2] for q in gap_direct).lower()
        assert "时间" in text_blob or "bergson" in text_blob
        assert "自由" in text_blob or "spinoza" in text_blob
        assert "承认" in text_blob or "levinas" in text_blob
        assert "涌现" in text_blob or "bedau" in text_blob
        assert "真理" in text_blob or "peirce" in text_blob

    def test_3_anchor_queries_present(self):
        anchors = [q for q in BENCHMARK_QUERIES if q[1].startswith("anchor_")]
        assert len(anchors) == 3
        ids = [q[0] for q in anchors]
        assert "Q06_V01_ANCHOR" in ids
        assert "Q07_V1256_ANCHOR" in ids
        assert "Q08_V1049_ANCHOR" in ids

    def test_2_cross_gap_queries_present(self):
        cross = [q for q in BENCHMARK_QUERIES if q[1].startswith("cross_gap_")]
        assert len(cross) == 2
        # Q09 should reference time × freedom
        assert any("time" in q[2].lower() and "freedom" in q[2].lower() for q in cross)

    def test_3_edge_case_queries_present(self):
        edges = [q for q in BENCHMARK_QUERIES if q[1].startswith("edge_case_")]
        assert len(edges) == 3
        ids = [q[0] for q in edges]
        assert "Q20_EMPTY" in ids
        assert "Q21_MINIMAL" in ids
        assert "Q22_MIXED" in ids


# ============================================================================
# Section 2: BenchmarkRunner — 8 tests
# ============================================================================


class TestBenchmarkRunner:
    def setup_method(self) -> None:
        self.runner = BenchmarkRunner()
        self.results = self.runner.run_benchmark()

    def test_substrate_and_citation(self):
        assert "V1323" in BenchmarkRunner.SUBSTRATE
        assert "V1322" in BenchmarkRunner.SUBSTRATE
        assert "V1322" in BenchmarkRunner.CITATION

    def test_run_returns_22_results(self):
        assert len(self.results) == 22

    def test_all_results_are_QueryResult(self):
        assert all(isinstance(r, QueryResult) for r in self.results)

    def test_all_results_have_crucible_result(self):
        assert all(isinstance(r.crucible_result, CrucibleResult) for r in self.results)

    def test_query_ids_match_benchmark(self):
        result_ids = [r.query_id for r in self.results]
        expected_ids = [q[0] for q in BENCHMARK_QUERIES]
        assert result_ids == expected_ids

    def test_custom_queries_override(self):
        custom = (("X01_TEST", "custom", "test query", "time"),)
        results = self.runner.run(custom)
        assert len(results) == 1
        assert results[0].query_id == "X01_TEST"

    def test_empty_query_detected(self):
        empty_q = next((r for r in self.results if r.query_id == "Q20_EMPTY"), None)
        assert empty_q is not None
        assert empty_q.is_empty is True

    def test_minimal_query_detected(self):
        min_q = next((r for r in self.results if r.query_id == "Q21_MINIMAL"), None)
        assert min_q is not None
        assert min_q.is_minimal is True
        assert min_q.is_empty is False


# ============================================================================
# Section 3: DimensionStats / _percentile — 8 tests
# ============================================================================


class TestDimensionStats:
    def setup_method(self) -> None:
        runner = BenchmarkRunner()
        results = runner.run_benchmark()
        self.results = results
        self.stats = compute_dimension_stats(results)

    def test_n_dimension_stats_is_18(self):
        # 5 gaps + 10 cross-gaps + 3 aggregates = 18
        assert len(self.stats) == 18

    def test_stats_have_correct_n_samples(self):
        for s in self.stats:
            assert s.n_samples == 22

    def test_stats_range_in_01(self):
        for s in self.stats:
            assert 0.0 <= s.mean <= 1.0
            assert 0.0 <= s.min_v <= 1.0
            assert 0.0 <= s.max_v <= 1.0
            assert 0.0 <= s.p25 <= 1.0
            assert 0.0 <= s.p50 <= 1.0
            assert 0.0 <= s.p75 <= 1.0

    def test_min_le_max(self):
        for s in self.stats:
            assert s.min_v <= s.max_v

    def test_p25_le_p50_le_p75(self):
        for s in self.stats:
            assert s.p25 <= s.p50
            assert s.p50 <= s.p75

    def test_percentile_function_single(self):
        assert _percentile([0.5], 50.0) == 0.5

    def test_percentile_function_empty(self):
        assert _percentile([], 50.0) == 0.0

    def test_percentile_function_sorted(self):
        vals = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9]
        # p50 = 0.5 (exact, sorted_v[4])
        assert _percentile(vals, 50.0) == 0.5
        # rank = 25/100 * 8 = 2.0 → lo=hi=2 → sorted_v[2] = 0.3
        assert _percentile(vals, 25.0) == 0.3
        # rank = 75/100 * 8 = 6.0 → lo=hi=6 → sorted_v[6] = 0.7
        assert _percentile(vals, 75.0) == 0.7
        # rank = 0/100 * 8 = 0 → sorted_v[0] = 0.1
        assert _percentile(vals, 0.0) == 0.1
        # rank = 100/100 * 8 = 8 → sorted_v[8] = 0.9
        assert _percentile(vals, 100.0) == 0.9
        # fractional rank: rank = 12.5/100 * 8 = 1.0 → sorted_v[1] = 0.2
        assert _percentile(vals, 12.5) == 0.2


# ============================================================================
# Section 4: CoverageReport — 6 tests
# ============================================================================


class TestCoverageReport:
    def setup_method(self) -> None:
        runner = BenchmarkRunner()
        results = runner.run_benchmark()
        self.coverage = compute_coverage(results)

    def test_n_gaps_is_5(self):
        assert self.coverage.n_gaps == 5

    def test_n_cross_gaps_is_10(self):
        assert self.coverage.n_cross_gaps == 10

    def test_all_5_gaps_nonzero(self):
        assert self.coverage.n_gaps_nonzero == 5

    def test_all_10_cross_gaps_nonzero(self):
        assert self.coverage.n_cross_gaps_nonzero == 10

    def test_gap_coverage_dict_complete(self):
        for gap in ASI_5_GAPS:
            assert gap in self.coverage.gap_coverage
            assert 0.0 < self.coverage.gap_coverage[gap] <= 1.0

    def test_cross_gap_coverage_dict_complete(self):
        for pair in CROSS_GAP_CELLS:
            assert pair in self.coverage.cross_gap_coverage
            assert 0.0 < self.coverage.cross_gap_coverage[pair] <= 1.0


# ============================================================================
# Section 5: EdgeCaseReport — 6 tests
# ============================================================================


class TestEdgeCaseReport:
    def setup_method(self) -> None:
        runner = BenchmarkRunner()
        results = runner.run_benchmark()
        self.edges = compute_edge_cases(results)

    def test_n_empty_is_1(self):
        assert self.edges.n_empty == 1

    def test_n_minimal_is_1(self):
        assert self.edges.n_minimal == 1

    def test_n_mixed_is_1(self):
        assert self.edges.n_mixed == 1

    def test_empty_aggregate_total_is_zero(self):
        # Per V1322 contract: empty query → 0.0 (no baseline for empty)
        assert self.edges.empty_aggregate_total == 0.0

    def test_minimal_aggregate_total_at_least_baseline(self):
        # Per V1322: non-empty short query hits baseline 0.20
        assert self.edges.minimal_aggregate_total >= 0.20 - 1e-9

    def test_mixed_aggregate_total_above_empty(self):
        # Mixed query (multi-keyword) should be much higher than empty
        assert self.edges.mixed_aggregate_total > self.edges.empty_aggregate_total


# ============================================================================
# Section 6: BenchmarkAggregate — 8 tests
# ============================================================================


class TestBenchmarkAggregate:
    def setup_method(self) -> None:
        runner = BenchmarkRunner()
        results = runner.run_benchmark()
        self.results = results
        self.agg = build_aggregate(results)

    def test_n_queries_is_22(self):
        assert self.agg.n_queries == 22

    def test_n_results_is_22(self):
        assert self.agg.n_results == 22

    def test_v1323_version(self):
        assert self.agg.v1323_version == V1323_VERSION

    def test_pole_star_anchors_locked(self):
        assert self.agg.pole_star_anchors["V0.1"] == 0.7905
        assert self.agg.pole_star_anchors["V0.2"] == 0.4467
        assert self.agg.pole_star_anchors["V1256_unio_mystica"] == 0.9291

    def test_v3_guards_count_is_5(self):
        assert self.agg.v3_guards_count == 5

    def test_substrate_chain_has_11_entries(self):
        assert len(self.agg.substrate_chain) == 11
        assert self.agg.substrate_chain[-1] == "V1323 22-sample real benchmark"

    def test_total_latency_nonneg(self):
        assert self.agg.total_latency_ms >= 0.0

    def test_mean_latency_nonneg(self):
        assert self.agg.mean_latency_ms >= 0.0


# ============================================================================
# Section 7: V1323Bridge — 6 tests
# ============================================================================


class TestV1323Bridge:
    def setup_method(self) -> None:
        runner = BenchmarkRunner()
        results = runner.run_benchmark()
        self.results = results
        self.bridge = build_bridge(results)

    def test_v1323_version(self):
        assert self.bridge.v1323_version == V1323_VERSION

    def test_pole_star_anchors_locked(self):
        assert self.bridge.pole_star_anchors == ASI_ANCHORS

    def test_v3_guards_match_v1322(self):
        assert self.bridge.v3_guards == V3_GUARD_MARKERS

    def test_substrate_chain_complete(self):
        assert len(self.bridge.substrate_chain) == 11
        assert self.bridge.substrate_chain[0].startswith("V1313")
        assert self.bridge.substrate_chain[-1].startswith("V1323")

    def test_operational_metadata_keys(self):
        meta = self.bridge.operational_metadata
        required_keys = (
            "n_queries", "n_results", "mean_aggregate_total_all",
            "mean_aggregate_total_nonempty",
            "delta_vs_V0.1", "delta_vs_V0.2", "delta_vs_V1256_unio_mystica",
            "total_latency_ms", "mean_latency_ms",
            "n_gaps_nonzero", "n_cross_gaps_nonzero",
            "n_empty", "n_minimal", "n_mixed",
        )
        for k in required_keys:
            assert k in meta

    def test_bridge_to_dict_serializable(self):
        d = self.bridge.to_dict()
        s = json.dumps(d, ensure_ascii=False)
        assert "V1323" in s
        assert "0.7905" in s
        assert "0.4467" in s

    def test_module_self_test(self):
        """Module _self_test should produce 16/16 pass."""
        result = _self_test()
        assert result["n_pass"] == 16
        assert result["n_total"] == 16
        assert result["all_pass"] is True

    def test_module_main_runs(self):
        """Module main should run without error and produce JSON."""
        result = main()
        assert result["n_pass"] == 16
        # main() returns _self_test() result which has bridge_dict + aggregate_summary
        assert "bridge_dict" in result
        assert "aggregate_summary" in result