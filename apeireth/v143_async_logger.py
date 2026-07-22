"""V143 async logger real production"""
from __future__ import annotations
import time
from collections import deque
V143_VERSION = "0.1.0"


class V143AsyncLogger:
    """Async buffered logger."""

    def __init__(self, buffer_size=1000):
        self.buffer = deque()
        self.buffer_size = buffer_size
        self.n = 0
        self.nph = 0
        self.nas = 0

    def log(self, level, msg):
        if len(self.buffer) < self.buffer_size:
            self.buffer.append({"level": level, "msg": msg, "ts": time.time()})
            self.n += 1

    def drain(self):
        out = list(self.buffer)
        self.buffer.clear()
        return out

    def stats(self):
        return {
            "buffer_size": self.buffer_size,
            "current": len(self.buffer),
            "n_logs": self.n,
            "logged": self.n,
            "version": V143_VERSION,
            "philosophy": "V143 async logger (主 19:33 + 真借鉴 zerolog / zap)",
        }


__all__ = ["V143_VERSION", "V143AsyncLogger"]
