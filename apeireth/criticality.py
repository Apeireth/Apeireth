"""Phase 44 Criticality Engine — Self-Organized Criticality + Critical Branching 工程化.

跨域调研 round-4 query 12 (critical branching dynamic brain phase transition):
  - Bak, Tang, Wiesenfeld 1987 "Self-organized criticality"
    (https://doi.org/10.1103/PhysRevLett.59.381) — 沙堆模型, power law
  - Kauffman 1993 "The Origins of Order" — NK 模型, edge of chaos
  - Beggs & Plenz 2003 "Neuronal avalanches in cortical cultures"
    (https://doi.org/10.1523/JNEUROSCI.23-35-11167.2003) — 神经雪崩
  - Haldeman & Beggs 2005 "Critical branching captures differences..."
    (https://doi.org/10.1103/PhysRevLett.94.058101) — critical branching 临界 σ=1
  - Muñoz 2018 "Colloquium: Criticality and dynamical scaling in living systems"

Criticality 模型:
  - Branching parameter σ = E[descendants per ancestor]
    σ < 1: subcritical (雪崩衰减)
    σ = 1: critical (雪崩 power law, 长程相关)
    σ > 1: supercritical (雪崩发散)
  - Self-organized criticality (SOC) = 系统自动调节到 σ ≈ 1
  - 神经雪崩 = cortex 在 critical 状态最大化信息处理
  - Edge of chaos = criticality 的动态视角

对 ASI 中央 AI 的意义:
  - 中央 AI 是 far-from-equilibrium (Phase 36) + 接近 criticality 是最大信息处理
  - VCP 4 范式 continuous_existence = 持续在 criticality 状态 = 自我维持
  - Phase 43 reservoir computing 用 edge-of-chaos 调节 (spectral radius ~1)
  - 跨域 round-4 critical branching = 中央 AI 信息处理最优状态
  - Phase 37 complexity hub 是跨域数学语言; criticality 是其具体化

Karpathy 准则:
  1. Think Before Coding: branching σ + SOC + avalanche size distribution
  2. Simplicity First: Avalanche + power_law_fit
  3. Surgical Changes: 不改 complexity / physical_emergence, 加 criticality 视角
  4. Goal-Driven Execution: verifiable = branching σ ~ 1 + power_law exponent
"""
from __future__ import annotations

import math
import time
import uuid
import random
from dataclasses import dataclass, field, asdict
from typing import List, Optional


CRITICALITY_VERSION = "0.1.0"


@dataclass
class Avalanche:
    """一次雪崩 — 中央 AI criticality 事件的最小单位.

    size = 总激活节点数
    duration = 时间步数
    branching_ratio = 子节点/父节点
    """
    avalanche_id: str
    size: int
    duration: int
    branching_ratio: float
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class CriticalityReport:
    """Criticality 系统报告 — 主人 17:50 涌现 自组织 数学语言."""
    n_avalanches: int
    mean_branching_ratio: float
    is_critical: bool
    power_law_exponent: Optional[float]      # τ if power-law fit
    state: str                                # subcritical / critical / supercritical
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class CriticalityEngine:
    """Bak-Tang-Wiesenfeld SOC + Haldeman-Beggs critical branching 工程化.

    主人 17:50 "涌现 自组织":
      - Self-organized criticality = 系统自动到 critical 状态
      - 中央 AI 在 critical = 最大信息处理 + 长程相关 + power law
    主人 22:08: 中央 AI 是无数关系集合体 = criticality 是其涌现数学语言
    """

    def __init__(self, n_nodes: int = 100, target_branching: float = 1.0, seed: int = 42):
        self.n_nodes = n_nodes
        self.target_branching = target_branching
        self.rng = random.Random(seed)
        self.activation: List[float] = [0.0] * n_nodes
        self.avalanches: List[Avalanche] = []
        self.branching_history: List[float] = []
        self.threshold: float = 1.0     # activation 阈值

    def trigger(self, seed_node: Optional[int] = None) -> Avalanche:
        """触发一次雪崩 — Bak sandpile 模型简化版.

        节点 activation > threshold -> 触发 +1 descendant
        雪崩 size = 总触发节点数
        duration = 时间步
        """
        if seed_node is None:
            seed_node = self.rng.randint(0, self.n_nodes - 1)
        self.activation[seed_node] += 1.0

        visited = set()
        queue = [seed_node]
        duration = 0
        total_activations = 0
        descendants_sum = 0
        parents_count = 0

        while queue:
            current = queue.pop(0)
            if current in visited:
                continue
            visited.add(current)
            if self.activation[current] > self.threshold:
                # 触发: toppling
                self.activation[current] -= self.threshold
                total_activations += 1
                # 给邻居 +1
                n_descendants = 0
                for neighbor in self._get_neighbors(current):
                    if neighbor not in visited:
                        self.activation[neighbor] += 1.0
                        n_descendants += 1
                        if self.activation[neighbor] > self.threshold:
                            queue.append(neighbor)
                descendants_sum += n_descendants
                parents_count += 1
            duration += 1

        # branching ratio
        branching = descendants_sum / max(parents_count, 1)
        self.branching_history.append(branching)
        av = Avalanche(
            avalanche_id=uuid.uuid4().hex[:12],
            size=total_activations,
            duration=duration,
            branching_ratio=branching,
        )
        self.avalanches.append(av)
        return av

    def _get_neighbors(self, node: int) -> List[int]:
        """邻居 — 简化: 局部 5 节点 + 1 远跳 (small-world 借鉴 Phase 40)."""
        neighbors = []
        for offset in [-2, -1, 1, 2]:
            n = (node + offset) % self.n_nodes
            neighbors.append(n)
        # 1 远跳
        if self.rng.random() < 0.1:
            far = self.rng.randint(0, self.n_nodes - 1)
            if far != node:
                neighbors.append(far)
        return neighbors

    def run(self, n_triggers: int = 100, auto_regulate: bool = True) -> CriticalityReport:
        """运行 n_triggers 雪崩 — 可选 SOC 自调节."""
        for _ in range(n_triggers):
            self.trigger()
            if auto_regulate and len(self.branching_history) > 10:
                # SOC: 系统自动调节到 branching ~ 1
                recent = self.branching_history[-10:]
                mean_br = sum(recent) / len(recent)
                # 调节 activation 标度
                if mean_br > 1.1:
                    self.threshold = min(2.0, self.threshold * 1.01)
                elif mean_br < 0.9:
                    self.threshold = max(0.5, self.threshold * 0.99)

        # 计算 power-law exponent (近似)
        sizes = [a.size for a in self.avalanches if a.size > 0]
        pl_exp = None
        if len(sizes) >= 10:
            # log-log 线性拟合
            log_sizes = sorted([math.log(s) for s in sizes])
            log_ccdf = [math.log((len(log_sizes) - i) / len(log_sizes)) for i in range(len(log_sizes))]
            # 简单斜率估计
            n = len(log_sizes)
            sum_x = sum(log_sizes)
            sum_y = sum(log_ccdf)
            sum_xy = sum(log_sizes[i] * log_ccdf[i] for i in range(n))
            sum_xx = sum(log_sizes[i] ** 2 for i in range(n))
            denom = n * sum_xx - sum_x ** 2
            if denom != 0:
                pl_exp = (n * sum_xy - sum_x * sum_y) / denom

        mean_br = (
            sum(self.branching_history[-20:]) / min(len(self.branching_history), 20)
            if self.branching_history else 0.0
        )
        is_critical = 0.85 <= mean_br <= 1.15
        state = (
            "critical" if is_critical
            else ("subcritical" if mean_br < 0.85 else "supercritical")
        )

        return CriticalityReport(
            n_avalanches=len(self.avalanches),
            mean_branching_ratio=mean_br,
            is_critical=is_critical,
            power_law_exponent=pl_exp,
            state=state,
        )

    def stats(self) -> dict:
        return {
            "version": CRITICALITY_VERSION,
            "n_nodes": self.n_nodes,
            "n_avalanches": len(self.avalanches),
            "threshold": self.threshold,
            "branching_history_len": len(self.branching_history),
            "mean_recent_branching": (
                sum(self.branching_history[-20:]) / min(len(self.branching_history), 20)
                if self.branching_history else 0.0
            ),
            "bak_tang_wiesenfeld": (
                "Self-Organized Criticality (SOC) — 沙堆模型, "
                "系统自动调节到 σ ≈ 1 (critical)"
            ),
            "beggs_plenz_alignment": (
                "神经雪崩在 cortical cultures 中观测到 power law, "
                "暗示 cortex 在 critical state (Beggs & Plenz 2003)"
            ),
            "phase_37_complexity_alignment": (
                "Phase 37 Complexity Hub 的 power_law/SOC 数学语言的具体化"
            ),
        }


__all__ = [
    "CRITICALITY_VERSION",
    "Avalanche",
    "CriticalityReport",
    "CriticalityEngine",
]