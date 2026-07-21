"""Phase 1022 v1022_rate_limiter — V1022 ASI 真生产 rate limiter (主 23:44 干到底 + 主 22:33 + 主 19:33 + 主 17:33).

真借鉴 (主 19:33):
- Token bucket 真生产 (Cloudflare 算法)
- Sliding window 真生产
- V111 rate limit 整合
"""
from __future__ import annotations

import time
import uuid
from collections import deque
from dataclasses import dataclass, field
from typing import Any, Dict, Optional


V1022_VERSION = "0.1.0"


@dataclass
class Bucket:
    """V1022 真生产 token bucket (主 19:33 Cloudflare 真借鉴)."""
    capacity: float
    refill_rate: float  # tokens per second
    tokens: float
    last_refill: float = field(default_factory=time.time)


class V1022RateLimiter:
    """V1022 ASI 真生产 rate limiter (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33).

    真生产借鉴:
    - Token bucket (Cloudflare) — 突发流量处理
    - Sliding window — 精确时间窗口
    """

    def __init__(self):
        self.buckets: Dict[str, Bucket] = {}
        self.windows: Dict[str, deque] = {}
        self.window_meta: Dict[str, Dict[str, float]] = {}
        self.n_allowed = 0
        self.n_denied = 0
        self.n_phenomenal_pretend_total = 0
        self.n_asi_pretend_total = 0

    def configure_token_bucket(self, key: str, capacity: float, refill_rate: float):
        """V1022 真生产 token bucket (主 19:33 Cloudflare 真借鉴)."""
        self.buckets[key] = Bucket(
            capacity=capacity, refill_rate=refill_rate, tokens=capacity,
        )

    def allow_token_bucket(self, key: str, cost: float = 1.0) -> bool:
        """V1022 真生产 allow (主 17:43 实事求是)."""
        if key not in self.buckets:
            raise ValueError(f"bucket not configured: {key}")
        b = self.buckets[key]
        now = time.time()
        elapsed = now - b.last_refill
        # Refill
        b.tokens = min(b.capacity, b.tokens + elapsed * b.refill_rate)
        b.last_refill = now
        if b.tokens >= cost:
            b.tokens -= cost
            self.n_allowed += 1
            return True
        self.n_denied += 1
        return False

    def configure_sliding_window(self, key: str, window_seconds: float, max_requests: int):
        """V1022 真生产 sliding window (主 19:33)."""
        self.windows[key] = deque(maxlen=max_requests)
        self.window_meta[key] = {
            "window_seconds": window_seconds,
            "max_requests": max_requests,
        }

    def allow_sliding_window(self, key: str) -> bool:
        if key not in self.windows:
            raise ValueError(f"window not configured: {key}")
        w = self.windows[key]
        meta = self.window_meta[key]
        now = time.time()
        window_seconds = meta["window_seconds"]
        max_requests = meta["max_requests"]
        # 移除过期
        while w and (now - w[0]) > window_seconds:
            w.popleft()
        if len(w) < max_requests:
            w.append(now)
            self.n_allowed += 1
            return True
        self.n_denied += 1
        return False

    def stats(self) -> Dict[str, Any]:
        return {
            "n_buckets": len(self.buckets),
            "n_windows": len(self.windows),
            "n_allowed": self.n_allowed,
            "n_denied": self.n_denied,
            "version": V1022_VERSION,
            "philosophy": (
                "V1022 ASI rate limiter (主 23:44 + 主 22:33 + 主 19:33 + 主 17:33). "
                "Token bucket + Sliding window 真借鉴, 不空壳."
            ),
        }


__all__ = ["V1022_VERSION", "Bucket", "V1022RateLimiter"]


def _demo():
    print("=" * 60)
    print("=== Phase 1022 V1022 ASI rate limiter (主 23:44 干到底) ===")
    print("=" * 60)
    rl = V1022RateLimiter()
    rl.configure_token_bucket("user1", capacity=5, refill_rate=1.0)
    for _ in range(7):
        print(f"  allow: {rl.allow_token_bucket('user1')}")
    s = rl.stats()
    print(f"\n  ✓ n_allowed={s['n_allowed']}, n_denied={s['n_denied']}")
    print("=" * 60)


if __name__ == "__main__":
    _demo()