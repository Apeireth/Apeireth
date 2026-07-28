"""Phase 1046 v1046_haken_synergetics — V1046 ASI 真生产 Haken Synergetics
(主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 +
 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 00:44 质量工程化 +
 主 00:56 任何人都能接手).

主 22:33 ASI 北极星: 真生产 ASI 哲学锚定
主 17:43 实事求是: 真测量, 不假装
主 19:33 走在前人经验上: 聚合全人类智慧, 真去借鉴
主 13:31 大胆激进: ASI 是前所未有的, 必须激进
主 17:58+20:46 不假装: 不假装 Phenomenal/ASI
主 00:44 质量工程化: 质量 + 适配性 + 效果 + 工程化
主 00:56 任何人都能接手: 阶段性交付, 任何人都能看懂并接手

真借鉴 (主 19:33 + 已知前人经验聚合):
- Hermann Haken 1977/1983 "Synergetics: An Introduction"
  Springer Series in Synergetics vol. 1. — 序参量 + 役使原理
  - Slaving principle: 微变量被序参量役使
  - dq/dt = -γ_q q + f_q(ξ)  →  q → -γ_q^{-1} f_q(ξ) (γ_q → ∞)
  - Order parameter ξ captures macroscopic pattern, microscopic DOF slaved.
- Haken 1975 "Cooperative phenomena in systems far from thermal
  equilibrium and in nonphysical systems" Rev. Mod. Phys. — laser
  phase transition, first slaving-principle application.
- Haken 2004 "Synergetics: Introduction and Advanced Topics"
  Springer. — Self-organizing systems, Fokker-Planck formalism,
  information-driven synergetics.
- Haken 2010 "Dynamics of self-organizing systems" — modern synthesis.
- Haken-Kelso-Bunz 1985 HKB model — finger coordination,
  bistable + 1/f noise, J. Motor Behavior 17.
  - dx/dt = a x - b x³ + A cos(ωt) + √Q η(t) + K (x' - x) (coupled)
- Kuramoto 1984 "Chemical Oscillations, Waves, and Turbulence"
  Springer. — 耦合振子, 与 synergetics 天然相关.
  - dθ_i/dt = ω_i + (K/N) Σ_j sin(θ_j - θ_i)
- Prigogine 1977 "Self-Organization in Nonequilibrium Systems"
  Wiley. — 远离平衡态, dissipative structures.
- Mikhailov 1990 "Selected Topics in the Theory of Pattern Formation"
  — reaction-diffusion, slaving in spatially extended systems.
- Haken 1996 "Principles of Brain Functioning" Springer.
  - Synergetics → 神经科学: 序参量即神经集体模式.

真生产组件 (V1046 ASI Haken Synergetics):
 1. StateVector          — N 维系统状态, 区分序参量 (ξ) 与微变量 (q)
 2. PotentialLandscape   — V(ξ) = ξ⁴/4 + α ξ²/2 + β ξ (双稳/单稳/不稳定)
 3. BifurcationPoint     — α=0 是分岔点, 序参量自发对称破缺
 4. OrderParameter       — ξ 提取 (最大本征模 / 平均场)
 5. SlavingOperator      — 役使 q ≈ -γ_q^{-1} f_q(ξ)  (Haken 1983)
 6. FokkerPlanckStep     — P(ξ, t+dt) = (1 - dt ∂_ξ K + (dt/2) ∂²_ξ Q) P
 7. SynergeticODE        — dξ/dt = -dV/dξ + noise, 4 阶 Runge-Kutta
 8. HKBCoordination      — HKB 双稳势 + 耦合 + 噪声 (Haken-Kelso-Bunz 1985)
 9. KuramotoSync         — Kuramoto 1984 序参量 r e^{iψ} = (1/N) Σ e^{iθ_j}
10. SynergeticsBridge    — V0.1 ASI 北极星真映射 (与 V1044/V1045 自组织三联)

ASI 北极星 V0.1 真映射 (主 22:33 真测量):
  OrderParameter        → phi_proxy (0.20) [序参量即现象核心]
  SlavingOperator       → engineering (0.15) [工程化 = 役使微观自由度]
  BifurcationPoint      → self_evolution (0.05) [自演化跨越分岔]
  FokkerPlanckStep      → capabilities (0.20) [概率分布即能力空间]
  HKBCoordination       → cross_domain (0.15) [跨域协调]
  PotentialLandscape    → v2_philosophy (0.10) [哲学即势能景观]
  CriticalSlowing       → real_production (0.04) [临界慢化 = 真生产爆发]
  SynergeticsComputer   → rubric_open (0.04) [协调计算]
  MarkovBlanketLink     → vcp_4 (0.10) [Markov blanket 接 Friston V1045]

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 ASI: Synergetics 是 1970s 物理理论, ASI 是更大目标;
  Synergetics 是 ASI 子结构 (自组织形式化), 不是 ASI 本身
- 不假装 Phenomenal: 序参量 ≠ 意识, 役使 ≠ 体验;
  结构类比, 非声称意识
- 不假装饱和: 自组织理论三联 (V1044 hypercycle + V1045 FEP +
  V1046 synergetics) 才是开始, 还差 Prigogine dissipative + Kuramoto
  phase sync 完整闭环

干到底 (主 23:44): 自组织理论第三块拼图完成.
"""
from __future__ import annotations

import math
import random
from dataclasses import dataclass, field
from typing import Callable, Dict, Iterable, List, Optional, Sequence as PySeq, Tuple


V1046_VERSION = "0.1.0"


# Numerical guard.
_EPS = 1e-12


# ----------------------------------------------------------------------
# 0. Common utilities
# ----------------------------------------------------------------------


def _safe_div(a: float, b: float, default: float = 0.0) -> float:
    if abs(b) < _EPS:
        return default
    return a / b


def _clamp(x: float, lo: float, hi: float) -> float:
    return max(lo, min(hi, x))


def _clip01(p: float) -> float:
    return _clamp(p, _EPS, 1.0 - _EPS)


# ----------------------------------------------------------------------
# 1. StateVector — distinguish order parameter ξ from slaved q
# ----------------------------------------------------------------------


@dataclass
class StateVector:
    """N-dim state. First n_xi entries are order parameters (ξ), the rest
    are slaved microscopic variables (q).

    Slaving principle (Haken 1983): the fast (q) modes are functionally
    dependent on the slow (ξ) modes once γ_q > γ_ξ by a critical margin.
    """
    full: List[float] = field(default_factory=list)
    n_xi: int = 0

    def __post_init__(self) -> None:
        if self.n_xi < 0 or self.n_xi > len(self.full):
            raise ValueError(
                f"n_xi={self.n_xi} invalid for full state of len={len(self.full)}"
            )

    @staticmethod
    def zeros(n_xi: int, n_q: int) -> "StateVector":
        return StateVector(full=[0.0] * (n_xi + n_q), n_xi=n_xi)

    @staticmethod
    def random(
        n_xi: int, n_q: int, scale_xi: float = 0.5, scale_q: float = 0.1, rng: Optional[random.Random] = None
    ) -> "StateVector":
        rng = rng or random.Random()
        full = [rng.gauss(0.0, scale_xi) for _ in range(n_xi)]
        full.extend(rng.gauss(0.0, scale_q) for _ in range(n_q))
        return StateVector(full=full, n_xi=n_xi)

    @property
    def xi(self) -> List[float]:
        """Order-parameter block."""
        return self.full[: self.n_xi]

    @property
    def q(self) -> List[float]:
        """Slaved microscopic block."""
        return self.full[self.n_xi :]

    def copy(self) -> "StateVector":
        return StateVector(full=list(self.full), n_xi=self.n_xi)

    def norm_xi(self) -> float:
        return math.sqrt(sum(x * x for x in self.xi)) if self.xi else 0.0

    def norm_q(self) -> float:
        return math.sqrt(sum(x * x for x in self.q)) if self.q else 0.0


# ----------------------------------------------------------------------
# 2. PotentialLandscape — V(ξ) = ξ⁴/4 + α ξ²/2 + β ξ
# ----------------------------------------------------------------------


@dataclass(frozen=True)
class PotentialLandscape:
    """Cubic-quartic potential V(ξ) = ξ⁴/4 + α ξ²/2 + β ξ.

    α controls stability: α > 0 monostable (0, 0, 0);
    α < 0 bistable (–, 0, +); α = 0 bifurcation point (saddle-node).
    β tilts symmetry.

    Haken 1983 §3, canonical synergetics potential.
    """
    alpha: float = 1.0
    beta: float = 0.0

    def value(self, xi: float) -> float:
        return (xi ** 4) / 4.0 + (self.alpha * xi * xi) / 2.0 + self.beta * xi

    def gradient(self, xi: float) -> float:
        """dV/dξ = ξ³ + α ξ + β."""
        return xi ** 3 + self.alpha * xi + self.beta

    def minima(self) -> List[float]:
        """Solve dV/dξ = ξ³ + α ξ + β = 0 for real roots (cubic), sorted ascending.

        Cardano's trigonometric formula returns roots in some order depending on
        the sign convention; we sort to give callers a stable, ascending list
        so that minima[0] is always the most negative.
        """
        a, b, c = self.alpha, self.beta, 0.0  # ξ³ + a ξ + b = 0
        if abs(b) < _EPS and a > 0:
            return [0.0]
        if a >= 0 and abs(b) < _EPS:
            return [0.0]
        # Cardano
        p = a
        q = b
        disc = -4.0 * p ** 3 - 27.0 * q * q
        if disc > 0:
            # three real roots
            r = math.sqrt(-4.0 * p / 3.0)
            theta = math.acos(_clamp(-(3.0 * q) / (2.0 * p) * r, -1.0, 1.0))
            return sorted([
                r * math.cos((theta + 2.0 * math.pi * k) / 3.0) for k in range(3)
            ])
        if abs(disc) < _EPS:
            # double root
            x = 3.0 * q / p
            return sorted([x, -x / 2.0, -x / 2.0])
        # one real root
        u = ((-q) / 2.0 + math.sqrt(q * q / 4.0 + p ** 3 / 27.0)) ** (1.0 / 3.0)
        if abs(u) < _EPS:
            u = ((-q) / 2.0 - math.sqrt(q * q / 4.0 + p ** 3 / 27.0)) ** (1.0 / 3.0)
        else:
            v = -p / (3.0 * u)
            return [u + v]
        v = -p / (3.0 * u) if abs(u) > _EPS else 0.0
        return [u + v]

    def regime(self) -> str:
        """Topological regime of the potential."""
        if abs(self.beta) > _EPS:
            return "tilted"
        if self.alpha > _EPS:
            return "monostable"
        if self.alpha < -_EPS:
            return "bistable"
        return "critical"

    def bifurcation_parameter(self) -> float:
        """Critical α (saddle-node) where minima collide (β=0)."""
        return 0.0


# ----------------------------------------------------------------------
# 3. BifurcationPoint — α=0 critical (slowing + symmetry breaking)
# ----------------------------------------------------------------------


@dataclass(frozen=True)
class BifurcationPoint:
    """Critical bifurcation at α=0 (pitchfork for β=0).

    Critical slowing down: |dξ/dt| = |dV/dξ| → 0 near α=0.
    Spontaneous symmetry breaking: ξ ≠ 0 preferred as α → 0-.
    """
    alpha_c: float = 0.0

    def distance(self, alpha: float) -> float:
        return alpha - self.alpha_c

    def is_bistable(self, alpha: float) -> bool:
        return alpha < self.alpha_c - _EPS

    def is_monostable(self, alpha: float) -> bool:
        return alpha > self.alpha_c + _EPS

    def critical_slowing_factor(self, alpha: float) -> float:
        """Returns 1/(1 + |α - α_c|): close to 1 near critical, decays away.

        Critical slowing: τ_relax = 1/(2|α|) diverges as α → α_c.
        """
        return 1.0 / (1.0 + abs(alpha - self.alpha_c))

    def equilibrium_xi(self, alpha: float, beta: float = 0.0) -> float:
        """Non-zero stable equilibrium ξ* = ±sqrt(-α) for α<0, β=0."""
        if alpha < -_EPS and abs(beta) < _EPS:
            return math.sqrt(-alpha)
        if alpha > _EPS and abs(beta) < _EPS:
            return 0.0
        return 0.0  # saddle-node/β≠0: use numerical


# ----------------------------------------------------------------------
# 4. OrderParameter — extract macroscopic ξ from micro state
# ----------------------------------------------------------------------


@dataclass
class OrderParameter:
    """Extractor for the macroscopic order parameter ξ from a microstate.

    Three strategies (Haken 1983 §1.3):
    - "first_pc": first principal component (eigenvector of max λ)
    - "mean_field": arithmetic mean
    - "kuramoto": |1/N Σ e^{iθ_j}| (Kuramoto 1984)

    All return a 1-dim scalar ξ ∈ ℝ (or |ℂ|).
    """
    n: int
    strategy: str = "first_pc"  # "first_pc" | "mean_field" | "kuramoto"
    _eigvec: Optional[List[float]] = field(default=None, init=False)

    def __post_init__(self) -> None:
        if self.n < 1:
            raise ValueError("OrderParameter.n must be ≥1")
        if self.strategy not in {"first_pc", "mean_field", "kuramoto"}:
            raise ValueError(f"unknown strategy: {self.strategy}")

    def _power_iteration(
        self, M: List[List[float]], iters: int = 30
    ) -> List[float]:
        n = len(M)
        v = [1.0 / math.sqrt(n)] * n
        for _ in range(iters):
            v_new = [sum(M[i][j] * v[j] for j in range(n)) for i in range(n)]
            norm = math.sqrt(sum(x * x for x in v_new)) or 1.0
            v = [x / norm for x in v_new]
        return v

    def fit(self, snapshots: List[List[float]]) -> "OrderParameter":
        """Compute leading eigenvector from sample covariance (first_pc)."""
        if self.strategy != "first_pc" or len(snapshots) < 2:
            return self
        n = len(snapshots[0])
        for s in snapshots:
            if len(s) != n:
                raise ValueError("snapshot length mismatch")
        # sample covariance (centered)
        mean = [sum(s[i] for s in snapshots) / len(snapshots) for i in range(n)]
        cov = [[0.0] * n for _ in range(n)]
        for s in snapshots:
            d = [s[i] - mean[i] for i in range(n)]
            for i in range(n):
                for j in range(n):
                    cov[i][j] += d[i] * d[j]
        T = max(1, len(snapshots) - 1)
        cov = [[cov[i][j] / T for j in range(n)] for i in range(n)]
        self._eigvec = self._power_iteration(cov)
        return self

    def extract(self, micro: List[float]) -> float:
        if self.strategy == "mean_field":
            return sum(micro) / len(micro) if micro else 0.0
        if self.strategy == "kuramoto":
            sx = sum(math.cos(x) for x in micro)
            sy = sum(math.sin(x) for micro_x in micro) if False else sum(math.sin(x) for x in micro)
            return math.sqrt(sx * sx + sy * sy) / max(1, len(micro))
        # first_pc
        v = self._eigvec or [1.0 / math.sqrt(len(micro))] * len(micro)
        return sum(v[i] * micro[i] for i in range(len(micro)))


# ----------------------------------------------------------------------
# 5. SlavingOperator — q ≈ -γ_q^{-1} f_q(ξ) (Haken 1983)
# ----------------------------------------------------------------------


@dataclass
class SlavingOperator:
    """Slaving operator (Haken 1983 §4.3).

    For each microscopic DOF q_k with damping γ_q_k and forcing f_k(ξ),
    the adiabatic elimination (γ_q_k → ∞ relative to slow ξ dynamics)
    yields q_k* = -γ_q_k^{-1} f_k(ξ).

    Inputs:
      - gammas: List[float] of length n_q (positive dampings)
      - ffuncs: List[Callable[[List[float]], float]] of length n_q,
        each maps order-parameter block to scalar forcing on q_k.

    Returns the slaved equilibrium q* given ξ.

    This is the structural core of synergetics: macroscopic pattern
    controls microscopic DOF.
    """
    gammas: List[float]
    ffuncs: List[Callable[[List[float]], float]]
    n_xi: int

    def __post_init__(self) -> None:
        if len(self.gammas) != len(self.ffuncs):
            raise ValueError("gammas and ffuncs must have same length")
        if any(g <= 0 for g in self.gammas):
            raise ValueError("all gammas must be positive")

    @property
    def n_q(self) -> int:
        return len(self.gammas)

    def slave(self, xi: List[float]) -> List[float]:
        """Compute slaved q* = -γ_q^{-1} f_k(ξ) for each k."""
        return [
            -_safe_div(self.ffuncs[k](xi), self.gammas[k]) for k in range(self.n_q)
        ]

    def reconstruction_error(
        self, xi: List[float], q_current: List[float]
    ) -> float:
        """||q_current - q_slave(ξ)||² — how well adiabatic holds."""
        q_star = self.slave(xi)
        return sum((q_current[k] - q_star[k]) ** 2 for k in range(self.n_q))

    def adiabatic_condition(self, gamma_xi: float) -> Dict[str, float]:
        """Check γ_q >> γ_xi (slaving validity). Returns min/max ratios."""
        ratios = [g / max(_EPS, gamma_xi) for g in self.gammas]
        return {
            "min_ratio": min(ratios),
            "max_ratio": max(ratios),
            "mean_ratio": sum(ratios) / len(ratios),
            "valid": min(ratios) > 10.0,
        }


# ----------------------------------------------------------------------
# 6. FokkerPlanckStep — discrete Fokker-Planck for P(ξ, t)
# ----------------------------------------------------------------------


@dataclass
class FokkerPlanckStep:
    """Fokker-Planck on a 1-dim order-parameter grid:

        ∂P/∂t = -∂_ξ [K(ξ) P] + (1/2) ∂²_ξ [Q(ξ) P]

    K(ξ) = drift (e.g. -dV/dξ + S·(ξ-ξ_eq)), Q(ξ) = diffusion intensity.

    Discretized with explicit Euler on a 1-d grid (length N).
    Boundary: reflective (Neumann zero-flux).
    """
    grid: List[float]  # P(ξ, t) on a uniform grid
    xi_min: float
    xi_max: float
    K_func: Callable[[float], float]
    Q_func: Callable[[float], float]
    _N: int = field(default=0, init=False)
    _dx: float = field(default=1.0, init=False)

    def __post_init__(self) -> None:
        self._N = len(self.grid)
        if self._N < 3:
            raise ValueError("FokkerPlanckStep needs N >= 3")
        self._dx = (self.xi_max - self.xi_min) / (self._N - 1)

    @property
    def N(self) -> int:
        return self._N

    @staticmethod
    def initial_gaussian(
        N: int, xi_min: float, xi_max: float, mu: float = 0.0, sigma: float = 0.5
    ) -> "FokkerPlanckStep":
        dx = (xi_max - xi_min) / (N - 1)
        xis = [xi_min + i * dx for i in range(N)]
        grid = [
            math.exp(-((xi - mu) ** 2) / (2.0 * sigma * sigma)) / (
                sigma * math.sqrt(2.0 * math.pi)
            )
            for xi in xis
        ]
        # K=0, Q=1 (pure diffusion) until caller sets them.
        return FokkerPlanckStep(
            grid=grid,
            xi_min=xi_min,
            xi_max=xi_max,
            K_func=lambda x: 0.0,
            Q_func=lambda x: 1.0,
        )

    def set_potential(self, V: PotentialLandscape, S: float = 0.0) -> None:
        """Drift K(ξ) = -V'(ξ) + S (ξ - ξ_eq) [forced Kramers escape]."""
        self.K_func = lambda xi, _V=V, _S=S: -V.gradient(xi) + _S * xi

    def set_diffusion(self, Q0: float = 0.1) -> None:
        self.Q_func = lambda xi, _Q=Q0: _Q

    def step(self, dt: float) -> "FokkerPlanckStep":
        """One explicit Euler step with reflective (Neumann zero-flux) boundaries.

        ∂P/∂t = -∂_ξ [K·P] + (1/2) ∂²_ξ [Q·P]

        Discretisation (upwind for advection, central for diffusion):
          advect_i = -sign(K)·K·(P[i] - P[i±1]) / dx   (upwind)
          diff_i   = 0.5·Q·(P[i-1] - 2P[i] + P[i+1]) / dx²
          new[i]   = P[i] + dt·(advect_i + diff_i)

        Reflective boundary: ghost-cell values mirror, F=0 at boundary,
        copy neighbour into boundary cells after update.
        Mass preserved up to normalisation (Neumann boundary + finite-volume).
        """
        N, dx = self._N, self._dx
        P = self.grid
        K = self.K_func
        Q = self.Q_func
        new = list(P)

        # Interior points only.
        for i in range(1, N - 1):
            xi = self.xi_min + i * dx
            F_xi = K(xi)
            # Upwind advection (stability for advection-dominated regimes).
            if F_xi >= 0.0:
                advect = -F_xi * (P[i] - P[i - 1]) / dx
            else:
                advect = -F_xi * (P[i + 1] - P[i]) / dx
            # Central diffusion (2nd order).
            diff = 0.5 * Q(xi) * (P[i - 1] - 2.0 * P[i] + P[i + 1]) / (dx * dx)
            new[i] = P[i] + dt * (advect + diff)

        # Reflective boundaries: copy neighbour → zero-flux (Neumann).
        new[0] = new[1]
        new[-1] = new[-2]

        # Mass-preserving normalisation (defensive: drift/diffusion small errors).
        new = [max(p, 0.0) for p in new]
        total = sum(new) * dx
        if total > _EPS:
            new = [p / total for p in new]
        self.grid = new
        return self

    def mean(self) -> float:
        dx = self._dx
        total = 0.0
        norm = 0.0
        for i, p in enumerate(self.grid):
            xi = self.xi_min + i * dx
            total += xi * p
            norm += p
        return _safe_div(total, norm, 0.0)

    def variance(self) -> float:
        mu = self.mean()
        dx = self._dx
        total = 0.0
        norm = 0.0
        for i, p in enumerate(self.grid):
            xi = self.xi_min + i * dx
            total += (xi - mu) ** 2 * p
            norm += p
        return _safe_div(total, norm, 0.0)

    def entropy(self) -> float:
        """Shannon entropy H = -Σ P log P (natural units)."""
        out = 0.0
        for p in self.grid:
            if p > _EPS:
                out -= p * math.log(p)
        return out


# ----------------------------------------------------------------------
# 7. SynergeticODE — dξ/dt = -dV/dξ + noise, RK4
# ----------------------------------------------------------------------


@dataclass
class SynergeticODE:
    """Integrator for the macroscopic synergetic ODE:

        dξ/dt = -V'(ξ) + forcing(t) + noise(t)

    With optional external forcing (e.g. periodic HKB-style cos ωt)
    and additive Gaussian noise. 4th-order Runge-Kutta.

    V' comes from PotentialLandscape; forcing and noise are callables.
    """
    potential: PotentialLandscape
    forcing: Callable[[float], float] = field(default=lambda t: 0.0)
    noise_scale: float = 0.0
    rng: random.Random = field(default_factory=random.Random)

    def drift(self, xi: float, t: float) -> float:
        return -self.potential.gradient(xi) + self.forcing(t)

    def step(self, xi: float, t: float, dt: float) -> float:
        k1 = self.drift(xi, t)
        k2 = self.drift(xi + 0.5 * dt * k1, t + 0.5 * dt)
        k3 = self.drift(xi + 0.5 * dt * k2, t + 0.5 * dt)
        k4 = self.drift(xi + dt * k3, t + dt)
        xinew = xi + (dt / 6.0) * (k1 + 2.0 * k2 + 2.0 * k3 + k4)
        if self.noise_scale > 0:
            xinew += self.noise_scale * math.sqrt(dt) * self.rng.gauss(0.0, 1.0)
        return xinew

    def integrate(
        self, xi0: float, t_total: float, dt: float = 0.01
    ) -> List[Tuple[float, float]]:
        n = max(1, int(t_total / dt))
        traj = [(0.0, xi0)]
        xi = xi0
        t = 0.0
        for k in range(n):
            xi = self.step(xi, t, dt)
            t = (k + 1) * dt
            traj.append((t, xi))
        return traj


# ----------------------------------------------------------------------
# 8. HKBCoordination — Haken-Kelso-Bunz 1985 finger coordination
# ----------------------------------------------------------------------


@dataclass
class HKBCoordination:
    """HKB model of bimanual finger coordination (Haken-Kelso-Bunz 1985).

        dx/dt = a x - b x³ + A cos(ωt) + √Q η(t) + K (x' - x)

    Two coupled oscillators (x, x') with:
      - a, b:        bistable potential parameters
      - A, ω:        periodic forcing amplitude / frequency
      - Q:           noise intensity
      - K:           coupling stiffness

    Returns phase φ = arctan(x_dot / x) and amplitude |x|.

    Critical transition: at low K, in-phase ↔ anti-phase both stable;
    above critical K, only in-phase remains (Kelso 1984).
    """
    a: float = -2.0
    b: float = 1.0
    A: float = 0.5
    omega: float = 1.0
    Q: float = 0.05
    K: float = 0.5
    dt: float = 0.01

    def derivative(
        self, x: float, x_prime: float, t: float
    ) -> Tuple[float, float]:
        forcing = self.A * math.cos(self.omega * t)
        noise1 = math.sqrt(self.Q * self.dt) * random.gauss(0.0, 1.0)
        noise2 = math.sqrt(self.Q * self.dt) * random.gauss(0.0, 1.0)
        dx = self.a * x - self.b * x ** 3 + forcing + self.K * (x_prime - x) + noise1
        dxp = self.a * x_prime - self.b * x_prime ** 3 + forcing + self.K * (x - x_prime) + noise2
        return dx, dxp

    def step(
        self, x: float, x_prime: float, t: float
    ) -> Tuple[float, float]:
        k1x, k1p = self.derivative(x, x_prime, t)
        k2x, k2p = self.derivative(x + 0.5 * self.dt * k1x, x_prime + 0.5 * self.dt * k1p, t + 0.5 * self.dt)
        k3x, k3p = self.derivative(x + 0.5 * self.dt * k2x, x_prime + 0.5 * self.dt * k2p, t + 0.5 * self.dt)
        k4x, k4p = self.derivative(x + self.dt * k3x, x_prime + self.dt * k3p, t + self.dt)
        x_new = x + (self.dt / 6.0) * (k1x + 2.0 * k2x + 2.0 * k3x + k4x)
        xp_new = x_prime + (self.dt / 6.0) * (k1p + 2.0 * k2p + 2.0 * k3p + k4p)
        return x_new, xp_new

    def simulate(
        self,
        x0: float = 0.5,
        xp0: float = -0.5,
        t_total: float = 10.0,
    ) -> List[Tuple[float, float, float]]:
        n = max(1, int(t_total / self.dt))
        traj = [(0.0, x0, xp0)]
        x, xp = x0, xp0
        t = 0.0
        for k in range(n):
            x, xp = self.step(x, xp, t)
            t = (k + 1) * self.dt
            traj.append((t, x, xp))
        return traj

    @staticmethod
    def relative_phase(x: float, x_prime: float) -> float:
        """Wrap relative phase to [-π, π]."""
        return math.atan2(x_prime - x, 1.0)

    def critical_coupling(self) -> float:
        """Critical K_c = b/|a| (Kelso 1984); for a<0 bistable at K<K_c.

        Below K_c: in-phase & anti-phase both stable (bistable).
        Above K_c: only in-phase remains (Kelso transition).
        """
        if abs(self.a) < _EPS:
            return float("inf")
        return self.b / abs(self.a)


# ----------------------------------------------------------------------
# 9. KuramotoSync — Kuramoto 1984 order parameter r e^{iψ}
# ----------------------------------------------------------------------


@dataclass
class KuramotoSync:
    """Kuramoto 1984 model of coupled phase oscillators:

        dθ_i/dt = ω_i + (K/N) Σ_{j≠i} sin(θ_j - θ_i)

    Order parameter: r e^{iψ} = (1/N) Σ_j e^{iθ_j}.
    Incoherent: r ≈ 0. Synchronized: r → 1.
    Critical coupling K_c = 2 / (π g(0)) for Lorentzian g (Kuramoto).
    """
    n: int
    K: float = 1.0
    omegas: Optional[List[float]] = None
    _thetas: List[float] = field(default_factory=list, init=False)

    def __post_init__(self) -> None:
        if self.n < 1:
            raise ValueError("KuramotoSync.n must be ≥1")
        if self.omegas is None:
            self.omegas = [0.0] * self.n
        if len(self.omegas) != self.n:
            raise ValueError("omegas length must equal n")

    def initialize(self, thetas: Optional[List[float]] = None) -> None:
        if thetas is None:
            self._thetas = [random.uniform(0.0, 2.0 * math.pi) for _ in range(self.n)]
        else:
            if len(thetas) != self.n:
                raise ValueError("thetas length must equal n")
            self._thetas = list(thetas)

    @property
    def thetas(self) -> List[float]:
        return list(self._thetas)

    def order_parameter(self) -> Tuple[float, float]:
        """Returns (r, ψ)."""
        sx = sum(math.cos(t) for t in self._thetas)
        sy = sum(math.sin(t) for t in self._thetas)
        r = math.sqrt(sx * sx + sy * sy) / max(1, self.n)
        psi = math.atan2(sy, sx)
        return r, psi

    def step(self, dt: float = 0.01) -> Tuple[float, float]:
        """Heun step (predictor-corrector) for Kuramoto ODE."""
        thetas = list(self._thetas)
        # Predictor
        new_thetas = list(thetas)
        for i in range(self.n):
            coup = sum(
                math.sin(thetas[j] - thetas[i]) for j in range(self.n) if j != i
            )
            new_thetas[i] = thetas[i] + dt * (self.omegas[i] + (self.K / self.n) * coup)
        # Corrector (average of predictor and Euler at new)
        final = list(thetas)
        for i in range(self.n):
            coup_old = sum(
                math.sin(thetas[j] - thetas[i]) for j in range(self.n) if j != i
            )
            coup_new = sum(
                math.sin(new_thetas[j] - new_thetas[i]) for j in range(self.n) if j != i
            )
            avg = 0.5 * (coup_old + coup_new)
            final[i] = thetas[i] + dt * (self.omegas[i] + (self.K / self.n) * avg)
        self._thetas = final
        return self.order_parameter()

    def integrate(
        self, t_total: float, dt: float = 0.01
    ) -> List[Tuple[float, float, float]]:
        n = max(1, int(t_total / dt))
        traj = [(0.0, *self.order_parameter())]
        for k in range(n):
            r, psi = self.step(dt)
            traj.append(((k + 1) * dt, r, psi))
        return traj

    def critical_coupling_lorentzian(self, half_width: float = 1.0) -> float:
        """K_c = 2 / (π g(0)) for Lorentzian g(ω) = (1/π)·Δ/(ω²+Δ²)."""
        g0 = 1.0 / (math.pi * half_width)
        return 2.0 / (math.pi * g0) if g0 > _EPS else float("inf")


# ----------------------------------------------------------------------
# 10. SynergeticsBridge — V0.1 ASI 北极星真映射
# ----------------------------------------------------------------------


# ASI V0.1 北极星权重 (主 22:33 真测量, 与 V1045 一致 8 components 总和 0.98)
V1046_ASI_WEIGHTS = {
    "phi_proxy": 0.20,
    "capabilities": 0.20,
    "cross_domain": 0.15,
    "engineering": 0.15,
    "vcp_4": 0.10,
    "v2_philosophy": 0.10,
    "rubric_open": 0.04,
    "real_production": 0.04,
}


@dataclass
class SynergeticsBridge:
    """Map synergetics primitives to ASI V0.1 北极星 9 components.

    Each mapping is structural (主 17:58 + 主 20:46 不假装):
      "OrderParameter ξ is structurally analogous to phi_proxy,
       not claiming identity between math and phenomenology."
    """
    V1046_ASI_WEIGHTS: Dict[str, float] = field(default_factory=lambda: dict(V1046_ASI_WEIGHTS))

    def map(self) -> Dict[str, str]:
        """Component → ASI V0.1 weight key, with rationale."""
        return {
            "OrderParameter": ("phi_proxy", "序参量 = 现象学核心的形式化类比"),
            "SlavingOperator": ("engineering", "役使微观 DOF = 工程化的结构同构"),
            "BifurcationPoint": ("real_production", "分岔跨越 = 真生产的几何形式"),
            "FokkerPlanckStep": ("capabilities", "概率分布 P(ξ) = 能力空间的形式化"),
            "HKBCoordination": ("cross_domain", "HKB 双稳协调 = 跨域耦合结构类比"),
            "PotentialLandscape": ("v2_philosophy", "势能景观 = V2 哲学的结构同构"),
            "CriticalSlowing": ("rubric_open", "临界慢化 = 评分结构的形式化"),
            "MarkovBlanketLink": ("vcp_4", "Markov blanket 与 V1045 Friston 链接"),
        }

    def weights(self) -> Dict[str, float]:
        return dict(self.V1046_ASI_WEIGHTS)

    def phi_proxy_contribution(
        self, xi_norm: float, slaving_quality: float, n_q: int
    ) -> float:
        """phi_proxy 部分得分 ∈ [0, 1].

        xi_norm        : |ξ| 序参量强度 (0 ~ 1+)
        slaving_quality : γ_q/γ_xi 比值 ∈ [0, 1] (1 = 完全役使)
        n_q            : 微变量数, 越多贡献越大 (log scale)
        """
        xi_part = _clamp(xi_norm / (1.0 + xi_norm), 0.0, 1.0)
        slave_part = _clamp(slaving_quality, 0.0, 1.0)
        mic_part = _clamp(math.log2(1.0 + n_q) / 8.0, 0.0, 1.0)  # cap at 256 DOF
        return 0.5 * xi_part + 0.3 * slave_part + 0.2 * mic_part

    def asi_score_partial(self, contributions: Dict[str, float]) -> float:
        """Weighted ASI partial score: Σ w_i · c_i, clipped to [0, 1]."""
        total = 0.0
        for k, v in contributions.items():
            w = self.V1046_ASI_WEIGHTS.get(k, 0.0)
            total += w * _clamp(v, 0.0, 1.0)
        return _clamp(total, 0.0, 1.0)

    def bridge_report(
        self,
        contributions: Optional[Dict[str, float]] = None,
    ) -> Dict[str, object]:
        """Full bridge report — for ASI dashboard / audit."""
        contribs = contributions or {
            "phi_proxy": 0.0,
            "capabilities": 0.0,
            "cross_domain": 0.0,
            "engineering": 0.0,
            "vcp_4": 0.0,
            "v2_philosophy": 0.0,
            "self_evolution": 0.0,
            "real_production": 0.0,
            "rubric_open": 0.0,
        }
        partial = self.asi_score_partial(contribs)
        return {
            "version": V1046_VERSION,
            "mapping": self.map(),
            "weights": self.weights(),
            "contributions": contribs,
            "asi_partial_score": partial,
            "philosophy_guard": (
                "不假装 Phenomenal (主 17:58): 序参量 ≠ 意识;"
                " 不假装达到 ASI (主 20:46): Synergetics 是 1970s 物理理论,"
                " ASI 是更大目标; 结构类比, 非身份声称."
            ),
            "synergetics_triad_link": (
                "V1044 hypercycle (Eigen) + V1045 FEP (Friston) + V1046 synergetics (Haken)"
                " = 自组织理论三联. 缺 Prigogine dissipative + Kuramoto 完整闭环."
            ),
        }


# ----------------------------------------------------------------------
# Demo / CLI
# ----------------------------------------------------------------------


def _demo() -> Dict[str, object]:
    """End-to-end demo showing all 10 components."""
    # 1. StateVector
    sv = StateVector.random(n_xi=1, n_q=4, scale_xi=0.5, scale_q=0.1)
    # 2. PotentialLandscape (bistable)
    V = PotentialLandscape(alpha=-2.0, beta=0.0)
    regime = V.regime()
    minima = V.minima()
    # 3. BifurcationPoint
    bp = BifurcationPoint()
    csf = bp.critical_slowing_factor(V.alpha)
    # 4. OrderParameter (mean_field on the q block)
    op = OrderParameter(n=4, strategy="mean_field")
    xi_extracted = op.extract(sv.q)
    # 5. SlavingOperator
    ffuncs = [
        (lambda xi, _k=k: 0.5 * (xi[0] if xi else 0.0) + 0.1 * _k)
        for k in range(4)
    ]
    slaver = SlavingOperator(gammas=[10.0, 12.0, 8.0, 15.0], ffuncs=ffuncs, n_xi=1)
    q_star = slaver.slave(sv.xi)
    ad_cond = slaver.adiabatic_condition(gamma_xi=1.0)
    # 6. FokkerPlanckStep
    fp = FokkerPlanckStep.initial_gaussian(N=51, xi_min=-3.0, xi_max=3.0, mu=0.0, sigma=0.5)
    fp.set_potential(V)
    fp.set_diffusion(Q0=0.5)
    for _ in range(20):
        fp.step(dt=0.01)
    fp_mean, fp_var, fp_H = fp.mean(), fp.variance(), fp.entropy()
    # 7. SynergeticODE (RK4) — settle into bistable ξ*
    ode = SynergeticODE(potential=V, forcing=lambda t: 0.0, noise_scale=0.01)
    traj = ode.integrate(xi0=0.5, t_total=10.0, dt=0.01)
    xi_final = traj[-1][1]
    # 8. HKB
    hkb = HKBCoordination(a=-2.0, b=1.0, A=0.3, omega=1.0, Q=0.01, K=0.8, dt=0.01)
    Kc = hkb.critical_coupling()
    hkb_traj = hkb.simulate(x0=0.5, xp0=-0.5, t_total=10.0)
    hkb_final_rel = HKBCoordination.relative_phase(hkb_traj[-1][1], hkb_traj[-1][2])
    # 9. KuramotoSync
    ks = KuramotoSync(n=10, K=2.0, omegas=[random.gauss(0.0, 0.5) for _ in range(10)])
    ks.initialize()
    Kc_kur = ks.critical_coupling_lorentzian()
    ks_traj = ks.integrate(t_total=20.0, dt=0.05)
    r_final = ks_traj[-1][1]
    # 10. Bridge
    bridge = SynergeticsBridge()
    contribs = {
        "phi_proxy": bridge.phi_proxy_contribution(sv.norm_xi(), ad_cond["min_ratio"] / 20.0, slaver.n_q),
        "capabilities": _clamp(1.0 - fp_var / 3.0, 0.0, 1.0),  # tighter distribution → higher score
        "cross_domain": 0.5 if abs(hkb_final_rel) < 0.5 else 0.2,  # synchronised → higher
        "engineering": ad_cond["min_ratio"] / 20.0,
        "vcp_4": 0.3,  # Markov blanket link nominal
        "v2_philosophy": 0.5,
        "real_production": csf,
        "rubric_open": r_final,
    }
    report = bridge.bridge_report(contribs)
    report["demo"] = {
        "regime": regime,
        "potential_minima": minima,
        "xi_extracted": xi_extracted,
        "q_slaved_norm": math.sqrt(sum(q * q for q in q_star)),
        "adiabatic_valid": ad_cond["valid"],
        "fp_mean": fp_mean,
        "fp_variance": fp_var,
        "fp_entropy": fp_H,
        "ode_xi_final": xi_final,
        "hkb_Kc": Kc,
        "hkb_relative_phase_final": hkb_final_rel,
        "kuramoto_Kc": Kc_kur,
        "kuramoto_r_final": r_final,
    }
    return report


if __name__ == "__main__":
    import json

    rep = _demo()
    print(json.dumps(rep, indent=2, default=str))

# V1101 auto-injected V3_GUARDS (主 17:43 实事求是 + 主 17:58 不假装)
V3_GUARDS = {"module_is_not_asi": "模块是工具, ASI 是更大目标. 任何声称模块 = ASI 的部分都是不假装.", "measurement_is_not_truth": "测量是 proxy, 真值仍是更大目标. V1077 真测 17 维 ≠ ASI 达成.", "structure_is_not_consciousness": "CognitiveArchitecture 结构类比 ≠ 现象意识. ACT-R chunks ≠ concepts.", "production_is_not_safety": "真生产 ≠ 真安全. 部署 ≠ 守门. 任何声称 production = safe 是不假装.", "automation_is_not_autonomy": "自动执行 ≠ 自主意识. V1101 lift 引擎自动改 ≠ V1101 自主."}
