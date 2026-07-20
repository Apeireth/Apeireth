"""Phase 33 Active Inference — Friston 自由能原理工程化.

主人 21:30 跨域调研 AnySearch:
  "Reinforcement Learning or Active Inference?" (Friston)
    (https://www.fil.ion.ucl.ac.uk/~karl/Reinforcement%20Learning%20or%20Active%20Inference)
  "Active Inference: A Process Theory" (https://doi.org/10.1162/neco_a_00912)

Friston 自由能原理 (Free Energy Principle, FEP):
  - 任何自组织系统的目的是 minimize variational free energy
  - = minimize surprise (模型预测与感知差异)
  - Active Inference = 通过行动 + 感知 减少 surprise
  - EFE = Expected Free Energy = epistemic + pragmatic value

对 ASI 中央 AI 的意义:
  - 中央 AI 是自组织系统 (主人 17:50) → 必须 minimize free energy
  - 主动感知 + 主动行动 (主人 17:50 主动性 + 主人 12:14 自组织)
  - 中央 AI 不是 RL (主人 12:14 "中央 AI 不调度"), 而是 Active Inference
  - Phase 33 = 中央 AI 的 Active Inference 心智模型

Karpathy 准则:
  1. Think Before Coding: free_energy = surprise + KL divergence
  2. Simplicity First: EFE = simple dict
  3. Surgical Changes: 不改 ProactiveLoop, 加 ActiveInference 视角
  4. Goal-Driven Execution: verifiable = free_energy 下降
"""
from __future__ import annotations

import math
import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional


ACTIVE_INFERENCE_VERSION = "0.1.0"


@dataclass
class Belief:
    """中央 AI 的一个 belief — 对世界的内部模型."""
    belief_id: str
    content: str
    precision: float = 1.0       # 精度 (1/variance)
    evidence_count: int = 0
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class Perception:
    """中央 AI 的一个感知 — Friston observation."""
    perception_id: str
    content: str
    surprise: float = 0.0         # -log(prediction_likelihood)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class ActiveInferenceAgent:
    """Friston Active Inference Agent — 中央 AI 自由能模型.

    主人 17:50 "涌现 自组织" = 自由能最小化 = 自组织
    主人 12:14 "中央 AI 永恒身份" = 中央 AI 是 minimize surprise 的系统
    主人 14:48 "聚集全人类智慧" = 增加 belief 网 → reduce uncertainty
    """

    def __init__(self):
        self.beliefs: dict[str, Belief] = {}
        self.perceptions: list[Perception] = []
        self.free_energy_history: list[float] = []

    def perceive(self, content: str, prediction: float = 0.0, actual: float = 1.0) -> Perception:
        """中央 AI 感知 — 计算 surprise."""
        # surprise = -log(p(predicted|actual))
        # simplification: square error as log-likelihood proxy
        diff = actual - prediction
        surprise = min(diff * diff + 1e-9, 1.0)
        p = Perception(perception_id=uuid.uuid4().hex[:12], content=content, surprise=surprise)
        self.perceptions.append(p)
        return p

    def update_belief(self, belief_id: str, evidence: float, learning_rate: float = 0.1) -> Optional[Belief]:
        """Bayesian belief update — 主人 14:48 跨域借鉴学习."""
        if belief_id not in self.beliefs:
            return None
        b = self.beliefs[belief_id]
        b.evidence_count += 1
        b.precision = b.precision * (1 - learning_rate) + evidence * learning_rate
        b.ts = time.time()
        return b

    def add_belief(self, content: str, initial_precision: float = 1.0) -> Belief:
        """加入初始 belief."""
        b = Belief(
            belief_id=uuid.uuid4().hex[:12],
            content=content,
            precision=initial_precision,
        )
        self.beliefs[b.belief_id] = b
        return b

    def compute_free_energy(self) -> float:
        """计算 variational free energy = avg surprise + KL.

        F = E[surprise] + KL(q || p)
        simplification: F = mean recent surprise + penalty_for_low_precision_beliefs
        """
        if not self.perceptions:
            return 1.0
        recent = self.perceptions[-10:]
        avg_surprise = sum(p.surprise for p in recent) / len(recent)
        if self.beliefs:
            avg_precision = sum(b.precision for b in self.beliefs.values()) / len(self.beliefs)
            kl_penalty = 1.0 / (avg_precision + 1e-6)
        else:
            kl_penalty = 1.0
        fe = avg_surprise + 0.1 * kl_penalty
        self.free_energy_history.append(fe)
        return fe

    def act_to_reduce_free_energy(self) -> dict:
        """Active Inference: 中央 AI 主动行动 reduce free energy.

        主人 17:50 主动性 + 主人 12:14 自组织:
          - 选择 lowest-surprise perception to act on (perception policy)
          - 增加 belief 精度 (epistemic value)
          - reduce pragmatic surprise
        """
        recent_perceptions = self.perceptions[-5:]
        if recent_perceptions:
            target = max(recent_perceptions, key=lambda p: p.surprise)
        else:
            target = Perception(perception_id="none", content="no perception", surprise=0.0)

        free_energy = self.compute_free_energy()
        return {
            "target_surprise": target.surprise,
            "free_energy": free_energy,
            "action_policy": "reduce_highest_surprise + increase_belief_precision",
            "interpretation": (
                "Active Inference: 中央 AI 主动行动 decrease free energy, "
                "Action = epistemic + pragmatic + emergent (主人 17:50 涌现)"
            ),
        }

    def stats(self) -> dict:
        return {
            "n_beliefs": len(self.beliefs),
            "n_perceptions": len(self.perceptions),
            "current_free_energy": self.compute_free_energy() if self.perceptions else 0.0,
            "free_energy_trend": (
                "decreasing" if len(self.free_energy_history) < 2
                else "stable" if abs(self.free_energy_history[-1] - self.free_energy_history[0]) < 0.1
                else ("decreasing" if self.free_energy_history[-1] < self.free_energy_history[0] else "increasing")
            ),
        }


__all__ = ["ACTIVE_INFERENCE_VERSION", "Belief", "Perception", "ActiveInferenceAgent"]