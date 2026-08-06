"""V1270 ASI Streaming Rate Limiter tests — 真生产 (主 00:44 质量工程化).

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 rate limit 真生效: 真测 100 真请求超 limit 真 deny.
- 不假装 token 估算 = 真 GPT BPE: 真标 estimate.
- 不假装 concurrent 真并发: 真 acquire 真 +1, release 真 -1.
- 不假装 V1270 = ASI 守门: V1270 是工具.

Tests cover:
 1.  V1270_VERSION + module 真 importable
 2.  V3_GUARDS 真 5 个 + 真 reference list 8 真前辈
 3.  V1270RateLimitConfig 真默认值 + 真 to_dict
 4.  V1270TokenEstimator 真 char-based 真 estimate (主 17:43 真标注)
 5.  V1270SlidingWindowCounter 真 sliding window 真 prune + 真 sum
 6.  V1270RateLimiter 真 RPM 真 deny 超 limit
 7.  V1270RateLimiter 真 TPM 真 deny 超 limit
 8.  V1270RateLimiter 真 concurrent 真 deny 超 limit
 9.  V1270RateLimiter 真 acquire 真 +1 concurrent
10.  V1270RateLimiter 真 release 真 -1 concurrent
11.  V1270RateLimiter 真 release 真修正 actual ≠ estimated
12.  run_v1270_load_test 真 100 真请求真统计 真通过率
13.  V1270RateLimitExceeded 真 exception 真 raise 真 with decision
14.  sanity_check_v1270 真 14 全过
15.  default_demo 真一行真跑
16.  任何人都能接手: --demo 一行真出
"""
from __future__ import annotations

import json
import threading

import pytest

from apeireth import v1270_asi_stream_rate_limiter as v1270


# ============================================================================
# 1. Module structure
# ============================================================================


def test_v1270_version_and_module_exists():
    assert hasattr(v1270, "V1270_VERSION")
    assert v1270.V1270_VERSION
    assert hasattr(v1270, "V1270RateLimiter")
    assert hasattr(v1270, "V1270RateLimitConfig")
    assert hasattr(v1270, "V1270TokenEstimator")
    assert hasattr(v1270, "V1270SlidingWindowCounter")
    assert hasattr(v1270, "V1270RateLimitDecision")
    assert hasattr(v1270, "V1270RateLimitExceeded")
    assert hasattr(v1270, "V1270LoadTestResult")


def test_v1270_v3_guards_complete():
    guards = v1270.V3_GUARDS
    assert len(guards) >= 5
    assert "v1270_rate_limit_actually_limits" in guards
    assert "v1270_token_estimate_labeled" in guards
    assert "v1270_concurrent_is_in_process" in guards
    assert "v1270_release_corrects_overestimate" in guards
    assert "v1270_is_not_asi_gate" in guards


def test_v1270_references_count():
    """真借鉴 8 真前辈 (主 19:33)."""
    src = open(v1270.__file__, "r", encoding="utf-8").read()
    # 真借鉴 真 8 个
    assert "token bucket algorithm 1977" in src.lower() or "Token bucket" in src
    assert "Leaky bucket" in src or "leaky bucket" in src
    assert "Redis" in src or "redis" in src
    assert "OpenAI" in src
    assert "LiteLLM" in src
    assert "Stripe" in src or "stripe" in src
    assert "V1269" in src
    assert "Lock" in src or "threading" in src


def test_v1270_sanity_check_pass():
    s = v1270.sanity_check_v1270()
    assert all(s.values()), f"sanity check failed: {s}"
    assert len(s) >= 14


# ============================================================================
# 2. Config
# ============================================================================


def test_v1270_config_defaults():
    cfg = v1270.V1270RateLimitConfig()
    assert cfg.requests_per_minute == 60
    assert cfg.tokens_per_minute == 60000
    assert cfg.max_concurrent == 8
    assert cfg.window_seconds == 60.0
    d = cfg.to_dict()
    assert d["requests_per_minute"] == 60
    assert d["tokens_per_minute"] == 60000


def test_v1270_config_custom():
    cfg = v1270.V1270RateLimitConfig(
        requests_per_minute=5,
        tokens_per_minute=1000,
        max_concurrent=2,
        max_cost_per_minute_usd=0.05,
    )
    assert cfg.requests_per_minute == 5
    assert cfg.max_cost_per_minute_usd == 0.05


# ============================================================================
# 3. Token estimator (主 17:43 真标注 estimate)
# ============================================================================


def test_v1270_token_estimator_basic():
    """真标 estimate — 真测启发式: 4 chars / token (主 19:33 走在前人肩上)."""
    assert v1270.V1270TokenEstimator.estimate("") == 0
    assert v1270.V1270TokenEstimator.estimate("a") == 1  # min 1
    assert v1270.V1270TokenEstimator.estimate("abcd") == 1
    assert v1270.V1270TokenEstimator.estimate("abcde") == 2  # ceil(5/4)
    assert v1270.V1270TokenEstimator.estimate("a" * 100) == 25  # 100/4
    assert v1270.V1270TokenEstimator.estimate("a" * 101) == 26  # ceil(101/4)


def test_v1270_token_estimator_messages():
    msgs = [{"role": "user", "content": "hello world"}]  # 11 chars → ceil(11/4)=3 + 4 = 7
    n = v1270.V1270TokenEstimator.estimate_messages(msgs)
    assert n == 7
    # Empty
    assert v1270.V1270TokenEstimator.estimate_messages([]) == 0


# ============================================================================
# 4. Sliding window
# ============================================================================


def test_v1270_sliding_window_basic():
    sw = v1270.V1270SlidingWindowCounter(window_seconds=10.0)
    sw.add(1.0, now=100.0)
    sw.add(2.0, now=100.0)
    sw.add(3.0, now=105.0)
    assert sw.count(now=105.0) == 3
    assert sw.sum(now=105.0) == 6.0
    # 真 prune: cutoff=105-10=95 → 全留
    sw.add(4.0, now=120.0)
    # cutoff=120-10=110 → (100,1) (100,2) (105,3) 全 < 110 真 prune
    # 剩下 (120, 4) → 1 真 entry
    assert sw.count(now=120.0) == 1
    assert sw.sum(now=120.0) == 4.0


def test_v1270_sliding_window_thread_safety():
    sw = v1270.V1270SlidingWindowCounter(window_seconds=60.0)

    def adder():
        for _ in range(100):
            sw.add(1.0)

    threads = [threading.Thread(target=adder) for _ in range(5)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    # 真测 thread-safe: 5 × 100 = 500
    assert sw.count() == 500
    assert sw.sum() == 500.0


def test_v1270_sliding_window_reset():
    sw = v1270.V1270SlidingWindowCounter()
    sw.add(1.0)
    assert sw.count() == 1
    sw.reset()
    assert sw.count() == 0


# ============================================================================
# 5. Rate limiter — 真测 4 维度 (主 13:31)
# ============================================================================


def test_v1270_rpm_denies_over_limit():
    """主 17:43 不假装: 超 RPM 真 deny."""
    cfg = v1270.V1270RateLimitConfig(requests_per_minute=3, max_concurrent=10)
    limiter = v1270.V1270RateLimiter(cfg)
    # 真测前 3 个真 allow
    for i in range(3):
        d = limiter.acquire(estimated_tokens=10)
        assert d.allowed, f"req {i} should be allowed"
        limiter.release(actual_tokens=10, estimated_tokens=10)
    # 第 4 个真 deny
    with pytest.raises(v1270.V1270RateLimitExceeded) as exc_info:
        limiter.acquire(estimated_tokens=10)
    assert "rpm_exceeded" in str(exc_info.value)


def test_v1270_tpm_denies_over_limit():
    cfg = v1270.V1270RateLimitConfig(
        requests_per_minute=100,
        tokens_per_minute=200,
        max_concurrent=10,
    )
    limiter = v1270.V1270RateLimiter(cfg)
    # 真测: 100 tokens 真 allow
    d = limiter.acquire(estimated_tokens=100)
    assert d.allowed
    limiter.release(actual_tokens=100, estimated_tokens=100)
    # 再 100 真 deny (累计 200, 但 limit = 200, 200+0 ok; 200+1 denied)
    # 真测: 第 2 个 101 tokens 真 deny (累计 201 > 200)
    with pytest.raises(v1270.V1270RateLimitExceeded) as exc_info:
        limiter.acquire(estimated_tokens=101)
    assert "tpm_exceeded" in str(exc_info.value)


def test_v1270_concurrent_denies_over_limit():
    cfg = v1270.V1270RateLimitConfig(
        requests_per_minute=100, tokens_per_minute=100000, max_concurrent=2,
    )
    limiter = v1270.V1270RateLimiter(cfg)
    # 真测: 第 1 个 真 acquire
    d1 = limiter.acquire(estimated_tokens=10)
    assert d1.allowed
    assert d1.concurrent_current == 0  # acquire 前
    # 真测: 第 2 个 真 acquire (max=2)
    d2 = limiter.acquire(estimated_tokens=10)
    assert d2.allowed
    # 真测: 第 3 个 真 deny (max=2)
    with pytest.raises(v1270.V1270RateLimitExceeded) as exc_info:
        limiter.acquire(estimated_tokens=10)
    assert "concurrent_exceeded" in str(exc_info.value)


def test_v1270_acquire_increments_concurrent():
    cfg = v1270.V1270RateLimitConfig(max_concurrent=5)
    limiter = v1270.V1270RateLimiter(cfg)
    assert limiter.snapshot()["concurrent"]["active"] == 0
    limiter.acquire(estimated_tokens=10)
    assert limiter.snapshot()["concurrent"]["active"] == 1
    limiter.acquire(estimated_tokens=10)
    assert limiter.snapshot()["concurrent"]["active"] == 2


def test_v1270_release_decrements_concurrent():
    cfg = v1270.V1270RateLimitConfig(max_concurrent=5)
    limiter = v1270.V1270RateLimiter(cfg)
    limiter.acquire(estimated_tokens=10)
    limiter.acquire(estimated_tokens=10)
    assert limiter.snapshot()["concurrent"]["active"] == 2
    limiter.release(actual_tokens=10, estimated_tokens=10)
    assert limiter.snapshot()["concurrent"]["active"] == 1
    limiter.release(actual_tokens=10, estimated_tokens=10)
    assert limiter.snapshot()["concurrent"]["active"] == 0


def test_v1270_release_corrects_overestimate():
    """主 17:43: 真修正 actual ≠ estimated."""
    cfg = v1270.V1270RateLimitConfig(
        requests_per_minute=100, tokens_per_minute=1000, max_concurrent=10,
    )
    limiter = v1270.V1270RateLimiter(cfg)
    # 真测: estimated=100, actual=50
    limiter.acquire(estimated_tokens=100)
    tpm_before = limiter.snapshot()["tpm"]["sum"]
    assert tpm_before == 100
    limiter.release(actual_tokens=50, estimated_tokens=100)
    tpm_after = limiter.snapshot()["tpm"]["sum"]
    # 真修: -100 + 50 = -50
    assert tpm_after == 50


# ============================================================================
# 6. Load test
# ============================================================================


def test_v1270_load_test_basic():
    """主 17:43: 真跑 N 真请求真统计 真通过率."""
    cfg = v1270.V1270RateLimitConfig(
        requests_per_minute=5,
        tokens_per_minute=10000,
        max_concurrent=10,
    )
    limiter = v1270.V1270RateLimiter(cfg)
    result = v1270.run_v1270_load_test(
        limiter, n_requests=20, tokens_per_request=50, release_each=True,
    )
    assert result.n_requests == 20
    # 20 真请求, RPM=5, 应该 ≤5 通过
    assert result.n_allowed <= 5
    assert result.n_denied >= 15
    assert result.pass_rate <= 0.3
    assert result.duration_s > 0
    assert any("rpm_exceeded" in k for k in result.denial_reasons), (
        f"expected rpm_exceeded reason, got {result.denial_reasons}"
    )


def test_v1270_load_test_high_limit():
    """高 limit 应几乎全过."""
    cfg = v1270.V1270RateLimitConfig(
        requests_per_minute=1000, tokens_per_minute=1000000, max_concurrent=100,
    )
    limiter = v1270.V1270RateLimiter(cfg)
    result = v1270.run_v1270_load_test(
        limiter, n_requests=20, tokens_per_request=50, release_each=True,
    )
    assert result.n_allowed == 20
    assert result.n_denied == 0
    assert result.pass_rate == 1.0


# ============================================================================
# 7. Exception
# ============================================================================


def test_v1270_rate_limit_exceeded_has_decision():
    cfg = v1270.V1270RateLimitConfig(requests_per_minute=1, max_concurrent=10)
    limiter = v1270.V1270RateLimiter(cfg)
    limiter.acquire(estimated_tokens=10)
    limiter.release(actual_tokens=10, estimated_tokens=10)
    with pytest.raises(v1270.V1270RateLimitExceeded) as exc_info:
        limiter.acquire(estimated_tokens=10)
    decision = exc_info.value.decision
    assert decision.allowed is False
    assert "rpm_exceeded" in decision.reason
    assert decision.rpm_limit == 1
    assert decision.wait_s > 0


# ============================================================================
# 8. Snapshot
# ============================================================================


def test_v1270_snapshot_shape():
    cfg = v1270.V1270RateLimitConfig(
        requests_per_minute=10, tokens_per_minute=1000, max_concurrent=3,
    )
    limiter = v1270.V1270RateLimiter(cfg)
    snap = limiter.snapshot()
    assert "rpm" in snap
    assert "tpm" in snap
    assert "concurrent" in snap
    assert "cost" in snap
    assert "config" in snap
    assert snap["rpm"]["limit"] == 10
    assert snap["tpm"]["limit"] == 1000
    assert snap["concurrent"]["limit"] == 3


# ============================================================================
# 9. Demo + CLI
# ============================================================================


def test_v1270_default_demo_runs():
    demo = v1270.default_demo()
    assert "config" in demo
    assert "snapshot" in demo
    assert "load_test" in demo
    assert "sanity" in demo
    assert demo["version"] == v1270.V1270_VERSION
    # 真测: RPM=10, tokens=2000, max_concurrent=2, n=20
    assert demo["load_test"]["n_requests"] == 20


def test_v1270_cli_demo_runs(capsys):
    """主 00:56: 任何人都能接手, demo 入口."""
    import subprocess
    import sys
    from pathlib import Path
    workspace = Path(v1270.__file__).parent.parent
    r = subprocess.run(
        [sys.executable, "-m", "apeireth.v1270_asi_stream_rate_limiter"],
        capture_output=True, text=True, cwd=str(workspace),
    )
    assert r.returncode == 0, f"stderr={r.stderr}"
    assert "v1270_asi_stream_rate_limiter demo" in r.stdout