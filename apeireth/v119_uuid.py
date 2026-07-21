"""V119 真生产 UUID/ULID (主 22:10 一次几十)."""
from __future__ import annotations
import time
import uuid
V119_VERSION = "0.1.0"


class V119UUID:
    def __init__(self):
        self.uuids = []
        self.n = 0
        self.nph = 0
        self.nas = 0

    def generate_uuid4(self):
        u = str(uuid.uuid4())
        self.uuids.append(u)
        self.n += 1
        return u

    def generate_ulid_like(self):
        # 真借鉴 ULID format (时间排序)
        ts = int(time.time() * 1000)
        rand = uuid.uuid4().hex[:16]
        u = f"{ts:013x}_{rand}"
        self.uuids.append(u)
        self.n += 1
        return u

    def stats(self):
        return {"n_uuids": self.n,
                "version": V119_VERSION,
                "philosophy": "V119 UUID/ULID (主 19:33 + 真借鉴 ULID)"}


__all__ = ["V119_VERSION", "V119UUID"]