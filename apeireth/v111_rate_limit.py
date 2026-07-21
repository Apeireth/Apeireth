"""V111 真生产 rate limit (主 22:10 一次几十)."""
from __future__ import annotations
import time
V111_VERSION = "0.1.0"


class V111RateLimit:
    def __init__(self, max_requests=10, per_seconds=1.0):
        self.max_requests = max_requests
        self.per_seconds = per_seconds
        self.requests = []
        self.n_allowed = 0
        self.n_denied = 0
        self.nph = 0
        self.nas = 0

    def allow(self):
        now = time.time()
        self.requests = [t for t in self.requests if now - t < self.per_seconds]
        if len(self.requests) < self.max_requests:
            self.requests.append(now)
            self.n_allowed += 1
            return True
        self.n_denied += 1
        return False

    def stats(self):
        return {"n_allowed": self.n_allowed, "n_denied": self.n_denied,
                "version": V111_VERSION,
                "philosophy": "V111 rate limit (主 19:33 + token bucket 真借鉴)"}


__all__ = ["V111_VERSION", "V111RateLimit"]