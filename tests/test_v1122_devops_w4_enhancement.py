"""Tests for v1122_devops_w4_enhancement (R9-DEV-003 / W4 收尾).

覆盖 6 个真功能:
  1. Matrix plan build / partition
  2. Retry policy + backoff + retry_with_policy
  3. CI artifact cache (TTL/LRU/hits)
  4. CI workflow DAG (topo sort + cycle detection)
  5. W4 lint (matrix plan + workflow YAML text)
  6. optimize_matrix_plan (cache-driven shrink)
"""
from __future__ import annotations

import random
import time

import pytest

from apeireth.v1122_devops_w4_enhancement import (
    CIArtifactCache,
    CIWorkflowDAG,
    CIWorkflowDAGError,
    MatrixJob,
    MatrixPlan,
    RetryPolicy,
    W4LintIssue,
    build_matrix_plan,
    compute_backoff_ms,
    lint_matrix_plan,
    lint_workflow_yaml_text,
    optimize_matrix_plan,
    partition_matrix_plan,
    retry_with_policy,
)


# ---------------------------------------------------------------------------
# 1. Matrix plan
# ---------------------------------------------------------------------------
class TestMatrixPlan:
    def test_build_cartesian_product(self):
        p = build_matrix_plan(families=["qwen", "llama"],
                              dims=["sc", "nr"], tasks=["t1", "t2"],
                              timeout_sec=30.0)
        assert len(p) == 8
        assert all(isinstance(j, MatrixJob) for j in p.jobs)

    def test_partition_under_max_concurrent(self):
        p = build_matrix_plan(families=["qwen"], dims=["sc"], tasks=[f"t{i}" for i in range(10)])
        batches = partition_matrix_plan(p, max_concurrent=3)
        assert len(batches) == 4  # 10/3 = 4 batches
        assert all(len(b) <= 3 for b in batches)
        assert sum(len(b) for b in batches) == 10

    def test_partition_invalid_concurrent_raises(self):
        p = build_matrix_plan(["qwen"], ["sc"], ["t1"])
        with pytest.raises(ValueError):
            partition_matrix_plan(p, max_concurrent=0)

    def test_partition_stable_ordering(self):
        p = build_matrix_plan(["qwen", "llama"], ["sc", "nr"], ["t1"])
        b1 = partition_matrix_plan(p, max_concurrent=2)
        b2 = partition_matrix_plan(p, max_concurrent=2)
        assert [[j.job_id() for j in b] for b in b1] == [[j.job_id() for j in b] for b in b2]

    def test_matrix_job_cache_key_deterministic(self):
        j1 = MatrixJob(family="qwen", dim="sc", task_id="t1", timeout_sec=30.0)
        j2 = MatrixJob(family="qwen", dim="sc", task_id="t1", timeout_sec=30.0)
        assert j1.cache_key() == j2.cache_key()
        assert len(j1.cache_key()) == 16

    def test_matrix_by_family_groups(self):
        p = build_matrix_plan(["qwen", "llama"], ["sc"], ["t1", "t2"])
        groups = p.by_family()
        assert set(groups.keys()) == {"qwen", "llama"}
        assert len(groups["qwen"]) == 2
        assert len(groups["llama"]) == 2


# ---------------------------------------------------------------------------
# 2. Retry
# ---------------------------------------------------------------------------
class TestRetry:
    def test_compute_backoff_full_jitter_bounded(self):
        p = RetryPolicy(max_attempts=3, base_ms=100.0, cap_ms=2000.0, jitter="full")
        rng = random.Random(42)
        for attempt in range(1, 4):
            ms = compute_backoff_ms(p, attempt, rng=rng)
            assert 0.0 <= ms <= min(p.cap_ms, p.base_ms * (2 ** (attempt - 1)))

    def test_compute_backoff_no_jitter_returns_cap(self):
        p = RetryPolicy(max_attempts=3, base_ms=100.0, cap_ms=2000.0, jitter="none")
        ms = compute_backoff_ms(p, 2)
        assert ms == 200.0  # 100 * 2

    def test_compute_backoff_attempt_zero_zero(self):
        p = RetryPolicy()
        assert compute_backoff_ms(p, 0) == 0.0
        assert compute_backoff_ms(p, 99) == 0.0

    def test_retry_policy_invalid_jitter_raises(self):
        with pytest.raises(ValueError):
            RetryPolicy(jitter="bad")

    def test_retry_policy_invalid_max_attempts(self):
        with pytest.raises(ValueError):
            RetryPolicy(max_attempts=0)

    def test_retry_with_policy_eventually_succeeds(self):
        counter = {"n": 0}

        def flaky():
            counter["n"] += 1
            if counter["n"] < 3:
                raise ConnectionError("transient")
            return "ok"

        sleeps: list[float] = []

        def fake_sleep(ms: float) -> None:
            sleeps.append(ms)

        p = RetryPolicy(max_attempts=5, base_ms=10.0, cap_ms=100.0,
                        retry_on=(ConnectionError,))
        result = retry_with_policy(p, flaky, sleep_fn=fake_sleep,
                                    rng=random.Random(1))
        assert result == "ok"
        assert counter["n"] == 3
        # 重试 2 次: 2 次 sleep
        assert len(sleeps) == 2
        assert all(s >= 0 for s in sleeps)

    def test_retry_with_policy_exhausts_raises_original(self):
        def always_fail():
            raise TimeoutError("net down")

        p = RetryPolicy(max_attempts=3, base_ms=10.0, cap_ms=50.0,
                        retry_on=(TimeoutError,))
        with pytest.raises(TimeoutError, match="net down"):
            retry_with_policy(p, always_fail, sleep_fn=lambda _ms: None,
                              rng=random.Random(1))

    def test_retry_with_policy_no_retry_on_unlisted_exception(self):
        counter = {"n": 0}

        def fail():
            counter["n"] += 1
            raise ValueError("not retryable")

        p = RetryPolicy(max_attempts=3, retry_on=(TimeoutError,))
        with pytest.raises(ValueError):
            retry_with_policy(p, fail, sleep_fn=lambda _ms: None,
                              rng=random.Random(1))
        assert counter["n"] == 1  # 不重试


# ---------------------------------------------------------------------------
# 3. CI Artifact Cache
# ---------------------------------------------------------------------------
class TestArtifactCache:
    def test_set_get_hit_miss(self):
        c = CIArtifactCache(default_ttl_sec=10.0)
        key = CIArtifactCache.compute_key("a", "b")
        assert c.get(key) is None  # miss
        c.set(key, {"v": 1})
        assert c.get(key) == {"v": 1}  # hit
        assert c.stats()["hits"] == 1
        assert c.stats()["misses"] == 1

    def test_ttl_expiration(self):
        c = CIArtifactCache(default_ttl_sec=0.1)
        key = CIArtifactCache.compute_key("x")
        c.set(key, "v")
        assert c.get(key) == "v"
        time.sleep(0.2)
        assert c.get(key) is None
        assert c.stats()["expirations"] == 1

    def test_lru_eviction(self):
        c = CIArtifactCache(max_entries=2, default_ttl_sec=10.0)
        c.set("a", 1)
        c.set("b", 2)
        c.set("c", 3)  # 触发 a 淘汰
        assert c.get("a") is None
        assert c.get("b") == 2
        assert c.get("c") == 3
        assert c.stats()["evictions"] == 1

    def test_compute_key_distinguishes_parts(self):
        k1 = CIArtifactCache.compute_key("a", "b")
        k2 = CIArtifactCache.compute_key("a", "c")
        k3 = CIArtifactCache.compute_key("a", "b")
        assert k1 != k2
        assert k1 == k3
        assert len(k1) == 16

    def test_invalidate(self):
        c = CIArtifactCache()
        c.set("x", 1)
        assert c.invalidate("x") is True
        assert c.invalidate("x") is False
        assert c.get("x") is None


# ---------------------------------------------------------------------------
# 4. CI Workflow DAG
# ---------------------------------------------------------------------------
class TestWorkflowDAG:
    def test_topo_sort_linear(self):
        g = CIWorkflowDAG()
        for n in ["a", "b", "c"]:
            g.add_node(n)
        g.add_edge("a", "b")
        g.add_edge("b", "c")
        order = g.topo_sort()
        assert order.index("a") < order.index("b") < order.index("c")

    def test_topo_sort_diamond(self):
        g = CIWorkflowDAG()
        for n in ["a", "b", "c", "d"]:
            g.add_node(n)
        g.add_edge("a", "b")
        g.add_edge("a", "c")
        g.add_edge("b", "d")
        g.add_edge("c", "d")
        order = g.topo_sort()
        assert order.index("a") < order.index("d")
        assert order.index("b") < order.index("d")
        assert order.index("c") < order.index("d")

    def test_cycle_raises(self):
        g = CIWorkflowDAG()
        g.add_node("a")
        g.add_node("b")
        g.add_edge("a", "b")
        g.add_edge("b", "a")
        with pytest.raises(CIWorkflowDAGError, match="cycle"):
            g.topo_sort()

    def test_self_loop_raises(self):
        g = CIWorkflowDAG()
        g.add_node("a")
        with pytest.raises(CIWorkflowDAGError, match="self-loop"):
            g.add_edge("a", "a")

    def test_edges_and_nodes_listing(self):
        g = CIWorkflowDAG()
        g.add_edge("x", "y")
        assert set(g.nodes) == {"x", "y"}
        assert ("x", "y") in g.edges


# ---------------------------------------------------------------------------
# 5. W4 Lint
# ---------------------------------------------------------------------------
class TestLint:
    def test_lint_matrix_too_large(self):
        # 5 families × 5 dims × 5 tasks = 125 jobs
        p = build_matrix_plan(
            families=[f"f{i}" for i in range(5)],
            dims=[f"d{i}" for i in range(5)],
            tasks=[f"t{i}" for i in range(5)],
        )
        issues = lint_matrix_plan(p, max_jobs=64)
        assert any(i.rule == "matrix_too_large" and i.level == "error" for i in issues)

    def test_lint_timeout_too_low(self):
        p = MatrixPlan(jobs=[MatrixJob("qwen", "sc", "t1", timeout_sec=2.0)])
        issues = lint_matrix_plan(p)
        assert any(i.rule == "timeout_too_low" for i in issues)

    def test_lint_duplicate_job_id(self):
        # 手动造 duplicate (build_matrix_plan 笛卡尔积不会重复)
        p = MatrixPlan(jobs=[
            MatrixJob("qwen", "sc", "t1"),
            MatrixJob("qwen", "sc", "t1"),
        ])
        issues = lint_matrix_plan(p)
        assert any(i.rule == "duplicate_job_id" for i in issues)

    def test_lint_workflow_yaml_text_missing_timeout(self):
        text = """
name: ci
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        python-version: ['3.10', '3.11']
    steps:
      - uses: actions/checkout@v4
"""
        issues = lint_workflow_yaml_text(text)
        assert any(i.rule == "missing_timeout" for i in issues)

    def test_lint_workflow_yaml_text_no_matrix_info(self):
        text = "name: ci\njobs:\n  test:\n    timeout-minutes: 10\n    runs-on: ubuntu-latest\n"
        issues = lint_workflow_yaml_text(text)
        assert any(i.rule == "no_matrix" for i in issues)


# ---------------------------------------------------------------------------
# 6. optimize_matrix_plan
# ---------------------------------------------------------------------------
class TestOptimizeMatrix:
    def test_optimize_cache_hit_shrinks_plan(self):
        cache = CIArtifactCache(default_ttl_sec=60.0)
        p = build_matrix_plan(["qwen"], ["sc", "nr"], ["t1", "t2"])
        # 预热: 把其中 2 个 job 写入缓存
        jobs_by_key = {j.cache_key(): j for j in p.jobs}
        hit_keys = sorted(jobs_by_key.keys())[:2]
        for k in hit_keys:
            cache.set(k, {"cached": True})
        remaining, cached = optimize_matrix_plan(p, cache)
        assert len(remaining) == 2
        assert len(cached) == 2
        assert all(j.cache_key() in hit_keys for j in cached)

    def test_optimize_no_cache_returns_all_remaining(self):
        cache = CIArtifactCache(default_ttl_sec=60.0)
        p = build_matrix_plan(["qwen"], ["sc"], ["t1", "t2"])
        remaining, cached = optimize_matrix_plan(p, cache)
        assert len(remaining) == len(p.jobs)
        assert cached == []

    def test_optimize_empty_plan(self):
        cache = CIArtifactCache()
        remaining, cached = optimize_matrix_plan(MatrixPlan(jobs=[]), cache)
        assert remaining.jobs == []
        assert cached == []