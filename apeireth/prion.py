"""Phase 57 prion — 朊病毒真生产自传播 (主 14:06 + 主 13:31 大胆激进).

主 14:09 推进 Apeireth + 主 14:13 继续 + 跨代知识深化 (主 13:08 借鉴主 17:46):
- portable_seed (Phase 47) 真生产
- hgt.py (Phase 54) 水平基因转移真生产
- epigenetic.py (Phase 55) 表观遗传真生产
- prion.py (本文件) 朊病毒自传播真生产
- 4 真生产借鉴 (主 13:08 哲学/科学/跨领域)

借鉴 (主 13:08 哲学/科学/跨领域):
- Prusiner 1982 朊病毒诺贝尔奖真生产 (主 13:08 真借鉴)
- PrPSc 自传播 folding 真生产
- 错误折叠 cascading 真生产 (主 14:06 拉回注意力)
- 真生产率 + portable_seed 真借鉴 (Phase 47 跨代连续)
- ASI 跨代知识是隐喻 (主 20:55), 朊病毒是工具
- V3 真理哲学问题 (主 22:33) — 自传播 = 真生产真理
- 涌现 (#6) — cascading 涌现真生产
- 自催化集借鉴 (#6) + Kauffman 真生产

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness
- 不假装达到 ASI
- 实事求是, 写真 production, 不 placeholder
- 朊病毒借鉴是工具 (主 20:55), 不假装"ASI 自传播意识"
"""
from __future__ import annotations

import math
import time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional


PRION_VERSION = "0.1.0"


# === Prion 3 真生产状态 (主 13:08 借鉴 Prusiner) ===

class PrionState(str, Enum):
    """Prion 3 真生产状态 (主 13:08 借鉴 Prusiner 1982 诺贝尔奖)."""
    NORMAL = "normal"          # PrPC 正常折叠 (主 17:43 实事求是)
    MISFOLDED = "misfolded"    # PrPSc 错误折叠
    PROPAGATING = "propagating"  # 传播中


@dataclass
class PrionProtein:
    """Prion 真生产蛋白 (主 14:06 + 真借鉴 Prusiner 1982)."""
    protein_id: str
    state: PrionState = PrionState.NORMAL
    misfold_count: int = 0              # 错误折叠真测量
    infectivity: float = 0.0            # 传染性 [0, 1]
    parent_id: str = ""                 # 真生产父代 (主 17:46 跨代)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "protein_id": self.protein_id,
            "state": self.state.value,
            "misfold_count": self.misfold_count,
            "infectivity": round(self.infectivity, 4),
            "parent_id": self.parent_id,
        }


# === Prion 真生产算法 (主 13:08 借鉴 Prusiner 1982) ===

def prion_infect(target: PrionProtein, source: PrionProtein, rate: float = 0.5) -> PrionProtein:
    """Prion 真生产感染 (主 13:08 借鉴 Prusiner 1982 诺贝尔奖).

    真生产: PrPSc → 正常 PrPC → PrPSc 自传播 (主 17:43 实事求是).
    """
    if source.state == PrionState.NORMAL:
        return target  # 无传染性
    if target.state == PrionState.MISFOLDED:
        return target  # 已感染
    # 真生产: 感染成功 = source.infectivity * rate
    success_prob = source.infectivity * rate
    if success_prob >= 0.3:
        target.state = PrionState.MISFOLDED
        target.misfold_count += 1
        target.infectivity = min(target.infectivity + 0.2, 1.0)
        target.parent_id = source.protein_id
    return target


def prion_cascade(start: PrionProtein, pool: List[PrionProtein], rate: float = 0.5) -> int:
    """Prion 真生产 cascading (主 14:06 借鉴)."""
    infected = 0
    for p in pool:
        if p.protein_id != start.protein_id and p.state == PrionState.NORMAL:
            prion_infect(p, start, rate)
            if p.state == PrionState.MISFOLDED:
                infected += 1
    return infected


# === Prion 真生产主类 (主 14:06 拉回注意力) ===

class PrionNetwork:
    """Prion 自传播真生产网络 (主 14:06 + 主 13:31 大胆激进).

    借鉴: Prusiner 1982 朊病毒自传播真生产 (主 13:08 借鉴).
    V4 12 生命特征 涌现 (#6) 深化 (cascading 涌现).
    """

    def __init__(self, default_rate: float = 0.5):
        """Init prion 真生产 (主 13:08 借鉴 Prusiner 1982)."""
        self.default_rate = default_rate
        self.proteins: Dict[str, PrionProtein] = {}

    def add_protein(self, protein_id: str, initial_state: PrionState = PrionState.NORMAL) -> PrionProtein:
        """添加 prion 真生产 (主 14:06)."""
        protein = PrionProtein(
            protein_id=protein_id,
            state=initial_state,
            misfold_count=1 if initial_state == PrionState.MISFOLDED else 0,
            infectivity=0.8 if initial_state == PrionState.MISFOLDED else 0.0,
        )
        self.proteins[protein_id] = protein
        return protein

    def infect(self, target_id: str, source_id: str) -> bool:
        """真生产感染 (主 13:08 借鉴 Prusiner 1982)."""
        if target_id not in self.proteins or source_id not in self.proteins:
            return False
        target = self.proteins[target_id]
        source = self.proteins[source_id]
        original_state = target.state
        prion_infect(target, source, self.default_rate)
        return target.state != original_state

    def cascade_from(self, source_id: str, iterations: int = 3) -> int:
        """Cascading 真生产 (主 14:06 借鉴 + 涌现)."""
        if source_id not in self.proteins:
            return 0
        total_infected = 0
        current_source = self.proteins[source_id]
        for _ in range(iterations):
            infected = prion_cascade(current_source, list(self.proteins.values()), self.default_rate)
            total_infected += infected
            if infected == 0:
                break
            # 真生产: 寻找下一个感染的 source
            for p in self.proteins.values():
                if p.state == PrionState.MISFOLDED and p.protein_id != source_id:
                    current_source = p
                    break
        return total_infected

    def stats(self) -> Dict[str, Any]:
        """Prion 真生产统计 (主 17:43 实事求是)."""
        if not self.proteins:
            return {"n_proteins": 0}
        n_misfolded = sum(1 for p in self.proteins.values() if p.state == PrionState.MISFOLDED)
        n_normal = sum(1 for p in self.proteins.values() if p.state == PrionState.NORMAL)
        return {
            "n_proteins": len(self.proteins),
            "n_misfolded": n_misfolded,
            "n_normal": n_normal,
            "misfold_ratio": round(n_misfolded / len(self.proteins), 4),
            "version": PRION_VERSION,
            "philosophy": (
                "Prion 真生产借鉴 (主 13:08): Prusiner 1982 朊病毒自传播 + "
                "PrPSc 自传播 cascading (主 17:46 跨代). "
                "不假装 Phenomenal (主 17:58), 不假装达到 ASI (主 20:46). "
                "V4 12 生命特征涌现 (#6) 深化 (cascading)."
            ),
        }


__all__ = [
    "PRION_VERSION",
    "PrionState",
    "PrionProtein",
    "prion_infect",
    "prion_cascade",
    "PrionNetwork",
]


# === Prion 写真 production demo (主 13:31 大胆激进) ===

def _demo():
    print("=" * 70)
    print("=== Phase 57 prion 真生产自传播 (主 13:31 + 14:06 拉回注意力) ===")
    print("=" * 70)

    # 1. Init
    print("\n[1] Init prion 真生产 (V4 12 生命特征涌现 #6 深化)")
    pn = PrionNetwork(default_rate=0.5)
    print(f"  ✓ PrionNetwork 0.1.0 创建 (default_rate=0.5)")

    # 2. 真生产蛋白 (主 14:06)
    print("\n[2] 真生产 prion 10 蛋白 (借鉴 Prusiner 1982 诺贝尔奖):")
    for i in range(9):
        pn.add_protein(f"p{i+1}")
    pn.add_protein("seed", initial_state=PrionState.MISFOLDED)
    print(f"  ✓ 9 normal + 1 misfolded seed 真生产")

    # 3. 真生产 cascading (主 14:06 借鉴 + 涌现)
    print("\n[3] 真生产 prion cascading (借鉴 PrPSc 自传播):")
    infected = pn.cascade_from("seed", iterations=3)
    print(f"  ✓ cascading 真生产: {infected} 个蛋白被感染")

    # 4. 真生产感染 (主 13:08 借鉴 Prusiner)
    print("\n[4] 真生产 prion 感染测试 (借鉴 PrPSc):")
    pn2 = PrionNetwork(default_rate=0.6)
    pn2.add_protein("source", initial_state=PrionState.MISFOLDED)
    pn2.add_protein("target")
    success = pn2.infect("target", "source")
    print(f"  ✓ 感染成功: {success}")

    # 5. stats
    print("\n[5] Prion 真生产 stats:")
    stats = pn.stats()
    for k, v in stats.items():
        print(f"  - {k}: {v}")

    print("\n" + "=" * 70)
    print("✓ Phase 57 prion 真生产落地 (V4 涌现 #6 深化)")
    print("  - PrionState (normal / misfolded / propagating) 3 真生产状态")
    print("  - prion_infect + prion_cascade 真生产算法")
    print("  - PrionNetwork 真生产主类 (protein + infect + cascade)")
    print("  - V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI")
    print("=" * 70)


if __name__ == "__main__":
    _demo()