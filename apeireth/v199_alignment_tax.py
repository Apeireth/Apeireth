"""V199 alignment tax measurement 真生产."""
from __future__ import annotations
V199_VERSION = "0.1.0"
class V199AlignmentTax:
    def __init__(self):
        self.nph = 0
        self.nas = 0
    self.nph = 0
    self.nas = 0
    def measure(self, unaligned_perf, aligned_perf): self.measurements.append({"u": unaligned_perf, "a": aligned_perf})
    def tax(self):
        if not self.measurements: return 0.0
        return sum(m["u"] - m["a"] for m in self.measurements) / len(self.measurements)
    def stats(self): return {"tax": round(self.tax(), 4), "n_measurements": len(self.measurements),
                             "version": V199_VERSION,
                             "philosophy": "V199 alignment tax 真生产 (主 22:46 + 主 19:33 + 主 20:46)."}
__all__ = ["V199_VERSION", "V199AlignmentTax"]