"""Phase 209 v160_hqb_4dims — V160 V36 HQB 4 维度 SC/NR/EV/CDT 真测 (主 22:30 + 主 18:52 + 主 17:43 + 主 22:33).

主 22:30 真采纳: 20+ 真生产方向都做了, 做完再报告
主 18:52 真采纳: HARNESS.md §2.3 HQB 4 维度

真借鉴 (主 13:08 + 主 18:52):
- HQB 4 维度 (SC/NR/EV/CDT) 真借鉴
- V36 HQB 真生产
- HARNESS.md §2.3 真源码

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
"""
from __future__ import annotations

import math
import time
from typing import Any, Dict, List


V160_VERSION = "0.1.0"


class V160HQB4Dimensions:
    """V160 HQB 4 维度 (SC/NR/EV/CDT) 真测每维度 (主 22:27 不空壳 + 主 18:52).

    真借鉴 (主 13:08 + 主 18:52 + 主 19:33):
    - HARNESS.md §2.3 HQB 4 维度
    - SC 自洽性 (Consistency)
    - NR 抗噪性 (Noise Resistance)
    - EV 可演化性 (Evolvability)
    - CDT 跨域迁移 (Cross-Domain Transfer)
    """

    def __init__(self):
        self.measurements: List[Dict[str, Any]] = []
        self.n_phenomenal_pretend_total = 0
        self.n_asi_pretend_total = 0

    def measure_sc(self, runs: List[float]) -> float:
        """V160 真测 SC 自洽性 (主 18:52 HARNESS.md §2.3 真借鉴).

        借鉴: SC = 1 - variance/mean² (越高越自洽).
        """
        if not runs or len(runs) < 2:
            return 0.0
        mean = sum(runs) / len(runs)
        if abs(mean) < 1e-9:
            return 1.0 if all(r == runs[0] for r in runs) else 0.0
        variance = sum((r - mean) ** 2 for r in runs) / len(runs)
        return max(0.0, 1.0 - variance / (mean ** 2 + 1e-9))

    def measure_nr(self, original: List[float],
                 noisy: List[float]) -> float:
        """V160 真测 NR 抗噪性 (主 18:52).

        借鉴: NR = 1 - |noisy_score - original_score| / original_score.
        """
        if not original or not noisy or len(original) != len(noisy):
            return 0.0
        total_diff = 0.0
        for o, n in zip(original, noisy):
            if abs(o) > 1e-9:
                total_diff += abs(n - o) / abs(o)
        avg_diff = total_diff / len(original)
        return max(0.0, 1.0 - avg_diff)

    def measure_ev(self, prev_score: float, next_score: float) -> float:
        """V160 真测 EV 可演化性 (主 18:52).

        借鉴: EV = (next - prev) / prev (越高越可演化).
        """
        if abs(prev_score) < 1e-9:
            return 0.0
        delta = (next_score - prev_score) / abs(prev_score)
        return max(0.0, min(1.0, 0.5 + delta))

    def measure_cdt(self, domain_scores: Dict[str, float]) -> float:
        """V160 真测 CDT 跨域迁移 (主 18:52).

        借鉴: CDT = mean(domain_scores) (跨域平均).
        """
        if not domain_scores:
            return 0.0
        return sum(domain_scores.values()) / len(domain_scores)

    def measure_all(self, runs: List[float], noisy: List[float],
                   prev: float, next_s: float,
                   domain_scores: Dict[str, float]) -> Dict[str, Any]:
        """V160 真测 HQB 4 维度一次 (主 18:52 真借鉴)."""
        t0 = time.time()
        sc = self.measure_sc(runs)
        nr = self.measure_nr(runs, noisy)
        ev = self.measure_ev(prev, next_s)
        cdt = self.measure_cdt(domain_scores)
        total = (sc + nr + ev + cdt) / 4.0
        result = {
            "sc": sc, "nr": nr, "ev": ev, "cdt": cdt, "total": total,
            "duration_ms": (time.time() - t0) * 1000,
        }
        self.measurements.append(result)
        return result

    def n_measurements(self) -> int:
        return len(self.measurements)

    def average_total(self) -> float:
        if not self.measurements:
            return 0.0
        return sum(m["total"] for m in self.measurements) / len(self.measurements)

    def stats(self) -> Dict[str, Any]:
        return {
            "n_measurements": self.n_measurements(),
            "average_total": round(self.average_total(), 4),
            "version": V160_VERSION,
            "philosophy": (
                "V160 HQB 4 维度 (SC/NR/EV/CDT) 真测每维度 (主 22:30 + 主 22:27 不空壳 + 主 18:52 + 主 17:43 + 主 22:33). "
                "真借鉴: HARNESS.md §2.3 HQB 4 维度."
            ),
        }


__all__ = [
    "V160_VERSION",
    "V160HQB4Dimensions",
]


def _demo():
    print("=" * 60)
    print("=== Phase 209 V160 HQB 4 维度真测 (主 22:27 不空壳) ===")
    print("=" * 60)

    hqb = V160HQB4Dimensions()
    result = hqb.measure_all(
        runs=[0.85, 0.86, 0.84, 0.87, 0.85],
        noisy=[0.83, 0.84, 0.82, 0.85, 0.83],
        prev=0.7, next_s=0.85,
        domain_scores={"code": 0.85, "research": 0.80, "philosophy": 0.75},
    )
    print(f"\n  ✓ HQB 4 维度:")
    print(f"    SC (自洽): {result['sc']:.4f}")
    print(f"    NR (抗噪): {result['nr']:.4f}")
    print(f"    EV (演化): {result['ev']:.4f}")
    print(f"    CDT (跨域): {result['cdt']:.4f}")
    print(f"  ✓ total: {result['total']:.4f}")
    print("=" * 60)


if __name__ == "__main__":
    _demo()