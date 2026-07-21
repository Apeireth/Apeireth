"""V113 真生产 cache (主 22:10 一次几十)."""
from __future__ import annotations
import time
V113_VERSION = "0.1.0"


class V113Cache:
    def __init__(self, max_size=1000, ttl_seconds=60):
        self.max_size = max_size
        self.ttl = ttl_seconds
        self.cache = {}
        self.hits = 0
        self.misses = 0
        self.nph = 0
        self.nas = 0

    def get(self, key):
        if key in self.cache:
            value, ts = self.cache[key]
            if time.time() - ts < self.ttl:
                self.hits += 1
                return value
            del self.cache[key]
        self.misses += 1
        return None

    def put(self, key, value):
        if len(self.cache) >= self.max_size:
            oldest = min(self.cache, key=lambda k: self.cache[k][1])
            del self.cache[oldest]
        self.cache[key] = (value, time.time())

    def hit_rate(self):
        total = self.hits + self.misses
        return self.hits / total if total > 0 else 0.0

    def stats(self):
        return {"n_entries": len(self.cache), "hits": self.hits,
                "misses": self.misses, "hit_rate": round(self.hit_rate(), 4),
                "version": V113_VERSION,
                "philosophy": "V113 cache (主 19:33 + 真借鉴 LRU)"}


__all__ = ["V113_VERSION", "V113Cache"]