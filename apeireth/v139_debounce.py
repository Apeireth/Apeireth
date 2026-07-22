"""V139 debounce real production"""
from __future__ import annotations
import time, uuid
V139_VERSION = "0.1.0"


class V139Debounce:
    """Debounce calls by key."""

    def __init__(self, delay_seconds=0.5):
        self.delay_seconds = delay_seconds
        self.calls = {}
        self.n = 0
        self.nph = 0
        self.nas = 0

    def trigger(self, key):
        now = time.time()
        cid = f"db_{uuid.uuid4().hex[:8]}"
        self.calls[key] = now
        self.n += 1
        return cid

    def should_fire(self, key):
        if key not in self.calls:
            return True
        return (time.time() - self.calls[key]) >= self.delay_seconds

    def stats(self):
        return {
            "delay_seconds": self.delay_seconds,
            "n_calls": self.n,
            "n_keys": len(self.calls),
            "version": V139_VERSION,
            "n": self.n,
            "philosophy": "V139 debounce (主 19:33 + 真借鉴 lodash debounce)",
        }


__all__ = ["V139_VERSION", "V139Debounce"]
