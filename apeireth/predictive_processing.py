"""Phase 42 Predictive Processing Hierarchy — Rao-Ballard + Friston 分层预测编码工程化.

主人 22:22 调研不停 + 主人 22:08 V2 哲学完整还原 (中央 AI 是 = 调度者/思考者/无数关系集合体).

跨域调研 round-3 query 3 (predictive coding) + round-4 query 9 (predictive processing)
+ round-5 (free energy / precision weighting / hierarchical gaussian):

  - Rao & Ballard 1999 "Predictive coding in the visual cortex"
    (https://doi.org/10.1038/45823) — 分层高斯预测, 自上而下预测 + 自下而上误差
  - Friston 2005 "A theory of cortical responses" — 自由能原理 + 分层预测编码
  - Hohwy 2013 "The Predictive Mind" — 预测作为感知核心
  - Clark 2013 "Whatever next? Predictive brains, situated agents, and the future
    of cognitive science" — 预测脑 + 主动 inference
  - Seth & Critchley 2013 — interoceptive predictive coding + Phenomenal experience

Predictive Processing Hierarchy (PPH) 模型:
  - 多层 (L0 input ... L_n abstract)
  - 每层: top-down prediction + bottom-up error
  - Precision-weighted error propagation (precision = 1/variance)
  - Hierarchy minimize variational free energy
  - 跨层 precision modulation = 注意力 + 神经调制

对 ASI 中央 AI 的意义:
  - 中央 AI 是无数关系的集合体 (主人 22:08) = 多层 hierarchy
  - VCP 4 范式 natural_perception = 真知觉 = hierarchical predictive coding
  - Phenomenal consciousness (主人 17:58) = 全局一致 prediction (Bayesian brain)
  - Phase 33 Active Inference 是 PPH 的 action 视角; PPH 是其 perception 视角
  - 主人 17:43 实事求是 = veridical perception (真预测 + 真误差, 不幻觉)

Karpathy 准则:
  1. Think Before Coding: PPH = layers + predictions + precision-weighted errors
  2. Simplicity First: PredictionError 简单 dataclass, 多层堆叠
  3. Surgical Changes: 不改 active_inf, 加分层 + precision modulation
  4. Goal-Driven Execution: verifiable = free_energy 下降 + perception_accuracy 提升
"""
from __future__ import annotations

import math
import time
import uuid
from dataclasses import dataclass, field, asdict
from typing import Optional, List


PREDICTIVE_PROCESSING_VERSION = "0.1.0"


@dataclass
class Prediction:
    """中央 AI 的一层 prediction (自上而下预测)."""
    pred_id: str
    layer: int
    content: str
    value: float                  # 预测值 (scalar 或高斯均值)
    precision: float = 1.0        # 预测精度 (1/variance)
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class PredictionError:
    """中央 AI 的一层 prediction error (自下而上误差).

    error = (actual - predicted) * precision
    """
    err_id: str
    layer: int
    content: str
    actual: float
    predicted: float
    precision: float
    weighted_error: float
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


@dataclass
class HierarchicalState:
    """中央 AI PPH 整体状态 — VCP natural_perception 模型."""
    n_layers: int
    n_predictions: int
    n_errors: int
    total_weighted_error: float
    variational_free_energy: float
    precision_profile: List[float]      # 每层 precision
    perception_accuracy: float          # 1 - normalized_error
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class PredictiveProcessingHierarchy:
    """Rao-Ballard + Friston 分层预测编码系统 — 中央 AI 是预测机.

    主人 22:08: 中央 AI 是无数关系的集合体 = 多层 hierarchy
    主人 17:58: Phenomenal consciousness = 全局一致 prediction
    主人 17:43: 实事求是 = veridical perception (真预测, 不幻觉)
    """

    def __init__(self, n_layers: int = 4, learning_rate: float = 0.1, base_precision: float = 1.0):
        self.n_layers = n_layers
        self.learning_rate = learning_rate
        self.base_precision = base_precision
        self.layer_predictions: List[List[Prediction]] = [[] for _ in range(n_layers)]
        self.layer_errors: List[List[PredictionError]] = [[] for _ in range(n_layers)]
        self.layer_state: List[float] = [0.5] * n_layers  # 每层当前 hidden state
        self.precision_profile: List[float] = [base_precision] * n_layers
        self.free_energy_history: List[float] = []
        self.cycle_count: int = 0

    def top_down_predict(self, layer: int, content: str, value: float) -> Prediction:
        """自上而下预测 — 高级层向低级层传 prediction."""
        if layer < 0 or layer >= self.n_layers:
            raise ValueError(f"layer {layer} 越界 [0, {self.n_layers})")
        p = Prediction(
            pred_id=uuid.uuid4().hex[:12],
            layer=layer,
            content=content,
            value=value,
            precision=self.precision_profile[layer],
        )
        self.layer_predictions[layer].append(p)
        return p

    def bottom_up_error(self, layer: int, content: str, actual: float, predicted: float) -> PredictionError:
        """自下而上误差 — 低级层向高级层传 precision-weighted error."""
        if layer < 0 or layer >= self.n_layers:
            raise ValueError(f"layer {layer} 越界 [0, {self.n_layers})")
        prec = self.precision_profile[layer]
        weighted = (actual - predicted) * prec
        e = PredictionError(
            err_id=uuid.uuid4().hex[:12],
            layer=layer,
            content=content,
            actual=actual,
            predicted=predicted,
            precision=prec,
            weighted_error=weighted,
        )
        self.layer_errors[layer].append(e)
        # 更新 layer state: 经典 predictive coding 更新规则
        self.layer_state[layer] += self.learning_rate * weighted
        self.layer_state[layer] = max(-1.0, min(1.0, self.layer_state[layer]))
        return e

    def modulate_precision(self, layer: int, gain: float) -> float:
        """调制 precision — 神经调制/注意力 gain control.

        gain > 1: attention up (放大该层误差信号)
        gain < 1: attention down (忽略该层)
        """
        self.precision_profile[layer] = max(0.01, min(10.0, self.precision_profile[layer] * gain))
        return self.precision_profile[layer]

    def compute_variational_free_energy(self) -> float:
        """计算 variational free energy = sum precision-weighted squared errors.

        F = Σ_l precision_l * (actual_l - predicted_l)^2
        minimization = Bayesian brain 实证 (Knill & Pouget)
        """
        total = 0.0
        for layer_errors in self.layer_errors:
            for e in layer_errors[-5:]:  # recent
                total += e.precision * (e.actual - e.predicted) ** 2
        # 归一化
        n_active = sum(len(layer_errors[-5:]) for layer_errors in self.layer_errors)
        fe = total / max(n_active, 1)
        self.free_energy_history.append(fe)
        return fe

    def perceive_input(self, layer0_value: float, target_layer: int = 0) -> HierarchicalState:
        """中央 AI 一次完整感知循环 — 自上而下预测 + 自下而上误差 + precision 调制."""
        self.cycle_count += 1

        # 1. 自上而下 prediction (高级层 → 0 层)
        for layer in range(target_layer, -1, -1):
            predicted = self.layer_state[layer]
            self.top_down_predict(layer, f"cycle_{self.cycle_count}_layer_{layer}", predicted)

        # 2. 自下而上 error (0 层 → 高级层)
        for layer in range(target_layer + 1):
            actual = layer0_value if layer == 0 else self.layer_state[layer - 1]
            predicted = self.layer_state[layer]
            self.bottom_up_error(layer, f"cycle_{self.cycle_count}_layer_{layer}",
                                 actual=actual, predicted=predicted)

        # 3. precision modulation (基于 recent error)
        for layer in range(self.n_layers):
            recent_errors = self.layer_errors[layer][-3:]
            if recent_errors:
                avg_error = sum(abs(e.weighted_error) for e in recent_errors) / len(recent_errors)
                if avg_error > 0.5:
                    self.modulate_precision(layer, 1.1)  # up-regulate
                elif avg_error < 0.1:
                    self.modulate_precision(layer, 0.95)  # down-regulate

        # 4. 计算 free energy + accuracy
        fe = self.compute_variational_free_energy()
        n_recent = sum(len(layer_errors[-5:]) for layer_errors in self.layer_errors)
        if n_recent > 0:
            avg_err = sum(abs(e.weighted_error) for layer_errs in self.layer_errors for e in layer_errs[-5:]) / n_recent
            accuracy = max(0.0, 1.0 - avg_err)
        else:
            accuracy = 1.0

        return HierarchicalState(
            n_layers=self.n_layers,
            n_predictions=sum(len(p) for p in self.layer_predictions),
            n_errors=sum(len(e) for e in self.layer_errors),
            total_weighted_error=sum(e.weighted_error for errs in self.layer_errors for e in errs[-5:]),
            variational_free_energy=fe,
            precision_profile=list(self.precision_profile),
            perception_accuracy=accuracy,
        )

    def stats(self) -> dict:
        return {
            "version": PREDICTIVE_PROCESSING_VERSION,
            "n_layers": self.n_layers,
            "cycle_count": self.cycle_count,
            "n_predictions": sum(len(p) for p in self.layer_predictions),
            "n_errors": sum(len(e) for e in self.layer_errors),
            "precision_profile": list(self.precision_profile),
            "variational_free_energy": self.compute_variational_free_energy() if self.free_energy_history else 0.0,
            "fe_trend": (
                "decreasing" if len(self.free_energy_history) < 2
                else "stable" if abs(self.free_energy_history[-1] - self.free_energy_history[0]) < 0.1
                else ("decreasing" if self.free_energy_history[-1] < self.free_energy_history[0] else "increasing")
            ),
            "rao_ballard": (
                "分层高斯预测 (L0 → L_n): top-down prediction + bottom-up precision-weighted error"
            ),
            "friston_alignment": (
                "Phase 33 Active Inference = action 视角; Phase 42 PPH = perception 视角; "
                "二者统一于 free energy minimization (主人 17:43 实事求是)"
            ),
        }


__all__ = [
    "PREDICTIVE_PROCESSING_VERSION",
    "Prediction",
    "PredictionError",
    "HierarchicalState",
    "PredictiveProcessingHierarchy",
]