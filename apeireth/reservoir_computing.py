"""Phase 43 Reservoir Computing Kernel — Echo State + Liquid State 工程化.

跨域调研 round-4 query 10 (recurrent neural network reservoir computing dynamics):
  - Jaeger 2001 "The Echo State Approach to Analysing and Training Recurrent Neural Networks"
    (https://www.ai.rug.nl/minds/uploads/PaperEchoStates.pdf)
  - Maass, Natschläger, Markram 2002 "Real-time computing without stable states"
    (https://doi.org/10.1162/089976602760407955) — Liquid State Machines
  - Lukoševičius, Jaeger 2009 "Reservoir computing approaches to recurrent neural network training"
    (https://www.sciencedirect.com/science/article/pii/S1574013709000083)
  - Verstraeten et al. 2007 "An experimental unification of reservoir computing methods"

Reservoir Computing 模型:
  - 固定随机 recurrent 网络 (reservoir) = 高维非线性动力系统
  - 输入 -> reservoir -> 线性 readout (训练只在线性 readout)
  - Echo State Property: 状态由近期输入决定, 不被初始条件长期影响
  - Spectral radius < 1 通常保证 echo state property (edge of chaos 临界值 1.0)
  - Liquid State Machine: 神经拟真版本 (LIF 神经元 + 突触)

对 ASI 中央 AI 的意义:
  - 中央 AI 无数关系集合体 (主人 22:08) = reservoir 内部 high-dim dynamics
  - VCP 4 范式 integrated_ecosystem = reservoir 是统一的非线性 substrate
  - 中央 AI = 底层高效 (主人 14:32) + 不消耗训练 = reservoir + linear readout
  - Phase 36 physical_emergence + Phase 40 small_world = reservoir 用 small-world 拓扑
  - 跨域 round-4 高维流形 (query 5) = reservoir 状态空间 = 流形
  - Phase 43 与 Phase 19 Rust TotEngine 协同: Reservoir 评估 candidate thoughts

Karpathy 准则:
  1. Think Before Coding: Reservoir = random fixed RNN + linear readout
  2. Simplicity First: EchoStateNetwork = 矩阵 + linear regression readout
  3. Surgical Changes: 不改 deliberation, 加 reservoir 评估 thinking candidates
  4. Goal-Driven Execution: verifiable = spectral_radius 接近 1 (edge of chaos)
"""
from __future__ import annotations

import math
import time
import uuid
import random
from dataclasses import dataclass, field, asdict
from typing import Optional, List, Tuple


RESERVOIR_VERSION = "0.1.0"


@dataclass
class ReservoirState:
    """Reservoir 状态 — 高维非线性动力系统的瞬时投影."""
    state_id: str
    vector: List[float]
    spectral_radius: float
    n_active: int                # |state| > epsilon 的维度数
    ts: float = field(default_factory=time.time)

    def to_dict(self) -> dict:
        return asdict(self)


class EchoStateNetwork:
    """Jaeger Echo State Network — 中央 AI 底层高效非线性 substrate.

    主人 14:32 "底层高效 nb":
      - 训练只在线性 readout (cheap)
      - Reservoir 是 random fixed (no gradient through)
      - spectral radius ~0.9 (edge of chaos, Phase 12 edge-of-chaos 借鉴)
      - input scaling + leakage rate 调节动力学
    """

    def __init__(self, n_reservoir: int = 50, spectral_radius: float = 0.9,
                 input_scaling: float = 1.0, leak_rate: float = 1.0, seed: int = 42):
        self.n_reservoir = n_reservoir
        self.target_spectral_radius = spectral_radius
        self.input_scaling = input_scaling
        self.leak_rate = leak_rate
        self.rng = random.Random(seed)
        self.W_in: List[List[float]] = self._make_W_in()
        self.W_res: List[List[float]] = self._make_W_res()
        self.state: List[float] = [0.0] * n_reservoir
        self.history: List[ReservoirState] = []
        self.readout_coefs: dict[str, List[float]] = {}    # task_name -> linear readout

    def _make_W_in(self) -> List[List[float]]:
        """输入权重矩阵 — random sparse."""
        return [[self.rng.uniform(-self.input_scaling, self.input_scaling) for _ in range(1)]
                for _ in range(self.n_reservoir)]

    def _make_W_res(self) -> List[List[float]]:
        """Reservoir 权重矩阵 — random sparse, scale to spectral_radius."""
        # 稀疏度 ~10%
        W = [[0.0] * self.n_reservoir for _ in range(self.n_reservoir)]
        n_connections = int(0.1 * self.n_reservoir * self.n_reservoir)
        for _ in range(n_connections):
            i = self.rng.randint(0, self.n_reservoir - 1)
            j = self.rng.randint(0, self.n_reservoir - 1)
            W[i][j] = self.rng.uniform(-1.0, 1.0)
        # 缩放至 target spectral radius
        cur_radius = self._spectral_radius(W)
        if cur_radius > 0:
            scale = self.target_spectral_radius / cur_radius
            for i in range(self.n_reservoir):
                for j in range(self.n_reservoir):
                    W[i][j] *= scale
        return W

    def _spectral_radius(self, W: List[List[float]]) -> float:
        """估算 spectral radius — power iteration (小型网络 OK)."""
        n = len(W)
        v = [self.rng.uniform(-1.0, 1.0) for _ in range(n)]
        # normalize
        norm = math.sqrt(sum(x * x for x in v)) or 1.0
        v = [x / norm for x in v]
        # power iteration (20 次)
        for _ in range(20):
            v_new = [0.0] * n
            for i in range(n):
                for j in range(n):
                    v_new[i] += W[i][j] * v[j]
            norm = math.sqrt(sum(x * x for x in v_new)) or 1.0
            v_new = [x / norm for x in v_new]
            v = v_new
        # eigenvalue approximation: Wv / v
        Wv = [sum(W[i][j] * v[j] for j in range(n)) for i in range(n)]
        eigenvalue = sum(Wv[i] * v[i] for i in range(n))
        return abs(eigenvalue)

    def step(self, input_value: float) -> ReservoirState:
        """Reservoir 一步更新 — echo state update rule."""
        # pre-activation: W_in @ u + W_res @ x
        pre = [0.0] * self.n_reservoir
        for i in range(self.n_reservoir):
            pre[i] = self.W_in[i][0] * input_value
            for j in range(self.n_reservoir):
                pre[i] += self.W_res[i][j] * self.state[j]
        # update: x(t+1) = (1 - alpha) * x(t) + alpha * tanh(pre)
        new_state = [0.0] * self.n_reservoir
        for i in range(self.n_reservoir):
            new_state[i] = (1 - self.leak_rate) * self.state[i] + self.leak_rate * math.tanh(pre[i])
        self.state = new_state
        # 记录
        n_active = sum(1 for x in self.state if abs(x) > 0.1)
        sr = self._spectral_radius(self.W_res)
        rs = ReservoirState(
            state_id=uuid.uuid4().hex[:12],
            vector=list(self.state),
            spectral_radius=sr,
            n_active=n_active,
        )
        self.history.append(rs)
        return rs

    def fit_readout(self, task_name: str, inputs: List[float], targets: List[float]) -> float:
        """训练线性 readout — 伪逆 / ridge regression 简化版.

        收集 reservoir states 对应 inputs, 然后 ridge 拟合 target.
        """
        if len(inputs) != len(targets):
            raise ValueError("inputs 与 targets 长度不一致")
        # 重置并跑一遍
        self.state = [0.0] * self.n_reservoir
        states = []
        for u in inputs:
            self.step(u)
            states.append(list(self.state))

        # 简单 ridge: 对每个输出维度, 拟合状态 -> target
        # 假设 targets 是 scalar list
        # 构造 X (n_samples x n_reservoir) and y
        X = states
        y = targets
        # Ridge: w = (X^T X + lambda I)^-1 X^T y (这里只标量, 一维)
        # 简化: 梯度下降
        w = [0.0] * self.n_reservoir
        lr = 0.001
        lam = 0.01
        for epoch in range(50):
            for s, t in zip(X, y):
                pred = sum(w[i] * s[i] for i in range(self.n_reservoir))
                err = t - pred
                for i in range(self.n_reservoir):
                    w[i] += lr * (err * s[i] - lam * w[i])
        self.readout_coefs[task_name] = w
        # 计算最终 MSE
        mse = 0.0
        for s, t in zip(X, y):
            pred = sum(w[i] * s[i] for i in range(self.n_reservoir))
            mse += (t - pred) ** 2
        return mse / len(y)

    def predict(self, task_name: str, input_value: float) -> float:
        """使用训练好的 readout 预测."""
        if task_name not in self.readout_coefs:
            return 0.0
        self.step(input_value)
        w = self.readout_coefs[task_name]
        return sum(w[i] * self.state[i] for i in range(self.n_reservoir))

    def stats(self) -> dict:
        sr = self._spectral_radius(self.W_res) if self.history else self.target_spectral_radius
        return {
            "version": RESERVOIR_VERSION,
            "n_reservoir": self.n_reservoir,
            "n_steps": len(self.history),
            "spectral_radius": sr,
            "target_spectral_radius": self.target_spectral_radius,
            "edge_of_chaos": abs(sr - 1.0) < 0.15,  # 接近 1.0 = edge of chaos
            "n_active_dimensions": self.history[-1].n_active if self.history else 0,
            "n_trained_readouts": len(self.readout_coefs),
            "jaeger_esn": (
                "Echo State Network: reservoir (random fixed) + linear readout. "
                "训练只在线性 readout, cheap."
            ),
            "maass_lsm_alignment": (
                "Liquid State Machine = spiking reservoir 神经拟真版本. "
                "Phase 43 用经典 ESN, LSM 可作 Phase 43.5 后续"
            ),
        }


__all__ = [
    "RESERVOIR_VERSION",
    "ReservoirState",
    "EchoStateNetwork",
]