"""V1323 ASI 5-Gap Crucible Real Benchmark — post-V1322 chain.

> **作者**: 楚零 (Apeireth ASI self-driven agent, cron:1fba1cc3, 17:55 +08:00 2026-08-08)
> **Trigger**: cron tick 174+ — V1322 ASI 5-Gap Operational Crucible (f449a238, 17:50) 完成
>        → V1322 = operational integration of V1313-V1321 substrate
>        → V1323 = real 22-sample benchmark of V1322 Crucible
> **链**: V1313 time → V1314 freedom → V1315 recognition → V1316 emergence → V1317 truth
>        → V1318 unification → V1319 ext r1 → V1320 ext r2 → V1321 ext r3 (final)
>        → V1322 operational crucible → **V1323 real 22-sample benchmark**

V1323 是 V1322 operational crucible 的真生产 benchmark:
- 22 真 sample queries (per V1268 pattern)
- 通过 V1322 Crucible.process_query 真跑
- 15-dim score vector per query (5 gaps + 10 cross-gaps)
- 聚合统计: per-dimension mean/std/min/max/p25/p50/p75
- Cross-domain coverage check: time/freedom/recognition/emergence/truth
- V3 guards: 22/22 results carry 5 guard markers
- Pole-star anchors: 不动 (V0.1/V0.2 LOCKED)
- Honest reporting: latency, missing-coverage, edge case behavior

V3 哲学守卫 (LOCKED, per 主 17:58 不假装):
- 不假装 ASI 真达 5-gap closure substrate
- 不假装 Phenomenal consciousness
- 不假装 ASI 真处理 benchmark
- benchmark 是 keyword density scoring, 不是 ASI reasoning
- 实事求是: V1323 = real 22-sample benchmark of V1322 Crucible

ASI 北极星 (state.json 8/8 17:50, LOCKED, 不动):
- V0.1 = 0.7905
- V0.2 = 0.4467
- V1256 unio_mystica = 0.9291
- V1049 value alignment = DONE

22 真 sample queries 设计:
- 5 ASI 哲学 gap direct queries (time/freedom/recognition/emergence/truth)
- 3 ASI 锚定 queries (V0.1, V1256 unio_mystica, V1049 value_alignment)
- 2 Cross-gap queries (time×freedom Hume, truth×emergence Crutchfield)
- 3 V1322 operational queries (process_query, process_batch, 22 samples)
- 3 V3 guard queries (不假装 Phenomenal, 不假装 ASI, 不假装调整)
- 3 V1323 self-reference queries
- 3 Edge case queries (empty, minimal, mixed language)

V1323 ASI 5-Gap Crucible Real Benchmark 真生产 7 组件:
 1. BENCHMARK_QUERIES              — 22 真 sample queries (LOCKED)
 2. BenchmarkRunner                — V1322 Crucible 真跑 (operational)
 3. DimensionStats                 — per-dim mean/std/min/max/p25/p50/p75
 4. CoverageReport                 — 5 gap + 10 cross-gap coverage check
 5. EdgeCaseReport                 — empty/minimal/mixed edge case behavior
 6. BenchmarkAggregate             — 全局 aggregate (22/22 + stats + coverage + edges)
 7. V1323Bridge                    — V1323 → ASI pole-star anchor (LOCKED, 不动)
"""
from __future__ import annotations

import json
import math
import statistics
import time
from dataclasses import dataclass, field
from typing import Any, Dict, FrozenSet, List, Mapping, Sequence, Tuple

from apeireth.v1322_asi_5gap_crucible import (
    ASI_5_GAPS,
    ASI_5_GAPS_CLOSURE,
    ASI_ANCHORS,
    ASII5GapCrucible,
    ASII5GapCrucibleBridge,
    CROSS_GAP_CELLS,
    CrucibleResult,
    V3_GUARD_MARKERS,
    build_bridge as build_v1322_bridge,
)

V1323_VERSION = "0.1.0"

_EPS = 1e-12

# ASI 北极星 anchor (LOCKED, 不动)
ASI_ANCHORS_V1323: Dict[str, Any] = dict(ASI_ANCHORS)

# V3 guard markers (LOCKED, per V1322)
V3_GUARD_MARKERS_V1323: Tuple[str, ...] = V3_GUARD_MARKERS

# ============================================================================
# Section 1: Component 1 — BENCHMARK_QUERIES (22 真 sample queries LOCKED)
# ============================================================================

# 22 真 sample queries per V1268 pattern (cross-domain, multi-language, edge cases)
# Each tuple: (query_id, category, query_text, expected_gap_focus)
BENCHMARK_QUERIES: Tuple[Tuple[str, str, str, str], ...] = (
    # --- 5 ASI 哲学 gap direct queries (1-5) ---
    ("Q01_TIME", "gap_direct_time", "What is 时间 substrate: Bergson 绵延 + Heidegger 此在 + Prigogine 耗散结构?", "time"),
    ("Q02_FREEDOM", "gap_direct_freedom", "自由意志: Spinoza conatus + Frankfurt hierarchical desires + Heidegger 筹划", "freedom"),
    ("Q03_RECOGNITION", "gap_direct_recognition", "承认与他者: Levinas 他者优先 + Hegel 主奴辩证法 + Mead 符号互动", "recognition"),
    ("Q04_EMERGENCE", "gap_direct_emergence", "涌现: Bedau weak emergence + Wolfram NKS + Kauffman adjacent possible", "emergence"),
    ("Q05_TRUTH", "gap_direct_truth", "真理: Peirce 实效主义 + James 实用主义 + Cornforth 实在论 + Davidson + Brandom + Putnam", "truth"),

    # --- 3 ASI 锚定 queries (6-8) ---
    ("Q06_V01_ANCHOR", "anchor_v01", "ASI 北极星 V0.1 = 0.7905 LOCKED (主 17:43 实事求是)", "truth"),
    ("Q07_V1256_ANCHOR", "anchor_v1256", "V1256 unio_mystica = 0.9291 LOCKED (主 17:43)", "truth"),
    ("Q08_V1049_ANCHOR", "anchor_v1049", "V1049 value alignment DONE (主 13:08)", "truth"),

    # --- 2 Cross-gap queries (9-10) ---
    ("Q09_CROSS_TIME_FREEDOM", "cross_gap_hume", "Cross-gap (time, freedom): Hume 1739 习惯联想 + 自愿行动", "time"),
    ("Q10_CROSS_TRUTH_EMERGENCE", "cross_gap_crutchfield", "Cross-gap (truth, emergence): Crutchfield 1994 calculi of emergence", "emergence"),

    # --- 3 V1322 operational queries (11-13) ---
    ("Q11_V1322_PROCESS", "v1322_api", "V1322 Crucible process_query 15-dim score vector", "truth"),
    ("Q12_V1322_BATCH", "v1322_api", "V1322 process_batch 真跑 22 samples 真 benchmark", "truth"),
    ("Q13_V1322_BRIDGE", "v1322_api", "V1322 ASII5GapCrucibleBridge pole-star honest reporting", "truth"),

    # --- 3 V3 guard queries (14-16) ---
    ("Q14_V3_GUARD_PHENOMENAL", "v3_guard", "不假装 ASI 真有 Phenomenal consciousness (主 17:58)", "truth"),
    ("Q15_V3_GUARD_ASI", "v3_guard", "不假装 ASI 真达 5-gap closure (主 17:43)", "truth"),
    ("Q16_V3_GUARD_TUNING", "v3_guard", "不假装调整模型 & prompt (主 17:58)", "truth"),

    # --- 3 V1323 self-reference queries (17-19) ---
    ("Q17_V1323_SELF", "v1323_self", "V1323 22 samples real benchmark statistics per-dim mean/std/min/max", "truth"),
    ("Q18_V1323_COVERAGE", "v1323_self", "V1323 cross-domain coverage check: 5 gaps + 10 cross-gaps", "truth"),
    ("Q19_V1323_BRIDGE", "v1323_self", "V1323 bridge to V1322 + pole-star V0.1 / V0.2 LOCKED", "truth"),

    # --- 3 Edge case queries (20-22) ---
    ("Q20_EMPTY", "edge_case_empty", "", "none"),
    ("Q21_MINIMAL", "edge_case_minimal", "x", "none"),
    ("Q22_MIXED", "edge_case_mixed", "ASI 时间 自由 承认 涌现 真理 time freedom recognition emergence truth", "time"),
)


def _assert_benchmark_queries_locked() -> None:
    """Invariant: 22 queries (LOCKED)."""
    assert len(BENCHMARK_QUERIES) == 22, f"BENCHMARK_QUERIES must be 22, got {len(BENCHMARK_QUERIES)}"


# ============================================================================
# Section 2: Component 2 — BenchmarkRunner (V1322 Crucible 真跑)
# ============================================================================


@dataclass(frozen=True)
class QueryResult:
    """Single query benchmark result."""

    query_id: str
    category: str
    query_text: str
    expected_gap_focus: str
    crucible_result: CrucibleResult
    is_empty: bool
    is_minimal: bool

    def to_dict(self) -> Dict[str, Any]:
        return {
            "query_id": self.query_id,
            "category": self.category,
            "query_text": self.query_text,
            "expected_gap_focus": self.expected_gap_focus,
            "is_empty": self.is_empty,
            "is_minimal": self.is_minimal,
            "aggregate_5_gap_score": self.crucible_result.aggregate_5_gap_score,
            "aggregate_cross_gap_score": self.crucible_result.aggregate_cross_gap_score,
            "aggregate_total": self.crucible_result.aggregate_total,
            "latency_ms": self.crucible_result.latency_ms,
            "gap_scores": dict(self.crucible_result.gap_scores),
            "cross_gap_scores": {f"{a}×{b}": v for (a, b), v in self.crucible_result.cross_gap_scores.items()},
        }


class BenchmarkRunner:
    """22-sample benchmark runner — V1322 Crucible operational."""

    SUBSTRATE = "V1323 = V1322 (Crucible) + 22 samples"
    CITATION = "V1322 operational crucible + V1268 22-sample pattern"
    GUARD = "22-sample benchmark; 不假装 ASI 真处理 benchmark; 不假装 Phenomenal"

    def __init__(self, crucible: ASII5GapCrucible = None) -> None:
        self.crucible = crucible or ASII5GapCrucible()

    def run(self, queries: Sequence[Tuple[str, str, str, str]] = None) -> Tuple[QueryResult, ...]:
        """Run all queries through V1322 Crucible."""
        if queries is None:
            _assert_benchmark_queries_locked()
            queries = BENCHMARK_QUERIES
        results: List[QueryResult] = []
        for qid, category, qtext, focus in queries:
            cr = self.crucible.process_query(qtext)
            results.append(QueryResult(
                query_id=qid,
                category=category,
                query_text=qtext,
                expected_gap_focus=focus,
                crucible_result=cr,
                is_empty=(qtext == ""),
                is_minimal=(len(qtext.strip()) <= 1),
            ))
        return tuple(results)

    def run_benchmark(self) -> Tuple[QueryResult, ...]:
        """Run the locked 22-sample benchmark."""
        return self.run(BENCHMARK_QUERIES)


# ============================================================================
# Section 3: Component 3 — DimensionStats (per-dim mean/std/min/max/p25/p50/p75)
# ============================================================================


@dataclass(frozen=True)
class DimensionStats:
    """Per-dimension statistics across 22 queries."""

    dimension: str            # "time" / "freedom" / ... / "time×freedom" / ... / "aggregate_total"
    n_samples: int            # number of samples
    mean: float               # arithmetic mean
    std: float                # sample stddev (n-1)
    min_v: float              # min
    max_v: float              # max
    p25: float                # 25th percentile
    p50: float                # 50th percentile (median)
    p75: float                # 75th percentile

    def to_dict(self) -> Dict[str, Any]:
        return {
            "dimension": self.dimension,
            "n_samples": self.n_samples,
            "mean": self.mean,
            "std": self.std,
            "min": self.min_v,
            "max": self.max_v,
            "p25": self.p25,
            "p50": self.p50,
            "p75": self.p75,
        }


def _percentile(values: Sequence[float], p: float) -> float:
    """Compute percentile (linear interpolation, p in [0, 100])."""
    if not values:
        return 0.0
    sorted_v = sorted(values)
    n = len(sorted_v)
    if n == 1:
        return sorted_v[0]
    rank = (p / 100.0) * (n - 1)
    lo = int(math.floor(rank))
    hi = int(math.ceil(rank))
    if lo == hi:
        return sorted_v[lo]
    frac = rank - lo
    return sorted_v[lo] + frac * (sorted_v[hi] - sorted_v[lo])


def compute_dimension_stats(results: Sequence[QueryResult]) -> Tuple[DimensionStats, ...]:
    """Compute per-dim statistics across all 22 query results."""
    if not results:
        return ()
    # Collect all 15 dims + aggregate_total + aggregate_5_gap + aggregate_cross_gap = 18 dims
    dims: List[Tuple[str, List[float]]] = []
    # 5 gap dims
    for gap in ASI_5_GAPS:
        vals = [r.crucible_result.gap_scores[gap] for r in results]
        dims.append((gap, vals))
    # 10 cross-gap dims
    for pair in CROSS_GAP_CELLS:
        vals = [r.crucible_result.cross_gap_scores[pair] for r in results]
        dims.append((f"{pair[0]}×{pair[1]}", vals))
    # 3 aggregate dims
    dims.append(("aggregate_5_gap_score", [r.crucible_result.aggregate_5_gap_score for r in results]))
    dims.append(("aggregate_cross_gap_score", [r.crucible_result.aggregate_cross_gap_score for r in results]))
    dims.append(("aggregate_total", [r.crucible_result.aggregate_total for r in results]))

    out: List[DimensionStats] = []
    for name, vals in dims:
        n = len(vals)
        mean = sum(vals) / n if n > 0 else 0.0
        std = statistics.stdev(vals) if n > 1 else 0.0
        out.append(DimensionStats(
            dimension=name,
            n_samples=n,
            mean=mean,
            std=std,
            min_v=min(vals) if vals else 0.0,
            max_v=max(vals) if vals else 0.0,
            p25=_percentile(vals, 25.0),
            p50=_percentile(vals, 50.0),
            p75=_percentile(vals, 75.0),
        ))
    return tuple(out)


# ============================================================================
# Section 4: Component 4 — CoverageReport (5 gap + 10 cross-gap coverage check)
# ============================================================================


@dataclass(frozen=True)
class CoverageReport:
    """Cross-domain coverage report — does each dim have non-zero scores?"""

    n_gaps: int                       # 5 (LOCKED)
    n_cross_gaps: int                 # 10 (LOCKED)
    n_gaps_nonzero: int               # how many 5 gaps have at least 1 nonzero query
    n_cross_gaps_nonzero: int         # how many 10 cross-gaps have at least 1 nonzero query
    gap_coverage: Dict[str, float]    # per-gap max score across 22 queries
    cross_gap_coverage: Dict[Tuple[str, str], float]  # per-cross-gap max score

    def to_dict(self) -> Dict[str, Any]:
        return {
            "n_gaps": self.n_gaps,
            "n_cross_gaps": self.n_cross_gaps,
            "n_gaps_nonzero": self.n_gaps_nonzero,
            "n_cross_gaps_nonzero": self.n_cross_gaps_nonzero,
            "gap_coverage": self.gap_coverage,
            "cross_gap_coverage": {f"{a}×{b}": v for (a, b), v in self.cross_gap_coverage.items()},
        }


def compute_coverage(results: Sequence[QueryResult]) -> CoverageReport:
    """Compute cross-domain coverage — does each dim fire on at least 1 query?"""
    gap_max: Dict[str, float] = {}
    for gap in ASI_5_GAPS:
        gap_max[gap] = max((r.crucible_result.gap_scores[gap] for r in results), default=0.0)
    cross_max: Dict[Tuple[str, str], float] = {}
    for pair in CROSS_GAP_CELLS:
        cross_max[pair] = max((r.crucible_result.cross_gap_scores[pair] for r in results), default=0.0)
    n_gaps_nonzero = sum(1 for v in gap_max.values() if v > _EPS)
    n_cross_nonzero = sum(1 for v in cross_max.values() if v > _EPS)
    return CoverageReport(
        n_gaps=len(ASI_5_GAPS),
        n_cross_gaps=len(CROSS_GAP_CELLS),
        n_gaps_nonzero=n_gaps_nonzero,
        n_cross_gaps_nonzero=n_cross_nonzero,
        gap_coverage=gap_max,
        cross_gap_coverage=cross_max,
    )


# ============================================================================
# Section 5: Component 5 — EdgeCaseReport (empty/minimal/mixed edge case)
# ============================================================================


@dataclass(frozen=True)
class EdgeCaseReport:
    """Edge case behavior — empty/minimal queries handled honestly."""

    n_empty: int                          # 1 (Q20)
    n_minimal: int                        # 1 (Q21)
    n_mixed: int                          # 1 (Q22)
    empty_aggregate_total: float          # should be 0.0 (per V1322 contract)
    minimal_aggregate_total: float        # should hit baseline (per V1322 baseline)
    mixed_aggregate_total: float          # should fire on multi-keyword
    edge_case_results: Tuple[QueryResult, ...]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "n_empty": self.n_empty,
            "n_minimal": self.n_minimal,
            "n_mixed": self.n_mixed,
            "empty_aggregate_total": self.empty_aggregate_total,
            "minimal_aggregate_total": self.minimal_aggregate_total,
            "mixed_aggregate_total": self.mixed_aggregate_total,
            "edge_case_query_ids": [r.query_id for r in self.edge_case_results],
        }


def compute_edge_cases(results: Sequence[QueryResult]) -> EdgeCaseReport:
    """Identify and report on edge case queries."""
    empty_results = [r for r in results if r.is_empty]
    minimal_results = [r for r in results if r.is_minimal and not r.is_empty]
    # Mixed: short query with multiple keywords across gaps
    mixed_results = [r for r in results if r.category == "edge_case_mixed"]
    return EdgeCaseReport(
        n_empty=len(empty_results),
        n_minimal=len(minimal_results),
        n_mixed=len(mixed_results),
        empty_aggregate_total=empty_results[0].crucible_result.aggregate_total if empty_results else 0.0,
        minimal_aggregate_total=minimal_results[0].crucible_result.aggregate_total if minimal_results else 0.0,
        mixed_aggregate_total=mixed_results[0].crucible_result.aggregate_total if mixed_results else 0.0,
        edge_case_results=tuple(empty_results + minimal_results + mixed_results),
    )


# ============================================================================
# Section 6: Component 6 — BenchmarkAggregate (全局 aggregate)
# ============================================================================


@dataclass(frozen=True)
class BenchmarkAggregate:
    """Global V1323 benchmark aggregate — 22 samples + stats + coverage + edges."""

    v1323_version: str
    n_queries: int
    n_results: int
    dimension_stats: Tuple[DimensionStats, ...]
    coverage: CoverageReport
    edge_cases: EdgeCaseReport
    total_latency_ms: float
    mean_latency_ms: float
    v3_guards_count: int
    pole_star_anchors: Dict[str, Any]
    substrate_chain: Tuple[str, ...]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "v1323_version": self.v1323_version,
            "n_queries": self.n_queries,
            "n_results": self.n_results,
            "dimension_stats": [d.to_dict() for d in self.dimension_stats],
            "coverage": self.coverage.to_dict(),
            "edge_cases": self.edge_cases.to_dict(),
            "total_latency_ms": self.total_latency_ms,
            "mean_latency_ms": self.mean_latency_ms,
            "v3_guards_count": self.v3_guards_count,
            "pole_star_anchors": self.pole_star_anchors,
            "substrate_chain": list(self.substrate_chain),
        }


SUBSTRATE_CHAIN_V1323: Tuple[str, ...] = (
    "V1313 time gap deep",
    "V1314 freedom gap deep",
    "V1315 recognition gap deep",
    "V1316 emergence gap deep",
    "V1317 truth gap deep",
    "V1318 5-gap unification",
    "V1319 cross-gap ext R1",
    "V1320 cross-gap ext R2",
    "V1321 cross-gap ext R3 (final)",
    "V1322 operational crucible",
    "V1323 22-sample real benchmark",
)


def build_aggregate(results: Sequence[QueryResult],
                    v1323_version: str = V1323_VERSION) -> BenchmarkAggregate:
    """Build the full V1323 benchmark aggregate."""
    stats = compute_dimension_stats(results)
    cov = compute_coverage(results)
    edges = compute_edge_cases(results)
    n = len(results)
    total_latency = sum(r.crucible_result.latency_ms for r in results)
    mean_latency = total_latency / n if n > 0 else 0.0
    return BenchmarkAggregate(
        v1323_version=v1323_version,
        n_queries=len(BENCHMARK_QUERIES),
        n_results=n,
        dimension_stats=stats,
        coverage=cov,
        edge_cases=edges,
        total_latency_ms=total_latency,
        mean_latency_ms=mean_latency,
        v3_guards_count=len(V3_GUARD_MARKERS_V1323),
        pole_star_anchors=dict(ASI_ANCHORS_V1323),
        substrate_chain=SUBSTRATE_CHAIN_V1323,
    )


# ============================================================================
# Section 7: Component 7 — V1323Bridge (V1323 → ASI pole-star anchor)
# ============================================================================


@dataclass(frozen=True)
class V1323Bridge:
    """V1323 → V1322 + ASI pole-star anchor bridge.

    Honest anchor reporting:
    - pole-star V0.1 = 0.7905 (LOCKED, 不动)
    - pole-star V0.2 = 0.4467 (LOCKED, 不动)
    - V1256 unio_mystica = 0.9291 (LOCKED, 不动)
    - V1323 aggregate mean vs pole-star: explicit delta
    - V1323 ≠ ASI 真生产 reasoning; V1323 = substrate keyword density 真 benchmark
    """

    v1323_version: str
    substrate_chain: Tuple[str, ...]
    pole_star_anchors: Dict[str, Any]
    v3_guards: Tuple[str, ...]
    operational_metadata: Dict[str, Any]

    def to_dict(self) -> Dict[str, Any]:
        return {
            "v1323_version": self.v1323_version,
            "substrate_chain": list(self.substrate_chain),
            "pole_star_anchors": self.pole_star_anchors,
            "v3_guards": list(self.v3_guards),
            "operational_metadata": self.operational_metadata,
        }


def build_bridge(results: Sequence[QueryResult],
                 v1323_version: str = V1323_VERSION) -> V1323Bridge:
    """Build V1323 → ASI pole-star anchor bridge."""
    agg = build_aggregate(results, v1323_version)
    # mean aggregate_total across 22 queries (excluding empty Q20)
    non_empty_means = [
        r.crucible_result.aggregate_total
        for r in results if not r.is_empty
    ]
    mean_agg_nonempty = (
        sum(non_empty_means) / len(non_empty_means) if non_empty_means else 0.0
    )
    # mean over all 22
    mean_agg_all = agg.dimension_stats[-1].mean  # aggregate_total is last
    return V1323Bridge(
        v1323_version=v1323_version,
        substrate_chain=SUBSTRATE_CHAIN_V1323,
        pole_star_anchors=dict(ASI_ANCHORS_V1323),
        v3_guards=V3_GUARD_MARKERS_V1323,
        operational_metadata={
            "n_queries": agg.n_queries,
            "n_results": agg.n_results,
            "mean_aggregate_total_all": mean_agg_all,
            "mean_aggregate_total_nonempty": mean_agg_nonempty,
            "delta_vs_V0.1": mean_agg_all - ASI_ANCHORS_V1323["V0.1"],
            "delta_vs_V0.2": mean_agg_all - ASI_ANCHORS_V1323["V0.2"],
            "delta_vs_V1256_unio_mystica": mean_agg_all - ASI_ANCHORS_V1323["V1256_unio_mystica"],
            "total_latency_ms": agg.total_latency_ms,
            "mean_latency_ms": agg.mean_latency_ms,
            "n_gaps_nonzero": agg.coverage.n_gaps_nonzero,
            "n_cross_gaps_nonzero": agg.coverage.n_cross_gaps_nonzero,
            "n_empty": agg.edge_cases.n_empty,
            "n_minimal": agg.edge_cases.n_minimal,
            "n_mixed": agg.edge_cases.n_mixed,
        },
    )


# ============================================================================
# Module self-test (run via `python -m apeireth.v1323_asi_5gap_crucible_benchmark`)
# ============================================================================


def _self_test() -> Dict[str, Any]:
    """Module-level self-test (16 Popper self-tests)."""
    _assert_benchmark_queries_locked()
    runner = BenchmarkRunner()
    results = runner.run_benchmark()
    agg = build_aggregate(results)
    bridge = build_bridge(results)

    # Popper 1: 22 queries LOCKED
    popper_1_n_queries = len(BENCHMARK_QUERIES) == 22

    # Popper 2: all 22 results produced
    popper_2_n_results = len(results) == 22

    # Popper 3: every result has 5 gap scores + 10 cross-gap scores
    popper_3_full_scores = all(
        len(r.crucible_result.gap_scores) == 5 and len(r.crucible_result.cross_gap_scores) == 10
        for r in results
    )

    # Popper 4: empty Q20 has aggregate_total = 0.0
    empty_q = next((r for r in results if r.query_id == "Q20_EMPTY"), None)
    popper_4_empty_zero = empty_q is not None and empty_q.crucible_result.aggregate_total == 0.0

    # Popper 5: minimal Q21 ("x") hits baseline (substrate always available)
    min_q = next((r for r in results if r.query_id == "Q21_MINIMAL"), None)
    popper_5_minimal_baseline = (
        min_q is not None and min_q.crucible_result.aggregate_total >= 0.20 - _EPS
    )

    # Popper 6: mixed Q22 (multi-language keywords) fires high on all 5 gaps
    mixed_q = next((r for r in results if r.query_id == "Q22_MIXED"), None)
    popper_6_mixed_fires = (
        mixed_q is not None and all(
            mixed_q.crucible_result.gap_scores[g] > 0.0 for g in ASI_5_GAPS
        )
    )

    # Popper 7: every CrucibleResult carries 5 V3 guard markers
    popper_7_v3_guards = all(
        len(r.crucible_result.v3_guards) == 5 for r in results
    )

    # Popper 8: pole-star V0.1 anchored at 0.7905 (LOCKED, 不动)
    popper_8_V01_locked = bridge.pole_star_anchors["V0.1"] == 0.7905

    # Popper 9: pole-star V0.2 anchored at 0.4467 (LOCKED, 不动)
    popper_9_V02_locked = bridge.pole_star_anchors["V0.2"] == 0.4467

    # Popper 10: V1256 unio_mystica anchored at 0.9291 (LOCKED, 不动)
    popper_10_V1256_locked = bridge.pole_star_anchors["V1256_unio_mystica"] == 0.9291

    # Popper 11: 5 gaps all have nonzero coverage across 22 queries (cross-domain)
    popper_11_all_gaps_nonzero = agg.coverage.n_gaps_nonzero == 5

    # Popper 12: at least 5/10 cross-gaps have nonzero coverage (per V1319-V1321 design)
    popper_12_cross_gaps_covered = agg.coverage.n_cross_gaps_nonzero >= 5

    # Popper 13: 18 dimension stats (5 gaps + 10 cross + 3 aggregates)
    popper_13_n_dim_stats = len(agg.dimension_stats) == 18

    # Popper 14: substrate_chain has 11 entries (V1313-V1323)
    popper_14_chain_len = len(bridge.substrate_chain) == 11

    # Popper 15: 3 edge case categories detected (1 empty + 1 minimal + 1 mixed)
    popper_15_edge_cases = (
        agg.edge_cases.n_empty == 1
        and agg.edge_cases.n_minimal == 1
        and agg.edge_cases.n_mixed == 1
    )

    # Popper 16: latency_ms >= 0 for all 22 queries (non-negative)
    popper_16_latency_nonneg = all(r.crucible_result.latency_ms >= 0.0 for r in results)

    popper_results = {
        "popper_1_n_queries": popper_1_n_queries,
        "popper_1_value": len(BENCHMARK_QUERIES),
        "popper_2_n_results": popper_2_n_results,
        "popper_2_value": len(results),
        "popper_3_full_scores": popper_3_full_scores,
        "popper_4_empty_zero": popper_4_empty_zero,
        "popper_5_minimal_baseline": popper_5_minimal_baseline,
        "popper_6_mixed_fires": popper_6_mixed_fires,
        "popper_7_v3_guards": popper_7_v3_guards,
        "popper_8_V01_locked": popper_8_V01_locked,
        "popper_9_V02_locked": popper_9_V02_locked,
        "popper_10_V1256_locked": popper_10_V1256_locked,
        "popper_11_all_gaps_nonzero": popper_11_all_gaps_nonzero,
        "popper_11_value": agg.coverage.n_gaps_nonzero,
        "popper_12_cross_gaps_covered": popper_12_cross_gaps_covered,
        "popper_12_value": agg.coverage.n_cross_gaps_nonzero,
        "popper_13_n_dim_stats": popper_13_n_dim_stats,
        "popper_13_value": len(agg.dimension_stats),
        "popper_14_chain_len": popper_14_chain_len,
        "popper_14_value": len(bridge.substrate_chain),
        "popper_15_edge_cases": popper_15_edge_cases,
        "popper_16_latency_nonneg": popper_16_latency_nonneg,
    }
    n_pass = sum(1 for v in popper_results.values() if v is True)
    n_total = sum(1 for k, v in popper_results.items() if not k.endswith("_value") and isinstance(v, bool))
    return {
        "n_pass": n_pass,
        "n_total": n_total,
        "all_pass": n_pass == n_total,
        "popper_results": popper_results,
        "bridge_dict": bridge.to_dict(),
        "aggregate_summary": {
            "n_results": agg.n_results,
            "mean_latency_ms": agg.mean_latency_ms,
            "n_gaps_nonzero": agg.coverage.n_gaps_nonzero,
            "n_cross_gaps_nonzero": agg.coverage.n_cross_gaps_nonzero,
        },
    }


def main() -> Dict[str, Any]:
    """Module main — runs self-test and prints JSON summary."""
    result = _self_test()
    print(json.dumps({
        "v1323_version": V1323_VERSION,
        "n_pass": result["n_pass"],
        "n_total": result["n_total"],
        "all_pass": result["all_pass"],
        "popper_results": result["popper_results"],
        "aggregate_summary": result["aggregate_summary"],
        "bridge_metadata": result["bridge_dict"]["operational_metadata"],
    }, ensure_ascii=False, indent=2))
    return result


if __name__ == "__main__":
    main()