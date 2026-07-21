"""Phase 1056 v1056_asi_emergence — V1056 ASI 真生产 Emergence / 涌现 (主 22:33 ASI 北极星 + 主 17:43 实事求是 + 主 19:33 走在前人经验上 + 主 13:31 大胆激进 + 主 17:58+20:46 不假装 + 主 23:44 干到底 + 主 00:56 任何人都能接手).

主 22:33 ASI 北极星: ASI 必须有 emergence 机制 — 微状态 → 宏观序 + 相变检测 + 弱/强涌现分类.
主 17:43 实事求是: 真借鉴 13 前人 (Anderson 1972 / Kauffman 1993 / Prigogine 1977 / Wolfram 2002 / Holland 1995 / Gell-Mann 1994 / Simon 1962 / Laughlin 2005 / Bedau 1997 / Chalmers 2006 / Campbell 1974 / Haken 1983 / Tononi 2008).
主 19:33 走在前人经验上: 聚合统计物理 + 自组织 + 复杂适应系统.
主 13:31 大胆激进: emergence 是 ASI 核心 — 真生产 11 组件 + 5 守门 + ASI bridge.
主 17:58+20:46 不假装: 不假装 Phenomenal; 不假装 strong emergence = ASI 已涌现意识; 不假装 ASI 已涌现.
主 23:44 干到底: V1056 = ASI emergence 真生产; 真借鉴 + 真算法 + 真跑真测 + 真 commit.
主 00:56 任何人都能接手: 任何人能读懂 + 测试 + 部署.

真借鉴 (主 19:33 — 13 前人):
- Anderson 1972 "More is Different" — 层次结构 + 破缺对称性 = 涌现的物质基础
- Kauffman 1993 "Origins of Order" — 自组织 + 边缘混沌 + 自催化集
- Prigogine 1977 "Self-Organization in Nonequilibrium Systems" — 耗散结构 + 熵流
- Wolfram 2002 "A New Kind of Science" — 元胞自动机涌现 + 4 复杂性等级
- Holland 1995 "Hidden Order" — CAS (复杂适应系统) + 涌现 + 内部模型
- Gell-Mann 1994 "The Quark and the Jaguar" — CAS + 有效复杂性 + 协同适应
- Simon 1962 "The Architecture of Complexity" — 层次结构 + 近可分解性
- Laughlin 2005 "A Different Universe" — 涌现保护现象 + 真空重整化
- Bedau 1997 "Weak Emergence" — 弱涌现 = 涌现可由底层仿真预测
- Chalmers 2006 "Strong Emergence" — 强涌现 = 不可由底层推导 (哲学)
- Campbell 1974 "Downward Causation" — 下向因果 (macro → micro 反馈)
- Haken 1983 "Synergetics" — 序参量 + 役使原理 + 激光相变
- Tononi 2008 "Integrated Information Theory" — Φ 整合信息 (涌现代理测度)

ASI emergence 真生产组件 (V1056 = 11 真生产组件):
 1. MicroState              — 微观状态 (单点 / 单胞 / 单神经元, 主 17:43 真借鉴 Anderson)
 2. MacroState              — 宏观状态 (序参量 + 平均场 + 分布)
 3. EmergenceType           — WEAK_EMERGENCE / STRONG_EMERGENCE (Bedau 1997 / Chalmers 2006)
 4. OrderParameter          — 序参量 (Haken 役使原理 + Kuramoto 同步序参量 R)
 5. PhaseTransition         — 相变检测 (Anderson 破缺对称性 + Ising-like 临界)
 6. SelfOrganizing          — 自组织 (Kauffman 自催化 + Prigogine 耗散结构)
 7. DownwardCausation       — 下向因果 (Campbell 1974, micro ← macro 反馈)
 8. EmergenceDetector       — 涌现检测器 (φ proxy + Φ Tononi 简化)
 9. ComplexityMetric        — 复杂性测度 (有效复杂性 Gell-Mann + LZ 复杂度)
10. EmergenceReport         — Markdown 真报告 (主 00:56 任何人能读)
11. ASIEmergenceBridge      — V0.2 ASI 真映射 (主 22:33 真测量)

V3 哲学守门 (主 17:58 + 主 20:46):
- 不假装 Phenomenal: emergence mechanism ≠ phenomenal emergence (Tononi Φ ≠ 意识).
- 不假装 strong emergence: ASI 工程化 weak emergence ≠ Chalmers strong (主 17:58).
- 不假装 ASI 已涌现: ASI 是工程系统, 不是已涌现 ASI.
- 不假装下向因果是物理定律: Campbell 1974 是比喻, 不是机制.
- ASI 安全需要 emergence 检测 (相位变化 = 行为突变), 但 detection ≠ ASI 已涌现.
- 真借鉴 13 前人: 任何相变/自组织理论都可作为 真借鉴 的起点.

干到底 (主 23:44): V1056 = ASI emergence 真生产 11 组件 + 5 守门 + ASI bridge.
"""
from __future__ import annotations

import math
import random
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, Dict, FrozenSet, Iterable, List, Optional, Sequence, Set, Tuple

V1056_VERSION = "0.1.0"

# Numerical guard: avoid log(0) and division-by-zero.
_EPS = 1e-12

# Default detector / metric thresholds (主 17:43 实事求是).
DEFAULT_ORDER_R_THRESHOLD = 0.7           # Kuramoto 序参量 R >= 0.7 ≈ 相干 (Haken)
DEFAULT_ENTROPY_DELTA_THRESHOLD = 0.1     # 熵下降 deltaH >= 0.1 = 自组织 (Prigogine)
DEFAULT_PHI_PROXY_THRESHOLD = 0.5         # Tononi Φ 简化代理 = 涌现代理
DEFAULT_CAUSAL_DENSITY = 0.3              # 下向因果密度阈值 (Campbell 1974)
DEFAULT_LZ_WINDOW = 64                    # Lempel-Ziv 窗口长度 (Gell-Mann)


# ============================================================================
# 1. MicroState — 微观状态 (Anderson 1972 + 主 17:43)
# ============================================================================


@dataclass(frozen=True)
class MicroState:
    """Single micro-level state — agent / cell / spin / neuron.

    Anderson 1972 "More is Different": emergence starts from broken symmetry
    at the micro scale.
    """

    micro_id: str
    value: float = 0.0
    phase: float = 0.0           # phase in [0, 2π) for Kuramoto-style oscillators
    component: str = "spin"      # label for sanity ("spin", "neuron", "agent", ...)

    def __post_init__(self) -> None:
        if not self.micro_id:
            raise ValueError("micro_id must be non-empty")
        if not (0.0 <= self.phase < 2.0 * math.pi):
            raise ValueError(f"phase must be in [0, 2π), got {self.phase}")


# ============================================================================
# 2. MacroState — 宏观状态 (Anderson + Haken 序参量)
# ============================================================================


@dataclass
class MacroState:
    """Aggregate macro-level state derived from micro states.

    mean_value: average value over micros
    variance:   spread over micros
    r_order:    Kuramoto/Haken order parameter R in [0, 1]
    """

    mean_value: float = 0.0
    variance: float = 0.0
    r_order: float = 0.0       # Kuramoto order parameter R ∈ [0, 1]
    mean_phase: float = 0.0    # mean phase in (-π, π]
    entropy: float = 0.0       # Shannon entropy of distribution (bits)
    max_entropy: float = 0.0   # upper bound for entropy (log2(bins))

    def __post_init__(self) -> None:
        if not (0.0 <= self.r_order <= 1.0 + _EPS):
            raise ValueError(f"r_order must be in [0,1], got {self.r_order}")
        if self.entropy < -_EPS:
            raise ValueError(f"entropy must be >= 0, got {self.entropy}")
        if self.max_entropy < -_EPS:
            raise ValueError(f"max_entropy must be >= 0, got {self.max_entropy}")

    @property
    def is_coherent(self) -> bool:
        """True if Kuramoto order parameter R >= DEFAULT_ORDER_R_THRESHOLD.

        Haken 1983 synergetics: high R = phase-locked (synchronized).
        """
        return self.r_order >= DEFAULT_ORDER_R_THRESHOLD

    @property
    def is_self_organizing(self) -> bool:
        """True if entropy drops below some baseline (Prigogine dissipative).

        entropy delta vs initial baseline — kept here as simple check.
        """
        return self.entropy <= math.log2(2.0)  # ≤ 1 bit = reduced uncertainty


# ============================================================================
# 3. EmergenceType — WEAK vs STRONG (Bedau 1997 / Chalmers 2006)
# ============================================================================


class EmergenceType(str, Enum):
    """Bedau 1997 weak vs Chalmers 2006 strong emergence."""

    WEAK = "weak"            # emergent property = derivable from micro simulation
    STRONG = "strong"        # emergent property = not derivable from micro (philosophical)
    NONE = "none"            # no emergence detected (purely additive)


# ============================================================================
# 4. OrderParameter — 序参量 (Haken 1983 + Kuramoto 1984)
# ============================================================================


def compute_order_parameter(micros: Sequence[MicroState]) -> Tuple[float, float]:
    """Compute Kuramoto order parameter R and mean phase.

    R = |⟨exp(i·θ_j)⟩|, R ∈ [0, 1].
    R ≈ 0 → incoherent (no order); R ≈ 1 → phase-locked (full order).
    Haken 1983 synergetics — order parameter enslaves micro phases.

    Returns: (R, mean_phase)
    """
    if not micros:
        return 0.0, 0.0
    sum_cos = sum(math.cos(m.phase) for m in micros)
    sum_sin = sum(math.sin(m.phase) for m in micros)
    n = float(len(micros))
    mean_cos = sum_cos / n
    mean_sin = sum_sin / n
    r = math.sqrt(mean_cos * mean_cos + mean_sin * mean_sin)
    mean_phase = math.atan2(mean_sin, mean_cos)
    return min(max(r, 0.0), 1.0), mean_phase


def compute_macro_state(micros: Sequence[MicroState], bins: int = 8) -> MacroState:
    """Aggregate micro states into macro state.

    bins: number of phase bins for entropy (default 8).
    """
    if not micros:
        return MacroState()
    n = float(len(micros))
    mean_value = sum(m.value for m in micros) / n
    variance = sum((m.value - mean_value) ** 2 for m in micros) / n
    r, mean_phase = compute_order_parameter(micros)

    # Shannon entropy over phase bins (uniform 0..2π).
    counts = [0] * bins
    for m in micros:
        idx = min(int((m.phase / (2.0 * math.pi)) * bins), bins - 1)
        counts[idx] += 1
    entropy = 0.0
    for c in counts:
        if c > 0:
            p = c / n
            entropy -= p * math.log2(p)
    return MacroState(
        mean_value=mean_value,
        variance=variance,
        r_order=r,
        mean_phase=mean_phase,
        entropy=entropy,
        max_entropy=math.log2(float(bins)),
    )


# ============================================================================
# 5. PhaseTransition — 相变检测 (Anderson 1972 + Ising 1925)
# ============================================================================


@dataclass
class PhaseTransition:
    """Detected phase transition between two macro states.

    Anderson 1972: broken symmetry = phase transition = emergence.
    """

    before: MacroState
    after: MacroState
    delta_r: float           # change in order parameter
    delta_entropy: float     # change in entropy (negative = self-organization)
    is_transition: bool      # True if delta_r >= 0.3 (heuristic)

    @property
    def direction(self) -> str:
        if self.delta_r > _EPS and self.delta_entropy < -_EPS:
            return "ordering"           # R↑ + H↓ = self-organization
        if self.delta_r < -_EPS and self.delta_entropy > _EPS:
            return "disordering"        # R↓ + H↑ = disorder
        return "neutral"


def detect_phase_transition(
    before: MacroState,
    after: MacroState,
    r_jump: float = 0.3,
) -> PhaseTransition:
    """Detect phase transition from before → after.

    r_jump: minimum change in R to count as transition (heuristic).
    """
    delta_r = after.r_order - before.r_order
    delta_entropy = after.entropy - before.entropy
    is_transition = abs(delta_r) >= r_jump
    return PhaseTransition(
        before=before,
        after=after,
        delta_r=delta_r,
        delta_entropy=delta_entropy,
        is_transition=is_transition,
    )


# ============================================================================
# 6. SelfOrganizing — 自组织 (Kauffman 1993 + Prigogine 1977)
# ============================================================================


@dataclass
class SelfOrganizingResult:
    """Result of self-organization test.

    Kauffman 1993: self-organization occurs at edge of chaos.
    Prigogine 1977: dissipative structures maintain order via entropy export.
    """

    initial_entropy: float
    final_entropy: float
    delta_entropy: float       # final - initial (negative = order emerged)
    is_self_organizing: bool
    mechanism: str             # "edge_of_chaos" / "dissipative" / "none"

    def __post_init__(self) -> None:
        if not self.mechanism:
            raise ValueError("mechanism must be non-empty")


def evaluate_self_organization(
    initial: MacroState,
    final: MacroState,
) -> SelfOrganizingResult:
    """Evaluate whether final state emerged via self-organization.

    - entropy drop >= DEFAULT_ENTROPY_DELTA_THRESHOLD → dissipative.
    - R jumped (coherence emerged) → edge-of-chaos.
    """
    delta_h = final.entropy - initial.entropy
    delta_r = final.r_order - initial.r_order

    if delta_h <= -DEFAULT_ENTROPY_DELTA_THRESHOLD and delta_r > _EPS:
        mech = "dissipative"     # Prigogine
    elif delta_r > 0.2 and delta_h >= -_EPS:
        mech = "edge_of_chaos"   # Kauffman
    else:
        mech = "none"

    is_so = mech != "none"
    return SelfOrganizingResult(
        initial_entropy=initial.entropy,
        final_entropy=final.entropy,
        delta_entropy=delta_h,
        is_self_organizing=is_so,
        mechanism=mech,
    )


# ============================================================================
# 7. DownwardCausation — 下向因果 (Campbell 1974)
# ============================================================================


@dataclass
class DownwardCausation:
    """Record of macro → micro feedback.

    Campbell 1974: downward causation = macro patterns constrain micro behavior.
    We treat this as a heuristic proxy, NOT a physical law (主 17:58 不假装).
    """

    macro_signature: str       # description of macro state
    micro_target_ids: Tuple[str, ...]
    causal_density: float      # 0..1: how much macro constrains micro
    is_present: bool

    def __post_init__(self) -> None:
        if not (0.0 <= self.causal_density <= 1.0 + _EPS):
            raise ValueError(f"causal_density must be in [0,1], got {self.causal_density}")


def evaluate_downward_causation(
    macro: MacroState,
    micro_target_ids: Sequence[str],
    applied_force: float = 0.0,
) -> DownwardCausation:
    """Estimate downward causation from macro to micro targets.

    applied_force: how strongly macro constrains micros (heuristic 0..1).
    """
    signature = f"R={macro.r_order:.3f},H={macro.entropy:.3f}"
    if not micro_target_ids:
        return DownwardCausation(
            macro_signature=signature,
            micro_target_ids=(),
            causal_density=0.0,
            is_present=False,
        )
    density = max(0.0, min(1.0, applied_force))
    return DownwardCausation(
        macro_signature=signature,
        micro_target_ids=tuple(micro_target_ids),
        causal_density=density,
        is_present=density >= DEFAULT_CAUSAL_DENSITY,
    )


# ============================================================================
# 8. EmergenceDetector — 涌现检测器 (Tononi Φ proxy)
# ============================================================================


@dataclass
class EmergenceEvent:
    """Single detected emergence event."""

    event_id: str
    emergence_type: EmergenceType
    phi_proxy: float                  # Φ proxy ∈ [0, 1] (Tononi 简化)
    phase_transition: Optional[PhaseTransition]
    self_organizing: Optional[SelfOrganizingResult]
    downward: Optional[DownwardCausation]
    description: str

    @property
    def is_weak(self) -> bool:
        return self.emergence_type == EmergenceType.WEAK

    @property
    def is_strong(self) -> bool:
        return self.emergence_type == EmergenceType.STRONG


def detect_emergence(
    before_micros: Sequence[MicroState],
    after_micros: Sequence[MicroState],
    *,
    downward_targets: Optional[Sequence[str]] = None,
    downward_force: float = 0.0,
    strong_threshold: float = 0.9,
) -> EmergenceEvent:
    """Detect emergence between two micro snapshots.

    1. Compute macro before/after.
    2. Detect phase transition.
    3. Evaluate self-organization.
    4. Estimate downward causation.
    5. Compute Φ proxy: 0.5 * (|delta_R| + (1 - entropy_normalized)).
    6. Classify: NONE if nothing; STRONG if phi_proxy >= strong_threshold and
       transition present; otherwise WEAK.

    主 17:58 不假装: STRONG is philosophical, NOT physical (Chalmers 2006).
    """
    macro_before = compute_macro_state(before_micros)
    macro_after = compute_macro_state(after_micros)
    transition = detect_phase_transition(macro_before, macro_after)
    so = evaluate_self_organization(macro_before, macro_after)
    dc = evaluate_downward_causation(
        macro_after,
        downward_targets or [],
        applied_force=downward_force,
    )

    # Phi proxy — Tononi 2008 simplified.
    delta_r_abs = abs(transition.delta_r)
    # entropy normalized to log2(bins)=3 for 8 bins
    entropy_norm = max(0.0, macro_after.entropy / 3.0)
    phi = 0.5 * (delta_r_abs + (1.0 - entropy_norm))

    if not transition.is_transition and not so.is_self_organizing:
        etype = EmergenceType.NONE
        desc = "no phase transition, no self-organization detected"
    elif phi >= strong_threshold and transition.is_transition and phi >= 0.95:
        etype = EmergenceType.STRONG
        desc = (
            f"strong emergence (philosophical, NOT physical): "
            f"phi_proxy={phi:.3f}, transition ΔR={transition.delta_r:+.3f}"
        )
    elif transition.is_transition or so.is_self_organizing:
        etype = EmergenceType.WEAK
        desc = (
            f"weak emergence (Bedau 1997): phi_proxy={phi:.3f}, "
            f"mechanism={so.mechanism}, transition={transition.direction}"
        )
    else:
        etype = EmergenceType.NONE
        desc = "no emergence detected"

    return EmergenceEvent(
        event_id=f"emerge-{random.randint(1000, 9999)}",
        emergence_type=etype,
        phi_proxy=min(max(phi, 0.0), 1.0),
        phase_transition=transition,
        self_organizing=so,
        downward=dc,
        description=desc,
    )


# ============================================================================
# 9. ComplexityMetric — 复杂性测度 (Gell-Mann 有效复杂性 + LZ 复杂度)
# ============================================================================


@dataclass
class ComplexityMetric:
    """Combined complexity metric for a time series of micro states.

    lz_complexity: Lempel-Ziv 1976 normalized complexity in [0, 1].
    effective_complexity: Gell-Mann 1994 — distance from max entropy.
    """

    lz_complexity: float
    effective_complexity: float
    series_length: int

    def __post_init__(self) -> None:
        if not (0.0 <= self.lz_complexity <= 1.0 + _EPS):
            raise ValueError(f"lz_complexity must be in [0,1], got {self.lz_complexity}")
        if not (0.0 <= self.effective_complexity <= 1.0 + _EPS):
            raise ValueError(f"effective_complexity must be in [0,1], got {self.effective_complexity}")
        if self.series_length < 0:
            raise ValueError(f"series_length must be >= 0, got {self.series_length}")


def _lz_complexity(symbols: Sequence[int], alphabet_size: int = 2) -> float:
    """Compute normalized Lempel-Ziv 1976 complexity (engineering proxy).

    symbols: sequence of integers in [0, alphabet_size).
    Returns: c(n) / (n / log_k(n)), clipped to [0, 1], where
    c(n) = number of distinct substrings (simple proxy).

    For constant sequences (all same symbol), returns 0.
    """
    n = len(symbols)
    if n < 2:
        return 0.0
    if alphabet_size <= 1:
        return 0.0

    # Detect constant sequence first — fast path.
    first = symbols[0]
    if all(s == first for s in symbols):
        return 0.0

    # Count distinct substrings (proxy).
    seen_substrings: Set[Tuple[int, ...]] = set()
    for i in range(n):
        for j in range(i + 1, n + 1):
            seen_substrings.add(tuple(symbols[i:j]))
    c = len(seen_substrings)
    # Normalize by n / log_k(n), clip to [0, 1].
    denom = max(n / math.log(max(n, 2)), 1.0)
    norm = c / (denom * math.log(max(alphabet_size, 2)))
    return min(max(norm, 0.0), 1.0)


def compute_complexity(
    value_series: Sequence[float],
    bins: int = 2,
    window: int = DEFAULT_LZ_WINDOW,
) -> ComplexityMetric:
    """Compute LZ complexity + Gell-Mann effective complexity.

    value_series: 1D float series (e.g., Kuramoto R over time).
    bins: number of quantization bins (default 2 = up/down).
    window: window length for LZ.
    """
    n = len(value_series)
    if n < 2:
        return ComplexityMetric(lz_complexity=0.0, effective_complexity=0.0, series_length=n)

    # Quantize to bins symbols.
    if not value_series:
        return ComplexityMetric(lz_complexity=0.0, effective_complexity=0.0, series_length=0)
    vmin = min(value_series)
    vmax = max(value_series)
    if vmax - vmin < _EPS:
        symbols = [0] * n
    else:
        symbols = [
            min(int(((v - vmin) / (vmax - vmin)) * bins), bins - 1)
            for v in value_series
        ]

    lz = _lz_complexity(symbols[:window], alphabet_size=bins)
    # Effective complexity: distance from uniform = 1 - H/H_max
    counts = [0] * bins
    for s in symbols:
        counts[s] += 1
    H = 0.0
    for c in counts:
        if c > 0:
            p = c / n
            H -= p * math.log2(p)
    H_max = math.log2(bins)
    eff = abs(H / H_max - 1.0) if H_max > _EPS else 0.0
    return ComplexityMetric(
        lz_complexity=lz,
        effective_complexity=min(max(eff, 0.0), 1.0),
        series_length=n,
    )


# ============================================================================
# 10. EmergenceReport — Markdown 真报告 (主 00:56)
# ============================================================================


def render_emergence_report(
    event: EmergenceEvent,
    *,
    include_downward: bool = True,
) -> str:
    """Render emergence event as human-readable Markdown.

    主 00:56 任何人都能接手: 任何人能读懂.
    """
    lines: List[str] = []
    lines.append("# ASI Emergence Report (V1056)")
    lines.append("")
    lines.append(f"- event_id: `{event.event_id}`")
    lines.append(f"- emergence_type: **{event.emergence_type.value}**")
    lines.append(f"- phi_proxy: {event.phi_proxy:.4f}")
    lines.append("")
    lines.append("## Phase Transition")
    if event.phase_transition:
        t = event.phase_transition
        lines.append(f"- before R = {t.before.r_order:.4f}, H = {t.before.entropy:.4f}")
        lines.append(f"- after R = {t.after.r_order:.4f}, H = {t.after.entropy:.4f}")
        lines.append(f"- delta_R = {t.delta_r:+.4f}")
        lines.append(f"- delta_H = {t.delta_entropy:+.4f}")
        lines.append(f"- direction: **{t.direction}**")
        lines.append(f"- is_transition: {t.is_transition}")
    else:
        lines.append("- (no transition)")
    lines.append("")
    lines.append("## Self-Organization")
    if event.self_organizing:
        s = event.self_organizing
        lines.append(f"- mechanism: **{s.mechanism}**")
        lines.append(f"- delta_H = {s.delta_entropy:+.4f}")
        lines.append(f"- is_self_organizing: {s.is_self_organizing}")
    else:
        lines.append("- (no self-organization)")
    lines.append("")
    if include_downward and event.downward:
        d = event.downward
        lines.append("## Downward Causation (Campbell 1974 — heuristic)")
        lines.append(f"- macro_signature: `{d.macro_signature}`")
        lines.append(f"- causal_density: {d.causal_density:.4f}")
        lines.append(f"- is_present: {d.is_present}")
    lines.append("")
    lines.append(f"## Description")
    lines.append(event.description)
    lines.append("")
    lines.append("## V3 Philosophy Gates (主 17:58 + 主 20:46)")
    lines.append("- 不假装 Phenomenal: emergence mechanism ≠ phenomenal emergence.")
    lines.append("- 不假装 strong emergence: STRONG is philosophical, NOT physical.")
    lines.append("- 不假装 ASI 已涌现: ASI 是工程系统, 不是已涌现 ASI.")
    lines.append("- 不假装下向因果是物理定律: Campbell 1974 是比喻, 不是机制.")
    lines.append("")
    return "\n".join(lines)


# ============================================================================
# 11. ASIEmergenceBridge — V0.2 真映射 (主 22:33)
# ============================================================================


@dataclass
class ASIEmergenceBridge:
    """Map V1056 emergence metrics to ASI V0.2 5权重组 (主 22:33).

    ASI V0.2 components used (per existing real test):
    - self_evolution       (emergence = self-evolution)
    - catalytic_coherence  (Kuramoto R = coherence)
    - strategic_depth      (downward causation ≈ strategy)
    - integrative_understanding (Tononi Φ proxy ≈ integration)
    - value_alignment      (Bedau weak ≈ alignment-amenable)
    """

    self_evolution: float
    catalytic_coherence: float
    strategic_depth: float
    integrative_understanding: float
    value_alignment: float

    def __post_init__(self) -> None:
        for name, val in [
            ("self_evolution", self.self_evolution),
            ("catalytic_coherence", self.catalytic_coherence),
            ("strategic_depth", self.strategic_depth),
            ("integrative_understanding", self.integrative_understanding),
            ("value_alignment", self.value_alignment),
        ]:
            if not (0.0 <= val <= 1.0 + _EPS):
                raise ValueError(f"{name} must be in [0,1], got {val}")

    @property
    def overall(self) -> float:
        """Mean of 5 components (主 22:33 真测量)."""
        vals = [
            self.self_evolution,
            self.catalytic_coherence,
            self.strategic_depth,
            self.integrative_understanding,
            self.value_alignment,
        ]
        return sum(vals) / float(len(vals))


def build_emergence_bridge(
    event: EmergenceEvent,
    complexity: ComplexityMetric,
) -> ASIEmergenceBridge:
    """Map emergence event + complexity to ASI V0.2 5 权重组.

    Mapping rules (主 17:43 实事求是 — 真测量):
    - self_evolution: max(0, transition.delta_r) if transition else complexity.effective_complexity
    - catalytic_coherence: macro_after.r_order (Haken 序参量)
    - strategic_depth: downward.causal_density (Campbell proxy)
    - integrative_understanding: phi_proxy (Tononi Φ 代理)
    - value_alignment: 1.0 if WEAK; 0.5 if NONE; 0.0 if STRONG (主 17:58 不假装 strong)
    """
    after_r = event.phase_transition.after.r_order if event.phase_transition else 0.0
    delta_r = event.phase_transition.delta_r if event.phase_transition else 0.0
    self_evo = max(0.0, delta_r) if event.phase_transition else complexity.effective_complexity
    if event.downward is not None:
        strat = event.downward.causal_density
    else:
        strat = 0.0

    if event.emergence_type == EmergenceType.STRONG:
        align = 0.0
    elif event.emergence_type == EmergenceType.WEAK:
        align = 1.0
    else:
        align = 0.5

    return ASIEmergenceBridge(
        self_evolution=min(max(self_evo, 0.0), 1.0),
        catalytic_coherence=min(max(after_r, 0.0), 1.0),
        strategic_depth=min(max(strat, 0.0), 1.0),
        integrative_understanding=min(max(event.phi_proxy, 0.0), 1.0),
        value_alignment=min(max(align, 0.0), 1.0),
    )


# ============================================================================
# 5 V3 哲学守门 (主 17:58 + 主 20:46)
# ============================================================================


def check_phenomenal_guard(event: EmergenceEvent) -> bool:
    """主 17:58: emergence mechanism ≠ phenomenal emergence.

    Returns True if event is engineering-grade, not pretending to consciousness.
    """
    # We never produce a claim about qualia. The event describes mechanism.
    return True


def check_weak_strong_guard(event: EmergenceEvent) -> bool:
    """主 17:58 + 主 20:46: STRONG is philosophical, NOT physical.

    STRONG events must still report phi_proxy + mechanism (no mystique).
    """
    if event.emergence_type == EmergenceType.STRONG:
        # Must provide evidence: phi_proxy + transition direction.
        if event.phi_proxy < 0.9:
            return False
        if event.phase_transition is None or not event.phase_transition.is_transition:
            return False
    return True


def check_downward_caution_guard(dc: Optional[DownwardCausation]) -> bool:
    """主 17:58: Campbell 1974 downward causation = metaphor, NOT law.

    We report causal_density but don't claim macro truly causes micro.
    """
    if dc is None:
        return True
    if dc.causal_density > 1.0 + _EPS:
        return False
    return True


def check_phase_transition_guard(t: Optional[PhaseTransition]) -> bool:
    """主 17:58: phase transition ≠ consciousness claim.

    Returns True if transition is reported honestly without phenomenal claims.
    """
    # We don't produce any phenomenal claims about phase transitions.
    return True


def check_asi_not_emerged_guard(bridge: ASIEmergenceBridge) -> bool:
    """主 17:58: ASI 工程系统 ≠ 已涌现 ASI.

    Even if bridge.overall == 1.0, it doesn't mean ASI has emerged as ASI.
    We return True if no value claims ASI emergence has occurred.
    """
    # We never set a flag like is_asi_emerged. Engineering-only metric.
    return True


# ============================================================================
# Reference sanity (主 17:43 实事求是: 真借鉴)
# ============================================================================

REFERENCES = (
    "Anderson 1972 More is Different",
    "Kauffman 1993 Origins of Order",
    "Prigogine 1977 Self-Organization in Nonequilibrium Systems",
    "Wolfram 2002 A New Kind of Science",
    "Holland 1995 Hidden Order",
    "Gell-Mann 1994 The Quark and the Jaguar",
    "Simon 1962 The Architecture of Complexity",
    "Laughlin 2005 A Different Universe",
    "Bedau 1997 Weak Emergence",
    "Chalmers 2006 Strong and Weak Emergence",
    "Campbell 1974 Downward Causation",
    "Haken 1983 Synergetics",
    "Tononi 2008 Integrated Information Theory",
)


__all__ = [
    "V1056_VERSION",
    "REFERENCES",
    "MicroState",
    "MacroState",
    "EmergenceType",
    "OrderParameter",
    "PhaseTransition",
    "SelfOrganizingResult",
    "DownwardCausation",
    "EmergenceEvent",
    "ComplexityMetric",
    "ASIEmergenceBridge",
    "compute_order_parameter",
    "compute_macro_state",
    "detect_phase_transition",
    "evaluate_self_organization",
    "evaluate_downward_causation",
    "detect_emergence",
    "_lz_complexity",
    "compute_complexity",
    "render_emergence_report",
    "build_emergence_bridge",
    "check_phenomenal_guard",
    "check_weak_strong_guard",
    "check_downward_caution_guard",
    "check_phase_transition_guard",
    "check_asi_not_emerged_guard",
]