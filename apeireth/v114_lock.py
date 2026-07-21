"""V114 真生产 lock (主 22:10 一次几十)."""
from __future__ import annotations
import time
import uuid
from dataclasses import dataclass, field
V114_VERSION = "0.1.0"


@dataclass
class Lock:
    lock_id: str
    holder: str
    acquired_at: float
    timeout_seconds: float
    ts: float = field(default_factory=time.time)


class V114Lock:
    def __init__(self):
        self.locks = {}
        self.history = []
        self.n = 0
        self.nph = 0
        self.nas = 0

    def acquire(self, name, holder, timeout_seconds=10):
        existing = self.locks.get(name)
        if existing and time.time() - existing.acquired_at < existing.timeout_seconds:
            return False
        lid = f"lk_{uuid.uuid4().hex[:12]}"
        self.locks[name] = Lock(lock_id=lid, holder=holder,
                                acquired_at=time.time(),
                                timeout_seconds=timeout_seconds)
        self.history.append((name, holder, "acquired"))
        self.n += 1
        return True

    def release(self, name, holder):
        existing = self.locks.get(name)
        if existing and existing.holder == holder:
            del self.locks[name]
            self.history.append((name, holder, "released"))
            return True
        return False

    def stats(self):
        return {"n_locks": len(self.locks), "n_acquisitions": self.n,
                "version": V114_VERSION,
                "philosophy": "V114 lock (主 19:33 + 真借鉴 distributed lock)"}


__all__ = ["V114_VERSION", "V114Lock"]