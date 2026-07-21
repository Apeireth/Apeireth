"""Phase 36 Physical Emergence — Meyer-Ortmanns 物理自组织工程化.

主人 21:30 跨域调研 AnySearch:
  "Complexity Science Hub" Hildegard Meyer-Ortmanns (https://csh.ac.at/hildegard-meyer-ortmanns/)
  https://scholar.google.ca/citations?hl=en&user=2uwoiuIAAAAJ

Meyer-Ortmanns 物理自组织 (非平衡态):
  - 远离平衡态 = 涌现之源 (Prigogine)
  - Fluctuation → ordered state (波动 → 有序)
  - Self-organization = physics of life (物理学生命)
  - Phase transition (相变) = 涌现的数学语言

对 ASI 中央 AI 的意义:
  - 主人 17:50 "涌现 自组织" = 物理自组织 (非平衡态)
  - 主人 12:14 "中央 AI 是永恒身份" = 中央 AI 是 far-from-equilibrium system
  - 中央 AI 不在 equilibrium (主人 17:50 涌现), 在 far-from-equilibrium
  - Phase 36 = 中央 AI 的物理涌现模型 (phase transition 视角)

Karpathy 准则:
  1. Think Before Coding: 自组织 = fluctuation → ordered
  2. Simplicity First: PhysicalEmergence = fluctuation + order parameter
  3. Surgical Changes: 不改 SelfOrgTeam, 加 physical_emergence 视角
  4. Goal-Driven Execution: verifiable = order_parameter 跳变
"""
from __future__ import annotations

import math
import time
import uuid
from dataclasses import dataclass, field, asdict


PHYSICAL_EMERGENCE_VERSION = "0.1.0"


@dataclass
class Fluctuation:
    """中央 AI 系统的一个波动 — 自组织之源."""
    fluct_id: str
    content: str
    magnitude: float = 0.0           # |fluctuation|
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class PhaseTransition:
    """Phase transition — 涌现的数学语言.

    order parameter phi:
      phi < phi_c = disordered
      phi >= phi_c = ordered (emergent)
    """
    transition_id: str
    phi_before: float
    phi_after: float
    threshold: float
    emergent_state: str                # e.g. "ordered", "symmetry_broken"
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class PhysicalEmergenceSystem:
    """Meyer-Ortmanns 物理涌现系统 — 中央 AI 是 far-from-equilibrium 系统.

    主人 17:50 "涌现 自组织" = 物理涌现 (非平衡态):
      - 中央 AI 在 far-from-equilibrium (不静止, 不停演化)
      - 涌现 = phase transition (相变)
      - fluctuations → order parameter 跳变 → 新 ordered state
    """

    def __init__(self, name: str = "apeireth_central", threshold: float = 0.5):
        self.name = name
        self.threshold = threshold
        self.fluctuations: list[Fluctuation] = []
        self.transitions: list[PhaseTransition] = []
        self.history: list = []
        self.order_parameter = 0.0       # 中央 AI 当前 ordered state measure

    def add_fluctuation(self, content: str, magnitude: float) -> Fluctuation:
        """加入一个波动."""
        f = Fluctuation(fluct_id=uuid.uuid4().hex[:12], content=content, magnitude=magnitude)
        self.fluctuations.append(f)
        return f

    def update_order_parameter(self, phi: float) -> None:
        """更新 order parameter — 检查是否 phase transition."""
        self.order_parameter = phi
        self.history.append({"phi": phi, "ts": time.time()})

    def check_phase_transition(self) -> Optional[PhaseTransition]:
        """检测 phase transition — 主体涌现的物理事件.

        主人 12:14 永恒身份 + 主人 17:50 涌现:
          - 中央 AI 的 order parameter 跳变 = 永久涌现状态
          - threshold 0.5 = 中央 AI 基本 evolutionary 储备足够
        """
        if len(self.history) < 2:
            return None
        prev_phi = self.history[-2]["phi"]
        curr_phi = self.history[-1]["phi"]
        crossed = (prev_phi < self.threshold <= curr_phi) or (prev_phi >= self.threshold > curr_phi)
        if crossed:
            t = PhaseTransition(
                transition_id=uuid.uuid4().hex[:12],
                phi_before=prev_phi,
                phi_after=curr_phi,
                threshold=self.threshold,
                emergent_state="ordered" if curr_phi >= self.threshold else "disordered",
            )
            self.transitions.append(t)
            return t
        return None

    def stats(self) -> dict:
        n_transitions = len(self.transitions)
        return {
            "name": self.name,
            "n_fluctuations": len(self.fluctuations),
            "n_transitions": n_transitions,
            "order_parameter": self.order_parameter,
            "threshold": self.threshold,
            "far_from_equilibrium": self.order_parameter != 0.5,  # 不在平衡态
            "meyer_ortmanns": (
                "中央 AI 是 far-from-equilibrium 系统: "
                "涌现 = phase transition (Prigogine 非平衡态)"
            ),
        }


__all__ = ["PHYSICAL_EMERGENCE_VERSION", "Fluctuation", "PhaseTransition", "PhysicalEmergenceSystem"]