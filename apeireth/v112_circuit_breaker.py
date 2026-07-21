"""V112 真生产 circuit breaker (主 22:10 一次几十)."""
from __future__ import annotations
import time
V112_VERSION = "0.1.0"


class V112CircuitBreaker:
    def __init__(self, failure_threshold=5, timeout_seconds=10):
        self.failure_threshold = failure_threshold
        self.timeout_seconds = timeout_seconds
        self.failures = 0
        self.state = "closed"
        self.last_failure_time = 0
        self.n_success = 0
        self.n_rejected = 0
        self.nph = 0
        self.nas = 0

    def call(self, fn, *args, **kwargs):
        if self.state == "open":
            if time.time() - self.last_failure_time < self.timeout_seconds:
                self.n_rejected += 1
                return None
            self.state = "half_open"
        try:
            result = fn(*args, **kwargs)
            self.n_success += 1
            if self.state == "half_open":
                self.state = "closed"
            self.failures = 0
            return result
        except Exception:
            self.failures += 1
            self.last_failure_time = time.time()
            if self.failures >= self.failure_threshold:
                self.state = "open"
            return None

    def stats(self):
        return {"state": self.state, "n_success": self.n_success,
                "n_rejected": self.n_rejected, "failures": self.failures,
                "version": V112_VERSION,
                "philosophy": "V112 circuit breaker (主 19:33 + 真借鉴 Hystrix)"}


__all__ = ["V112_VERSION", "V112CircuitBreaker"]