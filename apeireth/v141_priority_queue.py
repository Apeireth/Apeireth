"""V141 priority queue 真生产."""
from __future__ import annotations
import heapq, uuid
V141_VERSION = "0.1.0"
class V141PriorityQueue:
    self.nph = 0
    self.nas = 0
    def push(self, item, priority):
        eid = uuid.uuid4().hex[:8]; heapq.heappush(self.heap, (priority, eid, item)); self.n += 1
    def pop(self):
        if self.heap: return heapq.heappop(self.heap)
        return None
    def size(self): return len(self.heap)
    def stats(self): return {"size": self.size(), "n": self.n, "version": V141_VERSION,
                             "philosophy": "V141 priority queue (主 19:33 + heapq 真借鉴)"}
__all__ = ["V141_VERSION", "V141PriorityQueue"]