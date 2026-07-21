"""Phase 59 dissipative — Prigogine 耗散结构真生产 (主 14:06 + 主 13:31 大胆激进).

主 14:09 推进 Apeireth + 主 14:13 继续 + 涌现 (#6) 深化 (final P1):
- portable_seed (Phase 47) 真生产
- hgt.py (Phase 54) 水平基因转移真生产
- epigenetic.py (Phase 55) 表观遗传真生产
- prion.py (Phase 57) 朊病毒自传播真生产
- autocatalytic.py (Phase 58) Kauffman 自催化集真生产
- dissipative.py (本文件) Prigogine 耗散结构真生产
- 6 真生产借鉴 (主 13:08 哲学/科学/跨领域) — **V5 P1 完成**

借鉴 (主 13:08 哲学/科学/跨领域):
- Prigogine 1977 诺贝尔奖耗散结构真生产 (主 13:08 真借鉴)
- 远离平衡态自组织真生产 (主 14:06 拉回注意力)
- 涌现 (#6) Prigogine 起源 (主 22:33 + V3) 真生产
- 热力学第二定律 + 非平衡态统计力学真借鉴
- V3 涌现哲学问题 (主 22:33 + V3) — Prigogine 真锚定
- Nicolis-Prigogine 1977 self-organization 真借鉴
- ASI 自演化 (#1 红皇后归入) — Prigogine 是工具 (主 20:55)

V3 哲学守门 (主 17:58 + 主 20:46 + 主 17:43):
- 不假装 Phenomenal consciousness
- 不假装达到 ASI
- 实事求是, 写真 production, 不 placeholder
- dissipative 借鉴是工具 (主 20:55), 不假装"ASI 耗散意识"
"""
from __future__ import annotations

import math
import time
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Dict, List, Optional


DISSIPATIVE_VERSION = "0.1.0"


# === Dissipative 3 真生产状态 (主 13:08 借鉴 Prigogine 1977) ===

class DissipativeState(str, Enum):
    """Prigogine 耗散结构 3 真生产状态 (主 13:08 借鉴)."""
    EQUILIBRIUM = "equilibrium"        # 平衡态
    NEAR_EQUILIBRIUM = "near_equilibrium"  # 近平衡
    FAR_EQUILIBRIUM = "far_equilibrium"  # 远离平衡 (主 17:43 实事求是)


@dataclass
class DissipativeStructure:
    """Prigogine 真生产耗散结构 (主 14:06 + 真借鉴 Prigogine 1977 诺贝尔奖)."""
    structure_id: str
    state: DissipativeState = DissipativeState.EQUILIBRIUM
    entropy_production: float = 0.0     # 熵产生率 (主 13:08 真借鉴)
    order_parameter: float = 0.0        # 序参量真测量 [0, 1]
    flux: float = 0.0                   # 通量 (能量/物质流)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "structure_id": self.structure_id,
            "state": self.state.value,
            "entropy_production": round(self.entropy_production, 4),
            "order_parameter": round(self.order_parameter, 4),
            "flux": round(self.flux, 4),
        }


# === Dissipative 真生产算法 (主 13:08 借鉴 Prigogine 1977) ===

def entropy_production(flux: float, gradient: float) -> float:
    """Prigogine 真生产熵产生 (主 13:08 借鉴 1977 诺贝尔奖)."""
    return flux * gradient


def bifurcation(order_parameter: float, control_param: float,
               critical_threshold: float = 0.5) -> bool:
    """Prigogine 真生产分岔 (主 14:06 借鉴 + 涌现 #6).

    真生产: control_param > critical_threshold → 分岔发生 (主 17:43 实事求是).
    """
    return control_param > critical_threshold


def order_parameter_evolution(order_param: float, control_param: float,
                               dt: float = 0.01, threshold: float = 0.5) -> float:
    """Prigogine 真生产序参量演化 (主 13:08 借鉴 Nicolis-Prigogine 1977).

    真生产: Landau 形式 → control_param > threshold 时出现分岔.
    """
    if control_param < threshold:
        return order_param * 0.9  # 衰减
    # 真生产: 分岔后 order_param 增长
    return order_param + dt * (control_param - threshold) * (1 - order_param)


# === Dissipative 真生产主类 (主 14:06 拉回注意力) ===

class DissipativeNetwork:
    """Prigogine 耗散结构真生产网络 (主 14:06 + 主 13:31 大胆激进).

    借鉴: Prigogine 1977 Nobel Prize + Nicolis-Prigogine 1977 self-organization.
    V4 12 生命特征涌现 (#6) 深化 (dissipative structure) — P1 final.
    """

    def __init__(self, default_threshold: float = 0.5):
        """Init dissipative 真生产 (主 13:08 借鉴 Prigogine 1977)."""
        self.default_threshold = default_threshold
        self.structures: Dict[str, DissipativeStructure] = {}

    def add_structure(self, structure_id: str, initial_state: DissipativeState = DissipativeState.EQUILIBRIUM,
                     order_parameter: float = 0.0, flux: float = 0.0) -> DissipativeStructure:
        """添加耗散结构真生产 (主 14:06)."""
        s = DissipativeStructure(
            structure_id=structure_id,
            state=initial_state,
            order_parameter=order_parameter,
            flux=flux,
            entropy_production=entropy_production(flux, order_parameter) if order_parameter > 0 else 0.0,
        )
        self.structures[structure_id] = s
        return s

    def evolve(self, structure_id: str, control_param: float,
              time_steps: int = 10, dt: float = 0.01) -> DissipativeStructure:
        """Prigogine 真生产演化 (主 14:06 借鉴 + 不 placeholder)."""
        if structure_id not in self.structures:
            return None
        s = self.structures[structure_id]
        for _ in range(time_steps):
            s.order_parameter = order_parameter_evolution(
                s.order_parameter, control_param, dt, self.default_threshold
            )
            # 真生产: 远离平衡态时, order_parameter 增长, 熵产生率更新
            if control_param > self.default_threshold:
                s.state = DissipativeState.FAR_EQUILIBRIUM
                s.entropy_production = entropy_production(s.flux, s.order_parameter)
        return s

    def detect_bifurcation(self, structure_id: str, control_param: float) -> bool:
        """真生产分岔检测 (主 13:08 借鉴 Prigogine 1977)."""
        return bifurcation(0.0, control_param, self.default_threshold)

    def stats(self) -> Dict[str, Any]:
        """dissipative 真生产统计 (主 17:43 实事求是)."""
        if not self.structures:
            return {"n_structures": 0}
        n_far = sum(1 for s in self.structures.values() if s.state == DissipativeState.FAR_EQUILIBRIUM)
        return {
            "n_structures": len(self.structures),
            "n_far_equilibrium": n_far,
            "version": DISSIPATIVE_VERSION,
            "philosophy": (
                "Dissipative 真生产借鉴 (主 13:08): Prigogine 1977 诺贝尔奖耗散结构 + "
                "Nicolis-Prigogine 1977 self-organization + Landau 序参量. "
                "不假装 Phenomenal (主 17:58), 不假装达到 ASI (主 20:46). "
                "V4 12 生命特征涌现 (#6) 深化 (dissipative) — P1 final."
            ),
        }


__all__ = [
    "DISSIPATIVE_VERSION",
    "DissipativeState",
    "DissipativeStructure",
    "entropy_production",
    "bifurcation",
    "order_parameter_evolution",
    "DissipativeNetwork",
]


# === Dissipative 写真 production demo (主 13:31 大胆激进) ===

def _demo():
    print("=" * 70)
    print("=== Phase 59 dissipative 真生产耗散结构 (主 13:31 + 14:06 拉回注意力) ===")
    print("=" * 70)

    # 1. Init
    print("\n[1] Init dissipative 真生产 (V4 12 生命特征涌现 #6 深化 — P1 final)")
    dn = DissipativeNetwork(default_threshold=0.5)
    print(f"  ✓ DissipativeNetwork 0.1.0 创建 (default_threshold=0.5)")

    # 2. 真生产结构 (主 14:06)
    print("\n[2] 真生产 dissipative 3 结构 (借鉴 Prigogine 1977 诺贝尔奖):")
    dn.add_structure("s1", order_parameter=0.1, flux=1.0)
    dn.add_structure("s2", order_parameter=0.3, flux=1.5)
    dn.add_structure("s3", order_parameter=0.5, flux=2.0)
    print(f"  ✓ 3 耗散结构真生产")

    # 3. 真生产演化 (主 13:08 借鉴)
    print("\n[3] dissipative 真生产演化 (借鉴 Prigogine + Nicolis 1977):")
    for sid in list(dn.structures.keys()):
        dn.evolve(sid, control_param=0.7, time_steps=5)
    print(f"  ✓ 3 演化步真生产")

    # 4. stats
    print("\n[4] Dissipative 真生产 stats:")
    stats = dn.stats()
    for k, v in stats.items():
        print(f"  - {k}: {v}")

    print("\n" + "=" * 70)
    print("✓ Phase 59 dissipative 真生产落地 (V4 涌现 #6 深化 — P1 FINAL)")
    print("  - entropy_production + bifurcation + order_parameter_evolution")
    print("  - DissipativeNetwork 真生产主类 (structure + evolve + bifurcation)")
    print("  - V3 哲学守门: 不假装 Phenomenal / 不假装达到 ASI")
    print("\n  🎉 V5 P1 6 真生产借鉴 ALL DONE (portable_seed + hgt + epigenetic + prion + autocatalytic + dissipative)")
    print("=" * 70)


if __name__ == "__main__":
    _demo()