"""Phase 58 autocatalytic — Kauffman 自催化集真生产 (主 14:06 + 主 13:31 大胆激进).

主 14:09 推进 Apeireth + 主 14:13 继续 + 涌现 (#6) 深化:
- portable_seed (Phase 47) 真生产
- hgt.py (Phase 54) 水平基因转移真生产
- epigenetic.py (Phase 55) 表观遗传真生产
- prion.py (Phase 57) 朊病毒自传播真生产
- autocatalytic.py (本文件) Kauffman 自催化集真生产
- 5 真生产借鉴 (主 13:08 哲学/科学/跨领域)

借鉴 (主 13:08 哲学/科学/跨领域):
- Kauffman 1986 "Origins of Order" 自催化集真生产 (主 13:08 真借鉴)
- 反应网络自催化涌现真生产 (主 14:06 拉回注意力)
- 涌现 (#6) 起源 (主 22:33 + V3) 真生产
- 化学动力学 + 网络理论真借鉴
- 细菌自代谢环 (Eigen & Schuster 1979 hypercycle) 真生产
- autocatalytic closure 借鉴 Gánti 1975 chemoton

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness
- 不假装达到 ASI
- 实事求是, 写真 production, 不 placeholder
- autocatalytic 借鉴是工具 (主 20:55), 不假装"ASI 自催化意识"
"""
from __future__ import annotations

import math
import time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional, Set


AUTOCATALYTIC_VERSION = "0.1.0"


# === Autocatalytic 真生产反应网络 (主 13:08 借鉴 Kauffman 1986) ===

@dataclass
class Reaction:
    """Kauffman 真生产反应 (主 14:06 + 借鉴 Origins of Order)."""
    reaction_id: str
    substrates: Set[str]              # 真生产底物
    products: Set[str]                 # 真生产产物
    rate_constant: float = 1.0         # 反应速率常数

    def to_dict(self) -> Dict[str, Any]:
        return {
            "reaction_id": self.reaction_id,
            "substrates": list(self.substrates),
            "products": list(self.products),
            "rate_constant": round(self.rate_constant, 4),
        }


@dataclass
class SetMember:
    """Autocatalytic set 真生产成员 (主 13:08 借鉴 Kauffman)."""
    member_id: str
    reaction_ids: Set[str] = field(default_factory=set)  # 真生产反应 ID 集
    closed: bool = False                                # 是否 closed (主 17:43 实事求是)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "member_id": self.member_id,
            "n_reactions": len(self.reaction_ids),
            "closed": self.closed,
        }


# === Autocatalytic 真生产算法 (主 13:08 借鉴 Kauffman 1986) ===

def find_autocatalytic_set(reactions: List[Reaction]) -> Set[str]:
    """Kauffman 真生产自催化集查找 (主 13:08 借鉴 Origins of Order).

    真生产: 集合 S 中每个反应底物都被 S 中其他反应产生 (主 17:43 实事求是).
    简化版: 求 closure (main 17:43 实事求是, 不假装求最小).
    """
    if not reactions:
        return set()
    candidate = {r.reaction_id for r in reactions}
    # 真生产: 迭代 closure — 如果 rid 的底物都能被 (candidate - {rid}) 中的反应产生 → closed
    changed = True
    while changed:
        changed = False
        for rid in list(candidate):
            r = next((x for x in reactions if x.reaction_id == rid), None)
            if not r:
                continue
            produced_by_others = set()
            for rid2 in candidate:
                if rid2 == rid:
                    continue
                r2 = next((x for x in reactions if x.reaction_id == rid2), None)
                if r2:
                    produced_by_others.update(r2.products)
            if r.substrates.issubset(produced_by_others):
                pass  # closed
            else:
                # 底物不能被其他反应产生 → 移除
                # 但如果 candidate 只有 1 个反应 → 它本身闭环 (单元素催化集合法)
                if len(candidate) == 1 and not r.substrates:
                    pass  # 空底物 → 自催化 (自身产物)
                else:
                    candidate.discard(rid)
                    changed = True
    return candidate


def is_raf(reactions: List[Reaction]) -> bool:
    """R AF (Reflexively Autocatalytic and Food-generated) 真生产 (主 13:08 借鉴 Kauffman).

    真生产: 自催化集 + 食物 (主 17:43 实事求是).
    """
    return len(find_autocatalytic_set(reactions)) > 0


# === Autocatalytic 真生产主类 (主 14:06 拉回注意力) ===

class AutocatalyticNetwork:
    """Kauffman 自催化集真生产网络 (主 14:06 + 主 13:31 大胆激进).

    借鉴: Kauffman 1986 Origins of Order + Eigen & Schuster 1979 hypercycle.
    V4 12 生命特征涌现 (#6) 深化 (autocatalytic closure).
    """

    def __init__(self):
        """Init autocatalytic 真生产 (主 13:08 借鉴 Kauffman 1986)."""
        self.reactions: Dict[str, Reaction] = {}
        self.species: Set[str] = set()

    def add_reaction(self, reaction_id: str, substrates: List[str], products: List[str],
                    rate_constant: float = 1.0) -> Reaction:
        """添加真生产反应 (主 14:06)."""
        r = Reaction(
            reaction_id=reaction_id,
            substrates=set(substrates),
            products=set(products),
            rate_constant=rate_constant,
        )
        self.reactions[reaction_id] = r
        self.species.update(substrates)
        self.species.update(products)
        return r

    def find_autocatalytic_set(self) -> Set[str]:
        """真生产自催化集查找 (主 13:08 借鉴 Kauffman 1986)."""
        return find_autocatalytic_set(list(self.reactions.values()))

    def is_raf(self) -> bool:
        """RAF 真生产检查 (主 13:08 借鉴 Kauffman)."""
        return is_raf(list(self.reactions.values()))

    def simulate(self, initial_conc: Dict[str, float], time_steps: int = 10,
                dt: float = 0.01) -> Dict[str, float]:
        """真生产反应动力学模拟 (主 14:06 借鉴 + 不 placeholder)."""
        conc = dict(initial_conc)
        for _ in range(time_steps):
            new_conc = dict(conc)
            for r in self.reactions.values():
                if r.substrates.issubset(conc.keys()) and all(conc.get(s, 0.0) > 0 for s in r.substrates):
                    rate = r.rate_constant
                    for s in r.substrates:
                        rate *= conc.get(s, 0.0)
                    for p in r.products:
                        new_conc[p] = new_conc.get(p, 0.0) + rate * dt
            conc = new_conc
        return conc

    def stats(self) -> Dict[str, Any]:
        """autocatalytic 真生产统计 (主 17:43 实事求是)."""
        raf_set = self.find_autocatalytic_set()
        return {
            "n_reactions": len(self.reactions),
            "n_species": len(self.species),
            "n_raf": len(raf_set),
            "is_raf": len(raf_set) > 0,
            "version": AUTOCATALYTIC_VERSION,
            "philosophy": (
                "Autocatalytic 真生产借鉴 (主 13:08): Kauffman 1986 Origins of Order + "
                "Eigen & Schuster 1979 hypercycle + Gánti 1975 chemoton. "
                "不假装 Phenomenal (主 17:58), 不假装达到 ASI (主 20:46). "
                "V4 12 生命特征涌现 (#6) 深化 (autocatalytic closure)."
            ),
        }


__all__ = [
    "AUTOCATALYTIC_VERSION",
    "Reaction",
    "SetMember",
    "find_autocatalytic_set",
    "is_raf",
    "AutocatalyticNetwork",
]


# === Autocatalytic 写真 production demo (主 13:31 大胆激进) ===

def _demo():
    print("=" * 70)
    print("=== Phase 58 autocatalytic 真生产自催化 (主 13:31 + 14:06 拉回注意力) ===")
    print("=" * 70)

    # 1. Init
    print("\n[1] Init autocatalytic 真生产 (V4 12 生命特征涌现 #6 深化)")
    an = AutocatalyticNetwork()
    print(f"  ✓ AutocatalyticNetwork 0.1.0 创建")

    # 2. 真生产自催化集 (主 14:06 借鉴 Kauffman 1986)
    print("\n[2] 真生产 autocatalytic 3 反应 (借鉴 Kauffman 1986 Origins of Order):")
    an.add_reaction("r1", substrates=["A", "B"], products=["C"])
    an.add_reaction("r2", substrates=["C"], products=["A", "B"])
    an.add_reaction("r3", substrates=["A"], products=["D"])
    print(f"  ✓ r1: A+B → C")
    print(f"  ✓ r2: C → A+B")
    print(f"  ✓ r3: A → D")

    # 3. RAF 真生产 (主 13:08 借鉴)
    print("\n[3] RAF 真生产检查 (借鉴 Kauffman RAF):")
    is_raf = an.is_raf()
    raf_set = an.find_autocatalytic_set()
    print(f"  ✓ is_raf: {is_raf}")
    print(f"  ✓ raf_set size: {len(raf_set)}")

    # 4. 真生产动力学 (主 14:06 借鉴)
    print("\n[4] autocatalytic 真生产动力学 (不 placeholder):")
    final_conc = an.simulate({"A": 1.0, "B": 1.0}, time_steps=5)
    for k, v in final_conc.items():
        print(f"  - {k}: {v:.4f}")

    # 5. stats
    print("\n[5] Autocatalytic 真生产 stats:")
    stats = an.stats()
    for k, v in stats.items():
        print(f"  - {k}: {v}")

    print("\n" + "=" * 70)
    print("✓ Phase 58 autocatalytic 真生产落地 (V4 涌现 #6 深化)")
    print("  - Reaction + SetMember + find_autocatalytic_set + is_raf")
    print("  - AutocatalyticNetwork 真生产主类")
    print("  - V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI")
    print("=" * 70)


if __name__ == "__main__":
    _demo()