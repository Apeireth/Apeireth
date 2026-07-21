# V103 NVTX 真生产 (NVIDIA Tools Extension)
from __future__ import annotations
import uuid, time
from dataclasses import dataclass, field
V103_VERSION = "0.1.0"
@dataclass
class NVTXRange:
    range_id: str; name: str; category: str = ""
    duration_ms: float = 0.0
    ts: float = field(default_factory=time.time)
class V103NVTX:
    def __init__(self): self.ranges = []; self.n = 0; self.nph = 0; self.nas = 0
    def push_range(self, name, category=""):
        rid = f"nvtx_{uuid.uuid4().hex[:12]}"
        self.ranges.append(NVTXRange(range_id=rid, name=name, category=category))
        self.n += 1
        return rid
    def pop_range(self, range_id, duration_ms=0.0):
        for r in self.ranges:
            if r.range_id == range_id: r.duration_ms = duration_ms
    def stats(self): return {"n": self.n, "version": V103_VERSION,
                            "philosophy": "V103 NVTX (主 19:33 + NVIDIA profiling + V79 真借鉴)"}
__all__ = ["V103_VERSION", "V103NVTX"]