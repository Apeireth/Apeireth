"""V1022 真生产 tests (主 23:44 干到底)."""
from __future__ import annotations
import sys; sys.path.insert(0, '.')
import time
import pytest
from apeireth.v1022_rate_limiter import (
    V1022_VERSION, Bucket, V1022RateLimiter,
)


class TestV1022:
    def test_init(self):
        rl = V1022RateLimiter()
        assert rl.n_allowed == 0
        assert rl.n_denied == 0

    def test_configure_token_bucket(self):
        rl = V1022RateLimiter()
        rl.configure_token_bucket("u1", capacity=10, refill_rate=1.0)
        assert "u1" in rl.buckets

    def test_allow_token_bucket(self):
        rl = V1022RateLimiter()
        rl.configure_token_bucket("u1", capacity=5, refill_rate=1.0)
        assert rl.allow_token_bucket("u1") is True

    def test_allow_token_bucket_burst(self):
        """V1022 真测 token bucket 突发流量 (主 19:33 Cloudflare 真借鉴)."""
        rl = V1022RateLimiter()
        rl.configure_token_bucket("u1", capacity=3, refill_rate=0.1)
        assert rl.allow_token_bucket("u1") is True
        assert rl.allow_token_bucket("u1") is True
        assert rl.allow_token_bucket("u1") is True
        # 桶空了
        assert rl.allow_token_bucket("u1") is False

    def test_allow_token_bucket_refill(self):
        rl = V1022RateLimiter()
        rl.configure_token_bucket("u1", capacity=1, refill_rate=10.0)  # 1 token, 10/s
        assert rl.allow_token_bucket("u1") is True
        assert rl.allow_token_bucket("u1") is False
        time.sleep(0.15)  # refill ~1.5 tokens
        assert rl.allow_token_bucket("u1") is True

    def test_allow_token_bucket_not_configured(self):
        rl = V1022RateLimiter()
        with pytest.raises(ValueError):
            rl.allow_token_bucket("missing")

    def test_token_bucket_cost(self):
        rl = V1022RateLimiter()
        rl.configure_token_bucket("u1", capacity=10, refill_rate=1.0)
        # cost=5
        assert rl.allow_token_bucket("u1", cost=5) is True
        assert rl.allow_token_bucket("u1", cost=5) is True
        assert rl.allow_token_bucket("u1", cost=5) is False  # 只有 0 tokens

    def test_configure_sliding_window(self):
        rl = V1022RateLimiter()
        rl.configure_sliding_window("u1", window_seconds=60, max_requests=10)
        assert "u1" in rl.windows

    def test_allow_sliding_window(self):
        """V1022 真测 sliding window 真借鉴 (主 19:33)."""
        rl = V1022RateLimiter()
        rl.configure_sliding_window("u1", window_seconds=60, max_requests=3)
        assert rl.allow_sliding_window("u1") is True
        assert rl.allow_sliding_window("u1") is True
        assert rl.allow_sliding_window("u1") is True
        assert rl.allow_sliding_window("u1") is False

    def test_allow_sliding_window_not_configured(self):
        rl = V1022RateLimiter()
        with pytest.raises(ValueError):
            rl.allow_sliding_window("missing")

    def test_stats(self):
        rl = V1022RateLimiter()
        rl.configure_token_bucket("u1", capacity=1, refill_rate=1.0)
        rl.allow_token_bucket("u1")
        rl.allow_token_bucket("u1")
        s = rl.stats()
        assert s["n_buckets"] == 1
        assert s["n_allowed"] == 1
        assert s["n_denied"] == 1

    def test_v22_33_asi_integration(self):
        """V1022 真测主 22:33 ASI 北极星."""
        rl = V1022RateLimiter()
        s = rl.stats()
        assert "ASI" in s["philosophy"]

    def test_v19_33_cloudflare(self):
        """V1022 真测主 19:33 Cloudflare + sliding window 真借鉴."""
        rl = V1022RateLimiter()
        rl.configure_token_bucket("u1", capacity=5, refill_rate=1.0)
        rl.configure_sliding_window("u2", window_seconds=60, max_requests=3)
        # 真 token bucket
        assert rl.allow_token_bucket("u1") is True
        # 真 sliding window
        assert rl.allow_sliding_window("u2") is True

    def test_v17_43_truth(self):
        """V1022 真测主 17:43 实事求是 — 真计数."""
        rl = V1022RateLimiter()
        rl.configure_token_bucket("u1", capacity=2, refill_rate=0.1)
        rl.allow_token_bucket("u1")
        rl.allow_token_bucket("u1")
        rl.allow_token_bucket("u1")
        assert rl.n_allowed == 2
        assert rl.n_denied == 1

    def test_complete_integration(self):
        """V1022 真测完整 rate limiter (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33)."""
        rl = V1022RateLimiter()
        rl.configure_token_bucket("v1001", capacity=10, refill_rate=0.1)  # very slow refill
        rl.configure_sliding_window("v1002", window_seconds=60, max_requests=5)
        # token bucket 突发
        for _ in range(15):
            rl.allow_token_bucket("v1001")
        # sliding window 限速
        for _ in range(7):
            rl.allow_sliding_window("v1002")
        s = rl.stats()
        # token bucket: 10 allowed + 5 denied
        # sliding window: 5 allowed + 2 denied
        assert s["n_allowed"] == 15
        assert s["n_denied"] == 7